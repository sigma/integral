//! The plugin family (ADR 0007): the Studio Set plugin, a Tone plugin per
//! synth tone type and a Kit plugin per drum kit type, all one generic
//! [`Family`] over a [`Kind`].
//!
//! - **Parameters** are the kind's host list as `IntParam`s over the raw
//!   range ([`params`]).
//! - **The Binding is also host parameters** ([`binding_params`]): a Part, and a
//!   key on a Kit plugin, that the host can set and automate. A host change
//!   rebinds the instance as the editor's bind does, and the Binding sets the
//!   parameters back as a gesture. When a project is restored the saved
//!   Binding stands and the parameters follow it.
//! - **Audio** passes through; the plugins neither receive nor send host
//!   MIDI. Notes go to the service's note port.
//! - **The Binding** is saved with the host's state ([`binding`]). A new
//!   instance claims a free target from the Device service (`Request::Claim`,
//!   atomic), but only once it has settled: the host restores state after
//!   creating the instance (nih-plug even builds the editor then) and before
//!   processing audio or opening the editor window, and a restore replaces any
//!   target taken before it. See [`Instance::settle`].
//! - **Host parameters follow the device.** The audio thread writes the
//!   host's changes ([`out`]); the worker gives the host the device's
//!   ([`follow`], [`host`]), posted to the host's main thread.
//! - **The project's state is recalled** ([`recall`]): an instance the host
//!   restored pushes its state to the device once per device session, and
//!   saves the tone selection ([`tone`]) beside its parameters for it.
//! - **The connection** to the service reports one [`Status`] ([`link`]).
//!   It starts in `initialize()`, so a host that only scans the plugin
//!   connects nothing.

pub mod bank;
pub mod binding;
pub mod binding_params;
pub mod follow;
pub mod host;
pub mod instance;
pub mod kind;
#[cfg(unix)]
pub mod link;
pub mod notice;
#[cfg(unix)]
pub mod out;
pub mod params;
pub mod recall;
pub mod status;
pub mod tone;

use std::num::NonZeroU32;
use std::sync::Arc;

use nih_plug::prelude::*;

pub use host::HostTask;
pub use instance::{DefaultToneRefused, Instance, ToneFit};
pub use kind::{Kind, PcmDrum, PcmSynth, SnAcoustic, SnDrum, SnSynth, StudioSet};
pub use notice::Notice;
pub use status::Status;

#[cfg(unix)]
use link::Connection;
#[cfg(unix)]
use out::Outbound;
use params::FamilyParams;

/// The Studio Set plugin.
pub type StudioSetPlugin = Family<StudioSet>;
/// The SN Synth Tone plugin.
pub type SnSynthPlugin = Family<SnSynth>;
/// The SN Acoustic Tone plugin.
pub type SnAcousticPlugin = Family<SnAcoustic>;
/// The PCM Synth Tone plugin.
pub type PcmSynthPlugin = Family<PcmSynth>;
/// The SN Drum Kit plugin.
pub type SnDrumPlugin = Family<SnDrum>;
/// The PCM Drum Kit plugin.
pub type PcmDrumPlugin = Family<PcmDrum>;

/// A plugin of the family, `K` saying which.
pub struct Family<K: Kind> {
    params: Arc<FamilyParams>,
    /// The connection to the Device service, once `initialize()` has made it.
    #[cfg(unix)]
    connection: Option<Connection>,
    /// The audio thread's way to send host changes to the device.
    #[cfg(unix)]
    outbound: Option<Outbound>,
    kind: std::marker::PhantomData<K>,
}

impl<K: Kind> Family<K> {
    /// What the instance reports about its connection.
    pub fn status(&self) -> Status {
        self.params.instance.status.get()
    }

    /// The instance's shared state: Binding, status.
    pub fn instance(&self) -> &Arc<Instance> {
        &self.params.instance
    }
}

impl<K: Kind> Family<K> {
    /// The editor. It must not settle the instance: nih-plug calls
    /// `Plugin::editor()` while it builds the wrapper, before the host's
    /// `setState`. The window settles it when it opens.
    #[cfg(target_os = "macos")]
    fn make_editor(&self) -> Option<Box<dyn Editor>> {
        use crate::webview::link::DeviceLink;
        use crate::webview::service_link::ServiceLink;
        let instance = Arc::clone(&self.params.instance);
        let link: crate::webview::editor::LinkFactory = Arc::new(move || {
            Box::new(ServiceLink::new(Arc::clone(&instance))) as Box<dyn DeviceLink>
        });
        Some(Box::new(crate::webview::editor::WebviewEditor::new(
            self.params.editor_size.clone(),
            link,
            Arc::clone(&self.params),
        )))
    }

    /// The webview editor is macOS-only so far (see [`crate::webview`]).
    #[cfg(not(target_os = "macos"))]
    fn make_editor(&self) -> Option<Box<dyn Editor>> {
        None
    }
}

