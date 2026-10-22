//! Type-dependent FX parameter definitions.
//!
//! Each chorus/reverb type has a different set of meaningful parameters.
//! The param index maps to nibblized parameters at fixed SysEx offsets.

use std::fmt;

use crate::family::ParameterFamily::{self, Amp, Filter, Modulation, Time};

/// Definition of a single FX parameter.
///
/// Values are the stored slot values (raw − 32768), as read from the device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FxParamDef {
    /// Nibblized parameter slot index (0-based).
    pub index: u8,
    /// Human-readable name.
    pub name: &'static str,
    /// Minimum value (inclusive).
    pub min: i32,
    /// Maximum value (inclusive).
    pub max: i32,
    /// Default value.
    pub default_value: i32,
    /// Display unit/format hint.
    pub unit: FxUnit,
    /// The role the parameter plays in shaping the sound.
    pub family: ParameterFamily,
    /// Tempo sync: set when a msec/Hz-vs-note switch picks between this slot
    /// and a note slot.
    pub sync: Option<FxSync>,
}

/// The switch and note slots paired with a tempo-syncable parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FxSync {
    /// Slot of the switch: 0 = the parameter's own slot (msec/Hz), 1 = note.
    pub switch_index: u8,
    /// Slot holding the note value (an index into [`NOTE_NAMES`]).
    pub note_index: u8,
    /// Default note index.
    pub note_default: i32,
    /// Default switch position: 0 = own slot, 1 = note.
    pub switch_default: i32,
    /// Switch position labels: `[own slot, note]`, e.g. `["ms", "♪"]`.
    pub labels: [&'static str; 2],
}

/// How to format a parameter value for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FxUnit {
    /// Plain integer (no suffix).
    Plain,
    /// Percentage (`%`).
    Percent,
    /// Hertz (`Hz`).
    Hz,
    /// Milliseconds: value is in tenths, display as `X.Xms`.
    MillisTenths,
    /// Milliseconds: value is direct, display as `Xms`.
    Millis,
    /// Degrees (`°`).
    Degrees,
    /// Seconds with 1/10 resolution: display as `X.Xs`.
    SecondsTenths,
    /// Filter type: OFF / LPF / HPF.
    FilterType,
    /// Index into [`FREQ_HZ`]; one past the end is BYPASS.
    Freq,
    /// Chorus pre delay: piecewise index, 0.0–100 ms.
    PreDelay,
    /// Rate in 0.05 Hz steps.
    RateHz,
    /// Degrees in 2° steps.
    DegreesX2,
    /// Feedback: `2v − 98` percent.
    FeedbackPercent,
    /// Index into [`NOTE_NAMES`].
    Note,
    /// Signed decibels (`+3dB`).
    Decibels,
    /// Index into a table of numbers that share one unit, e.g.
    /// [`EQ_LOW_FREQ`] in [`HZ`].
    Table {
        /// The numbers, index = stored value.
        values: &'static [&'static str],
        /// Their unit.
        unit: &'static str,
    },
    /// EQ band Q: index into [`EQ_Q`].
    EqQ,
    /// Rate in 0.01 Hz steps from 0.10 Hz at stored 1.
    StepRateHz,
    /// Pan: `L64`–`0`–`63R`, centre at 64.
    Pan,
    /// Dry/wet balance: `D100:0W`–`D0:100W`.
    Balance,
    /// Linear `(v − offset) × step` with a unit suffix; signed when `offset > 0`.
    Offset {
        /// Stored value shown as 0.
        offset: i32,
        /// Display step per stored step.
        step: i32,
        /// Unit suffix, e.g. `dB`.
        suffix: &'static str,
    },
    /// Name list, index = stored value.
    Names(&'static [&'static str]),
}

/// EQ low band frequencies, in [`HZ`].
pub static EQ_LOW_FREQ: &[&str] = &["200", "400"];

