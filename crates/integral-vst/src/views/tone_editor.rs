//! Tone editor page — dispatches to per-tone-type sub-editors.
//!
//! Mirrors the `ToneEditorPage` React component from `web/src/ToneEditorPage.tsx`.
//! Reads the selected part's bank MSB to determine the tone type and renders
//! the appropriate sub-editor.  Currently only the SN-S (SuperNATURAL Synth)
//! editor is implemented; other tone types show stub placeholders.

use std::sync::Arc;
use std::time::Duration;

use integral_core::sn_synth::{SnSynthCommon, SnSynthPartial};
use nih_plug_vizia::vizia::prelude::*;

use crate::editor::{EditorEvent, PageTab};
use crate::SharedState;

use super::{SectionPanel, SynthKnob, SynthKnobExt, SynthSwitch};

/// Refresh interval for reading device state (milliseconds).
const REFRESH_INTERVAL_MS: u64 = 100;

/// Number of SN-S partials.
const NUM_SNS_PARTIALS: usize = 3;

// ---------------------------------------------------------------------------
// SN-S parameter display names
// ---------------------------------------------------------------------------
// TODO: migrate these to `DeviceSpec` once tone editor param metadata is added.

/// SN-S oscillator wave type names (indexed by OSC Wave value 0-7).
const SNS_OSC_WAVE_NAMES: &[&str] = &["SAW", "SQR", "PWSQ", "TRI", "SIN", "NSE", "SSAW", "PCM"];

/// SN-S filter mode names (indexed by Filter Mode value 0-7).
const SNS_FILTER_MODE_NAMES: &[&str] = &["BYP", "LPF", "HPF", "BPF", "PKG", "LP2", "LP3", "LP4"];

// ---------------------------------------------------------------------------
// Tone type constants (bank MSB values)
// ---------------------------------------------------------------------------

/// Bank MSB for PCM Synth tones.
const MSB_PCM_SYNTH: u8 = 87;
/// Bank MSB for SN Acoustic tones.
const MSB_SN_ACOUSTIC: u8 = 89;
/// Bank MSB for SN Synth tones.
const MSB_SN_SYNTH: u8 = 95;
/// Bank MSB for PCM Drum kits.
const MSB_PCM_DRUM: u8 = 86;
/// Bank MSB for SN Drum kits.
const MSB_SN_DRUM: u8 = 88;

/// Return a human-readable tone type from the bank MSB.
fn tone_type_label(bank_msb: u8) -> &'static str {
    match bank_msb {
        MSB_PCM_SYNTH => "PCM Synth",
        MSB_SN_ACOUSTIC => "SN Acoustic",
        MSB_SN_SYNTH => "SN Synth",
        MSB_PCM_DRUM => "PCM Drum",
        MSB_SN_DRUM => "SN Drum",
        93 => "ExPCM",
        121 => "GM2",
        _ => "Unknown",
    }
}

// ---------------------------------------------------------------------------
// View-local data model
// ---------------------------------------------------------------------------

/// Model driving the tone editor page.
#[derive(Lens)]
pub struct ToneEditorData {
    /// Currently selected part index (0-15).
    pub selected_part: usize,
    /// Bank MSB of the selected part's tone.
    pub tone_msb: u8,
    /// Tone name of the selected part.
    pub tone_name: String,
    /// Header info text: "Part N : ToneType : ToneName".
    pub header_info: String,

    // -- SN-S state ---------------------------------------------------------
    /// SN-S Common parameters (loaded from device).
    pub sns_common: SnSynthCommonView,
    /// SN-S Partial parameters (3 partials).
    pub sns_partials: [SnSynthPartialView; NUM_SNS_PARTIALS],
    /// Whether SN-S data has been loaded at least once.
    pub sns_loaded: bool,
    /// Whether an RQ1 load has been queued for the current part.
    pub sns_load_queued: bool,

    /// Shared state handle.
    #[lens(ignore)]
    shared: Arc<SharedState>,
}

