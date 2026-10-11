//! XV-series Rhythm Sets (Model ID `00 10`) → PCM Drum Kits.
//!
//! The PCM Drum Kit is the XV Rhythm Set with the same offsets and keys
//! 21–108, one to one. A Rhythm Set converts by copying its Common, MFX and
//! Rhythm Tone blocks byte for byte into an INIT kit, then applying one fix
//! row per field that is not identical. A key with no Rhythm Tone (Fantom
//! builds send only some) stays OFF.
//!
//! Reference: `docs/research/foreign-rhythm-format.md` (section 3) and its
//! per-field `foreign-rhythm-format.csv`.

use super::frame::{self, Frame};
use super::pcm_kit::PcmKit;
use super::report::{ConversionItem, ConversionReport, Location};
use super::xv::{self, OUTPUT_MFX, Rejected, WaveSource};
use crate::address::Address;
use crate::librarian::pack::PackEntry;
use crate::parse_helpers::{nibble2, nibble4};
use crate::pcm_drum::{
    PCMD_KEYS, PCMD_WMT_COUNT, PcmdPartialField as F, PcmdWmtField, pcmd_wmt_byte,
};

/// A Rhythm block: its offset (address bytes 1–2, 7-bit, below the slot
/// bits) and size in bytes.
type Block = (u16, usize);

/// Rhythm Common, `00 00 00 12`.
const COMMON: Block = (0x0000, 18);
/// Rhythm Common MFX, `00 00 01 11`.
const MFX: Block = (0x0002, 145);
/// Rhythm Common Chorus, `00 00 00 34`.
const CHORUS: Block = (0x0004, 52);
/// Rhythm Common Reverb, `00 00 00 53`.
const REVERB: Block = (0x0006, 83);
/// Rhythm Tone length, `00 00 01 41`; the INTEGRA-7 Partial adds 2 bytes.
const TONE_LEN: usize = 193;
/// Offset of key 21's Rhythm Tone; each key is 2 further.
const FIRST_TONE: u16 = 0x0010;

/// Common: Rhythm Clock Source, RHYTHM or SYSTEM.
const CLOCK_SOURCE: usize = 0x0D;
/// Common: Rhythm Tempo (2 nibbles).
const TEMPO: usize = 0x0E;
/// Common: One Shot Mode.
const ONE_SHOT_MODE: usize = 0x10;
/// Common: Rhythm Output Assign.
const OUTPUT_ASSIGN: usize = 0x11;

/// Tone: Chorus and Reverb Send Level (MFX).
const MFX_SENDS: [usize; 2] = [0x17, 0x18];
/// Tone: Tone Receive Pan Mode, CONTINUOUS or KEY-ON.
const RECEIVE_PAN_MODE: usize = 0x1F;
/// Partial Output Assign PART.
const OUTPUT_PART: u8 = 0;

/// A Rhythm Set being converted.
struct Rhythm<'a> {
    /// The XV Rhythm Common, MFX, Chorus and Reverb blocks.
    common: &'a [u8],
    mfx: &'a [u8],
    chorus: &'a [u8],
    reverb: &'a [u8],
    /// The XV Rhythm Tone of each key, if sent.
    tones: Vec<(u8, Vec<u8>)>,
    kit: PcmKit,
    report: ConversionReport,
}