/// EQ high band frequencies, in [`KHZ`].
pub static EQ_HIGH_FREQ: &[&str] = &["2", "4", "8"];

/// EQ band Q values.
pub static EQ_Q: &[&str] = &["0.5", "1.0", "2.0", "4.0", "8.0"];

/// `table[v]`, or `v` itself when out of range.
fn label(table: &[&str], v: i32) -> String {
    usize::try_from(v)
        .ok()
        .and_then(|i| table.get(i))
        .map_or_else(|| v.to_string(), |s| s.to_string())
}

/// Note values for tempo-synced times and rates.
pub static NOTE_NAMES: &[&str] = &[
    "1/64T", "1/64", "1/32T", "1/32", "1/16T", "1/32.", "1/16", "1/8T", "1/16.", "1/8", "1/4T",
    "1/8.", "1/4", "1/2T", "1/4.", "1/2", "1/1T", "1/2.", "1/1", "2/1T", "1/1.", "2/1",
];

/// Frequencies (Hz) for cutoff and HF damp indices.
pub static FREQ_HZ: &[u16] = &[
    200, 250, 315, 400, 500, 630, 800, 1000, 1250, 1600, 2000, 2500, 3150, 4000, 5000, 6300, 8000,
];

/// Milliseconds' unit: a time's readout and its sync toggle's free label.
pub(crate) const MS: &str = "ms";

/// Hertz' unit: a rate's readout and its sync toggle's free label.
pub(crate) const HZ: &str = "Hz";

/// A free time without a unit (a bare 0-127 step), as its sync toggle's
/// free label: a clock.
pub(crate) const CLOCK: &str = "⏱";

/// Kilohertz' unit.
pub(crate) const KHZ: &str = "kHz";

/// Percent's unit, written against its number (`20%`).
pub(crate) const PERCENT: &str = "%";

/// Degrees' unit, written against its number (`180°`).
pub(crate) const DEGREES: &str = "°";

/// Units a readout writes against its number; every other unit follows it
/// after a space (`200 ms`, `+3 dB`).
const TIGHT_UNITS: [&str; 2] = [PERCENT, DEGREES];

/// A readout from its [`parts`](FxUnit::parts): the number, then its unit,
/// after a space unless it is one of [`TIGHT_UNITS`].
pub(crate) fn join_parts(number: String, unit: &str) -> String {
    if unit.is_empty() || TIGHT_UNITS.contains(&unit) {
        number + unit
    } else {
        format!("{number} {unit}")
    }
}

/// Switch labels for msec/note times.
pub(crate) const SYNC_MS: [&str; 2] = [MS, "♪"];

/// Switch labels for unitless-time/note times.
pub(crate) const SYNC_CLOCK: [&str; 2] = [CLOCK, "♪"];

/// Switch labels for Hz/note rates.
pub(crate) const SYNC_HZ: [&str; 2] = [HZ, "♪"];

/// Highest note index.
const NOTE_MAX: i32 = NOTE_NAMES.len() as i32 - 1;

/// Highest frequency index; `FREQ_MAX + 1` is BYPASS.
const FREQ_MAX: i32 = FREQ_HZ.len() as i32 - 1;

/// Chorus pre delay index → tenths of a millisecond.
pub(crate) fn pre_delay_tenths(v: i32) -> i32 {
    match v {
        ..50 => v,
        50..60 => 50 + (v - 50) * 5,
        60..100 => 100 + (v - 60) * 10,
        _ => 500 + (v - 100) * 20,
    }
}

impl FxUnit {
    /// Format a stored value for display: its [`parts`](Self::parts), joined.
    pub fn format(self, v: i32) -> String {
        let (number, unit) = self.parts(v);
        join_parts(number, unit)
    }

