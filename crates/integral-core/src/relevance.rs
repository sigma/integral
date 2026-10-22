//! Field relevance: whether a field is active given the device state, and
//! if not, why.
//!
//! Relevance is device knowledge, so it lives here beside
//! [`FieldSpec`](crate::field_spec::FieldSpec) rather than in a layout. A
//! layout only says how an inactive Control looks (dimmed or hidden); it
//! never holds the predicate. Each engine has one `*_relevance` function per
//! block, taking the field and the state it depends on.
//!
//! A field is inactive in one of two ways ([`Relevance`]). Switched off, its
//! block or Scope is off: it acts again once switched on, so it stays
//! editable, as on the hardware where an EQ can be set up while it's off.
//! Irrelevant, it has no effect in the current mode.
//!
//! Relevance comes in two layers. A partial switched off makes its whole
//! Scope inactive, and the Section holding it dims as one: the
//! `*_scope_relevance` functions answer that. A field's own `*_relevance`
//! answers within an active Scope, so a Control stays as it is drawn when
//! its Section is already dimmed.

use crate::param_registry::MixerField;
use crate::params;
use crate::pcm_drum::PcmDrumPartial;
use crate::pcm_structure;
use crate::pcm_synth::{
    LFO_WAVEFORM_STEP, PCMS_PARTIAL_COUNT, PcmSynthPartial, PcmSynthPmt, PcmsPartialField,
    PcmsPmtField,
};
use crate::sn_acoustic::{SnAcousticCommon, SnaCommonField, sna_offsets};
use crate::sn_synth::{
    OSC_WAVE_PCM, OSC_WAVE_SUPER_SAW, SnSynthCommon, SnSynthPartial, SnsPartialField,
};
use crate::state::{CompEqUnit, DrumCompEqState, FxState, PartState};

/// Whether a field (or a Scope) acts in the device state, and if not, why.
/// Ordered from most to least live, so the most live of several fields is
/// their minimum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Relevance {
    /// It acts.
    Active,
    /// Its block or Scope is switched off (an EQ, a COMP, an FX, a partial,
    /// a WMT): it acts again once switched on, so it stays editable.
    SwitchedOff,
    /// It has no effect in the current mode (Var on a wave with no
    /// variations, an Output Assign under Motional Surround), or doesn't
    /// exist.
    Irrelevant,
}

impl Relevance {
    /// Whether it acts.
    pub fn is_active(self) -> bool {
        self == Self::Active
    }

    /// [`Active`](Self::Active) while the block's switch is `on`, else
    /// [`SwitchedOff`](Self::SwitchedOff).
    pub fn switched(on: bool) -> Self {
        if on { Self::Active } else { Self::SwitchedOff }
    }

    /// [`Active`](Self::Active) while the field `applies` in this mode, else
    /// [`Irrelevant`](Self::Irrelevant).
    pub fn applies(applies: bool) -> Self {
        if applies {
            Self::Active
        } else {
            Self::Irrelevant
        }
    }
}

// ---------------------------------------------------------------------------
// SN Synth
// ---------------------------------------------------------------------------

/// Relevance of SN Synth Partial `partial`'s (0-based) Scope: switched off
/// with its switch.
/// [`Irrelevant`](Relevance::Irrelevant) for a partial the tone doesn't
/// have.
pub fn sns_partial_scope_relevance(common: &SnSynthCommon, partial: usize) -> Relevance {
    let switch = match partial {
        0 => common.partial1_switch,
        1 => common.partial2_switch,
        2 => common.partial3_switch,
        _ => return Relevance::Irrelevant,
    };
    Relevance::switched(switch != 0)
}