/// The fix rows, in `foreign-rhythm-format.csv` order: kit-wide first, then
/// per key. Switched-off WMTs are reset last.
const FIXES: [fn(&mut Rhythm<'_>); 10] = [
    tempo,
    output_assign,
    mfx,
    chorus_reverb,
    one_shot_mode,
    tone_sends,
    tone_output_assign,
    receive_pan_mode,
    wave_group,
    off_wmts,
];

/// Convert the Rhythm Set whose frames share address byte `key[0]` and the
/// slot bits `key[1]`. Its Common, MFX, Chorus and Reverb are needed.
pub(super) fn convert(
    key: [u8; 2],
    frames: &[Frame<'_>],
) -> Result<(PackEntry, ConversionReport), Rejected> {
    let dt1s = || frames.iter().map(|f| (Address(f.address), f.data));
    let assemble = |(offset, size): Block| {
        let [hi, lo] = [(offset >> 7) as u8, (offset & 0x7F) as u8];
        let address = Address::new(key[0], key[1] | hi, lo, 0).value();
        frame::assemble(&[(address, size)], dt1s()).map(|mut b| b.remove(0))
    };
    let [common, mfx, chorus, reverb] =
        [COMMON, MFX, CHORUS, REVERB].map(|b| assemble(b).ok_or(Rejected::Incomplete));
    let (common, mfx, chorus, reverb) = (common?, mfx?, chorus?, reverb?);
    let tones = (PCMD_KEYS.first..=PCMD_KEYS.last)
        .filter_map(|k| {
            let offset = FIRST_TONE + 2 * u16::from(k - PCMD_KEYS.first);
            assemble((offset, TONE_LEN)).map(|tone| (k, tone))
        })
        .collect();

    let mut rhythm = Rhythm {
        common: &common,
        mfx: &mfx,
        chorus: &chorus,
        reverb: &reverb,
        tones,
        kit: PcmKit::init(),
        report: ConversionReport::new(),
    };
    rhythm.kit.common().copy_from_slice(&common);
    rhythm.kit.mfx()[..mfx.len()].copy_from_slice(&mfx);
    for (k, tone) in &rhythm.tones {
        rhythm.kit.key(*k)[..TONE_LEN].copy_from_slice(tone);
    }
    for fix in FIXES {
        fix(&mut rhythm);
    }
    Ok((rhythm.kit.into_entry(), rhythm.report))
}

impl Rhythm<'_> {
    /// Whether the Rhythm Tone `tone` plays: one of its WMTs is on.
    fn active(tone: &[u8]) -> bool {
        (0..PCMD_WMT_COUNT).any(|w| tone[pcmd_wmt_byte(w, PcmdWmtField::Switch)] != 0)
    }
}

/// Rhythm Tempo: the INTEGRA-7 takes the tempo from the Studio Set. The
/// bytes stay where the kit keeps them.
fn tempo(r: &mut Rhythm<'_>) {
    xv::tempo(
        r.common[CLOCK_SOURCE],
        nibble2(r.common, TEMPO),
        &mut r.report,
    );
}

/// Rhythm Output Assign, by the Patch rule ([`xv::output_assign`]).
fn output_assign(r: &mut Rhythm<'_>) {
    r.kit.common()[OUTPUT_ASSIGN] = xv::output_assign(r.common[OUTPUT_ASSIGN], &mut r.report);
}

/// Rhythm MFX, by the Patch rule ([`xv::mfx_thru`]).
fn mfx(r: &mut Rhythm<'_>) {
    xv::mfx_thru(r.mfx, r.kit.mfx(), &mut r.report);
}

/// Rhythm Chorus and Reverb, by the Patch rule ([`xv::chorus_reverb`]).
fn chorus_reverb(r: &mut Rhythm<'_>) {
    xv::chorus_reverb(r.chorus, r.reverb, &mut r.report);
}

/// One Shot Mode: once per kit on the XV, per key on the INTEGRA-7. The
/// Common byte stays too.
fn one_shot_mode(r: &mut Rhythm<'_>) {
    let one_shot = r.common[ONE_SHOT_MODE];
    for (k, _) in &r.tones {
        r.kit.key(*k)[F::OneShotMode as usize] = one_shot;
    }
}

/// Tone Chorus/Reverb Send Levels: an XV Tone holds a pair for when it goes
/// through the MFX and a pair for when it doesn't, and its Output Assign
/// picks one. The INTEGRA-7 reads the second, so an MFX Tone's pair goes
/// there. The first stays in its reserves.
fn tone_sends(r: &mut Rhythm<'_>) {
    for (k, tone) in &r.tones {
        if tone[F::OutputAssign as usize] == OUTPUT_MFX {
            let partial = r.kit.key(*k);
            partial[F::ChorusSend as usize] = tone[MFX_SENDS[0]];
            partial[F::ReverbSend as usize] = tone[MFX_SENDS[1]];
        }
    }
}

/// Tone Output Assign: MFX, A–D, 1–8 on the XV, PART and COMP+EQ1–6 on the
/// INTEGRA-7. Copied, output A (1) would land on COMP+EQ1, so every key
/// gets PART.
fn tone_output_assign(r: &mut Rhythm<'_>) {
    // ponytail: with the MFX THRU (#213), PART loses nothing. Once the MFX
    // converts, a Tone that bypassed it (output ≠ MFX) goes through it on
    // PART; report that then.
    for (k, _) in &r.tones {
        r.kit.key(*k)[F::OutputAssign as usize] = OUTPUT_PART;
    }
}

/// Tone Receive Pan Mode: no INTEGRA-7 equivalent. The byte stays in its
/// reserve.
fn receive_pan_mode(r: &mut Rhythm<'_>) {
    for (k, tone) in &r.tones {
        if Rhythm::active(tone) && tone[RECEIVE_PAN_MODE] != 0 {
            r.report.push(ConversionItem::ParamDropped {
                at: Some(Location::Key(*k)),
                param: "Receive Pan Mode",
            });
        }
    }
}

/// WMT Wave Group Type and ID, by the Patch rule ([`xv::wave_source`]).
fn wave_group(r: &mut Rhythm<'_>) {
    for (k, tone) in &r.tones {
        for w in 0..PCMD_WMT_COUNT {
            let field = |f| nibble4(tone, pcmd_wmt_byte(w, f));
            let group_type = tone[pcmd_wmt_byte(w, PcmdWmtField::WaveGroupType)];
            let numbers = [PcmdWmtField::WaveNumberL, PcmdWmtField::WaveNumberR].map(field);
            match xv::wave_source(group_type, field(PcmdWmtField::WaveGroupId)) {
                WaveSource::Int => {}
                WaveSource::Srx(group) => r.kit.write_waves(*k, w, group, numbers),
                WaveSource::Foreign(group) => {
                    r.kit.map_waves(*k, w, group, numbers, &mut r.report);
                }
            }
        }
    }
}

/// A switched-off WMT may hold out-of-range bytes (`XV 5080 All.mid`, key
/// 22), so it gets the default WMT, still off. It was unused: no report.
fn off_wmts(r: &mut Rhythm<'_>) {
    for (k, tone) in &r.tones {
        for w in 0..PCMD_WMT_COUNT {
            if tone[pcmd_wmt_byte(w, PcmdWmtField::Switch)] == 0 {
                r.kit.reset_wmt(*k, w);
            }
        }
    }
}
