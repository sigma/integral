//! Field relevance: whether a field is active given the device state.
//!
//! Relevance is device knowledge, so it lives here beside
//! [`FieldSpec`](crate::field_spec::FieldSpec) rather than in a layout. A
//! layout only says how an inactive Control looks (dimmed or hidden); it
//! never holds the predicate. Each engine has one `*_is_active` function per
//! block, taking the field and the state it depends on.
//!
//! Relevance comes in two layers. A partial switched off makes its whole
//! Scope inactive, and the Section holding it dims as one: the
//! `*_scope_is_active` functions answer that. A field's own `*_is_active`
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

// ---------------------------------------------------------------------------
// SN Synth
// ---------------------------------------------------------------------------

/// Whether SN Synth Partial `partial` (0-based) is switched on. `false` for
/// a partial the tone doesn't have.
pub fn sns_partial_scope_is_active(common: &SnSynthCommon, partial: usize) -> bool {
    let switch = match partial {
        0 => common.partial1_switch,
        1 => common.partial2_switch,
        2 => common.partial3_switch,
        _ => return false,
    };
    switch != 0
}

/// Whether `field` of an SN Synth Partial in state `partial` is active: its
/// OSC Wave uses it. Wave Variation has no effect on the PCM and Super Saw
/// waves; Super Saw Detune acts on Super Saw only.
pub fn sns_partial_is_active(field: SnsPartialField, partial: &SnSynthPartial) -> bool {
    let wave = partial.osc_wave;
    match field {
        SnsPartialField::OscWaveVariation => wave != OSC_WAVE_PCM && wave != OSC_WAVE_SUPER_SAW,
        SnsPartialField::SuperSawDetune => wave == OSC_WAVE_SUPER_SAW,
        _ => true,
    }
}

// ---------------------------------------------------------------------------
// PCM Synth
// ---------------------------------------------------------------------------

