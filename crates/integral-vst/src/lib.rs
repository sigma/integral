//! VST3 plugin for the Integral Integra-7 control surface.
//!
//! This crate integrates with nih-plug to expose Integral as a VST3/CLAP
//! plugin. Its editor runs the web UI in a webview ([`webview`], ADR 0004),
//! and the page reaches the device through the plugin transport: today a
//! direct MIDI link the plugin opens itself, since hosts drop plugin SysEx.

use nih_plug::prelude::*;
use std::num::NonZeroU32;
use std::sync::{Arc, RwLock};

// Only the macOS editor uses the transport so far; elsewhere it is built
// and tested, but unused.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod webview;

/// The editor's size before the page asks for its own, logical px.
const DEFAULT_EDITOR_SIZE: (u32, u32) = (1280, 820);

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// The main plugin struct for the Integral control surface.
pub struct Integral {
    params: Arc<IntegralParams>,
}

/// Plugin state the host persists. Integral exposes no DAW-automatable
/// parameters: the page edits the device directly.
#[derive(Params)]
struct IntegralParams {
    /// The editor window's size in logical px, as the page last set it.
    #[persist = "editor-size"]
    editor_size: Arc<RwLock<(u32, u32)>>,
}

impl Default for Integral {
    fn default() -> Self {
        Self {
            params: Arc::new(IntegralParams {
                editor_size: Arc::new(RwLock::new(DEFAULT_EDITOR_SIZE)),
            }),
        }
    }
}

impl Plugin for Integral {
    const NAME: &'static str = "Integral";
    const VENDOR: &'static str = "sigma";
    const URL: &'static str = "https://github.com/sigma/integral";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    // A minimal stereo layout is needed for the standalone backend to initialize
    // correctly on macOS. The plugin doesn't process audio — it just passes through.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;
    // No host SysEx: the page reaches the device over its own link.
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    #[cfg(target_os = "macos")]
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        use webview::link::{DeviceLink, MidirLink};
        let link: webview::editor::LinkFactory =
            Arc::new(|| Box::new(MidirLink::default()) as Box<dyn DeviceLink>);
        Some(Box::new(webview::editor::WebviewEditor::new(
            self.params.editor_size.clone(),
            link,
        )))
    }

    /// The webview editor is macOS-only so far (see [`webview`]).
    #[cfg(not(target_os = "macos"))]
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        None
    }

    fn process(
        &mut self,
        _buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Pass MIDI through.
        while let Some(event) = context.next_event() {
            context.send_event(event);
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Integral {
    const CLAP_ID: &'static str = "com.sigma.integral";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Integra-7 Control Surface");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Utility, ClapFeature::NoteEffect];
}

impl Vst3Plugin for Integral {
    const VST3_CLASS_ID: [u8; 16] = *b"IntgrI7CtrlSurf\0";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Tools];
}

nih_export_clap!(Integral);
nih_export_vst3!(Integral);
