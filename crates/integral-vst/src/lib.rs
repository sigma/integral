//! VST3 plugin for the Integral Integra-7 control surface.
//!
//! This crate integrates with nih-plug to expose Integral as a VST3/CLAP
//! plugin. Its editor runs the web UI in a webview ([`webview`], ADR 0004),
//! and the page reaches the device through the plugin transport: today a
//! direct MIDI link the plugin opens itself, since hosts drop plugin SysEx.

use integral_core::device::DeviceState;
use integral_core::sysex;
use nih_plug::prelude::*;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

pub mod views;
// Only the macOS editor uses the transport so far; elsewhere it is built
// and tested, but unused.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod webview;

/// Default SysEx device ID for the Integra-7 (0x10 = device 17).
const DEFAULT_DEVICE_ID: u8 = 0x10;

/// The editor's size before the page asks for its own, logical px.
const DEFAULT_EDITOR_SIZE: (u32, u32) = (1280, 820);

/// Maximum SysEx message size (256 bytes per INTEGRA-7 spec).
const MAX_SYSEX_SIZE: usize = 256;

// ---------------------------------------------------------------------------
// SysEx message wrapper for nih-plug
// ---------------------------------------------------------------------------

/// Wrapper for raw SysEx messages passed through nih-plug's typed SysEx system.
#[derive(Debug, Clone, PartialEq)]
pub struct RawSysEx {
    data: Vec<u8>,
}

impl SysExMessage for RawSysEx {
    type Buffer = [u8; MAX_SYSEX_SIZE];

    fn from_buffer(buffer: &[u8]) -> Option<Self> {
        if buffer.len() >= 2 && buffer[0] == 0xF0 && buffer[buffer.len() - 1] == 0xF7 {
            Some(RawSysEx {
                data: buffer.to_vec(),
            })
        } else {
            None
        }
    }

    fn to_buffer(self) -> (Self::Buffer, usize) {
        let len = self.data.len().min(MAX_SYSEX_SIZE);
        let mut buf = [0u8; MAX_SYSEX_SIZE];
        buf[..len].copy_from_slice(&self.data[..len]);
        (buf, len)
    }
}

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

/// Shared state accessible from both the process thread and the vizia
/// views. The webview editor's page keeps its own device state; this goes
/// with the views in #367.
pub struct SharedState {
    /// The Integra-7 device state machine.
    pub device: Mutex<DeviceState>,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// The main plugin struct for the Integral control surface.
pub struct Integral {
    params: Arc<IntegralParams>,
    /// Shared between process() and the editor GUI.
    shared: Arc<SharedState>,
    /// Origin of the `now_ms` clock passed to [`DeviceState`].
    started: Instant,
}

/// Plugin parameters (currently empty — Integral uses SysEx, not DAW automation).
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
            shared: Arc::new(SharedState {
                device: Mutex::new(DeviceState::new(DEFAULT_DEVICE_ID)),
            }),
            started: Instant::now(),
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
    type SysExMessage = RawSysEx;
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
        // Not the DAW transport: it jumps back on loop or seek and stalls
        // while stopped, which would break send spacing and retry timeouts.
        let now_ms = self.started.elapsed().as_secs_f64() * 1000.0;

        // Process incoming MIDI events.
        while let Some(event) = context.next_event() {
            match event {
                NoteEvent::MidiSysEx { timing: _, message } => {
                    if let Ok(dt1) = sysex::parse_dt1(&message.data) {
                        let mut dev = self.shared.device.lock().unwrap();
                        dev.handle_dt1(&dt1.address, &dt1.data, now_ms);
                    }
                }
                other => {
                    context.send_event(other);
                }
            }
        }

        // Drain outgoing SysEx from the DeviceState queue.
        {
            let mut dev = self.shared.device.lock().unwrap();
            while let Some(msg) = dev.drain(now_ms) {
                context.send_event(NoteEvent::MidiSysEx {
                    timing: 0,
                    message: RawSysEx { data: msg },
                });
            }
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
