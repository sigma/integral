//! JV/XP Rhythm Setups (Model ID `6A`) → PCM Drum Kits.
//!
//! A Rhythm Setup is a name-only Common plus one Rhythm Note per key, for
//! keys 35–98. Each Note converts into the Partial of the same key of an
//! INIT kit: one mapping row per Note field with an INTEGRA-7 target
//! ([`NOTE_ROWS`]), reusing the Patch's value tables, then the named
//! [`CODE_ROWS`]. Keys 21–34 and 99–108 stay OFF. A JV kit has no effects
//! of its own, so the kit MFX stays THRU.
//!
//! Reference: `docs/research/foreign-rhythm-format.md` (section 2) and its
//! per-field `foreign-rhythm-format.csv`.

use super::frame::{self, Frame};
use super::jv::{
    self, How, OUTPUT_EFX, Row, TIME_SENS, TONE_OUTPUTS, VELOCITY_SENS, copies, copy, row,
};
use super::pcm_kit::PcmKit;
use super::report::{ConversionItem, ConversionReport, Location};
use crate::address::Address;
use crate::librarian::foreign::report::Skipped;
use crate::librarian::pack::PackEntry;
use crate::pcm_drum::{PcmdPartialField as F, PcmdWmtField, pcmd_wmt_byte};

/// Rhythm Common length: the name only.
const COMMON_LEN: usize = 0x0C;
/// Rhythm Note length.
const NOTE_LEN: usize = 0x3A;
/// The keys a Rhythm Setup has a Note for. A Note sits at offset `key 00`.
const KEYS: core::ops::RangeInclusive<u8> = 35..=98;

/// Note: Tone Switch.
const TONE_SWITCH: usize = 0x00;
/// Note: Pan Control Switch.
const PAN_CONTROL: usize = 0x0B;
/// Note: Output Assign (MIX, EFX, OUTPUT 1, OUTPUT 2).
const OUTPUT_ASSIGN: usize = 0x36;

/// Partial: the reserve that keeps the XV Receive Pan Mode.
const RECEIVE_PAN_MODE: usize = 0x1F;
/// Partial: the reserves that keep the XV "(MFX)" send pair.
const MFX_SENDS: [usize; 2] = [0x17, 0x18];

/// WMT1's byte of `field`.
const fn wmt1(field: PcmdWmtField) -> usize {
    pcmd_wmt_byte(0, field)
}

/// The Partial byte of `field`, `k` steps in.
const fn at(field: F, k: usize) -> usize {
    field as usize + k
}

use How::{Offset, Table};

/// Velocity Sens, by Roland's table.
const VELOCITY: How = Table(&VELOCITY_SENS);
/// Velocity Time Sens, by Roland's rule.
const TIME: How = Table(&TIME_SENS);