impl<K: Kind> Default for Family<K> {
    fn default() -> Self {
        Self {
            params: Arc::new(FamilyParams::new(K::KIND)),
            #[cfg(unix)]
            connection: None,
            #[cfg(unix)]
            outbound: None,
            kind: std::marker::PhantomData,
        }
    }
}

impl<K: Kind> Plugin for Family<K> {
    const NAME: &'static str = K::NAME;
    const VENDOR: &'static str = "sigma";
    const URL: &'static str = "https://github.com/sigma/integral";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    // Stereo in and out: the plugins sit in an audio chain (after External
    // Instrument, or on the master track) and pass it through untouched. The
    // standalone backend needs a layout to initialize on macOS.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    // No host MIDI (the defaults): notes go to the service's note port.
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;
    // No host SysEx: the device is reached through the service.
    type SysExMessage = ();
    // What the instance's worker asks of the host's main thread.
    type BackgroundTask = HostTask;

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn task_executor(&mut self) -> TaskExecutor<Self> {
        // Runs on the host's main thread (`ProcessContext::execute_gui`).
        let instance = Arc::clone(&self.params.instance);
        Box::new(move |task| host::run(&instance, task))
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        self.make_editor()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        // The context the device's changes reach the host through, and the
        // executor that takes them to its main thread, with or without an
        // editor and whether or not the host calls `process()` (the vendored
        // nih-plug). Set once: restoring a state for a bulk update initializes
        // again, on the audio thread, and must not replace them.
        let instance = &self.params.instance;
        instance.set_posting(true);
        if instance.bank.edits().is_none()
            && let Some(gui) = context.gui_context()
        {
            instance
                .bank
                .set_edits_once(Arc::new(host::GuiEdits::new(gui)));
        }
        if !instance.has_poster()
            && let Some(executor) = context.async_executor()
        {
            instance.set_poster(instance::Poster::new(move |task| {
                executor.execute_gui(task)
            }));
        }
        // A host may initialize more than once; keep the one connection. A
        // socket path that cannot be made leaves the status at "not
        // running" and is tried again at the next initialize.
        #[cfg(unix)]
        if self.connection.is_none() {
            match integral_device_service::default_socket_path() {
                Ok(path) => {
                    let connection = Connection::start(path, Arc::clone(&self.params.instance));
                    // Made here, off the audio thread: `process()` only
                    // pushes to its ring.
                    self.outbound = connection.sender().map(Outbound::new);
                    self.connection = Some(connection);
                }
                Err(error) => nih_log!("no Device service socket path: {error}"),
            }
        }
        true
    }

    fn deactivate(&mut self) {
        // The host may destroy the plugin next: the worker posts nothing more
        // to the main thread until `initialize()`.
        self.params.instance.set_posting(false);
    }

    fn process(
        &mut self,
        _buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Audio passes through; the host restores state before it gets here.
        self.params.instance.settle();
        // The host's changes go to the device.
        #[cfg(unix)]
        if let Some(outbound) = &mut self.outbound {
            outbound.poll(&self.params.instance);
        }
        // The poll above is the only path for the host's changes to reach the
        // device, and the input is usually silent: a host that suspends a
        // silent plugin would stall them.
        ProcessStatus::KeepAlive
    }
}

impl<K: Kind> Vst3Plugin for Family<K> {
    const VST3_CLASS_ID: [u8; 16] = K::CLASS_ID;
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

#[cfg(test)]
mod tests {
    use super::*;
    use nih_plug::wrapper::vst3::Wrapper;

    #[test]
    fn building_the_editor_does_not_settle_the_instance() {
        let plugin = PcmSynthPlugin::default();
        let editor = plugin.make_editor();
        // macOS builds one, other targets none; either way the instance
        // waits for the host's state.
        assert_eq!(editor.is_some(), cfg!(target_os = "macos"));
        assert!(!plugin.instance().is_settled());
    }

    #[test]
    fn a_wrapper_builds_the_plugin_and_its_editor_without_a_connection() {
        // nih-plug calls `Plugin::editor()` here (see `make_editor`); a
        // wrapper that connected or adopted a target at this point would do
        // so before any state is restored.
        let wrapper = Wrapper::<PcmSynthPlugin>::new();
        drop(wrapper);
    }

    #[test]
    fn the_plugins_are_effects_with_audio_and_no_host_midi() {
        assert_eq!(<PcmSynthPlugin as Plugin>::MIDI_INPUT, MidiConfig::None);
        assert_eq!(<PcmSynthPlugin as Plugin>::MIDI_OUTPUT, MidiConfig::None);
        assert_eq!(
            <PcmSynthPlugin as Vst3Plugin>::VST3_SUBCATEGORIES,
            &[Vst3SubCategory::Fx, Vst3SubCategory::Tools]
        );
    }
}