/// Vizia-compatible snapshot of SN-S Common parameters.
///
/// We store values as normalized `f32` (0.0–1.0) for knobs and raw `usize`
/// for switches, matching the control APIs.
#[derive(Debug, Clone, Data, Lens)]
pub struct SnSynthCommonView {
    /// Tone name.
    pub tone_name: String,
    /// Tone Level (0–127).
    pub tone_level: f32,
    /// Portamento Switch (0/1).
    pub portamento_switch: usize,
    /// Portamento Time (0–127).
    pub portamento_time: f32,
    /// Mono Switch (0/1).
    pub mono_switch: usize,
    /// Octave Shift (raw 61–67 mapped to 0.0–1.0).
    pub octave_shift: f32,
    /// Pitch Bend Range Up (0–24).
    pub bend_range_up: f32,
    /// Pitch Bend Range Down (0–24).
    pub bend_range_down: f32,
    /// Partial switches (0/1) for partials 1–3.
    pub partial1_switch: usize,
    /// Partial 2 switch.
    pub partial2_switch: usize,
    /// Partial 3 switch.
    pub partial3_switch: usize,
    /// Ring Switch (0=OFF, 2=ON → mapped to 0/1).
    pub ring_switch: usize,
    /// Unison Switch (0/1).
    pub unison_switch: usize,
    /// Unison Size (0–3).
    pub unison_size: f32,
    /// Portamento Mode (0=NORMAL, 1=LEGATO).
    pub portamento_mode: usize,
    /// Legato Switch (0/1).
    pub legato_switch: usize,
    /// Analog Feel (0–127).
    pub analog_feel: f32,
    /// Wave Shape (0–127).
    pub wave_shape: f32,
}

impl Default for SnSynthCommonView {
    fn default() -> Self {
        Self {
            tone_name: String::new(),
            tone_level: 1.0,
            portamento_switch: 0,
            portamento_time: 0.0,
            mono_switch: 0,
            octave_shift: 0.5,
            bend_range_up: 2.0 / 24.0,
            bend_range_down: 2.0 / 24.0,
            partial1_switch: 1,
            partial2_switch: 0,
            partial3_switch: 0,
            ring_switch: 0,
            unison_switch: 0,
            unison_size: 0.0,
            portamento_mode: 0,
            legato_switch: 0,
            analog_feel: 0.0,
            wave_shape: 0.0,
        }
    }
}

impl SnSynthCommonView {
    /// Populate from a parsed [`SnSynthCommon`].
    #[allow(dead_code)]
    fn from_core(c: &SnSynthCommon) -> Self {
        Self {
            tone_name: c.tone_name.clone(),
            tone_level: c.tone_level as f32 / 127.0,
            portamento_switch: c.portamento_switch as usize,
            portamento_time: c.portamento_time as f32 / 127.0,
            mono_switch: c.mono_switch as usize,
            octave_shift: (c.octave_shift as f32 - 61.0) / 6.0,
            bend_range_up: c.pitch_bend_range_up as f32 / 24.0,
            bend_range_down: c.pitch_bend_range_down as f32 / 24.0,
            partial1_switch: c.partial1_switch as usize,
            partial2_switch: c.partial2_switch as usize,
            partial3_switch: c.partial3_switch as usize,
            ring_switch: if c.ring_switch >= 1 { 1 } else { 0 },
            unison_switch: c.unison_switch as usize,
            unison_size: c.unison_size as f32 / 3.0,
            portamento_mode: c.portamento_mode as usize,
            legato_switch: c.legato_switch as usize,
            analog_feel: c.analog_feel as f32 / 127.0,
            wave_shape: c.wave_shape as f32 / 127.0,
        }
    }
}

/// Vizia-compatible snapshot of a single SN-S Partial.
#[derive(Debug, Clone, Data, Lens)]
pub struct SnSynthPartialView {
    /// OSC Wave (0–7).
    pub osc_wave: usize,
    /// Filter Cutoff (0–127 → normalized).
    pub filter_cutoff: f32,
    /// Filter Resonance (0–127 → normalized).
    pub filter_resonance: f32,
    /// Amp Level (0–127 → normalized).
    pub amp_level: f32,
    /// Amp Pan (0–127 → normalized).
    pub amp_pan: f32,
    /// OSC Pitch (40–88 → normalized).
    pub osc_pitch: f32,
    /// OSC Detune (14–114 → normalized).
    pub osc_detune: f32,
    /// Filter Mode (0–7).
    pub filter_mode: usize,
    /// AMP Env Attack (0–127 → normalized).
    pub amp_env_attack: f32,
    /// AMP Env Decay (0–127 → normalized).
    pub amp_env_decay: f32,
    /// AMP Env Sustain (0–127 → normalized).
    pub amp_env_sustain: f32,
    /// AMP Env Release (0–127 → normalized).
    pub amp_env_release: f32,
    /// Filter Env Attack (0–127 → normalized).
    pub filter_env_attack: f32,
    /// Filter Env Decay (0–127 → normalized).
    pub filter_env_decay: f32,
    /// Filter Env Sustain (0–127 → normalized).
    pub filter_env_sustain: f32,
    /// Filter Env Release (0–127 → normalized).
    pub filter_env_release: f32,
    /// Filter Env Depth (1–127 → normalized).
    pub filter_env_depth: f32,
    /// Amp Velocity Sensitivity (1–127 → normalized).
    pub amp_vel_sens: f32,
}