/// Rhythm Note → Partial of the same key (`foreign-rhythm-format.md`,
/// section 2). The Wave, Pan Control Switch and Output Assign are code
/// rows. Envelope start levels need no row: the INIT Pitch L0 (64) and TVF
/// L0 (0) are what Roland converts to.
const NOTE_ROWS: [Row<usize>; 41] = [
    copy(0x00, wmt1(PcmdWmtField::Switch)),         // Tone Switch
    copy(0x05, wmt1(PcmdWmtField::WaveGain)),       // Wave Gain
    copy(0x06, at(F::PitchBendRange, 0)),           // Bender Range
    copy(0x07, at(F::MuteGroup, 0)),                // Mute Group
    copy(0x08, at(F::EnvMode, 0)),                  // Envelope Mode
    copy(0x09, at(F::ReceiveExpression, 0)),        // Volume Control Switch, as Roland
    copy(0x0A, at(F::ReceiveHold1, 0)),             // Hold-1 Control Switch
    copy(0x0C, at(F::CoarseTune, 0)),               // Source Key
    row(0x0D, at(F::FineTune, 0), Offset(14)),      // Fine Tune
    copy(0x0E, at(F::RandomPitchDepth, 0)),         // Random Pitch Depth
    row(0x0F, at(F::PitchEnvDepth, 0), Offset(52)), // P-Env Depth
    row(0x10, at(F::PitchEnvVelocitySens, 0), VELOCITY), // P-Env Velocity Sens
    row(0x11, at(F::PitchEnvT1VelocitySens, 0), TIME), // P-Env Velocity Time
    row(0x11, at(F::PitchEnvT4VelocitySens, 0), TIME), // … into Time 4 too
    copies(0x12, at(F::PitchEnvTime, 0), 4),        // P-Env Time 1–4
    row(0x16, at(F::PitchEnvLevel, 1), Offset(1)),  // P-Env Level 1
    row(0x17, at(F::PitchEnvLevel, 2), Offset(1)),  // P-Env Level 2
    row(0x18, at(F::PitchEnvLevel, 3), Offset(1)),  // P-Env Level 3
    row(0x19, at(F::PitchEnvLevel, 4), Offset(1)),  // P-Env Level 4
    copy(0x1A, at(F::TvfFilterType, 0)),            // Filter Type
    copy(0x1B, at(F::TvfCutoffFrequency, 0)),       // Cutoff Frequency
    copy(0x1C, at(F::TvfResonance, 0)),             // Resonance
    row(0x1D, at(F::TvfResonanceVelocitySens, 0), VELOCITY), // Resonance Velocity Sens
    row(0x1E, at(F::TvfEnvDepth, 0), Offset(1)),    // F-Env Depth
    row(0x1F, at(F::TvfEnvVelocitySens, 0), VELOCITY), // F-Env Velocity Sens
    row(0x20, at(F::TvfEnvT1VelocitySens, 0), TIME), // F-Env Velocity Time
    row(0x20, at(F::TvfEnvT4VelocitySens, 0), TIME), // … into Time 4 too
    copies(0x21, at(F::TvfEnvTime, 0), 4),          // F-Env Time 1–4
    copies(0x25, at(F::TvfEnvLevel, 1), 4),         // F-Env Level 1–4
    copy(0x29, at(F::Level, 0)),                    // Tone Level
    row(0x2A, at(F::TvaLevelVelocitySens, 0), VELOCITY), // A-Env Velocity Sens
    row(0x2B, at(F::TvaEnvT1VelocitySens, 0), TIME), // A-Env Velocity Time
    row(0x2B, at(F::TvaEnvT4VelocitySens, 0), TIME), // … into Time 4 too
    copies(0x2C, at(F::TvaEnvTime, 0), 4),          // A-Env Time 1–4
    copies(0x30, at(F::TvaEnvLevel, 0), 3),         // A-Env Level 1–3
    copies(0x33, at(F::Pan, 0), 3),                 // Tone Pan, Random and Alternate Pan Depth
    copy(0x37, at(F::OutputLevel, 0)),              // Mix/EFX Send Level
    copy(0x38, at(F::ChorusSend, 0)),               // Chorus Send Level
    copy(0x39, at(F::ReverbSend, 0)),               // Reverb Send Level
    copy(0x38, MFX_SENDS[0]),                       // … and its MFX-path reserve, as Roland
    copy(0x39, MFX_SENDS[1]),                       // … and its MFX-path reserve, as Roland
];

