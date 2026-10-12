//! The Foreign effect map: which INTEGRA-7 MFX type a JV/XP EFX type or an
//! XV MFX type becomes, and how its parameters convert, generated at build
//! time from `docs/research/foreign-fx-map.json`.
//!
//! Shared by every Foreign patch converter. XV MFX types 1–40 store the JV
//! EFX parameters verbatim, so they convert with the JV rows. Reference:
//! `docs/research/foreign-fx-map.md` and `docs/research/map-check-roland-pairs.md`.

use super::report::{ConversionItem, ConversionReport, EffectFamily, ForeignEffect};
use crate::mfx::{MFX_PARAM_COUNT, MFX_PARAMS_START, encode_mfx_param, parse_mfx_block};
use crate::mfx_params::{default_slots, mfx_type_def};

/// MFX Parameters 1–32, as stored values.
pub(super) type Slots = [i32; MFX_PARAM_COUNT];

/// How a source type lands on the INTEGRA-7 (`classes` of the map).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    /// Every parameter copies.
    Same,
    /// Every parameter lands, some through a formula.
    Rescaled,
    /// Some parameters have no target.
    RescaledLossy,
    /// A different algorithm: the closest INTEGRA-7 type.
    Substitute,
    /// No INTEGRA-7 type: THRU.
    None,
}

/// How a JV EFX value becomes MFX Parameter values (`conversions` of the
/// map).
#[derive(Debug, Clone, Copy)]
enum Conv {
    /// The same value.
    Copy,
    /// No INTEGRA-7 equivalent.
    Drop,
    /// 4/8 kHz onto the 2/4/8 kHz list.
    EqHiJv,
    /// A rate in Hz, with its sync switch.
    RateJv,
    /// A rate in Hz, with no sync switch.
    RateJvPlain,
    /// A Step Rate in Hz, at half the resolution.
    StepRateJv,
    /// A delay of up to 500 ms.
    Delay500Jv,
    /// A delay of 200–1000 ms, or a note.
    Delay1000Jv,
    /// The Time Control Delay's 200–1000 ms.
    DelayTcdJv,
    /// ×1/×2/×4/×8 onto 0–18 dB.
    PostGain,
    /// Sustain has no Threshold formula: the INIT value stays.
    SustainToThreshold,
    /// A Hz table onto a 0–127 knob with no published curve.
    PhaserManualJv,
    /// One balance onto two pitch levels.
    LevelBalance,
}

/// One JV EFX parameter and where it lands.
struct ParamRow {
    /// The JV EFX Parameter number (1–12).
    from: u8,
    /// Its JV name.
    name: &'static str,
    conv: Conv,
    /// MFX Parameter numbers (1–32) it writes.
    to: &'static [u8],
    /// MFX Parameter number of its sync switch.
    sync: Option<u8>,
    /// MFX Parameter number of its note.
    note: Option<u8>,
    /// For a dropped parameter, the value that loses nothing.
    neutral: Option<u8>,
}

/// A JV EFX type (and XV MFX type of the same number).
struct JvEffect {
    name: &'static str,
    class: Class,
    /// The INTEGRA-7 MFX type, 0 for none.
    target: u8,
    rows: &'static [ParamRow],
    /// `(MFX Parameter number, value)` of the targets with no source.
    unmapped: &'static [(u8, i32)],
}

/// An XV MFX type 41–90.
struct XvEffect {
    name: &'static str,
    class: Class,
    /// The INTEGRA-7 MFX type, 0 for none.
    target: u8,
}

include!(concat!(env!("OUT_DIR"), "/foreign_fx_map_data.rs"));

/// XV MFX types stored like JV EFX types.
const XV_JV_TYPES: u8 = 40;