impl Default for SnSynthPartialView {
    fn default() -> Self {
        Self {
            osc_wave: 0,
            filter_cutoff: 1.0,
            filter_resonance: 0.0,
            amp_level: 1.0,
            amp_pan: 0.5,
            osc_pitch: 0.5,
            osc_detune: 0.5,
            filter_mode: 1,
            amp_env_attack: 0.0,
            amp_env_decay: 0.0,
            amp_env_sustain: 1.0,
            amp_env_release: 0.0,
            filter_env_attack: 0.0,
            filter_env_decay: 0.0,
            filter_env_sustain: 0.0,
            filter_env_release: 0.0,
            filter_env_depth: 0.5,
            amp_vel_sens: 0.5,
        }
    }
}

impl SnSynthPartialView {
    /// Populate from a parsed [`SnSynthPartial`].
    #[allow(dead_code)]
    fn from_core(p: &SnSynthPartial) -> Self {
        Self {
            osc_wave: p.osc_wave as usize,
            filter_cutoff: p.filter_cutoff as f32 / 127.0,
            filter_resonance: p.filter_resonance as f32 / 127.0,
            amp_level: p.amp_level as f32 / 127.0,
            amp_pan: p.amp_pan as f32 / 127.0,
            osc_pitch: (p.osc_pitch as f32 - 40.0) / 48.0,
            osc_detune: (p.osc_detune as f32 - 14.0) / 100.0,
            filter_mode: p.filter_mode as usize,
            amp_env_attack: p.amp_env_attack as f32 / 127.0,
            amp_env_decay: p.amp_env_decay as f32 / 127.0,
            amp_env_sustain: p.amp_env_sustain as f32 / 127.0,
            amp_env_release: p.amp_env_release as f32 / 127.0,
            filter_env_attack: p.filter_env_attack as f32 / 127.0,
            filter_env_decay: p.filter_env_decay as f32 / 127.0,
            filter_env_sustain: p.filter_env_sustain as f32 / 127.0,
            filter_env_release: p.filter_env_release as f32 / 127.0,
            filter_env_depth: (p.filter_env_depth as f32 - 1.0) / 126.0,
            amp_vel_sens: (p.amp_vel_sens as f32 - 1.0) / 126.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Events handled by [`ToneEditorData`].
enum ToneEditorEvent {
    /// Periodic refresh from device state.
    Refresh,
    /// Set an SN-S Common parameter (offset, raw MIDI value).
    SetSnsCommon(u8, u8),
    /// Set an SN-S Partial parameter (partial index, offset, raw MIDI value).
    SetSnsPartial(u8, u8, u8),
}

impl Model for ToneEditorData {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|e, _| match e {
            ToneEditorEvent::Refresh => {
                self.refresh_from_device();
            }
            ToneEditorEvent::SetSnsCommon(offset, value) => {
                let Ok(mut dev) = self.shared.device.lock() else {
                    return;
                };
                dev.set_sns_common_param(self.selected_part as u8, *offset, *value);
            }
            ToneEditorEvent::SetSnsPartial(partial, offset, value) => {
                let Ok(mut dev) = self.shared.device.lock() else {
                    return;
                };
                dev.set_sns_partial_param(
                    self.selected_part as u8,
                    *partial,
                    *offset,
                    *value,
                );
            }
        });
    }
}

impl ToneEditorData {
    /// Create a new model seeded from the shared device state.
    fn new(shared: Arc<SharedState>) -> Self {
        let mut data = Self {
            selected_part: 0,
            tone_msb: 0,
            tone_name: String::new(),
            header_info: String::new(),
            sns_common: SnSynthCommonView::default(),
            sns_partials: std::array::from_fn(|_| SnSynthPartialView::default()),
            sns_loaded: false,
            sns_load_queued: false,
            shared,
        };
        data.refresh_from_device();
        data
    }

