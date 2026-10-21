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
use integral_core::family::ParameterFamily;
use integral_core::field_spec::{FieldSpec, sns_common_spec, sns_partial_spec};
use integral_core::params::{ToneType, tone_type_from_bank_msb};
use integral_core::sn_synth::{
    self, SnSynthCommon, SnSynthPartial, SnsCommonField, SnsPartialField,
};
use integral_core::tone_banks::{find_bank, group_of, user_bank_for};
use integral_core::waves::WaveGroup;
use nih_plug_vizia::vizia::prelude::*;

use crate::SharedState;
use crate::views::{EditorEvent, PageTab};

use super::{SectionPanel, SectionPanelExt, SynthKnob, SynthKnobExt, SynthSwitch, WavePicker};

/// Refresh interval for reading device state (milliseconds).
const REFRESH_INTERVAL_MS: u64 = 100;

/// Number of SN-S partials.
const NUM_SNS_PARTIALS: usize = INTEGRA7.sn_synth_partial_count as usize;

// ---------------------------------------------------------------------------
// SN-S parameter display names
// ---------------------------------------------------------------------------

/// OSC Wave value selecting a PCM Wave (index of "PCM" in the core
/// [`SnsPartialField::OscWave`] names).
const SNS_OSC_WAVE_PCM: usize = sn_synth::OSC_WAVE_PCM as usize;

/// The option labels of an SN-S switch `spec`: each value in its range
/// formatted by the field's core spec, as the web's `switchOptions` helper
/// does.
fn switch_labels(spec: &FieldSpec) -> Vec<String> {
    (spec.min..=spec.max)
        .map(|value| spec.format.format(value))
        .collect()
}

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

/// `raw` normalized to `spec`'s range, 0.0 at its minimum and 1.0 at its maximum.
fn to_norm(spec: &FieldSpec, raw: impl Into<i32>) -> f32 {
    (raw.into() - spec.min) as f32 / (spec.max - spec.min).max(1) as f32
}

/// The raw value of `spec` at normalized position `norm`.
fn from_norm(spec: &FieldSpec, norm: f32) -> i32 {
    spec.min + (norm.clamp(0.0, 1.0) * (spec.max - spec.min) as f32).round() as i32
}

/// An SN-S Common field's raw value, normalized to its range.
fn common_norm(field: SnsCommonField, raw: u8) -> f32 {
    to_norm(&sns_common_spec(field), raw)
}

/// An SN-S Partial field's raw value, normalized to its range.
fn partial_norm(field: SnsPartialField, raw: u8) -> f32 {
    to_norm(&sns_partial_spec(field), raw)
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
    /// The view of the core's INIT SnSynthCommon.
    fn default() -> Self {
        Self::from_core(&SnSynthCommon::default())
    }
}