    /// A stored value's readout in two parts: the value and its unit (empty
    /// for a value without one, such as a note or a name). A tempo-syncable
    /// Control draws the unit itself, as its free/note toggle.
    pub fn parts(self, v: i32) -> (String, &'static str) {
        match self {
            FxUnit::Plain => (v.to_string(), ""),
            FxUnit::Percent => (v.to_string(), PERCENT),
            FxUnit::Hz => (v.to_string(), HZ),
            FxUnit::MillisTenths => (format!("{:.1}", v as f64 / 10.0), MS),
            FxUnit::Millis => (v.to_string(), MS),
            FxUnit::Degrees => (v.to_string(), DEGREES),
            FxUnit::SecondsTenths => (format!("{:.1}", v as f64 / 10.0), "s"),
            FxUnit::FilterType => (
                match v {
                    0 => "OFF".to_string(),
                    1 => "LPF".to_string(),
                    2 => "HPF".to_string(),
                    _ => v.to_string(),
                },
                "",
            ),
            FxUnit::Freq => match usize::try_from(v).ok().and_then(|i| FREQ_HZ.get(i)) {
                Some(hz) => (hz.to_string(), HZ),
                None if v == FREQ_MAX + 1 => ("BYP".to_string(), ""),
                None => (v.to_string(), ""),
            },
            FxUnit::PreDelay => {
                let t = pre_delay_tenths(v);
                if t < 100 {
                    (format!("{:.1}", t as f64 / 10.0), MS)
                } else {
                    ((t / 10).to_string(), MS)
                }
            }
            FxUnit::RateHz => (format!("{:.2}", v as f64 * 0.05), HZ),
            FxUnit::DegreesX2 => ((v * 2).to_string(), DEGREES),
            FxUnit::FeedbackPercent => ((v * 2 - 98).to_string(), PERCENT),
            FxUnit::Note => (label(NOTE_NAMES, v), ""),
            FxUnit::Decibels => (format!("{v:+}"), "dB"),
            FxUnit::Table { values, unit } => (label(values, v), unit),
            FxUnit::EqQ => (label(EQ_Q, v), ""),
            FxUnit::StepRateHz => (format!("{:.2}", (v + 9) as f64 * 0.01), HZ),
            FxUnit::Pan => (
                match v - 64 {
                    p if p < 0 => format!("L{}", -p),
                    0 => "0".to_string(),
                    p => format!("{p}R"),
                },
                "",
            ),
            FxUnit::Balance => (format!("D{}:{v}W", 100 - v), ""),
            FxUnit::Offset {
                offset,
                step,
                suffix,
            } => {
                let d = (v - offset) * step;
                if offset > 0 {
                    (format!("{d:+}"), suffix)
                } else {
                    (d.to_string(), suffix)
                }
            }
            FxUnit::Names(names) => (label(names, v), ""),
        }
    }
}

impl FxParamDef {
    /// Format a stored parameter value for display.
    pub fn format_value(&self, v: i32) -> String {
        self.unit.format(v)
    }

    /// A stored value's readout without its unit ([`FxUnit::parts`]): a
    /// tempo-syncable parameter's readout draws its unit as the toggle.
    pub fn format_number(&self, v: i32) -> String {
        self.unit.parts(v).0
    }

    /// The note-mode view of a tempo-syncable parameter: same name, but it
    /// reads and writes the note slot.
    pub fn note_param(&self) -> Option<FxParamDef> {
        self.sync.map(|s| FxParamDef {
            index: s.note_index,
            min: 0,
            max: NOTE_MAX,
            default_value: s.note_default,
            unit: FxUnit::Note,
            sync: None,
            ..*self
        })
    }

    /// Whether `slots` (all stored values of the block) put this parameter
    /// in note mode.
    pub fn note_mode(&self, slots: &[i32]) -> bool {
        self.sync
            .is_some_and(|s| slots.get(s.switch_index as usize) == Some(&1))
    }

    /// The definition that applies to `slots`: the note view in note mode,
    /// else `self`.
    pub fn resolve(&self, slots: &[i32]) -> FxParamDef {
        match self.note_param() {
            Some(n) if self.note_mode(slots) => n,
            _ => self.clone(),
        }
    }
}