    /// Read current state from the device and update the view snapshot.
    fn refresh_from_device(&mut self) {
        let Ok(dev) = self.shared.device.lock() else {
            return;
        };
        let mixer = dev.state();
        let idx = self.selected_part;
        let part = &mixer.parts[idx];

        self.tone_msb = part.tone_bank_msb;
        self.tone_name = part.tone_name.clone();

        let type_label = tone_type_label(part.tone_bank_msb);
        self.header_info = if part.tone_name.is_empty() {
            format!("Part {} : {}", idx + 1, type_label)
        } else {
            format!("Part {} : {} : {}", idx + 1, type_label, part.tone_name)
        };

        // Queue SN-S data requests if this is an SN-S tone and we haven't loaded yet.
        if part.tone_bank_msb == MSB_SN_SYNTH && !self.sns_load_queued {
            self.sns_load_queued = true;
            // We'll queue the requests after dropping the lock.
        }

        drop(dev);

        // Queue RQ1 requests for SN-S data if needed.
        if self.tone_msb == MSB_SN_SYNTH && !self.sns_loaded {
            self.queue_sns_requests();
        }
    }

    /// Queue RQ1 requests to load SN-S Common + 3 Partials from the device.
    fn queue_sns_requests(&mut self) {
        let Ok(mut dev) = self.shared.device.lock() else {
            return;
        };
        let part = self.selected_part as u8;

        // Common
        let rq = dev.build_sns_common_request(part);
        dev.send_raw(&format!("sns-common-{part}"), rq);

        // Partials
        for i in 0..NUM_SNS_PARTIALS as u8 {
            let rq = dev.build_sns_partial_request(part, i);
            dev.send_raw(&format!("sns-partial-{part}-{i}"), rq);
        }

        // MFX
        let rq = dev.build_mfx_request(part);
        dev.send_raw(&format!("mfx-{part}"), rq);
    }
}

// ---------------------------------------------------------------------------
// ToneEditorPage view
// ---------------------------------------------------------------------------

/// The tone editor page view.
///
/// Reads the selected part's bank MSB and dispatches to the appropriate
/// sub-editor.  Currently only the SN-S editor is fully implemented.
pub struct ToneEditorPage;

impl ToneEditorPage {
    /// Creates a new [`ToneEditorPage`] view.
    pub fn new(cx: &mut Context, shared: Arc<SharedState>) -> Handle<'_, Self> {
        Self.build(cx, move |cx| {
            ToneEditorData::new(shared).build(cx);

            // Periodic refresh timer.
            let timer = cx.add_timer(
                Duration::from_millis(REFRESH_INTERVAL_MS),
                None,
                |cx, action| {
                    if let TimerAction::Tick(_) = action {
                        cx.emit(ToneEditorEvent::Refresh);
                    }
                },
            );
            cx.start_timer(timer);

            // --- Header ---
            HStack::new(cx, |cx| {
                // Back button
                Label::new(cx, "< Back to Mixer")
                    .class("tone-editor__back-btn")
                    .cursor(CursorIcon::Hand)
                    .on_press(|cx| {
                        cx.emit(EditorEvent::SetTab(PageTab::Mixer));
                    });

                // Part info
                Binding::new(cx, ToneEditorData::header_info, |cx, info_lens| {
                    let text = info_lens.get(cx);
                    Label::new(cx, &text).class("tone-editor__part-info");
                });
            })
            .class("tone-editor__header");

            // --- Content: dispatch by tone type ---
            Binding::new(cx, ToneEditorData::tone_msb, |cx, msb_lens| {
                let msb = msb_lens.get(cx);
                match msb {
                    MSB_SN_SYNTH => {
                        build_sns_editor(cx);
                    }
                    MSB_PCM_SYNTH => {
                        build_stub(cx, "PCM Synth editor — coming soon");
                    }
                    MSB_SN_ACOUSTIC => {
                        build_stub(cx, "SN Acoustic editor — coming soon");
                    }
                    MSB_PCM_DRUM => {
                        build_stub(cx, "PCM Drum editor — coming soon");
                    }
                    MSB_SN_DRUM => {
                        build_stub(cx, "SN Drum editor — coming soon");
                    }
                    0 => {
                        build_stub(cx, "No tone loaded — select a part with a tone assigned");
                    }
                    _ => {
                        let msg = format!(
                            "Unsupported tone type (MSB {})",
                            msb
                        );
                        build_stub(cx, &msg);
                    }
                }
            });
        })
    }
}