/// A map entry.
enum Entry {
    Jv(&'static JvEffect),
    Xv(&'static XvEffect),
}

impl ForeignEffect {
    /// The type's name on the source synth, when the map lists it.
    pub fn name(self) -> Option<&'static str> {
        match self.entry()? {
            Entry::Jv(e) => Some(e.name),
            Entry::Xv(e) => Some(e.name),
        }
    }

    fn entry(self) -> Option<Entry> {
        let n = usize::from(self.number);
        match self.family {
            EffectFamily::JvEfx => JV_EFX.get(n.checked_sub(1)?).map(Entry::Jv),
            EffectFamily::XvMfx if self.number <= XV_JV_TYPES => {
                JV_EFX.get(n.checked_sub(1)?).map(Entry::Jv)
            }
            EffectFamily::XvMfx => XV_MFX.get(n - usize::from(XV_JV_TYPES) - 1).map(Entry::Xv),
        }
    }
}

/// Convert `effect`, whose MFX Parameters 1–32 hold `source` (a JV EFX
/// writes its Parameters 1–12 there), into the MFX Type and Parameters of
/// `mfx`, an INTEGRA-7 MFX block. Returns whether it converted: an effect
/// with no target leaves `mfx` THRU.
pub(super) fn convert(
    effect: ForeignEffect,
    source: &Slots,
    mfx: &mut [u8],
    report: &mut ConversionReport,
) -> bool {
    // `(target, slots, substituted)`
    let converted = match effect.entry() {
        Some(Entry::Jv(e)) if e.class != Class::None => Some((
            e.target,
            jv_slots(e, source, report),
            e.class == Class::Substitute,
        )),
        // ponytail: XV 41–90 are read in the INTEGRA-7 encoding, which no
        // dump has checked (foreign-fx-map.md, Unresolved 1); only the
        // `same` types count as faithful until it is.
        Some(Entry::Xv(e)) if e.class == Class::Substitute => Some((
            e.target,
            default_slots(e.target).expect("a mapped type"),
            true,
        )),
        Some(Entry::Xv(e)) if e.class != Class::None => {
            Some((e.target, *source, e.class != Class::Same))
        }
        _ => None,
    };
    let Some((target, slots, substituted)) = converted else {
        mfx[0] = 0;
        report.push(ConversionItem::EffectDropped { source: effect });
        return false;
    };
    if substituted {
        report.push(ConversionItem::EffectSubstituted {
            source: effect,
            target,
        });
    }
    mfx[0] = target;
    for (i, value) in slots.into_iter().enumerate() {
        let at = usize::from(MFX_PARAMS_START) + 4 * i;
        mfx[at..at + 4].copy_from_slice(&encode_mfx_param(value));
    }
    true
}

/// The MFX Parameters of JV effect `e` with Parameters `source`: the target
/// type's defaults, the map's values for targets with no source, then each
/// row. Dropped parameters are reported.
fn jv_slots(e: &JvEffect, source: &Slots, report: &mut ConversionReport) -> Slots {
    let mut slots = default_slots(e.target).expect("a mapped type");
    for &(n, value) in e.unmapped {
        slots[usize::from(n) - 1] = value;
    }
    for row in e.rows {
        let v = source[usize::from(row.from) - 1];
        if !apply(row, v, &mut slots) && row.neutral.map(i32::from) != Some(v) {
            report.push(ConversionItem::ParamDropped {
                at: None,
                param: row.name,
            });
        }
    }
    slots
}

/// Write JV value `v` through `row`. Returns `false` when the value has no
/// target. A value out of the JV range leaves the INIT value.
fn apply(row: &ParamRow, v: i32, slots: &mut Slots) -> bool {
    let mut set = |n: u8, value: i32| slots[usize::from(n) - 1] = value;
    let value = match row.conv {
        Conv::Drop | Conv::SustainToThreshold => return false,
        // shortcut: the INTEGRA-7 Manual has no published Hz curve, so the
        // raw value copies; PHASER is reported as a substitute.
        Conv::Copy | Conv::PhaserManualJv => Some(v),
        // ponytail: the map's +1, against Roland's raw copy; settled by ear
        // in the device checks (#219, item 5).
        Conv::EqHiJv => Some(v + 1),
        Conv::RateJv | Conv::RateJvPlain => rate_20ths(v),
        Conv::StepRateJv => rate_20ths(v).map(|h| ((h + 1) / 2).max(1)),
        Conv::Delay500Jv => delay500_tenths(v).map(|t| (t + 5) / 10),
        Conv::Delay1000Jv if (116..=125).contains(&v) => {
            // ponytail: the note order is assumed to be XV list *1
            // (foreign-fx-map.md, Unresolved 2).
            set(row.sync.expect("a synced delay"), 1);
            set(row.note.expect("a synced delay"), 6 + v - 116);
            return true;
        }
        Conv::Delay1000Jv => match v {
            0..70 => Some(200 + 5 * v),
            70..116 => Some(550 + 10 * (v - 70)),
            _ => None,
        },
        Conv::DelayTcdJv => match v {
            0..80 => Some(200 + 5 * v),
            80..=120 => Some(600 + 10 * (v - 80)),
            _ => None,
        },
        Conv::PostGain => Some(6 * v),
        Conv::LevelBalance => {
            // ponytail: a linear crossfade, as the crossfade law is not
            // published (foreign-fx-map.md, Unresolved 3).
            if (0..=100).contains(&v) {
                let level = |share: i32| (2 * 127 * share.min(50) + 50) / 100;
                set(row.to[0], level(100 - v));
                set(row.to[1], level(v));
            }
            return true;
        }
    };
    if let Some(value) = value {
        if let Some(sync) = row.sync {
            set(sync, 0);
        }
        set(row.to[0], value);
    }
    true
}

/// A JV rate (0–125) in 1/20 Hz: 0.05–5 Hz by 0.05, to 7 Hz by 0.1, to
/// 10 Hz by 0.5.
fn rate_20ths(v: i32) -> Option<i32> {
    match v {
        0..99 => Some(v + 1),
        99..119 => Some(100 + 2 * (v - 99)),
        119..=125 => Some(140 + 10 * (v - 119)),
        _ => None,
    }
}

/// A JV short delay (0–126) in 1/10 ms: 0–5 ms by 0.1, to 10 by 0.5, to 40
/// by 1, to 300 by 10, to 500 by 20.
fn delay500_tenths(v: i32) -> Option<i32> {
    match v {
        0..50 => Some(v),
        50..60 => Some(50 + 5 * (v - 50)),
        60..90 => Some(100 + 10 * (v - 60)),
        90..116 => Some(400 + 100 * (v - 90)),
        116..=126 => Some(3000 + 200 * (v - 116)),
        _ => None,
    }
}

/// Whether MFX block `mfx` follows the tempo: one of its type's synced
/// parameters is set to a note.
pub(super) fn syncs_to_tempo(mfx: &[u8]) -> bool {
    let state = parse_mfx_block(mfx);
    mfx_type_def(state.mfx_type).is_some_and(|def| {
        def.params
            .iter()
            .filter_map(|p| p.sync)
            .any(|s| state.params.get(usize::from(s.switch_index)) == Some(&1))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::librarian::foreign::pcm_tone;
    use crate::mfx::MfxState;

    fn jv(number: u8) -> ForeignEffect {
        ForeignEffect {
            family: EffectFamily::JvEfx,
            number,
        }
    }

    fn xv(number: u8) -> ForeignEffect {
        ForeignEffect {
            family: EffectFamily::XvMfx,
            number,
        }
    }

    /// JV Parameters 1–12 as MFX source slots.
    fn source(params: &[i32]) -> Slots {
        let mut slots = [0; MFX_PARAM_COUNT];
        slots[..params.len()].copy_from_slice(params);
        slots
    }

    /// Convert `effect`: its MFX block and report.
    fn converted(effect: ForeignEffect, params: &[i32]) -> (MfxState, ConversionReport) {
        let mut mfx = pcm_tone::init_mfx(MfxState::default());
        let mut report = ConversionReport::new();
        convert(effect, &source(params), &mut mfx, &mut report);
        (parse_mfx_block(&mfx), report)
    }

    #[test]
    fn names_the_source_types() {
        assert_eq!(jv(1).name(), Some("STEREO-EQ"));
        assert_eq!(xv(19).name(), Some("TRIPLE-TAP-DELAY"));
        assert_eq!(xv(90).name(), Some("3D MANUAL"));
        assert_eq!([jv(0), jv(41), xv(91)].map(ForeignEffect::name), [None; 3]);
    }

    #[test]
    fn every_row_writes_a_slot_its_target_type_has() {
        for (i, e) in JV_EFX.iter().enumerate().filter(|(_, e)| e.target != 0) {
            let def = mfx_type_def(e.target).expect("an MFX type");
            let slots: Vec<u8> = def
                .params
                .iter()
                .flat_map(|p| {
                    let sync = p.sync.map(|s| [s.switch_index, s.note_index]);
                    [Some(p.index)]
                        .into_iter()
                        .chain(sync.into_iter().flatten().map(Some))
                })
                .flatten()
                .map(|slot| slot + 1)
                .collect();
            let written = e
                .rows
                .iter()
                .flat_map(|r| r.to.iter().copied().chain(r.sync).chain(r.note));
            // `unmapped` may set hidden slots (COMPRESSOR's 7th), which
            // `mfx_params` leaves out.
            for n in written {
                assert!(slots.contains(&n), "EFX type {}: Parameter {n}", i + 1);
            }
        }
    }

    #[test]
    fn rates_and_delays_hit_the_tables_ends() {
        assert_eq!(
            [0, 98, 99, 118, 119, 125].map(rate_20ths),
            [
                Some(1),
                Some(99),
                Some(100),
                Some(138),
                Some(140),
                Some(200)
            ]
        );
        assert_eq!(rate_20ths(126), None);
        assert_eq!(
            [0, 49, 50, 60, 90, 116, 126].map(delay500_tenths),
            [
                Some(0),
                Some(49),
                Some(50),
                Some(100),
                Some(400),
                Some(3000),
                Some(5000)
            ]
        );
    }

    #[test]
    fn copies_and_rescales_every_jv_parameter() {
        // TRIPLE-TAP-DELAY: three 200–1000 ms delays, feedback, levels.
        let (mfx, report) = converted(jv(19), &[0, 69, 115, 50, 10, 127, 100, 90, 15, 15, 80, 127]);
        assert_eq!(mfx.mfx_type, 36);
        // Each delay: sync switch (msec), time, then its note left at INIT.
        assert_eq!(mfx.params[..9], [0, 200, 10, 0, 545, 13, 0, 1000, 15]);
        assert_eq!(mfx.params[9..18], [50, 10, 127, 100, 90, 15, 15, 80, 127]);
        assert!(report.is_empty(), "{report:?}");
    }

    #[test]
    fn a_note_delay_switches_to_sync() {
        let (mfx, _) = converted(jv(19), &[116, 125, 0]);
        assert_eq!(mfx.params[..6], [1, 400, 6, 1, 800, 15]);
        let mut block = pcm_tone::init_mfx(MfxState::default());
        convert(jv(19), &source(&[116]), &mut block, &mut Vec::new());
        assert!(syncs_to_tempo(&block));
        convert(jv(19), &source(&[0, 0, 0]), &mut block, &mut Vec::new());
        assert!(!syncs_to_tempo(&block));
    }

    #[test]
    fn applies_the_named_conversions() {
        // STEREO-EQ High Frequency 8 kHz: list index 1 → 2.
        let (eq, _) = converted(jv(1), &[0, 15, 1, 15, 7, 1, 15, 7, 1, 15, 127]);
        assert_eq!(eq.params[8], 2);
        // LIMITER Post Gain ×8 → +18 dB.
        let (limiter, _) = converted(jv(10), &[0, 0, 0, 64, 3]);
        assert_eq!(limiter.params[3], 18);
        // ROTARY Slow Rates 0.05 Hz and 10 Hz, no sync switch.
        let (rotary, _) = converted(jv(8), &[0, 125]);
        assert_eq!((rotary.params[5], rotary.params[1]), (1, 200));
        // STEP-FLANGER Step Rate 0.05 Hz clamps to 0.10 Hz.
        let (step, _) = converted(jv(16), &[0, 0, 0, 0, 0]);
        assert_eq!(step.params[9..11], [0, 1]);
        // 2VOICE-PITCH-SHIFTER Level Balance A100:0B and A50:50B.
        let (a, _) = converted(jv(22), &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let (ab, _) = converted(jv(22), &[0, 0, 0, 0, 0, 0, 0, 0, 0, 50]);
        assert_eq!([a.params[7], a.params[15]], [127, 0]);
        assert_eq!([ab.params[7], ab.params[15]], [127, 127]);
        // Integra-only parameters take the map's values: OVERDRIVE Amp Sw ON.
        let (od, _) = converted(jv(2), &[]);
        assert_eq!(od.params[1..3], [50, 1]);
    }

    #[test]
    fn reports_drops_and_substitutes() {
        // SPECTRUM Output Pan: centre loses nothing.
        let spectrum = |pan| converted(jv(5), &[15, 15, 15, 15, 15, 15, 15, 15, 0, pan]).1;
        assert!(spectrum(64).is_empty());
        assert_eq!(
            spectrum(0),
            [ConversionItem::ParamDropped {
                at: None,
                param: "Output Pan"
            }]
        );
        // COMPRESSOR: a substitute whose Sustain has no target.
        assert_eq!(
            converted(jv(9), &[0, 0, 64]).1,
            [
                ConversionItem::ParamDropped {
                    at: None,
                    param: "Sustain"
                },
                ConversionItem::EffectSubstituted {
                    source: jv(9),
                    target: 31
                },
            ]
        );
    }

    #[test]
    fn an_effect_with_no_target_goes_thru() {
        for effect in [jv(24), xv(59), xv(91)] {
            let (mfx, report) = converted(effect, &[1, 2, 3]);
            assert_eq!(mfx.mfx_type, 0, "{effect}");
            assert_eq!(report, [ConversionItem::EffectDropped { source: effect }]);
        }
    }

    #[test]
    fn xv_types_convert_with_the_jv_rows_or_verbatim() {
        let params = [0, 69, 115, 50, 10, 127, 100, 90, 15, 15, 80, 127];
        assert_eq!(converted(xv(19), &params), converted(jv(19), &params));
        // SPEAKER SIMULATOR, the same type: copied, clean.
        let (same, report) = converted(xv(52), &[3, 40, 50, 127]);
        assert_eq!(
            (same.mfx_type, &same.params[..4]),
            (8, &[3, 40, 50, 127][..])
        );
        assert!(report.is_empty());
        // MULTI TAP DELAY, rescaled: copied, approximate.
        let (rescaled, report) = converted(xv(45), &[7, 8]);
        assert_eq!(
            (rescaled.mfx_type, &rescaled.params[..2]),
            (38, &[7, 8][..])
        );
        assert_eq!(
            report,
            [ConversionItem::EffectSubstituted {
                source: xv(45),
                target: 38
            }]
        );
        // 3D DELAY, a substitute: the target's defaults.
        let (substitute, _) = converted(xv(48), &[7, 8]);
        assert_eq!(substitute.mfx_type, 36);
        assert_eq!(substitute.params, default_slots(36).unwrap());
    }
}