/// Relevance of `field` of an SN Synth Partial in state `partial`: whether
/// its OSC Wave uses it. Wave Variation has no effect on the PCM and Super
/// Saw waves; Super Saw Detune acts on Super Saw only.
pub fn sns_partial_relevance(field: SnsPartialField, partial: &SnSynthPartial) -> Relevance {
    let wave = partial.osc_wave;
    Relevance::applies(match field {
        SnsPartialField::OscWaveVariation => wave != OSC_WAVE_PCM && wave != OSC_WAVE_SUPER_SAW,
        SnsPartialField::SuperSawDetune => wave == OSC_WAVE_SUPER_SAW,
        _ => true,
    })
}

// ---------------------------------------------------------------------------
// PCM Synth
// ---------------------------------------------------------------------------

/// Relevance of PCM Synth Partial `partial`'s (0-based) Scope: switched off
/// with its PMT Partial Switch. [`Irrelevant`](Relevance::Irrelevant) for a partial the tone
/// doesn't have.
pub fn pcms_partial_scope_relevance(pmt: &PcmSynthPmt, partial: usize) -> Relevance {
    pmt.partial_entries
        .get(partial)
        .map_or(Relevance::Irrelevant, |e| {
            Relevance::switched(e.partial_switch != 0)
        })
}

/// Whether the pair holding partial `partial` (0-based) merges its two
/// paths, so that its upper partial follows the lower one's pan settings.
fn pair_is_merged(pmt: &PcmSynthPmt, partial: usize) -> bool {
    let structure = if partial < 2 {
        pmt.structure_type_12
    } else {
        pmt.structure_type_34
    };
    pcm_structure::routing(structure).is_some_and(|r| r.join.is_some())
}

/// Relevance of `field` of PCM Synth Partial `partial` (0-based, state
/// `partial_state`) under `pmt`, [`Irrelevant`](Relevance::Irrelevant) for a
/// partial the tone doesn't have and for:
/// - the pan settings of a pair's upper partial (1 or 3) follow the lower
///   one's once the Structure merges the pair (Types 2-10): Pan Keyfollow,
///   Random and Alternate Pan Depth, and LFO1/LFO2 Pan Depth;
/// - the Step LFO plays only while LFO1 or LFO2 has the STEP waveform.
pub fn pcms_partial_relevance(
    field: PcmsPartialField,
    partial: usize,
    pmt: &PcmSynthPmt,
    partial_state: &PcmSynthPartial,
) -> Relevance {
    if partial >= PCMS_PARTIAL_COUNT {
        return Relevance::Irrelevant;
    }
    use PcmsPartialField as F;
    Relevance::applies(match field {
        F::PanKeyfollow
        | F::RandomPanDepth
        | F::AlternatePanDepth
        | F::Lfo1PanDepth
        | F::Lfo2PanDepth => partial % 2 == 1 || !pair_is_merged(pmt, partial),
        F::LfoStepType | F::LfoStep => {
            partial_state.lfo1_waveform == LFO_WAVEFORM_STEP
                || partial_state.lfo2_waveform == LFO_WAVEFORM_STEP
        }
        _ => true,
    })
}

/// Relevance of the pair of PCM Synth partials holding partial `partial`
/// (0-based): the most live of its two partials' Scopes, so switched off
/// once both are. The pair's own blocks (Booster, ring modulator) dim then.
pub fn pcms_pair_scope_relevance(pmt: &PcmSynthPmt, partial: usize) -> Relevance {
    let first = partial - partial % 2;
    pcms_partial_scope_relevance(pmt, first).min(pcms_partial_scope_relevance(pmt, first + 1))
}

/// Relevance of PMT `field` in `pmt`. A pair's Booster follows its pair
/// (see [`pcms_pair_scope_relevance`]); every other PMT field is always
/// active.
pub fn pcms_pmt_relevance(field: PcmsPmtField, pmt: &PcmSynthPmt) -> Relevance {
    match field {
        PcmsPmtField::Booster12 => pcms_pair_scope_relevance(pmt, 0),
        PcmsPmtField::Booster34 => pcms_pair_scope_relevance(pmt, 2),
        _ => Relevance::Active,
    }
}

// ---------------------------------------------------------------------------
// PCM Drum
// ---------------------------------------------------------------------------