impl View for ToneEditorPage {
    fn element(&self) -> Option<&'static str> {
        Some("tone-editor")
    }
}

// ---------------------------------------------------------------------------
// Stub view for unimplemented tone types
// ---------------------------------------------------------------------------

/// Build a placeholder stub for an unimplemented tone type.
fn build_stub(cx: &mut Context, message: &str) {
    VStack::new(cx, |cx| {
        Label::new(cx, message).class("tone-editor__stub-label");
    })
    .class("tone-editor__stub");
}

// ---------------------------------------------------------------------------
// SN-S Editor
// ---------------------------------------------------------------------------

/// Build the SN Synth editor content.
fn build_sns_editor(cx: &mut Context) {
    ScrollView::new(cx, 0.0, 0.0, false, true, |cx| {
        VStack::new(cx, |cx| {
            // -- Common section --
            build_sns_common_section(cx);

            // -- Partial sections --
            for i in 0..NUM_SNS_PARTIALS {
                build_sns_partial_section(cx, i);
            }

            // -- MFX stub --
            SectionPanel::new(cx, "MFX", |cx| {
                Label::new(cx, "MFX parameters — coming soon")
                    .class("tone-editor__stub-label");
            });
        })
        .class("tone-editor__sns-body");
    })
    .class("tone-editor__scroll");
}

/// Build the SN-S Common parameters section.
fn build_sns_common_section(cx: &mut Context) {
    SectionPanel::new(cx, "COMMON", |cx| {
        // Tone name display
        Binding::new(
            cx,
            ToneEditorData::sns_common.then(SnSynthCommonView::tone_name),
            |cx, name_lens| {
                let name = name_lens.get(cx);
                let display = if name.is_empty() {
                    "(loading...)".to_string()
                } else {
                    name.to_string()
                };
                Label::new(cx, &display).class("tone-editor__tone-name");
            },
        );

        // -- Knob rows --
        HStack::new(cx, |cx| {
            // Tone Level
            SynthKnob::new(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::tone_level),
                |cx, val| {
                    let raw = (val * 127.0).round() as u8;
                    cx.emit(ToneEditorEvent::SetSnsCommon(0x0C, raw));
                },
            )
            .label("Level")
;

            // Wave Shape
            SynthKnob::new(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::wave_shape),
                |cx, val| {
                    let raw = (val * 127.0).round() as u8;
                    cx.emit(ToneEditorEvent::SetSnsCommon(0x35, raw));
                },
            )
            .label("WaveShape")
;

            // Analog Feel
            SynthKnob::new(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::analog_feel),
                |cx, val| {
                    let raw = (val * 127.0).round() as u8;
                    cx.emit(ToneEditorEvent::SetSnsCommon(0x34, raw));
                },
            )
            .label("AnalogFeel")
;

            // Portamento Time
            SynthKnob::new(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::portamento_time),
                |cx, val| {
                    let raw = (val * 127.0).round() as u8;
                    cx.emit(ToneEditorEvent::SetSnsCommon(0x13, raw));
                },
            )
            .label("Porta Time")
;

            // Bend Range Up
            SynthKnob::new(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::bend_range_up),
                |cx, val| {
                    let raw = (val * 24.0).round() as u8;
                    cx.emit(ToneEditorEvent::SetSnsCommon(0x16, raw));
                },
            )
            .label("BendUp")
;

            // Bend Range Down
            SynthKnob::new(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::bend_range_down),
                |cx, val| {
                    let raw = (val * 24.0).round() as u8;
                    cx.emit(ToneEditorEvent::SetSnsCommon(0x17, raw));
                },
            )
            .label("BendDown")
;

            // Octave Shift
            SynthKnob::new(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::octave_shift),
                |cx, val| {
                    let raw = (val * 6.0 + 61.0).round() as u8;
                    cx.emit(ToneEditorEvent::SetSnsCommon(0x15, raw));
                },
            )
            .label("OctShift")
;

            // Unison Size
            SynthKnob::new(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::unison_size),
                |cx, val| {
                    let raw = (val * 3.0).round() as u8;
                    cx.emit(ToneEditorEvent::SetSnsCommon(0x3C, raw));
                },
            )
            .label("UniSize")