impl fmt::Display for FxParamDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// A plain (not tempo-syncable) parameter definition.
pub(crate) const fn param(
    index: u8,
    name: &'static str,
    family: ParameterFamily,
    min: i32,
    max: i32,
    default_value: i32,
    unit: FxUnit,
) -> FxParamDef {
    FxParamDef {
        index,
        name,
        min,
        max,
        default_value,
        unit,
        family,
        sync: None,
    }
}

impl FxParamDef {
    /// Make this a tempo-syncable parameter: switch at `index − 1`, value at
    /// `index`, note at `index + 1` (the only layout the device uses).
    pub(crate) const fn synced(self, note_default: i32, labels: [&'static str; 2]) -> FxParamDef {
        FxParamDef {
            sync: Some(FxSync {
                switch_index: self.index - 1,
                note_index: self.index + 1,
                note_default,
                switch_default: 0,
                labels,
            }),
            ..self
        }
    }

    /// Make a tempo-syncable parameter start in note mode.
    pub(crate) const fn note_by_default(self) -> FxParamDef {
        match self.sync {
            Some(sync) => FxParamDef {
                sync: Some(FxSync {
                    switch_default: 1,
                    ..sync
                }),
                ..self
            },
            None => self,
        }
    }
}

// ---------------------------------------------------------------------------
// Chorus type parameter maps (docs/research/fx-param-layout.md)
// ---------------------------------------------------------------------------

const CHORUS_CHORUS: &[FxParamDef] = &[
    param(0, "Filter", Filter, 0, 2, 2, FxUnit::FilterType),
    param(1, "Cutoff", Filter, 0, FREQ_MAX, 6, FxUnit::Freq),
    param(2, "PreDly", Time, 0, 125, 20, FxUnit::PreDelay),
    param(4, "Rate", Modulation, 1, 200, 10, FxUnit::RateHz).synced(18, SYNC_HZ),
    param(6, "Depth", Modulation, 0, 127, 30, FxUnit::Plain),
    param(7, "Phase", Modulation, 0, 90, 90, FxUnit::DegreesX2),
    param(8, "Fdbk", Filter, 0, 127, 0, FxUnit::Plain),
];

const CHORUS_DELAY: &[FxParamDef] = &[
    param(1, "Dly L", Time, 0, 1000, 200, FxUnit::Millis).synced(7, SYNC_MS),
    param(4, "Dly R", Time, 0, 1000, 400, FxUnit::Millis).synced(10, SYNC_MS),
    param(7, "Dly C", Time, 0, 1000, 600, FxUnit::Millis).synced(12, SYNC_MS),
    param(9, "C Fdbk", Time, 0, 98, 59, FxUnit::FeedbackPercent),
    param(
        10,
        "HFDamp",
        Filter,
        0,
        FREQ_MAX + 1,
        FREQ_MAX + 1,
        FxUnit::Freq,
    ),
    param(11, "Lv L", Amp, 0, 127, 127, FxUnit::Plain),
    param(12, "Lv R", Amp, 0, 127, 127, FxUnit::Plain),
    param(13, "Lv C", Amp, 0, 127, 127, FxUnit::Plain),
];

const CHORUS_GM2: &[FxParamDef] = &[
    param(0, "PreLPF", Filter, 0, 7, 0, FxUnit::Plain),
    param(1, "Level", Amp, 0, 127, 64, FxUnit::Plain),
    param(2, "Fdbk", Filter, 0, 127, 8, FxUnit::Plain),
    param(3, "Delay", Time, 0, 127, 80, FxUnit::Plain),
    param(4, "Rate", Modulation, 0, 127, 3, FxUnit::Plain),
    param(5, "Depth", Modulation, 0, 127, 19, FxUnit::Plain),
    param(6, "→Rev", Amp, 0, 127, 0, FxUnit::Plain),
];

/// Chorus parameter definitions indexed by chorus type (0=OFF, 1=Chorus,
/// 2=Delay, 3=GM2).
pub fn chorus_params(chorus_type: u8) -> &'static [FxParamDef] {
    match chorus_type {
        1 => CHORUS_CHORUS,
        2 => CHORUS_DELAY,
        3 => CHORUS_GM2,
        _ => &[],
    }
}