/// Relevance of WMT `wmt`'s (0-based) Scope on a PCM Drum key in state
/// `partial`: switched off with its WMT Wave Switch. [`Irrelevant`](Relevance::Irrelevant) for
/// a WMT the key doesn't have.
pub fn pcmd_wmt_scope_relevance(partial: &PcmDrumPartial, wmt: usize) -> Relevance {
    partial.wmt.get(wmt).map_or(Relevance::Irrelevant, |w| {
        Relevance::switched(w.switch != 0)
    })
}

// ---------------------------------------------------------------------------
// SN Acoustic
// ---------------------------------------------------------------------------

/// Relevance of SN Acoustic Common `field` for the instrument `common`
/// selects: some instruments ignore the tone-wide offsets (see
/// [`sna_offsets`]).
pub fn sna_common_relevance(field: SnaCommonField, common: &SnAcousticCommon) -> Relevance {
    let offsets = sna_offsets(common.inst_variation, common.inst_number);
    use SnaCommonField as F;
    Relevance::applies(match field {
        F::CutoffOffset | F::ResonanceOffset | F::AttackTimeOffset | F::ReleaseTimeOffset => {
            offsets.tone
        }
        F::PortamentoTimeOffset => offsets.portamento,
        _ => true,
    })
}

// ---------------------------------------------------------------------------
// Mixer
// ---------------------------------------------------------------------------

/// Relevance of mixer EQ `field` on a strip (a part, a Drum Comp+EQ unit or
/// the master) whose EQ switch is `eq_switch`: the EQ's settings are
/// switched off with it. The switch itself, and every field outside the EQ,
/// is always active.
pub fn mixer_eq_relevance(field: MixerField, eq_switch: bool) -> Relevance {
    use MixerField as F;
    match field {
        F::EqLowFreq
        | F::EqLowGain
        | F::EqMidFreq
        | F::EqMidGain
        | F::EqMidQ
        | F::EqHighFreq
        | F::EqHighGain => Relevance::switched(eq_switch),
        _ => Relevance::Active,
    }
}

/// Relevance of mixer `field` on the strip of part `part` (0-based, state
/// `part_state`), given the Drum Comp+EQ state:
/// - only a drum kit routes keys to the Comp+EQ units, so the
///   [`MixerField::DrumCompEqPart`] assignment is irrelevant on any other
///   part, unless it's the one assigned, so it can be turned off;
/// - the part's EQ settings follow its EQ switch ([`mixer_eq_relevance`]).
///
/// Every other field is always active.
pub fn mixer_relevance(
    field: MixerField,
    part: usize,
    part_state: &PartState,
    drum_comp_eq: &DrumCompEqState,
) -> Relevance {
    match field {
        MixerField::DrumCompEqPart => {
            let assigned = drum_comp_eq.enabled && usize::from(drum_comp_eq.part) == part;
            Relevance::applies(assigned || params::is_drum_kit_bank(part_state.tone_bank_msb))
        }
        _ => mixer_eq_relevance(field, part_state.eq.enabled),
    }
}

/// Relevance of mixer `field` while Motional Surround is `surround_enabled`:
/// surround routes the parts and the Drum Comp+EQ units itself, so their
/// Output Assigns are irrelevant while it is on. Every other field is
/// always active.
pub fn mixer_output_relevance(field: MixerField, surround_enabled: bool) -> Relevance {
    Relevance::applies(match field {
        MixerField::PartOutputAssign | MixerField::CompOutputAssign => !surround_enabled,
        _ => true,
    })
}

/// Relevance of mixer `field` of a Drum Comp+EQ unit in state `unit`: the
/// compressor's settings are switched off with its switch, and the EQ's with
/// the EQ switch ([`mixer_eq_relevance`]). The switches themselves and the
/// unit's routing are always active.
pub fn comp_eq_unit_relevance(field: MixerField, unit: &CompEqUnit) -> Relevance {
    use MixerField as F;
    match field {
        F::CompAttack | F::CompRelease | F::CompThreshold | F::CompRatio | F::CompOutputGain => {
            Relevance::switched(unit.comp_switch)
        }
        _ => mixer_eq_relevance(field, unit.eq_switch),
    }
}