;
        })
        .class("tone-editor__knob-row");

        // -- Switches row --
        HStack::new(cx, |cx| {
            // Unison
            VStack::new(cx, |cx| {
                Label::new(cx, "Unison").class("tone-editor__switch-label");
                SynthSwitch::new(
                    cx,
                    ToneEditorData::sns_common.then(SnSynthCommonView::unison_switch),
                    &["OFF", "ON"],
                    |cx, idx| {
                        cx.emit(ToneEditorEvent::SetSnsCommon(0x2E, idx as u8));
                    },
                );
            });

            // Mono
            VStack::new(cx, |cx| {
                Label::new(cx, "Mono").class("tone-editor__switch-label");
                SynthSwitch::new(
                    cx,
                    ToneEditorData::sns_common.then(SnSynthCommonView::mono_switch),
                    &["OFF", "ON"],
                    |cx, idx| {
                        cx.emit(ToneEditorEvent::SetSnsCommon(0x14, idx as u8));
                    },
                );
            });

            // Portamento
            VStack::new(cx, |cx| {
                Label::new(cx, "Porta").class("tone-editor__switch-label");
                SynthSwitch::new(
                    cx,
                    ToneEditorData::sns_common
                        .then(SnSynthCommonView::portamento_switch),
                    &["OFF", "ON"],
                    |cx, idx| {
                        cx.emit(ToneEditorEvent::SetSnsCommon(0x12, idx as u8));
                    },
                );
            });

            // Portamento Mode
            VStack::new(cx, |cx| {
                Label::new(cx, "PortaMode").class("tone-editor__switch-label");
                SynthSwitch::new(
                    cx,
                    ToneEditorData::sns_common
                        .then(SnSynthCommonView::portamento_mode),
                    &["NORMAL", "LEGATO"],
                    |cx, idx| {
                        cx.emit(ToneEditorEvent::SetSnsCommon(0x31, idx as u8));
                    },
                );
            });

            // Legato
            VStack::new(cx, |cx| {
                Label::new(cx, "Legato").class("tone-editor__switch-label");
                SynthSwitch::new(
                    cx,
                    ToneEditorData::sns_common.then(SnSynthCommonView::legato_switch),
                    &["OFF", "ON"],
                    |cx, idx| {
                        cx.emit(ToneEditorEvent::SetSnsCommon(0x32, idx as u8));
                    },
                );
            });

            // Ring
            VStack::new(cx, |cx| {
                Label::new(cx, "Ring").class("tone-editor__switch-label");
                SynthSwitch::new(
                    cx,
                    ToneEditorData::sns_common.then(SnSynthCommonView::ring_switch),
                    &["OFF", "ON"],
                    |cx, idx| {
                        // Ring switch: OFF=0, ON=2 in the MIDI spec.
                        let raw = if idx == 1 { 2u8 } else { 0u8 };
                        cx.emit(ToneEditorEvent::SetSnsCommon(0x1F, raw));
                    },
                );
            });
        })
        .class("tone-editor__switch-row");
    });
}

