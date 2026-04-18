//! VST3 plugin for the Integral Integra-7 control surface.
//!
//! This crate integrates with nih-plug to expose the Integral core
//! engine as a VST3/CLAP plugin for DAW integration. It provides
//! MIDI SysEx I/O, with the Integra-7 state machine handled by
//! [`integral_core::device::DeviceState`].

use integral_core::device::DeviceState;
use integral_core::sysex;
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::{Arc, Mutex};

mod editor;

/// Default SysEx device ID for the Integra-7 (0x10 = device 17).
const DEFAULT_DEVICE_ID: u8 = 0x10;

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
        // Accept any SysEx message (F0 ... F7).
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

/// Shared state accessible from both the process thread and the editor.
pub struct SharedState {
    /// The Integra-7 device state machine.
    pub device: Mutex<DeviceState>,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// The main plugin struct for the Integral control surface.
struct Integral {
    params: Arc<IntegralParams>,
    /// Shared between process() and the editor GUI.
    shared: Arc<SharedState>,
}

/// Plugin parameters (currently empty — Integral uses SysEx, not DAW automation).
#[derive(Params)]
struct IntegralParams {
    /// Persisted editor window state (size, position).
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
}

impl Default for Integral {
    fn default() -> Self {
        Self {
            params: Arc::new(IntegralParams {
                editor_state: editor::default_state(),
            }),
            shared: Arc::new(SharedState {
                device: Mutex::new(DeviceState::new(DEFAULT_DEVICE_ID)),
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
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[];
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;
    type SysExMessage = RawSysEx;
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.shared.clone(), self.params.editor_state.clone())
    }

    fn process(
        &mut self,
        _buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Use a monotonic-ish timestamp for throttle/echo suppression.
        // In a real host this comes from the transport; we use sample position.
        let now_ms = context.transport().pos_seconds().unwrap_or(0.0) * 1000.0;

        // Process incoming MIDI events.
        while let Some(event) = context.next_event() {
            match event {
                NoteEvent::MidiSysEx { timing: _, message } => {
                    // Parse as DT1 and feed to DeviceState.
                    if let Ok(dt1) = sysex::parse_dt1(&message.data) {
                        let mut dev = self.shared.device.lock().unwrap();
                        dev.handle_dt1(&dt1.address, &dt1.data, now_ms);
                    }
                }
                other => {
                    // Pass through non-SysEx MIDI events.
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
    const CLAP_FEATURES: &'static [ClapFeature] =
        &[ClapFeature::Utility, ClapFeature::NoteEffect];
}

impl Vst3Plugin for Integral {
    const VST3_CLASS_ID: [u8; 16] = *b"IntgrI7CtrlSurf\0";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Tools];
}

nih_export_clap!(Integral);
nih_export_vst3!(Integral);