/// Relevance of the Chorus or Reverb block in state `fx`: switched off with
/// its switch, and every parameter of its type with it.
pub fn fx_block_relevance(fx: &FxState) -> Relevance {
    Relevance::switched(fx.enabled)
}

/// Relevance of Studio Set common `field` of the Chorus or Reverb block in
/// state `fx` (the Chorus's state for a Chorus field, the Reverb's for a
/// Reverb one): its type and level follow the block
/// ([`fx_block_relevance`]). The switch itself, the routing and every field
/// outside the two blocks are always active.
pub fn mixer_fx_relevance(field: MixerField, fx: &FxState) -> Relevance {
    use MixerField as F;
    match field {
        F::ChorusType | F::ChorusLevel | F::ReverbType | F::ReverbLevel => fx_block_relevance(fx),
        _ => Relevance::Active,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_spec::pcms_partial_spec;
    use crate::pcm_drum::PCMD_WMT_COUNT;
    use crate::sn_synth::SNS_PARTIAL_COUNT;

    // -- SN Synth -----------------------------------------------------------

    /// A partial playing `wave`.
    fn sns(wave: u8) -> SnSynthPartial {
        SnSynthPartial {
            osc_wave: wave,
            ..SnSynthPartial::default()
        }
    }

    #[test]
    fn sns_variation_is_inactive_for_pcm_and_super_saw() {
        let f = SnsPartialField::OscWaveVariation;
        assert_eq!(
            sns_partial_relevance(f, &sns(OSC_WAVE_PCM)),
            Relevance::Irrelevant
        );
        assert_eq!(
            sns_partial_relevance(f, &sns(OSC_WAVE_SUPER_SAW)),
            Relevance::Irrelevant
        );
        assert_eq!(sns_partial_relevance(f, &sns(0)), Relevance::Active); // SAW
    }

    #[test]
    fn sns_super_saw_detune_is_active_for_super_saw_only() {
        let f = SnsPartialField::SuperSawDetune;
        assert_eq!(
            sns_partial_relevance(f, &sns(OSC_WAVE_SUPER_SAW)),
            Relevance::Active
        );
        assert_eq!(
            sns_partial_relevance(f, &sns(OSC_WAVE_PCM)),
            Relevance::Irrelevant
        );
        assert_eq!(sns_partial_relevance(f, &sns(0)), Relevance::Irrelevant);
    }

    #[test]
    fn sns_other_fields_are_always_active() {
        assert_eq!(
            sns_partial_relevance(SnsPartialField::WaveNumber, &sns(OSC_WAVE_PCM)),
            Relevance::Active
        );
        assert_eq!(
            sns_partial_relevance(SnsPartialField::OscPitch, &sns(OSC_WAVE_SUPER_SAW)),
            Relevance::Active
        );
    }

    #[test]
    fn sns_partial_switch_sets_its_scope_active() {
        let mut common = SnSynthCommon {
            partial1_switch: 0,
            partial2_switch: 1,
            partial3_switch: 1,
            ..SnSynthCommon::default()
        };
        assert_eq!(
            sns_partial_scope_relevance(&common, 0),
            Relevance::SwitchedOff
        );
        assert_eq!(sns_partial_scope_relevance(&common, 1), Relevance::Active);
        assert_eq!(sns_partial_scope_relevance(&common, 2), Relevance::Active);
        common.partial3_switch = 0;
        assert_eq!(
            sns_partial_scope_relevance(&common, 2),
            Relevance::SwitchedOff
        );
        assert_eq!(
            sns_partial_scope_relevance(&common, SNS_PARTIAL_COUNT),
            Relevance::Irrelevant
        );
    }

    // -- PCM Synth ----------------------------------------------------------

    /// A PMT with every partial on and both pairs on Structure `structure`.
    fn pmt(structure: u8) -> PcmSynthPmt {
        let mut pmt = PcmSynthPmt {
            structure_type_12: structure,
            structure_type_34: structure,
            ..PcmSynthPmt::default()
        };
        for e in &mut pmt.partial_entries {
            e.partial_switch = 1;
        }
        pmt
    }

    const PAN_FOLLOWERS: [PcmsPartialField; 5] = [
        PcmsPartialField::PanKeyfollow,
        PcmsPartialField::RandomPanDepth,
        PcmsPartialField::AlternatePanDepth,
        PcmsPartialField::Lfo1PanDepth,
        PcmsPartialField::Lfo2PanDepth,
    ];

    #[test]
    fn pcms_upper_partial_pan_follows_the_lower_in_merged_structures() {
        let p = PcmSynthPartial::default();
        for structure in 1..=9 {
            let pmt = pmt(structure);
            for f in PAN_FOLLOWERS {
                assert_eq!(
                    pcms_partial_relevance(f, 0, &pmt, &p),
                    Relevance::Irrelevant,
                    "{f:?} type {structure}"
                );
                assert_eq!(
                    pcms_partial_relevance(f, 2, &pmt, &p),
                    Relevance::Irrelevant,
                    "{f:?} type {structure}"
                );
                assert_eq!(
                    pcms_partial_relevance(f, 1, &pmt, &p),
                    Relevance::Active,
                    "{f:?} type {structure}"
                );
                assert_eq!(
                    pcms_partial_relevance(f, 3, &pmt, &p),
                    Relevance::Active,
                    "{f:?} type {structure}"
                );
            }
            // The partial's own Pan stays its own.
            assert_eq!(
                pcms_partial_relevance(PcmsPartialField::Pan, 0, &pmt, &p),
                Relevance::Active
            );
        }
    }

    #[test]
    fn pcms_pan_is_its_own_in_independent_structure() {
        let p = PcmSynthPartial::default();
        let pmt = pmt(0);
        for f in PAN_FOLLOWERS {
            for partial in 0..PCMS_PARTIAL_COUNT {
                assert_eq!(
                    pcms_partial_relevance(f, partial, &pmt, &p),
                    Relevance::Active
                );
            }
        }
    }

    #[test]
    fn pcms_pairs_follow_their_own_structure() {
        let p = PcmSynthPartial::default();
        let mut pmt = pmt(0);
        pmt.structure_type_34 = 4;
        let f = PcmsPartialField::PanKeyfollow;
        assert_eq!(pcms_partial_relevance(f, 0, &pmt, &p), Relevance::Active);
        assert_eq!(
            pcms_partial_relevance(f, 2, &pmt, &p),
            Relevance::Irrelevant
        );
    }

    #[test]
    fn pcms_step_lfo_is_active_while_an_lfo_plays_step() {
        assert_eq!(
            pcms_partial_spec(PcmsPartialField::Lfo1Waveform)
                .format
                .format(i32::from(LFO_WAVEFORM_STEP)),
            "STEP"
        );
        let pmt = pmt(0);
        let mut p = PcmSynthPartial {
            lfo1_waveform: 0,
            lfo2_waveform: 0,
            ..PcmSynthPartial::default()
        };
        for f in [PcmsPartialField::LfoStepType, PcmsPartialField::LfoStep] {
            assert_eq!(
                pcms_partial_relevance(f, 0, &pmt, &p),
                Relevance::Irrelevant
            );
        }
        p.lfo2_waveform = LFO_WAVEFORM_STEP;
        assert_eq!(
            pcms_partial_relevance(PcmsPartialField::LfoStep, 0, &pmt, &p),
            Relevance::Active
        );
        p.lfo2_waveform = 0;
        p.lfo1_waveform = LFO_WAVEFORM_STEP;
        assert_eq!(
            pcms_partial_relevance(PcmsPartialField::LfoStepType, 0, &pmt, &p),
            Relevance::Active
        );
    }

    #[test]
    fn pcmd_wmt_switch_sets_its_scope_active() {
        let mut key = PcmDrumPartial::default();
        key.wmt[2].switch = 1;
        assert_eq!(pcmd_wmt_scope_relevance(&key, 2), Relevance::Active);
        assert_eq!(pcmd_wmt_scope_relevance(&key, 0), Relevance::SwitchedOff);
        assert_eq!(
            pcmd_wmt_scope_relevance(&key, PCMD_WMT_COUNT),
            Relevance::Irrelevant
        );
    }

    #[test]
    fn pcms_partial_switch_sets_its_scope_active() {
        let mut pmt = pmt(0);
        pmt.partial_entries[1].partial_switch = 0;
        assert_eq!(
            pcms_partial_scope_relevance(&pmt, 1),
            Relevance::SwitchedOff
        );
        assert_eq!(pcms_partial_scope_relevance(&pmt, 0), Relevance::Active);
        assert_eq!(
            pcms_partial_scope_relevance(&pmt, PCMS_PARTIAL_COUNT),
            Relevance::Irrelevant
        );
    }

    #[test]
    fn pcms_fields_of_a_missing_partial_are_inactive() {
        let p = PcmSynthPartial::default();
        let pmt = pmt(0);
        assert_eq!(
            pcms_partial_relevance(PcmsPartialField::Level, 0, &pmt, &p),
            Relevance::Active
        );
        assert_eq!(
            pcms_partial_relevance(PcmsPartialField::Level, PCMS_PARTIAL_COUNT, &pmt, &p),
            Relevance::Irrelevant
        );
    }

    #[test]
    fn pcms_booster_is_inactive_once_both_partials_of_its_pair_are_off() {
        let mut pmt = pmt(2);
        pmt.partial_entries[0].partial_switch = 0;
        assert_eq!(
            pcms_pmt_relevance(PcmsPmtField::Booster12, &pmt),
            Relevance::Active
        );
        pmt.partial_entries[1].partial_switch = 0;
        assert_eq!(
            pcms_pmt_relevance(PcmsPmtField::Booster12, &pmt),
            Relevance::SwitchedOff
        );
        assert_eq!(
            pcms_pmt_relevance(PcmsPmtField::Booster34, &pmt),
            Relevance::Active
        );
        assert_eq!(
            pcms_pmt_relevance(PcmsPmtField::StructureType12, &pmt),
            Relevance::Active
        );
        assert_eq!(pcms_pair_scope_relevance(&pmt, 1), Relevance::SwitchedOff);
        assert_eq!(pcms_pair_scope_relevance(&pmt, 3), Relevance::Active);
        assert_eq!(
            pcms_pair_scope_relevance(&pmt, PCMS_PARTIAL_COUNT),
            Relevance::Irrelevant
        );
    }

    // -- SN Acoustic --------------------------------------------------------

    fn sna(variation: u8, number: u8) -> SnAcousticCommon {
        SnAcousticCommon {
            inst_variation: variation,
            inst_number: number,
            ..SnAcousticCommon::default()
        }
    }

    const TONE_OFFSETS: [SnaCommonField; 4] = [
        SnaCommonField::CutoffOffset,
        SnaCommonField::ResonanceOffset,
        SnaCommonField::AttackTimeOffset,
        SnaCommonField::ReleaseTimeOffset,
    ];

    #[test]
    fn sna_tone_offsets_follow_the_instrument() {
        for (variation, number) in [(64, 0), (64, 8), (65, 0)] {
            for f in TONE_OFFSETS {
                assert_eq!(
                    sna_common_relevance(f, &sna(variation, number)),
                    Relevance::Irrelevant,
                    "{f:?}"
                );
            }
        }
        for f in TONE_OFFSETS {
            assert_eq!(
                sna_common_relevance(f, &sna(64, 1)),
                Relevance::Active,
                "{f:?}"
            ); // Grand Piano1
        }
    }

    #[test]
    fn sna_portamento_offset_is_inactive_for_tw_organ_only() {
        let f = SnaCommonField::PortamentoTimeOffset;
        assert_eq!(sna_common_relevance(f, &sna(65, 0)), Relevance::Irrelevant); // TW Organ
        assert_eq!(sna_common_relevance(f, &sna(64, 0)), Relevance::Active); // Concert Grand
        assert_eq!(sna_common_relevance(f, &sna(0, 4)), Relevance::Active);
    }

    #[test]
    fn sna_other_fields_are_always_active() {
        assert_eq!(
            sna_common_relevance(SnaCommonField::ToneLevel, &sna(65, 0)),
            Relevance::Active
        );
        assert_eq!(
            sna_common_relevance(SnaCommonField::VibratoRate, &sna(65, 0)),
            Relevance::Active
        );
    }

    // -- Mixer --------------------------------------------------------------

    fn part(msb: u8) -> PartState {
        PartState {
            tone_bank_msb: msb,
            ..PartState::default()
        }
    }

    #[test]
    fn mixer_comp_eq_is_active_for_drum_kits_only() {
        let off = DrumCompEqState::default();
        assert!(!off.enabled);
        assert_eq!(
            mixer_relevance(MixerField::DrumCompEqPart, 0, &part(86), &off),
            Relevance::Active
        ); // PCM drum
        assert_eq!(
            mixer_relevance(MixerField::DrumCompEqPart, 0, &part(88), &off),
            Relevance::Active
        ); // SN drum
        assert_eq!(
            mixer_relevance(MixerField::DrumCompEqPart, 0, &part(87), &off),
            Relevance::Irrelevant
        ); // PCM synth
    }

    #[test]
    fn mixer_comp_eq_stays_active_on_its_assigned_part() {
        let assigned = DrumCompEqState {
            enabled: true,
            part: 3,
            ..DrumCompEqState::default()
        };
        assert_eq!(
            mixer_relevance(MixerField::DrumCompEqPart, 3, &part(87), &assigned),
            Relevance::Active
        );
        assert_eq!(
            mixer_relevance(MixerField::DrumCompEqPart, 2, &part(87), &assigned),
            Relevance::Irrelevant
        );
        let disabled = DrumCompEqState {
            enabled: false,
            ..assigned
        };
        assert_eq!(
            mixer_relevance(MixerField::DrumCompEqPart, 3, &part(87), &disabled),
            Relevance::Irrelevant
        );
    }

    #[test]
    fn mixer_other_controls_are_always_active() {
        assert_eq!(
            mixer_relevance(
                MixerField::SoloPart,
                0,
                &part(87),
                &DrumCompEqState::default()
            ),
            Relevance::Active
        );
    }

    #[test]
    fn surround_overrides_the_output_assigns() {
        for field in [MixerField::PartOutputAssign, MixerField::CompOutputAssign] {
            assert_eq!(
                mixer_output_relevance(field, false),
                Relevance::Active,
                "{field:?}"
            );
            assert_eq!(
                mixer_output_relevance(field, true),
                Relevance::Irrelevant,
                "{field:?}"
            );
        }
        assert_eq!(
            mixer_output_relevance(MixerField::PartLevel, true),
            Relevance::Active
        );
    }

    #[test]
    fn a_switched_off_compressor_leaves_its_settings_inactive() {
        let on = CompEqUnit {
            comp_switch: true,
            eq_switch: true,
            ..CompEqUnit::default()
        };
        let off = CompEqUnit {
            comp_switch: false,
            ..on.clone()
        };
        for field in [
            MixerField::CompAttack,
            MixerField::CompRelease,
            MixerField::CompThreshold,
            MixerField::CompRatio,
            MixerField::CompOutputGain,
        ] {
            assert_eq!(
                comp_eq_unit_relevance(field, &on),
                Relevance::Active,
                "{field:?}"
            );
            assert_eq!(
                comp_eq_unit_relevance(field, &off),
                Relevance::SwitchedOff,
                "{field:?}"
            );
        }
        for field in [
            MixerField::CompSwitch,
            MixerField::EqLowGain,
            MixerField::CompOutputAssign,
        ] {
            assert_eq!(
                comp_eq_unit_relevance(field, &off),
                Relevance::Active,
                "{field:?}"
            );
        }
    }

    const EQ_SETTINGS: [MixerField; 7] = [
        MixerField::EqLowFreq,
        MixerField::EqLowGain,
        MixerField::EqMidFreq,
        MixerField::EqMidGain,
        MixerField::EqMidQ,
        MixerField::EqHighFreq,
        MixerField::EqHighGain,
    ];

    #[test]
    fn a_switched_off_eq_leaves_its_settings_inactive() {
        for field in EQ_SETTINGS {
            assert_eq!(
                mixer_eq_relevance(field, true),
                Relevance::Active,
                "{field:?}"
            );
            assert_eq!(
                mixer_eq_relevance(field, false),
                Relevance::SwitchedOff,
                "{field:?}"
            );
        }
        assert_eq!(
            mixer_eq_relevance(MixerField::EqSwitch, false),
            Relevance::Active
        );
        assert_eq!(
            mixer_eq_relevance(MixerField::PartLevel, false),
            Relevance::Active
        );
    }

    #[test]
    fn a_part_and_a_comp_eq_unit_follow_their_own_eq_switch() {
        let mut p = part(87);
        p.eq.enabled = false;
        let none = DrumCompEqState::default();
        let unit = CompEqUnit {
            comp_switch: true,
            eq_switch: false,
            ..CompEqUnit::default()
        };
        for field in EQ_SETTINGS {
            assert_eq!(
                mixer_relevance(field, 0, &p, &none),
                Relevance::SwitchedOff,
                "{field:?}"
            );
            assert_eq!(
                comp_eq_unit_relevance(field, &unit),
                Relevance::SwitchedOff,
                "{field:?}"
            );
        }
        assert_eq!(
            mixer_relevance(MixerField::EqSwitch, 0, &p, &none),
            Relevance::Active
        );
        assert_eq!(
            comp_eq_unit_relevance(MixerField::EqSwitch, &unit),
            Relevance::Active
        );
        assert_eq!(
            comp_eq_unit_relevance(MixerField::CompAttack, &unit),
            Relevance::Active
        );
        p.eq.enabled = true;
        assert_eq!(
            mixer_relevance(MixerField::EqMidQ, 0, &p, &none),
            Relevance::Active
        );
    }

    #[test]
    fn a_switched_off_fx_leaves_its_type_level_and_parameters_inactive() {
        let on = FxState::default();
        assert!(on.enabled);
        let off = FxState {
            enabled: false,
            ..FxState::default()
        };
        assert_eq!(fx_block_relevance(&on), Relevance::Active);
        assert_eq!(fx_block_relevance(&off), Relevance::SwitchedOff);
        for field in [
            MixerField::ChorusType,
            MixerField::ChorusLevel,
            MixerField::ReverbType,
            MixerField::ReverbLevel,
        ] {
            assert_eq!(
                mixer_fx_relevance(field, &on),
                Relevance::Active,
                "{field:?}"
            );
            assert_eq!(
                mixer_fx_relevance(field, &off),
                Relevance::SwitchedOff,
                "{field:?}"
            );
        }
        for field in [
            MixerField::ChorusSwitch,
            MixerField::ChorusOutputAssign,
            MixerField::ChorusOutputSelect,
            MixerField::ReverbSwitch,
            MixerField::ReverbOutputAssign,
        ] {
            assert_eq!(
                mixer_fx_relevance(field, &off),
                Relevance::Active,
                "{field:?}"
            );
        }
    }
}