impl SnSynthCommonView {
    /// Populate from a parsed [`SnSynthCommon`].
    fn from_core(c: &SnSynthCommon) -> Self {
        use SnsCommonField as C;
        Self {
            tone_name: c.tone_name.clone(),
            tone_level: common_norm(C::ToneLevel, c.tone_level),
            portamento_switch: c.portamento_switch as usize,
            portamento_time: common_norm(C::PortamentoTime, c.portamento_time),
            mono_switch: c.mono_switch as usize,
            octave_shift: common_norm(C::OctaveShift, c.octave_shift),
            bend_range_up: common_norm(C::PitchBendRangeUp, c.pitch_bend_range_up),
            bend_range_down: common_norm(C::PitchBendRangeDown, c.pitch_bend_range_down),
            partial1_switch: c.partial1_switch as usize,
            partial2_switch: c.partial2_switch as usize,
            partial3_switch: c.partial3_switch as usize,
            ring_switch: if c.ring_switch >= 1 { 1 } else { 0 },
            unison_switch: c.unison_switch as usize,
            unison_size: common_norm(C::UnisonSize, c.unison_size),
            portamento_mode: c.portamento_mode as usize,
            legato_switch: c.legato_switch as usize,
            analog_feel: common_norm(C::AnalogFeel, c.analog_feel),
            wave_shape: common_norm(C::WaveShape, c.wave_shape),
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
    /// The view of the core's INIT SnSynthPartial.
    fn default() -> Self {
        Self::from_core(&SnSynthPartial::default())
    }
}

impl SnSynthPartialView {
    /// Populate from a parsed [`SnSynthPartial`].
    fn from_core(p: &SnSynthPartial) -> Self {
        use SnsPartialField as P;
        Self {
            osc_wave: p.osc_wave as usize,
            wave_number: p.wave_number,
            filter_cutoff: partial_norm(P::FilterCutoff, p.filter_cutoff),
            filter_resonance: partial_norm(P::FilterResonance, p.filter_resonance),
            amp_level: partial_norm(P::AmpLevel, p.amp_level),
            amp_pan: partial_norm(P::AmpPan, p.amp_pan),
            osc_pitch: partial_norm(P::OscPitch, p.osc_pitch),
            osc_detune: partial_norm(P::OscDetune, p.osc_detune),
            filter_mode: p.filter_mode as usize,
            amp_env_attack: partial_norm(P::AmpEnvAttack, p.amp_env_attack),
            amp_env_decay: partial_norm(P::AmpEnvDecay, p.amp_env_decay),
            amp_env_sustain: partial_norm(P::AmpEnvSustain, p.amp_env_sustain),
            amp_env_release: partial_norm(P::AmpEnvRelease, p.amp_env_release),
            filter_env_attack: partial_norm(P::FilterEnvAttack, p.filter_env_attack),
            filter_env_decay: partial_norm(P::FilterEnvDecay, p.filter_env_decay),
            filter_env_sustain: partial_norm(P::FilterEnvSustain, p.filter_env_sustain),
            filter_env_release: partial_norm(P::FilterEnvRelease, p.filter_env_release),
            filter_env_depth: partial_norm(P::FilterEnvDepth, p.filter_env_depth),
            amp_vel_sens: partial_norm(P::AmpVelSens, p.amp_vel_sens),
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
                self.refresh_from_device(false);
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
        // Read the tone afresh: it may have been edited on the device.
        data.refresh_from_device(true);
        data
    }

    /// Read current state from the device and update the view snapshot,
    /// starting to read the SN-S tone if it isn't cached, or again if
    /// `reload`.
    fn refresh_from_device(&mut self, reload: bool) {
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
        if reload {
            dev.reload_sns_tone(part);
        }
        if reload || dev.load_sns_tone(part) {
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
            let common = ToneEditorData::sns_common;
            use SnSynthCommonView as V;
            use SnsCommonField as F;
            sns_common_knob(cx, common.then(V::tone_level), F::ToneLevel, "Level");
            sns_common_knob(cx, common.then(V::wave_shape), F::WaveShape, "WaveShape");
            sns_common_knob(cx, common.then(V::analog_feel), F::AnalogFeel, "AnalogFeel");
            sns_common_knob(
                cx,
                common.then(V::portamento_time),
                F::PortamentoTime,
                "Porta Time",
            );
            sns_common_knob(
                cx,
                common.then(V::bend_range_up),
                F::PitchBendRangeUp,
                "BendUp",
            );
            sns_common_knob(
                cx,
                common.then(V::bend_range_down),
                F::PitchBendRangeDown,
                "BendDown",
            );
            sns_common_knob(cx, common.then(V::octave_shift), F::OctaveShift, "OctShift");
            sns_common_knob(cx, common.then(V::unison_size), F::UnisonSize, "UniSize");
        })
        .class("tone-editor__knob-row");

        // -- Switches row --
        HStack::new(cx, |cx| {
            // Unison
            sns_common_switch(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::unison_switch),
                SnsCommonField::UnisonSwitch,
                "Unison",
                |cx, idx| {
                    cx.emit(ToneEditorEvent::SetSnsCommon(
                        SnsCommonField::UnisonSwitch,
                        idx as u8,
                    ));
                },
            );

            // Mono
            sns_common_switch(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::mono_switch),
                SnsCommonField::MonoSwitch,
                "Mono",
                |cx, idx| {
                    cx.emit(ToneEditorEvent::SetSnsCommon(
                        SnsCommonField::MonoSwitch,
                        idx as u8,
                    ));
                },
            );

            // Portamento
            sns_common_switch(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::portamento_switch),
                SnsCommonField::PortamentoSwitch,
                "Porta",
                |cx, idx| {
                    cx.emit(ToneEditorEvent::SetSnsCommon(
                        SnsCommonField::PortamentoSwitch,
                        idx as u8,
                    ));
                },
            );

            // Portamento Mode
            sns_common_switch(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::portamento_mode),
                SnsCommonField::PortamentoMode,
                "PortaMode",
                |cx, idx| {
                    cx.emit(ToneEditorEvent::SetSnsCommon(
                        SnsCommonField::PortamentoMode,
                        idx as u8,
                    ));
                },
            );

            // Legato
            sns_common_switch(
                cx,
                ToneEditorData::sns_common.then(SnSynthCommonView::legato_switch),
                SnsCommonField::LegatoSwitch,
                "Legato",
                |cx, idx| {
                    cx.emit(ToneEditorEvent::SetSnsCommon(
                        SnsCommonField::LegatoSwitch,
                        idx as u8,
                    ));
                },
            );

            // Ring: an OFF/ON toggle over the spec's OFF (0) and ON (2)
            // values; the middle "---" (1) still shows as ON, as before.
            let ring_labels = switch_labels(&sns_common_spec(SnsCommonField::RingSwitch));
            let ring_options = [ring_labels[0].clone(), ring_labels[2].clone()];
            VStack::new(cx, move |cx| {
                Label::new(cx, "Ring").class("tone-editor__switch-label");
                let options: Vec<&str> = ring_options.iter().map(String::as_str).collect();
                SynthSwitch::new(
                    cx,
                    ToneEditorData::sns_common.then(SnSynthCommonView::ring_switch),
                    &options,
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
    })
    .family(ParameterFamily::Common);
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
            sns_common_switch(cx, switch_lens, switch_field, "SW", move |cx, idx| {
                cx.emit(ToneEditorEvent::SetSnsCommon(switch_field, idx as u8));
            });

            // OSC Wave selector
            let partial_idx_u8 = partial_idx as u8;
            let wave_lens = ToneEditorData::sns_partials
                .map(move |p| p[partial_idx].clone())
                .then(SnSynthPartialView::osc_wave);
            sns_partial_switch(
                cx,
                wave_lens,
                SnsPartialField::OscWave,
                "Wave",
                move |cx, idx| {
                    cx.emit(ToneEditorEvent::SetSnsPartial(
                        partial_idx_u8,
                        SnsPartialField::OscWave,
                        idx as u16,
                    ));
                },
            );

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
            sns_partial_switch(
                cx,
                fmode_lens,
                SnsPartialField::FilterMode,
                "FiltMode",
                move |cx, idx| {
                    cx.emit(ToneEditorEvent::SetSnsPartial(
                        partial_idx_u8,
                        SnsPartialField::FilterMode,
                        idx as u16,
                    ));
                },
            );
        })
        .class("tone-editor__switch-row");