/// Whether PCM Synth Partial `partial` (0-based) is switched on (PMT Partial
/// Switch). `false` for a partial the tone doesn't have.
pub fn pcms_partial_scope_is_active(pmt: &PcmSynthPmt, partial: usize) -> bool {
    pmt.partial_entries
        .get(partial)
        .is_some_and(|e| e.partial_switch != 0)
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

/// Whether `field` of PCM Synth Partial `partial` (0-based, state
/// `partial_state`) is active under `pmt`, `false` for a partial the tone
/// doesn't have:
/// - the pan settings of a pair's upper partial (1 or 3) follow the lower
///   one's once the Structure merges the pair (Types 2-10): Pan Keyfollow,
///   Random and Alternate Pan Depth, and LFO1/LFO2 Pan Depth;
/// - the Step LFO plays only while LFO1 or LFO2 has the STEP waveform.
pub fn pcms_partial_is_active(
    field: PcmsPartialField,
    partial: usize,
    pmt: &PcmSynthPmt,
    partial_state: &PcmSynthPartial,
) -> bool {
    if partial >= PCMS_PARTIAL_COUNT {
        return false;
    }
    use PcmsPartialField as F;
    match field {
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
    }
}

/// Whether the pair of PCM Synth partials holding partial `partial`
/// (0-based) is active: either of its partials is switched on. The pair's
/// own blocks (Booster, ring modulator) dim once both are off.
pub fn pcms_pair_scope_is_active(pmt: &PcmSynthPmt, partial: usize) -> bool {
    let first = partial - partial % 2;
    pcms_partial_scope_is_active(pmt, first) || pcms_partial_scope_is_active(pmt, first + 1)
}

/// Whether PMT `field` is active in `pmt`. A pair's Booster is active while
/// its pair is (see [`pcms_pair_scope_is_active`]); every other PMT field
/// always is.
pub fn pcms_pmt_is_active(field: PcmsPmtField, pmt: &PcmSynthPmt) -> bool {
    match field {
        PcmsPmtField::Booster12 => pcms_pair_scope_is_active(pmt, 0),
        PcmsPmtField::Booster34 => pcms_pair_scope_is_active(pmt, 2),
        _ => true,
    }
}

// ---------------------------------------------------------------------------
// PCM Drum
// ---------------------------------------------------------------------------

/// Whether WMT `wmt` (0-based) of a PCM Drum key in state `partial` is
/// switched on (WMT Wave Switch). `false` for a WMT the key doesn't have.
pub fn pcmd_wmt_scope_is_active(partial: &PcmDrumPartial, wmt: usize) -> bool {
    partial.wmt.get(wmt).is_some_and(|w| w.switch != 0)
}

// ---------------------------------------------------------------------------
// SN Acoustic
// ---------------------------------------------------------------------------

/// Whether SN Acoustic Common `field` is active for the instrument `common`
/// selects: some instruments ignore the tone-wide offsets (see
/// [`sna_offsets`]).
pub fn sna_common_is_active(field: SnaCommonField, common: &SnAcousticCommon) -> bool {
    let offsets = sna_offsets(common.inst_variation, common.inst_number);
    use SnaCommonField as F;
    match field {
        F::CutoffOffset | F::ResonanceOffset | F::AttackTimeOffset | F::ReleaseTimeOffset => {
            offsets.tone
        }
        F::PortamentoTimeOffset => offsets.portamento,
        _ => true,
    }
}

// ---------------------------------------------------------------------------
// Mixer
// ---------------------------------------------------------------------------

/// Whether mixer EQ `field` is active on a strip (a part, a Drum Comp+EQ
/// unit or the master) whose EQ switch is `eq_switch`: the EQ's settings act
/// only while it is on. The switch itself, and every field outside the EQ,
/// is always active.
pub fn mixer_eq_is_active(field: MixerField, eq_switch: bool) -> bool {
    use MixerField as F;
    match field {
        F::EqLowFreq
        | F::EqLowGain
        | F::EqMidFreq
        | F::EqMidGain
        | F::EqMidQ
        | F::EqHighFreq
        | F::EqHighGain => eq_switch,
        _ => true,
    }
}

/// Whether mixer `field` on the strip of part `part` (0-based, state
/// `part_state`) is active, given the Drum Comp+EQ state:
/// - only a drum kit routes keys to the Comp+EQ units, so the
///   [`MixerField::DrumCompEqPart`] assignment is active on a drum kit part,
///   or on the part it's assigned to so it can be turned off;
/// - the part's EQ settings follow its EQ switch ([`mixer_eq_is_active`]).
///
/// Every other field is always active.
pub fn mixer_is_active(
    field: MixerField,
    part: usize,
    part_state: &PartState,
    drum_comp_eq: &DrumCompEqState,
) -> bool {
    match field {
        MixerField::DrumCompEqPart => {
            let assigned = drum_comp_eq.enabled && usize::from(drum_comp_eq.part) == part;
            assigned || params::is_drum_kit_bank(part_state.tone_bank_msb)
        }
        _ => mixer_eq_is_active(field, part_state.eq.enabled),
    }
}

/// Whether mixer `field` is active while Motional Surround is
/// `surround_enabled`: surround routes the parts and the Drum Comp+EQ units
/// itself, so their Output Assigns have no effect while it is on. Every
/// other field is always active.
pub fn mixer_output_is_active(field: MixerField, surround_enabled: bool) -> bool {
    match field {
        MixerField::PartOutputAssign | MixerField::CompOutputAssign => !surround_enabled,
        _ => true,
    }
}

/// Whether mixer `field` of a Drum Comp+EQ unit in state `unit` is active:
/// the compressor's settings act only while its switch is on, and the EQ's
/// while the EQ switch is ([`mixer_eq_is_active`]). The switches themselves
/// and the unit's routing are always active.
pub fn comp_eq_unit_is_active(field: MixerField, unit: &CompEqUnit) -> bool {
    use MixerField as F;
    match field {
        F::CompAttack | F::CompRelease | F::CompThreshold | F::CompRatio | F::CompOutputGain => {
            unit.comp_switch
        }
        _ => mixer_eq_is_active(field, unit.eq_switch),
    }
}

/// Whether the Chorus or Reverb block in state `fx` acts: while its switch
/// is on. A block switched off leaves every parameter of its type inactive.
pub fn fx_block_is_active(fx: &FxState) -> bool {
    fx.enabled
}

/// Whether Studio Set common `field` of the Chorus or Reverb block in state
/// `fx` (the Chorus's state for a Chorus field, the Reverb's for a Reverb
/// one) is active: its type and level act only while its switch is on
/// ([`fx_block_is_active`]). The switch itself, the routing and every field
/// outside the two blocks are always active.
pub fn mixer_fx_is_active(field: MixerField, fx: &FxState) -> bool {
    use MixerField as F;
    match field {
        F::ChorusType | F::ChorusLevel | F::ReverbType | F::ReverbLevel => fx_block_is_active(fx),
        _ => true,
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
        assert!(!sns_partial_is_active(f, &sns(OSC_WAVE_PCM)));
        assert!(!sns_partial_is_active(f, &sns(OSC_WAVE_SUPER_SAW)));
        assert!(sns_partial_is_active(f, &sns(0))); // SAW
    }

    #[test]
    fn sns_super_saw_detune_is_active_for_super_saw_only() {
        let f = SnsPartialField::SuperSawDetune;
        assert!(sns_partial_is_active(f, &sns(OSC_WAVE_SUPER_SAW)));
        assert!(!sns_partial_is_active(f, &sns(OSC_WAVE_PCM)));
        assert!(!sns_partial_is_active(f, &sns(0)));
    }

    #[test]
    fn sns_other_fields_are_always_active() {
        assert!(sns_partial_is_active(
            SnsPartialField::WaveNumber,
            &sns(OSC_WAVE_PCM)
        ));
        assert!(sns_partial_is_active(
            SnsPartialField::OscPitch,
            &sns(OSC_WAVE_SUPER_SAW)
        ));
    }

    #[test]
    fn sns_partial_switch_sets_its_scope_active() {
        let mut common = SnSynthCommon {
            partial1_switch: 0,
            partial2_switch: 1,
            partial3_switch: 1,
            ..SnSynthCommon::default()
        };
        assert!(!sns_partial_scope_is_active(&common, 0));
        assert!(sns_partial_scope_is_active(&common, 1));
        assert!(sns_partial_scope_is_active(&common, 2));
        common.partial3_switch = 0;
        assert!(!sns_partial_scope_is_active(&common, 2));
        assert!(!sns_partial_scope_is_active(&common, SNS_PARTIAL_COUNT));
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
                assert!(
                    !pcms_partial_is_active(f, 0, &pmt, &p),
                    "{f:?} type {structure}"
                );
                assert!(
                    !pcms_partial_is_active(f, 2, &pmt, &p),
                    "{f:?} type {structure}"
                );
                assert!(
                    pcms_partial_is_active(f, 1, &pmt, &p),
                    "{f:?} type {structure}"
                );
                assert!(
                    pcms_partial_is_active(f, 3, &pmt, &p),
                    "{f:?} type {structure}"
                );
            }
            // The partial's own Pan stays its own.
            assert!(pcms_partial_is_active(PcmsPartialField::Pan, 0, &pmt, &p));
        }
    }

    #[test]
    fn pcms_pan_is_its_own_in_independent_structure() {
        let p = PcmSynthPartial::default();
        let pmt = pmt(0);
        for f in PAN_FOLLOWERS {
            for partial in 0..PCMS_PARTIAL_COUNT {
                assert!(pcms_partial_is_active(f, partial, &pmt, &p));
            }
        }
    }

    #[test]
    fn pcms_pairs_follow_their_own_structure() {
        let p = PcmSynthPartial::default();
        let mut pmt = pmt(0);
        pmt.structure_type_34 = 4;
        let f = PcmsPartialField::PanKeyfollow;
        assert!(pcms_partial_is_active(f, 0, &pmt, &p));
        assert!(!pcms_partial_is_active(f, 2, &pmt, &p));
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
            assert!(!pcms_partial_is_active(f, 0, &pmt, &p));
        }
        p.lfo2_waveform = LFO_WAVEFORM_STEP;
        assert!(pcms_partial_is_active(
            PcmsPartialField::LfoStep,
            0,
            &pmt,
            &p
        ));
        p.lfo2_waveform = 0;
        p.lfo1_waveform = LFO_WAVEFORM_STEP;
        assert!(pcms_partial_is_active(
            PcmsPartialField::LfoStepType,
            0,
            &pmt,
            &p
        ));
    }

    #[test]
    fn pcmd_wmt_switch_sets_its_scope_active() {
        let mut key = PcmDrumPartial::default();
        key.wmt[2].switch = 1;
        assert!(pcmd_wmt_scope_is_active(&key, 2));
        assert!(!pcmd_wmt_scope_is_active(&key, 0));
        assert!(!pcmd_wmt_scope_is_active(&key, PCMD_WMT_COUNT));
    }

    #[test]
    fn pcms_partial_switch_sets_its_scope_active() {
        let mut pmt = pmt(0);
        pmt.partial_entries[1].partial_switch = 0;
        assert!(!pcms_partial_scope_is_active(&pmt, 1));
        assert!(pcms_partial_scope_is_active(&pmt, 0));
        assert!(!pcms_partial_scope_is_active(&pmt, PCMS_PARTIAL_COUNT));
    }

    #[test]
    fn pcms_fields_of_a_missing_partial_are_inactive() {
        let p = PcmSynthPartial::default();
        let pmt = pmt(0);
        assert!(pcms_partial_is_active(PcmsPartialField::Level, 0, &pmt, &p));
        assert!(!pcms_partial_is_active(
            PcmsPartialField::Level,
            PCMS_PARTIAL_COUNT,
            &pmt,
            &p
        ));
    }

    #[test]
    fn pcms_booster_is_inactive_once_both_partials_of_its_pair_are_off() {
        let mut pmt = pmt(2);
        pmt.partial_entries[0].partial_switch = 0;
        assert!(pcms_pmt_is_active(PcmsPmtField::Booster12, &pmt));
        pmt.partial_entries[1].partial_switch = 0;
        assert!(!pcms_pmt_is_active(PcmsPmtField::Booster12, &pmt));
        assert!(pcms_pmt_is_active(PcmsPmtField::Booster34, &pmt));
        assert!(pcms_pmt_is_active(PcmsPmtField::StructureType12, &pmt));
        assert!(!pcms_pair_scope_is_active(&pmt, 1));
        assert!(pcms_pair_scope_is_active(&pmt, 3));
        assert!(!pcms_pair_scope_is_active(&pmt, PCMS_PARTIAL_COUNT));
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
                assert!(!sna_common_is_active(f, &sna(variation, number)), "{f:?}");
            }
        }
        for f in TONE_OFFSETS {
            assert!(sna_common_is_active(f, &sna(64, 1)), "{f:?}"); // Grand Piano1
        }
    }

    #[test]
    fn sna_portamento_offset_is_inactive_for_tw_organ_only() {
        let f = SnaCommonField::PortamentoTimeOffset;
        assert!(!sna_common_is_active(f, &sna(65, 0))); // TW Organ
        assert!(sna_common_is_active(f, &sna(64, 0))); // Concert Grand
        assert!(sna_common_is_active(f, &sna(0, 4)));
    }

    #[test]
    fn sna_other_fields_are_always_active() {
        assert!(sna_common_is_active(SnaCommonField::ToneLevel, &sna(65, 0)));
        assert!(sna_common_is_active(
            SnaCommonField::VibratoRate,
            &sna(65, 0)
        ));
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
        assert!(mixer_is_active(
            MixerField::DrumCompEqPart,
            0,
            &part(86),
            &off
        )); // PCM drum
        assert!(mixer_is_active(
            MixerField::DrumCompEqPart,
            0,
            &part(88),
            &off
        )); // SN drum
        assert!(!mixer_is_active(
            MixerField::DrumCompEqPart,
            0,
            &part(87),
            &off
        )); // PCM synth
    }

    #[test]
    fn mixer_comp_eq_stays_active_on_its_assigned_part() {
        let assigned = DrumCompEqState {
            enabled: true,
            part: 3,
            ..DrumCompEqState::default()
        };
        assert!(mixer_is_active(
            MixerField::DrumCompEqPart,
            3,
            &part(87),
            &assigned
        ));
        assert!(!mixer_is_active(
            MixerField::DrumCompEqPart,
            2,
            &part(87),
            &assigned
        ));
        let disabled = DrumCompEqState {
            enabled: false,
            ..assigned
        };
        assert!(!mixer_is_active(
            MixerField::DrumCompEqPart,
            3,
            &part(87),
            &disabled
        ));
    }

    #[test]
    fn mixer_other_controls_are_always_active() {
        assert!(mixer_is_active(
            MixerField::SoloPart,
            0,
            &part(87),
            &DrumCompEqState::default()
        ));
    }

    #[test]
    fn surround_overrides_the_output_assigns() {
        for field in [MixerField::PartOutputAssign, MixerField::CompOutputAssign] {
            assert!(mixer_output_is_active(field, false), "{field:?}");
            assert!(!mixer_output_is_active(field, true), "{field:?}");
        }
        assert!(mixer_output_is_active(MixerField::PartLevel, true));
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
            assert!(comp_eq_unit_is_active(field, &on), "{field:?}");
            assert!(!comp_eq_unit_is_active(field, &off), "{field:?}");
        }
        for field in [
            MixerField::CompSwitch,
            MixerField::EqLowGain,
            MixerField::CompOutputAssign,
        ] {
            assert!(comp_eq_unit_is_active(field, &off), "{field:?}");
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
            assert!(mixer_eq_is_active(field, true), "{field:?}");
            assert!(!mixer_eq_is_active(field, false), "{field:?}");
        }
        assert!(mixer_eq_is_active(MixerField::EqSwitch, false));
        assert!(mixer_eq_is_active(MixerField::PartLevel, false));
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
            assert!(!mixer_is_active(field, 0, &p, &none), "{field:?}");
            assert!(!comp_eq_unit_is_active(field, &unit), "{field:?}");
        }
        assert!(mixer_is_active(MixerField::EqSwitch, 0, &p, &none));
        assert!(comp_eq_unit_is_active(MixerField::EqSwitch, &unit));
        assert!(comp_eq_unit_is_active(MixerField::CompAttack, &unit));
        p.eq.enabled = true;
        assert!(mixer_is_active(MixerField::EqMidQ, 0, &p, &none));
    }

    #[test]
    fn a_switched_off_fx_leaves_its_type_level_and_parameters_inactive() {
        let on = FxState::default();
        assert!(on.enabled);
        let off = FxState {
            enabled: false,
            ..FxState::default()
        };
        assert!(fx_block_is_active(&on));
        assert!(!fx_block_is_active(&off));
        for field in [
            MixerField::ChorusType,
            MixerField::ChorusLevel,
            MixerField::ReverbType,
            MixerField::ReverbLevel,
        ] {
            assert!(mixer_fx_is_active(field, &on), "{field:?}");
            assert!(!mixer_fx_is_active(field, &off), "{field:?}");
        }
        for field in [
            MixerField::ChorusSwitch,
            MixerField::ChorusOutputAssign,
            MixerField::ChorusOutputSelect,
            MixerField::ReverbSwitch,
            MixerField::ReverbOutputAssign,
        ] {
            assert!(mixer_fx_is_active(field, &off), "{field:?}");
        }
    }
}