/// Build a single SN-S Partial section.
fn build_sns_partial_section(cx: &mut Context, partial_idx: usize) {
    let title = format!("PARTIAL {}", partial_idx + 1);
    SectionPanel::new(cx, &title, |cx| {
        // Partial switch — use .map() to unify the lens types.
        let switch_lens = ToneEditorData::sns_common.map(move |c| match partial_idx {
            0 => c.partial1_switch,
            1 => c.partial2_switch,
            _ => c.partial3_switch,
        });
        let switch_offset: u8 = match partial_idx {
            0 => 0x19,
            1 => 0x1B,
            _ => 0x1D,
        };

        HStack::new(cx, |cx| {
            VStack::new(cx, |cx| {
                Label::new(cx, "SW").class("tone-editor__switch-label");
                SynthSwitch::new(cx, switch_lens, &["OFF", "ON"], move |cx, idx| {
                    cx.emit(ToneEditorEvent::SetSnsCommon(switch_offset, idx as u8));
                });
            });

            // OSC Wave selector
            let partial_idx_u8 = partial_idx as u8;
            let wave_lens = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::osc_wave);
            VStack::new(cx, |cx| {
                Label::new(cx, "Wave").class("tone-editor__switch-label");
                SynthSwitch::new(
                    cx,
                    wave_lens,
                    SNS_OSC_WAVE_NAMES,
                    move |cx, idx| {
                        cx.emit(ToneEditorEvent::SetSnsPartial(
                            partial_idx_u8,
                            0x00,
                            idx as u8,
                        ));
                    },
                );
            });

            // Filter Mode selector
            let fmode_lens = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::filter_mode);
            VStack::new(cx, |cx| {
                Label::new(cx, "FiltMode").class("tone-editor__switch-label");
                SynthSwitch::new(
                    cx,
                    fmode_lens,
                    SNS_FILTER_MODE_NAMES,
                    move |cx, idx| {
                        cx.emit(ToneEditorEvent::SetSnsPartial(
                            partial_idx_u8,
                            0x0A,
                            idx as u8,
                        ));
                    },
                );
            });
        })
        .class("tone-editor__switch-row");

        // Knob row: key partial parameters
        HStack::new(cx, |cx| {
            let partial_idx_u8 = partial_idx as u8;

            // OSC Pitch
            let pitch_lens = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::osc_pitch);
            SynthKnob::new(cx, pitch_lens, move |cx, val| {
                let raw = (val * 48.0 + 40.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(partial_idx_u8, 0x03, raw));
            })
            .label("Pitch")
;

            // OSC Detune
            let detune_lens = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::osc_detune);
            SynthKnob::new(cx, detune_lens, move |cx, val| {
                let raw = (val * 100.0 + 14.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(partial_idx_u8, 0x04, raw));
            })
            .label("Detune")
;

            // Filter Cutoff
            let cutoff_lens = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::filter_cutoff);
            SynthKnob::new(cx, cutoff_lens, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(partial_idx_u8, 0x0C, raw));
            })
            .label("Cutoff")
;

            // Filter Resonance
            let reso_lens = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::filter_resonance);
            SynthKnob::new(cx, reso_lens, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(partial_idx_u8, 0x0F, raw));
            })
            .label("Reso")
;

            // Amp Level
            let level_lens = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::amp_level);
            SynthKnob::new(cx, level_lens, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(partial_idx_u8, 0x15, raw));
            })
            .label("Level")
;

            // Amp Pan
            let pan_lens = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::amp_pan);
            SynthKnob::new(cx, pan_lens, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(partial_idx_u8, 0x1B, raw));
            })
            .label("Pan")
;

            // Amp Vel Sens
            let vel_lens = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::amp_vel_sens);
            SynthKnob::new(cx, vel_lens, move |cx, val| {
                let raw = (val * 126.0 + 1.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(partial_idx_u8, 0x16, raw));
            })
            .label("VelSns")
;
        })
        .class("tone-editor__knob-row");

        // Envelope row: Filter ADSR + Amp ADSR
        HStack::new(cx, |cx| {
            let partial_idx_u8 = partial_idx as u8;

            // Filter Env
            let f_a = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::filter_env_attack);
            SynthKnob::new(cx, f_a, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(partial_idx_u8, 0x10, raw));
            })
            .label("F.Atk")
;

            let f_d = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::filter_env_decay);
            SynthKnob::new(cx, f_d, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(partial_idx_u8, 0x11, raw));
            })
            .label("F.Dec")
;

            let f_s = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::filter_env_sustain);
            SynthKnob::new(cx, f_s, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(partial_idx_u8, 0x12, raw));
            })
            .label("F.Sus")
;

            let f_r = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::filter_env_release);
            SynthKnob::new(cx, f_r, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(partial_idx_u8, 0x13, raw));
            })
            .label("F.Rel")
;

            let f_dep = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::filter_env_depth);
            SynthKnob::new(cx, f_dep, move |cx, val| {
                let raw = (val * 126.0 + 1.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(partial_idx_u8, 0x14, raw));
            })
            .label("F.Dep")
;

            // Amp Env
            let a_a = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::amp_env_attack);
            SynthKnob::new(cx, a_a, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(partial_idx_u8, 0x17, raw));
            })
            .label("A.Atk")
;

            let a_d = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::amp_env_decay);
            SynthKnob::new(cx, a_d, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(partial_idx_u8, 0x18, raw));
            })
            .label("A.Dec")
;

            let a_s = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::amp_env_sustain);
            SynthKnob::new(cx, a_s, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(partial_idx_u8, 0x19, raw));
            })
            .label("A.Sus")
;

            let a_r = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::amp_env_release);
            SynthKnob::new(cx, a_r, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(partial_idx_u8, 0x1A, raw));
            })
            .label("A.Rel")
;
        })
        .class("tone-editor__knob-row");
    });
}