        // Knob row: key partial parameters
        HStack::new(cx, |cx| {
            use SnSynthPartialView as V;
            use SnsPartialField as F;
            sns_partial_knob(cx, partial_idx, V::osc_pitch, F::OscPitch, "Pitch");
            sns_partial_knob(cx, partial_idx, V::osc_detune, F::OscDetune, "Detune");
            sns_partial_knob(cx, partial_idx, V::filter_cutoff, F::FilterCutoff, "Cutoff");
            sns_partial_knob(
                cx,
                partial_idx,
                V::filter_resonance,
                F::FilterResonance,
                "Reso",
            );
            sns_partial_knob(cx, partial_idx, V::amp_level, F::AmpLevel, "Level");
            sns_partial_knob(cx, partial_idx, V::amp_pan, F::AmpPan, "Pan");
            sns_partial_knob(cx, partial_idx, V::amp_vel_sens, F::AmpVelSens, "VelSns");
        })
        .class("tone-editor__knob-row");

        // Envelope row: Filter ADSR + Amp ADSR
        HStack::new(cx, |cx| {
            use SnSynthPartialView as V;
            use SnsPartialField as F;
            sns_partial_knob(
                cx,
                partial_idx,
                V::filter_env_attack,
                F::FilterEnvAttack,
                "F.Atk",
            );
            sns_partial_knob(
                cx,
                partial_idx,
                V::filter_env_decay,
                F::FilterEnvDecay,
                "F.Dec",
            );
            sns_partial_knob(
                cx,
                partial_idx,
                V::filter_env_sustain,
                F::FilterEnvSustain,
                "F.Sus",
            );
            sns_partial_knob(
                cx,
                partial_idx,
                V::filter_env_release,
                F::FilterEnvRelease,
                "F.Rel",
            );
            sns_partial_knob(
                cx,
                partial_idx,
                V::filter_env_depth,
                F::FilterEnvDepth,
                "F.Dep",
            );
            sns_partial_knob(cx, partial_idx, V::amp_env_attack, F::AmpEnvAttack, "A.Atk");
            sns_partial_knob(cx, partial_idx, V::amp_env_decay, F::AmpEnvDecay, "A.Dec");
            sns_partial_knob(
                cx,
                partial_idx,
                V::amp_env_sustain,
                F::AmpEnvSustain,
                "A.Sus",
            );
            sns_partial_knob(
                cx,
                partial_idx,
                V::amp_env_release,
                F::AmpEnvRelease,
                "A.Rel",
            );
        })
        .class("tone-editor__knob-row");
    });
}