/// Chorus type display names.
pub static CHORUS_TYPE_NAMES: &[&str] = &["OFF", "Chorus", "Delay", "GM2 Cho"];

// ---------------------------------------------------------------------------
// Reverb type parameter maps
//
// Slot 0 of Room/Hall/Plate and slots 1–2 of GM2 are hidden level/pre delay
// slots of unknown effect. They are left out so the UI never writes them.
// ---------------------------------------------------------------------------

// ponytail: one table for five types; Roland's inits differ per type (Time 9
// for Rooms, Pre Delay 20 for Plate). Split it if per-type reset matters.
const REVERB_ROOM_HALL_PLATE: &[FxParamDef] = &[
    param(1, "PreDly", Time, 0, 100, 0, FxUnit::Millis),
    param(2, "Time", Time, 1, 100, 26, FxUnit::SecondsTenths),
    param(3, "Densty", Time, 0, 127, 127, FxUnit::Plain),
    param(4, "Diffus", Time, 0, 127, 127, FxUnit::Plain),
    param(5, "LFDamp", Filter, 0, 100, 50, FxUnit::Plain),
    param(6, "HFDamp", Filter, 0, 100, 50, FxUnit::Plain),
    param(7, "Spread", Time, 0, 127, 127, FxUnit::Plain),
    param(8, "Tone", Filter, 0, 127, 64, FxUnit::Plain),
];

const REVERB_GM2: &[FxParamDef] = &[
    param(0, "Char", Time, 0, 5, 4, FxUnit::Plain),
    param(3, "Time", Time, 0, 127, 64, FxUnit::Plain),
];

/// Reverb parameter definitions indexed by reverb type (0=OFF, 1–5=Room/Hall/Plate,
/// 6=GM2).
pub fn reverb_params(reverb_type: u8) -> &'static [FxParamDef] {
    match reverb_type {
        1..=5 => REVERB_ROOM_HALL_PLATE,
        6 => REVERB_GM2,
        _ => &[],
    }
}

/// Reverb type display names.
pub static REVERB_TYPE_NAMES: &[&str] = &[
    "OFF", "Room 1", "Room 2", "Hall 1", "Hall 2", "Plate", "GM2 Rev",
];

/// Chorus output routing names.
pub static CHORUS_OUTPUT_NAMES: &[&str] = &["MAIN", "REV", "MAIN+REV"];

/// Output assign names (A–D), shared by Reverb and Chorus Output Assign.
pub static OUTPUT_ASSIGN_NAMES: &[&str] = &["A", "B", "C", "D"];

#[cfg(test)]
mod tests {
    use super::*;

    /// Display strings for every visible param of `defs` given a slot dump.
    fn show(defs: &[FxParamDef], slots: &[i32]) -> Vec<(&'static str, String)> {
        defs.iter()
            .map(|d| {
                let r = d.resolve(slots);
                (d.name, r.format_value(slots[r.index as usize]))
            })
            .collect()
    }

    #[test]
    fn delay_device_dump() {
        // `raw-read 18000400 54`, Delay type: slots 1..15.
        let slots = [1, 200, 6, 1, 400, 9, 1, 600, 12, 59, 17, 127, 127, 127, 0];
        let shown = show(chorus_params(2), &slots);
        let expect = [
            ("Dly L", "1/16"),
            ("Dly R", "1/8"),
            ("Dly C", "1/4"),
            ("C Fdbk", "20%"),
            ("HFDamp", "BYP"),
            ("Lv L", "127"),
            ("Lv R", "127"),
            ("Lv C", "127"),
        ];
        let expect: Vec<_> = expect.iter().map(|(n, v)| (*n, v.to_string())).collect();
        assert_eq!(shown, expect);
    }

