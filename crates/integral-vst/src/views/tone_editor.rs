//! Tone editor page — dispatches to per-tone-type sub-editors.
//!
//! Mirrors the `ToneEditorPage` React component from `web/src/ToneEditorPage.tsx`.
//! Reads the selected part's bank MSB to determine the tone type and renders
//! the appropriate sub-editor.  Currently only the SN-S (SuperNATURAL Synth)
//! editor is implemented; other tone types show stub placeholders.

use std::sync::Arc;
use std::time::Duration;

use integral_core::device::DeviceState;
use integral_core::device_spec::INTEGRA7;
use integral_core::params::{ToneType, tone_type_from_bank_msb};
use integral_core::sn_synth::{
    self, SnSynthCommon, SnSynthPartial, SnsCommonField, SnsPartialField,
};
use integral_core::tone_banks::{find_bank, group_of, user_bank_for};
use integral_core::waves::WaveGroup;
use nih_plug_vizia::vizia::prelude::*;

use crate::SharedState;
use crate::editor::{EditorEvent, PageTab};

use super::{SectionPanel, SynthKnob, SynthKnobExt, SynthSwitch, WavePicker};

/// Refresh interval for reading device state (milliseconds).
const REFRESH_INTERVAL_MS: u64 = 100;

/// Number of SN-S partials.
const NUM_SNS_PARTIALS: usize = INTEGRA7.sn_synth_partial_count as usize;

// ---------------------------------------------------------------------------
// SN-S parameter display names
// ---------------------------------------------------------------------------
// TODO: migrate these to `DeviceSpec` once tone editor param metadata is added.

/// SN-S oscillator wave type names (indexed by OSC Wave value 0-7).
const SNS_OSC_WAVE_NAMES: &[&str] = &["SAW", "SQR", "PWSQ", "TRI", "SIN", "NSE", "SSAW", "PCM"];

/// OSC Wave value selecting a PCM Wave (index of "PCM" in [`SNS_OSC_WAVE_NAMES`]).
const SNS_OSC_WAVE_PCM: usize = sn_synth::OSC_WAVE_PCM as usize;

/// SN-S filter mode names (indexed by Filter Mode value 0-7).
const SNS_FILTER_MODE_NAMES: &[&str] = &["BYP", "LPF", "HPF", "BPF", "PKG", "LP2", "LP3", "LP4"];

/// The name of the tone type selected by `bank_msb` (e.g. "PCM Synth"),
/// taken from the core tone bank table.
fn tone_type_label(bank_msb: u8) -> &'static str {
    user_bank_for(bank_msb)
        .and_then(group_of)
        .map_or("Unknown", |group| group.label)
}