/// The named code rows, in order, run on each Note.
const CODE_ROWS: [fn(&mut Kit<'_>, u8, &[u8]); 4] = [roland_fills, waves, pan_control, outputs];

/// A Rhythm Setup being converted.
struct Kit<'a> {
    kit: PcmKit,
    report: &'a mut ConversionReport,
    /// Whether an active Note goes through the EFX.
    efx: bool,
}

/// Convert the Rhythm Setup whose frames share address bytes `key`. It
/// needs its Common and at least one Note; a missing Note is a key OFF.
pub(super) fn convert(
    key: [u8; 2],
    frames: &[Frame<'_>],
) -> Result<(PackEntry, ConversionReport), Skipped> {
    let block = |c: u8, size: usize| [(Address::new(key[0], key[1], c, 0).value(), size)];
    let dt1s = || frames.iter().map(|f| (Address(f.address), f.data));
    let assemble = |c, size| frame::assemble(&block(c, size), dt1s()).map(|mut b| b.remove(0));
    let common = assemble(0, COMMON_LEN).ok_or(Skipped::Incomplete)?;
    let notes: Vec<(u8, Vec<u8>)> = KEYS
        .filter_map(|k| assemble(k, NOTE_LEN).map(|note| (k, note)))
        .collect();
    if notes.is_empty() {
        return Err(Skipped::Incomplete);
    }

    let mut report = ConversionReport::new();
    let mut kit = Kit {
        kit: PcmKit::init(),
        report: &mut report,
        efx: false,
    };
    kit.kit.common()[..COMMON_LEN].copy_from_slice(&common);
    for (k, note) in &notes {
        let partial = kit.kit.key(*k);
        for row in &NOTE_ROWS {
            for i in 0..row.count {
                if let Some(value) = row.how.apply(note[row.from + i]) {
                    partial[row.to + i] = value;
                }
            }
        }
        for code in CODE_ROWS {
            code(&mut kit, *k, note);
        }
    }
    if kit.efx {
        kit.report.push(ConversionItem::ParamDropped {
            at: None,
            param: "Output Assign EFX",
        });
    }
    let entry = kit.kit.into_entry();
    Ok((entry, report))
}

/// The Integra fields a Note has no source for, as Roland's own Orch Set
/// conversion fills them: WMT Velocity Control ON, WMT1 Random and
/// Alternate Pan Switches ON, F-Env and TVA Velocity Curves 1.
fn roland_fills(p: &mut Kit<'_>, k: u8, _: &[u8]) {
    let partial = p.kit.key(k);
    partial[F::WmtVelocityControl as usize] = 1;
    partial[wmt1(PcmdWmtField::RandomPanSwitch)] = 1;
    partial[wmt1(PcmdWmtField::AlternatePanSwitch)] = 1;
    partial[F::TvfEnvVelocityCurve as usize] = 1;
    partial[F::TvaLevelVelocityCurve as usize] = 1;
}

/// Wave Group and Number, through the Wave map into WMT1.
fn waves(p: &mut Kit<'_>, k: u8, note: &[u8]) {
    let (group, number) = jv::foreign_wave(note);
    p.kit.map_waves(k, 0, group, [number, 0], p.report);
}

/// Pan Control Switch, by the Patch rule ([`jv::receive_pan_mode`]).
fn pan_control(p: &mut Kit<'_>, k: u8, note: &[u8]) {
    let (mode, dropped) = jv::receive_pan_mode(note[PAN_CONTROL]);
    p.kit.key(k)[RECEIVE_PAN_MODE] = mode;
    if note[TONE_SWITCH] != 0 && dropped {
        p.report.push(ConversionItem::ParamDropped {
            at: Some(Location::Key(k)),
            param: "Pan Control Switch",
        });
    }
}

/// Output Assign: every key goes to PART, the INIT value. The EFX is the
/// Performance's, so a Note routed through it loses it, reported once per
/// kit. A direct output lives in the Studio Set on the INTEGRA-7.
fn outputs(p: &mut Kit<'_>, k: u8, note: &[u8]) {
    let output = note[OUTPUT_ASSIGN];
    if note[TONE_SWITCH] == 0 {
        return;
    }
    if output == OUTPUT_EFX {
        p.efx = true;
    } else if output > OUTPUT_EFX {
        p.report.push(ConversionItem::NotApplied {
            what: format!("Key {k} Output Assign {}", jv::named(&TONE_OUTPUTS, output)),
        });
    }
}