    #[test]
    fn delay_msec_mode_reads_msec_slot() {
        let slots = [0, 200, 6, 1, 400, 9, 0, 600, 12, 49, 6, 127, 127, 127];
        let shown = show(chorus_params(2), &slots);
        assert_eq!(shown[0].1, "200 ms");
        assert_eq!(shown[1].1, "1/8");
        assert_eq!(shown[2].1, "600 ms");
        assert_eq!(shown[3].1, "0%");
        assert_eq!(shown[4].1, "800 Hz");
    }

    #[test]
    fn hall_device_dump() {
        // FX2 Hall 1 dump: slot 0 is the hidden level and is not shown.
        let slots = [95, 0, 21, 127, 127, 50, 50, 127, 64];
        let shown: Vec<_> = show(reverb_params(3), &slots)
            .into_iter()
            .map(|(_, v)| v)
            .collect();
        assert_eq!(
            shown,
            ["0 ms", "2.1 s", "127", "127", "50", "50", "127", "64"]
        );
        assert!(reverb_params(3).iter().all(|d| d.index != 0));
    }

    #[test]
    fn gm2_reverb_skips_hidden_slots() {
        let idx: Vec<_> = reverb_params(6).iter().map(|d| d.index).collect();
        assert_eq!(idx, [0, 3]);
    }

    #[test]
    fn chorus_roland_inits() {
        // Roland's Chorus inits: 2, 6, 20, 0, 10, 18, 30, 90, 0.
        let slots = [2, 6, 20, 0, 10, 18, 30, 90, 0];
        let shown: Vec<_> = show(chorus_params(1), &slots)
            .into_iter()
            .map(|(_, v)| v)
            .collect();
        assert_eq!(
            shown,
            ["HPF", "800 Hz", "2.0 ms", "0.50 Hz", "30", "180°", "0"]
        );
        let mut note = slots;
        note[3] = 1;
        assert_eq!(show(chorus_params(1), &note)[3].1, "1/1");
    }

    #[test]
    fn synced_params_point_at_their_slots() {
        let rate = &chorus_params(1)[3];
        assert_eq!(
            rate.sync,
            Some(FxSync {
                switch_index: 3,
                note_index: 5,
                note_default: 18,
                switch_default: 0,
                labels: ["Hz", "♪"],
            })
        );
        let note = rate.note_param().unwrap();
        assert_eq!((note.index, note.min, note.max), (5, 0, 21));
        assert_eq!(NOTE_NAMES.len(), 22);
    }

    #[test]
    fn pre_delay_table() {
        let p = &chorus_params(1)[2];
        let f = |v| p.format_value(v);
        assert_eq!(
            [f(0), f(49), f(50), f(59), f(60), f(99), f(100), f(125)],
            [
                "0.0 ms", "4.9 ms", "5.0 ms", "9.5 ms", "10 ms", "49 ms", "50 ms", "100 ms"
            ]
        );
    }

    #[test]
    fn rate_phase_feedback_encodings() {
        let rate = &chorus_params(1)[3];
        assert_eq!(rate.format_value(1), "0.05 Hz");
        assert_eq!(rate.format_value(200), "10.00 Hz");
        assert_eq!(chorus_params(1)[5].format_value(45), "90°");
        let fb = &chorus_params(2)[3];
        assert_eq!(fb.format_value(0), "-98%");
        assert_eq!(fb.format_value(98), "98%");
    }

    #[test]
    fn delay_has_no_reverb_send_slot() {
        assert!(chorus_params(2).iter().all(|d| d.index < 14));
    }

    #[test]
    fn visible_knobs_fit_the_strip() {
        for t in 0..4 {
            assert!(chorus_params(t).len() <= 9);
        }
        for t in 0..7 {
            assert!(reverb_params(t).len() <= 9);
        }
    }
}