/// The header name of the bank selected by `bank_msb`/`bank_lsb`: its
/// Expansion (e.g. "ExPCM", "SRX-05") or its group (e.g. "GM2", "PCM
/// Synth"), falling back to the tone type.
pub(crate) fn bank_label(bank_msb: u8, bank_lsb: u8) -> &'static str {
    let Some(bank) = find_bank(bank_msb, bank_lsb) else {
        return tone_type_label(bank_msb);
    };
    bank.expansion
        .map(|e| e.name)
        .or_else(|| group_of(bank).map(|group| group.label))
        .unwrap_or_else(|| tone_type_label(bank_msb))
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
    /// Whether the selected part's SN-S tone has been read.
    pub sns_loaded: bool,

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
    /// PCM Wave Number (0 = OFF), used when OSC Wave is PCM.
    pub wave_number: u16,
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
            wave_number: 0,
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
    fn from_core(p: &SnSynthPartial) -> Self {
        Self {
            osc_wave: p.osc_wave as usize,
            wave_number: p.wave_number,
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
    /// Set an SN-S Common field (field, raw MIDI value).
    SetSnsCommon(SnsCommonField, u8),
    /// Set an SN-S Partial field (partial index, field, raw value; the
    /// PCM Wave Number is sent nibblized).
    SetSnsPartial(u8, SnsPartialField, u16),
}

impl Model for ToneEditorData {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|e, _| {
            if let ToneEditorEvent::Refresh = e {
                self.refresh_from_device();
                return;
            }
            let shared = Arc::clone(&self.shared);
            let Ok(mut dev) = shared.device.lock() else {
                return;
            };
            let part = self.selected_part as u8;
            match e {
                ToneEditorEvent::Refresh => {}
                ToneEditorEvent::SetSnsCommon(field, value) => {
                    dev.set_sns_common_param(part, *field, (*value).into());
                }
                ToneEditorEvent::SetSnsPartial(partial, field, value) => {
                    dev.set_sns_partial_param(part, *partial, *field, *value);
                }
            }
            self.show_sns_tone(&dev);
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
            shared,
        };
        data.refresh_from_device();
        data
    }

    /// Read current state from the device and update the view snapshot.
    fn refresh_from_device(&mut self) {
        let shared = Arc::clone(&self.shared);
        let Ok(mut dev) = shared.device.lock() else {
            return;
        };
        let mixer = dev.state();
        let idx = self.selected_part;
        let part = &mixer.parts[idx];

        self.tone_msb = part.tone_bank_msb;
        self.tone_name = part.tone_name.clone();

        let type_label = bank_label(part.tone_bank_msb, part.tone_bank_lsb);
        self.header_info = if part.tone_name.is_empty() {
            format!("Part {} : {}", idx + 1, type_label)
        } else {
            format!("Part {} : {} : {}", idx + 1, type_label, part.tone_name)
        };

        if tone_type_from_bank_msb(part.tone_bank_msb) != Some(ToneType::SnSynth) {
            return;
        }
        let part = idx as u8;
        if dev.load_sns_tone(part) {
            let rq = dev.build_mfx_request(part, ToneType::SnSynth);
            dev.send_raw(&format!("mfx-{part}"), rq);
        }
        self.show_sns_tone(&dev);
    }

    /// Show the selected part's SN-S tone as cached in core, once read.
    fn show_sns_tone(&mut self, dev: &DeviceState) {
        match dev.sns_tone(self.selected_part as u8) {
            Some(tone) => {
                self.sns_common = SnSynthCommonView::from_core(&tone.common);
                self.sns_partials = tone.partials.each_ref().map(SnSynthPartialView::from_core);
                self.sns_loaded = true;
            }
            None => self.sns_loaded = false,
        }
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
                match (msb, tone_type_from_bank_msb(msb)) {
                    (_, Some(ToneType::SnSynth)) => {
                        build_sns_editor(cx);
                    }
                    (_, Some(_)) => {
                        let msg = format!("{} editor — coming soon", tone_type_label(msb));
                        build_stub(cx, &msg);
                    }
                    (0, None) => {
                        build_stub(cx, "No tone loaded — select a part with a tone assigned");
                    }
                    (_, None) => {
                        let msg = format!("Unsupported tone type (MSB {})", msb);
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
                Label::new(cx, "MFX parameters — coming soon").class("tone-editor__stub-label");
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
                    cx.emit(ToneEditorEvent::SetSnsCommon(
                        SnsCommonField::ToneLevel,
                        raw,
                    ));
                },
            )
            .label("Level");

            // Wave Shape
            SynthKnob::new(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::wave_shape),
                |cx, val| {
                    let raw = (val * 127.0).round() as u8;
                    cx.emit(ToneEditorEvent::SetSnsCommon(
                        SnsCommonField::WaveShape,
                        raw,
                    ));
                },
            )
            .label("WaveShape");

            // Analog Feel
            SynthKnob::new(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::analog_feel),
                |cx, val| {
                    let raw = (val * 127.0).round() as u8;
                    cx.emit(ToneEditorEvent::SetSnsCommon(
                        SnsCommonField::AnalogFeel,
                        raw,
                    ));
                },
            )
            .label("AnalogFeel");

            // Portamento Time
            SynthKnob::new(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::portamento_time),
                |cx, val| {
                    let raw = (val * 127.0).round() as u8;
                    cx.emit(ToneEditorEvent::SetSnsCommon(
                        SnsCommonField::PortamentoTime,
                        raw,
                    ));
                },
            )
            .label("Porta Time");

            // Bend Range Up
            SynthKnob::new(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::bend_range_up),
                |cx, val| {
                    let raw = (val * 24.0).round() as u8;
                    cx.emit(ToneEditorEvent::SetSnsCommon(
                        SnsCommonField::PitchBendRangeUp,
                        raw,
                    ));
                },
            )
            .label("BendUp");

            // Bend Range Down
            SynthKnob::new(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::bend_range_down),
                |cx, val| {
                    let raw = (val * 24.0).round() as u8;
                    cx.emit(ToneEditorEvent::SetSnsCommon(
                        SnsCommonField::PitchBendRangeDown,
                        raw,
                    ));
                },
            )
            .label("BendDown");

            // Octave Shift
            SynthKnob::new(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::octave_shift),
                |cx, val| {
                    let raw = (val * 6.0 + 61.0).round() as u8;
                    cx.emit(ToneEditorEvent::SetSnsCommon(
                        SnsCommonField::OctaveShift,
                        raw,
                    ));
                },
            )
            .label("OctShift");

            // Unison Size
            SynthKnob::new(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::unison_size),
                |cx, val| {
                    let raw = (val * 3.0).round() as u8;
                    cx.emit(ToneEditorEvent::SetSnsCommon(
                        SnsCommonField::UnisonSize,
                        raw,
                    ));
                },
            )
            .label("UniSize");
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
                        cx.emit(ToneEditorEvent::SetSnsCommon(
                            SnsCommonField::UnisonSwitch,
                            idx as u8,
                        ));
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
                        cx.emit(ToneEditorEvent::SetSnsCommon(
                            SnsCommonField::MonoSwitch,
                            idx as u8,
                        ));
                    },
                );
            });

            // Portamento
            VStack::new(cx, |cx| {
                Label::new(cx, "Porta").class("tone-editor__switch-label");
                SynthSwitch::new(
                    cx,
                    ToneEditorData::sns_common.then(SnSynthCommonView::portamento_switch),
                    &["OFF", "ON"],
                    |cx, idx| {
                        cx.emit(ToneEditorEvent::SetSnsCommon(
                            SnsCommonField::PortamentoSwitch,
                            idx as u8,
                        ));
                    },
                );
            });

            // Portamento Mode
            VStack::new(cx, |cx| {
                Label::new(cx, "PortaMode").class("tone-editor__switch-label");
                SynthSwitch::new(
                    cx,
                    ToneEditorData::sns_common.then(SnSynthCommonView::portamento_mode),
                    &["NORMAL", "LEGATO"],
                    |cx, idx| {
                        cx.emit(ToneEditorEvent::SetSnsCommon(
                            SnsCommonField::PortamentoMode,
                            idx as u8,
                        ));
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
                        cx.emit(ToneEditorEvent::SetSnsCommon(
                            SnsCommonField::LegatoSwitch,
                            idx as u8,
                        ));
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
                        cx.emit(ToneEditorEvent::SetSnsCommon(
                            SnsCommonField::RingSwitch,
                            raw,
                        ));
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
        let switch_field = SnsCommonField::PARTIAL_SWITCHES[partial_idx];

        HStack::new(cx, |cx| {
            VStack::new(cx, |cx| {
                Label::new(cx, "SW").class("tone-editor__switch-label");
                SynthSwitch::new(cx, switch_lens, &["OFF", "ON"], move |cx, idx| {
                    cx.emit(ToneEditorEvent::SetSnsCommon(switch_field, idx as u8));
                });
            });

            // OSC Wave selector
            let partial_idx_u8 = partial_idx as u8;
            let wave_lens = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::osc_wave);
            VStack::new(cx, |cx| {
                Label::new(cx, "Wave").class("tone-editor__switch-label");
                SynthSwitch::new(cx, wave_lens, SNS_OSC_WAVE_NAMES, move |cx, idx| {
                    cx.emit(ToneEditorEvent::SetSnsPartial(
                        partial_idx_u8,
                        SnsPartialField::OscWave,
                        idx as u16,
                    ));
                });
            });

            // PCM Wave picker, shown only when OSC Wave is PCM.
            Binding::new(cx, wave_lens, move |cx, osc_wave| {
                if osc_wave.get(cx) != SNS_OSC_WAVE_PCM {
                    return;
                }
                let number_lens = ToneEditorData::sns_partials
                    .map(move |p| p[partial_idx].clone())
                    .then(SnSynthPartialView::wave_number);
                VStack::new(cx, |cx| {
                    Label::new(cx, "PCM Wave").class("tone-editor__switch-label");
                    WavePicker::new(cx, WaveGroup::SnSynth, number_lens, move |cx, n| {
                        cx.emit(ToneEditorEvent::SetSnsPartial(
                            partial_idx_u8,
                            SnsPartialField::WaveNumber,
                            n,
                        ));
                    });
                });
            });

            // Filter Mode selector
            let fmode_lens = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::filter_mode);
            VStack::new(cx, |cx| {
                Label::new(cx, "FiltMode").class("tone-editor__switch-label");
                SynthSwitch::new(cx, fmode_lens, SNS_FILTER_MODE_NAMES, move |cx, idx| {
                    cx.emit(ToneEditorEvent::SetSnsPartial(
                        partial_idx_u8,
                        SnsPartialField::FilterMode,
                        idx as u16,
                    ));
                });
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
                cx.emit(ToneEditorEvent::SetSnsPartial(
                    partial_idx_u8,
                    SnsPartialField::OscPitch,
                    raw.into(),
                ));
            })
            .label("Pitch");

            // OSC Detune
            let detune_lens = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::osc_detune);
            SynthKnob::new(cx, detune_lens, move |cx, val| {
                let raw = (val * 100.0 + 14.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(
                    partial_idx_u8,
                    SnsPartialField::OscDetune,
                    raw.into(),
                ));
            })
            .label("Detune");

            // Filter Cutoff
            let cutoff_lens = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::filter_cutoff);
            SynthKnob::new(cx, cutoff_lens, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(
                    partial_idx_u8,
                    SnsPartialField::FilterCutoff,
                    raw.into(),
                ));
            })
            .label("Cutoff");

            // Filter Resonance
            let reso_lens = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::filter_resonance);
            SynthKnob::new(cx, reso_lens, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(
                    partial_idx_u8,
                    SnsPartialField::FilterResonance,
                    raw.into(),
                ));
            })
            .label("Reso");

            // Amp Level
            let level_lens = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::amp_level);
            SynthKnob::new(cx, level_lens, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(
                    partial_idx_u8,
                    SnsPartialField::AmpLevel,
                    raw.into(),
                ));
            })
            .label("Level");

            // Amp Pan
            let pan_lens = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::amp_pan);
            SynthKnob::new(cx, pan_lens, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(
                    partial_idx_u8,
                    SnsPartialField::AmpPan,
                    raw.into(),
                ));
            })
            .label("Pan");

            // Amp Vel Sens
            let vel_lens = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::amp_vel_sens);
            SynthKnob::new(cx, vel_lens, move |cx, val| {
                let raw = (val * 126.0 + 1.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(
                    partial_idx_u8,
                    SnsPartialField::AmpVelSens,
                    raw.into(),
                ));
            })
            .label("VelSns");
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
                cx.emit(ToneEditorEvent::SetSnsPartial(
                    partial_idx_u8,
                    SnsPartialField::FilterEnvAttack,
                    raw.into(),
                ));
            })
            .label("F.Atk");

            let f_d = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::filter_env_decay);
            SynthKnob::new(cx, f_d, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(
                    partial_idx_u8,
                    SnsPartialField::FilterEnvDecay,
                    raw.into(),
                ));
            })
            .label("F.Dec");

            let f_s = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::filter_env_sustain);
            SynthKnob::new(cx, f_s, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(
                    partial_idx_u8,
                    SnsPartialField::FilterEnvSustain,
                    raw.into(),
                ));
            })
            .label("F.Sus");

            let f_r = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::filter_env_release);
            SynthKnob::new(cx, f_r, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(
                    partial_idx_u8,
                    SnsPartialField::FilterEnvRelease,
                    raw.into(),
                ));
            })
            .label("F.Rel");

            let f_dep = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::filter_env_depth);
            SynthKnob::new(cx, f_dep, move |cx, val| {
                let raw = (val * 126.0 + 1.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(
                    partial_idx_u8,
                    SnsPartialField::FilterEnvDepth,
                    raw.into(),
                ));
            })
            .label("F.Dep");

            // Amp Env
            let a_a = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::amp_env_attack);
            SynthKnob::new(cx, a_a, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(
                    partial_idx_u8,
                    SnsPartialField::AmpEnvAttack,
                    raw.into(),
                ));
            })
            .label("A.Atk");

            let a_d = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::amp_env_decay);
            SynthKnob::new(cx, a_d, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(
                    partial_idx_u8,
                    SnsPartialField::AmpEnvDecay,
                    raw.into(),
                ));
            })
            .label("A.Dec");

            let a_s = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::amp_env_sustain);
            SynthKnob::new(cx, a_s, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(
                    partial_idx_u8,
                    SnsPartialField::AmpEnvSustain,
                    raw.into(),
                ));
            })
            .label("A.Sus");

            let a_r = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::amp_env_release);
            SynthKnob::new(cx, a_r, move |cx, val| {
                let raw = (val * 127.0).round() as u8;
                cx.emit(ToneEditorEvent::SetSnsPartial(
                    partial_idx_u8,
                    SnsPartialField::AmpEnvRelease,
                    raw.into(),
                ));
            })
            .label("A.Rel");
        })
        .class("tone-editor__knob-row");
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_names_expansion_and_gm2_banks() {
        assert_eq!(bank_label(97, 0), "ExPCM");
        assert_eq!(bank_label(121, 0), "GM2");
        assert_eq!(bank_label(87, 0), "PCM Synth");
        // The editor placeholder still names the tone type.
        assert_eq!(tone_type_label(97), "PCM Synth");
    }

    #[test]
    fn osc_wave_pcm_index_names_pcm() {
        assert_eq!(SNS_OSC_WAVE_NAMES[SNS_OSC_WAVE_PCM], "PCM");
    }
}