/// The normalized centre of `spec`'s LED arc, when its format is signed
/// around a value (e.g. pan 64); the arc then lights from that LED.
fn spec_center(spec: &FieldSpec) -> Option<f32> {
    spec.format.center().map(|center| to_norm(spec, center))
}

/// A labelled switch for an SN-S Common `field`, its option labels taken
/// from the field's core spec.
fn sns_common_switch(
    cx: &mut Context,
    lens: impl Lens<Target = usize>,
    field: SnsCommonField,
    title: &str,
    on_change: impl Fn(&mut EventContext, usize) + 'static,
) {
    let labels = switch_labels(&sns_common_spec(field));
    let options: Vec<&str> = labels.iter().map(String::as_str).collect();
    VStack::new(cx, move |cx| {
        Label::new(cx, title).class("tone-editor__switch-label");
        SynthSwitch::new(cx, lens, &options, on_change);
    });
}

/// A labelled switch for a Partial's SN-S `field`, its option labels taken
/// from the field's core spec.
fn sns_partial_switch(
    cx: &mut Context,
    lens: impl Lens<Target = usize>,
    field: SnsPartialField,
    title: &str,
    on_change: impl Fn(&mut EventContext, usize) + 'static,
) {
    let labels = switch_labels(&sns_partial_spec(field));
    let options: Vec<&str> = labels.iter().map(String::as_str).collect();
    VStack::new(cx, move |cx| {
        Label::new(cx, title).class("tone-editor__switch-label");
        SynthSwitch::new(cx, lens, &options, on_change);
    });
}

/// A knob for SN-S Common `field`, ranged and coloured by its core spec.
fn sns_common_knob(
    cx: &mut Context,
    lens: impl Lens<Target = f32>,
    field: SnsCommonField,
    label: &str,
) {
    let spec = sns_common_spec(field);
    let family = spec.family;
    let center = spec_center(&spec);
    SynthKnob::new(cx, lens, move |cx, val| {
        let raw = from_norm(&spec, val) as u8;
        cx.emit(ToneEditorEvent::SetSnsCommon(field, raw));
    })
    .label(label)
    .family(family)
    .center(center);
}

/// A knob for Partial `partial_idx`'s SN-S `field`, shown through
/// `view_lens`, ranged and coloured by its core spec.
fn sns_partial_knob(
    cx: &mut Context,
    partial_idx: usize,
    view_lens: impl Lens<Source = SnSynthPartialView, Target = f32>,
    field: SnsPartialField,
    label: &str,
) {
    let spec = sns_partial_spec(field);
    let family = spec.family;
    let center = spec_center(&spec);
    let lens = ToneEditorData::sns_partials
        .map(move |p| p[partial_idx].clone())
        .then(view_lens);
    SynthKnob::new(cx, lens, move |cx, val| {
        let raw = from_norm(&spec, val) as u16;
        cx.emit(ToneEditorEvent::SetSnsPartial(
            partial_idx as u8,
            field,
            raw,
        ));
    })
    .label(label)
    .family(family)
    .center(center);
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
    fn knob_values_round_trip_through_the_spec_range() {
        let octave = sns_common_spec(SnsCommonField::OctaveShift);
        assert_eq!(common_norm(SnsCommonField::OctaveShift, 61), 0.0);
        assert_eq!(common_norm(SnsCommonField::OctaveShift, 67), 1.0);
        assert_eq!(from_norm(&octave, 0.5), 64);
        let depth = sns_partial_spec(SnsPartialField::FilterEnvDepth);
        for raw in 1..=127u8 {
            let norm = partial_norm(SnsPartialField::FilterEnvDepth, raw);
            assert_eq!(from_norm(&depth, norm), i32::from(raw));
        }
    }

    #[test]
    fn osc_wave_pcm_index_names_pcm() {
        let names = switch_labels(&sns_partial_spec(SnsPartialField::OscWave));
        assert_eq!(names[SNS_OSC_WAVE_PCM], "PCM");
    }

    #[test]
    fn switch_labels_come_from_the_spec_format() {
        // The core names, not the deleted local table: the doc spells
        // the slopes LPF2, LPF3, LPF4.
        let mode = switch_labels(&sns_partial_spec(SnsPartialField::FilterMode));
        assert_eq!(mode.last().map(String::as_str), Some("LPF4"));
        // Boolean fields format as OFF/ON.
        let on_off = switch_labels(&sns_common_spec(SnsCommonField::MonoSwitch));
        assert_eq!(on_off, ["OFF", "ON"]);
    }
}
