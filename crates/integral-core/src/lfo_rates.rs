//! Tone LFO rate in Hz, measured on the device ([`docs/research/lfo-rates.md`]).
//!
//! Roland documents the tone LFO rates only as raw `0`–`127`, so
//! [`docs/research/lfo-rates.md`] measured the INTEGRA-7's audio output to
//! map them to Hz. The constants below are that note's fits; take-to-take
//! repeatability on the device is about ±3%, so the displays are estimates
//! to that precision, not exact tables.
//!
//! [`docs/research/lfo-rates.md`]: ../../../docs/research/lfo-rates.md

use std::fmt::Write;

/// PCM Synth LFO1/LFO2: Hz at rate 0.
///
/// Single exponential fit `f = PCM_RATE0_HZ * 2^(rate / PCM_DOUBLING_STEPS)`,
/// residuals within ±3.2% on well-resolved points (the take-to-take scatter).
const PCM_RATE0_HZ: f64 = 0.05438;
/// PCM Synth LFO1/LFO2: rate steps per frequency doubling.
const PCM_DOUBLING_STEPS: f64 = 14.172;

/// SN Synth LFO / Modulation LFO: `log2 f` at rate 0.
const SN_LOG2_0: f64 = -4.38176;
/// SN Synth, segment 1: `log2 f` slope up to the first join.
const SN_SLOPE1: f64 = 0.142372;
/// SN Synth, segment 2: slope change past the first join.
const SN_SLOPE2: f64 = 0.106941;
/// SN Synth, segment 3: slope change past the second join.
const SN_SLOPE3: f64 = 0.034641;
/// SN Synth: rate of the first segment join (Hz 0.80).
const SN_JOIN1: f64 = 28.5;
/// SN Synth: rate of the second segment join (Hz 3.75).
const SN_JOIN2: f64 = 91.5;
/// SN Synth: Hz at rate 0, an unresolved estimate — only 1.2 cycles fit in
/// the 41 s take, so it may be off by more than the ±10% of the other
/// poorly resolved points.
const SN_RATE0_HZ: f64 = 0.03;
/// SN Synth: measured Hz for the two rates the fit misses (a jump from
/// 19.0 Hz at 125), each repeated across both LFOs.
const SN_MEASURED: [(i32, f64); 2] = [(126, 25.6), (127, 26.9)];

/// The Hz a PCM Synth Tone LFO1/LFO2 rate value runs at.
///
/// The fit covers rate 0–127; the measured points stay within ±3.2% of it
/// (and within the take-to-take scatter), so a lookup table would add
/// nothing.
pub fn pcm_lfo_hz(rate: i32) -> f64 {
    PCM_RATE0_HZ * (rate as f64 / PCM_DOUBLING_STEPS).exp2()
}

/// The Hz an SN Synth Tone LFO or Modulation LFO rate value runs at.
///
/// Three exponential segments joined at 28.5 and 91.5 (the rate doubles
/// every 7.0 / 28.2 / 14.3 steps), within ±2.5% of the measurements for
/// 4–125. Rates 126 and 127 jump off the curve and come from the
/// measurements; rate 0 is the unresolved `SN_RATE0_HZ` estimate. Rates
/// 1–3 extrapolate the first segment (never measured).
pub fn sn_lfo_hz(rate: i32) -> f64 {
    if let Some((_, hz)) = SN_MEASURED.iter().find(|(r, _)| *r == rate) {
        return *hz;
    }
    if rate == 0 {
        return SN_RATE0_HZ;
    }
    let r = rate as f64;
    let log2 = SN_LOG2_0 + SN_SLOPE1 * r - SN_SLOPE2 * (r - SN_JOIN1).max(0.0)
        + SN_SLOPE3 * (r - SN_JOIN2).max(0.0);
    log2.exp2()
}

/// Format a rate in Hz, without the unit: two decimals under 1 Hz, one at
/// and above.
pub(crate) fn format_hz(hz: f64) -> String {
    let mut s = String::new();
    if hz >= 1.0 {
        let _ = write!(s, "{hz:.1}");
    } else {
        let _ = write!(s, "{hz:.2}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compact table from [`docs/research/lfo-rates.md`], rate → Hz.
    const PCM_TABLE: [(i32, f64); 18] = [
        (0, 0.054),
        (8, 0.080),
        (16, 0.119),
        (24, 0.176),
        (32, 0.26),
        (40, 0.39),
        (48, 0.57),
        (56, 0.84),
        (64, 1.24),
        (72, 1.84),
        (80, 2.72),
        (88, 4.02),
        (96, 5.95),
        (104, 8.80),
        (112, 13.0),
        (120, 19.2),
        (126, 25.8),
        (127, 27.1),
    ];

    /// Compact table from [`docs/research/lfo-rates.md`], rate → Hz. The
    /// entries marked `*` there (0, 126, 127) are measured, not fit.
    const SN_TABLE: [(i32, f64); 18] = [
        (0, 0.03),
        (8, 0.106),
        (16, 0.233),
        (24, 0.512),
        (32, 0.870),
        (40, 1.06),
        (48, 1.29),
        (56, 1.57),
        (64, 1.91),
        (72, 2.33),
        (80, 2.83),
        (88, 3.44),
        (96, 4.67),
        (104, 6.89),
        (112, 10.2),
        (120, 15.0),
        (126, 25.6),
        (127, 26.9),
    ];

    /// The paper's stated tolerance plus the table's own rounding.
    const PCM_TOLERANCE: f64 = 0.04;
    const SN_TOLERANCE: f64 = 0.035;

    fn near(fit: f64, table: f64, tolerance: f64, rate: i32) {
        let error = (fit - table).abs() / table;
        assert!(
            error <= tolerance,
            "rate {rate}: fit {fit} vs table {table} ({:.1}%)",
            error * 100.0
        );
    }

    #[test]
    fn pcm_fit_tracks_the_paper_table() {
        for (rate, hz) in PCM_TABLE {
            near(pcm_lfo_hz(rate), hz, PCM_TOLERANCE, rate);
        }
    }

    #[test]
    fn sn_fit_tracks_the_paper_table() {
        for (rate, hz) in SN_TABLE {
            near(sn_lfo_hz(rate), hz, SN_TOLERANCE, rate);
        }
    }

    #[test]
    fn sn_segments_double_as_the_paper_says() {
        // Doubling every 7.0 / 28.2 / 14.3 steps across the three segments.
        for (a, b, steps) in [(4.0, 28.0, 7.0), (32.0, 88.0, 28.2), (92.0, 120.0, 14.3)] {
            let ratio = sn_lfo_hz(b as i32) / sn_lfo_hz(a as i32);
            near(ratio, 2.0f32.powf((b - a) / steps) as f64, 0.02, b as i32);
        }
    }

    #[test]
    fn rates_format_as_hz() {
        assert_eq!(format_hz(pcm_lfo_hz(0)), "0.05");
        assert_eq!(format_hz(pcm_lfo_hz(64)), "1.2");
        assert_eq!(format_hz(pcm_lfo_hz(127)), "27.1");
        assert_eq!(format_hz(sn_lfo_hz(0)), "0.03");
        assert_eq!(format_hz(sn_lfo_hz(4)), "0.07");
        assert_eq!(format_hz(sn_lfo_hz(126)), "25.6");
        assert_eq!(format_hz(sn_lfo_hz(127)), "26.9");
    }
}
