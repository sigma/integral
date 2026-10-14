# Tone LFO Rates in Hz, Measured on the Device

Research for [#270](https://github.com/sigma/integral/issues/270) (follows
the discussion on [#269](https://github.com/sigma/integral/issues/269)).
Roland documents the tone LFO rates only as raw `0`–`127`
([docs/midi/06-pcm-synth-tone.md](../midi/06-pcm-synth-tone.md),
[docs/midi/08-supernatural-synth-tone.md](../midi/08-supernatural-synth-tone.md)),
so this note measures the INTEGRA-7's audio output to map them to Hz. The raw
numbers, one entry per take, are in [lfo-rates.json](lfo-rates.json).

**Answer:**

- **PCM Synth LFO1 and LFO2** follow one exponential:
  **f = 0.0544 Hz × 2^(rate / 14.17)**, from 0.054 Hz at 0 to about 27 Hz at
  127. Residuals stay within ±3.2% on every well-resolved point (≥ 4 cycles
  recorded), which is the take-to-take repeatability (±3%). A closed form is
  enough. LFO2 matches LFO1 within 2.3%.
- **SN Synth LFO and Modulation LFO** share one curve, but it's not a single
  exponential (a single fit misses by 67%). It's three exponential segments
  with joins at about 28.5 and 91.5. The rate doubles every 7.0 steps up to
  28.5, every 28.2 steps up to 91.5 and every 14.3 steps up to 125. Within
  4–125 this fits within ±2.5% (rms 1.1%). Two values don't fit and need a
  table: **126 = 25.6 Hz** (a jump from 19.0 Hz at 125, repeated three times
  across both LFOs) and **127 = 26.9 Hz**. **Rate 0** is about 0.03 Hz (a
  33 s period), but only 1.2 cycles fit in a 41 s take, so it's unresolved.
  The Mod LFO matches the LFO within 3.4% (mean −0.8%).
- The Mod LFO is **silent without the modulation wheel**: CC1 = 0 gives no
  tremolo, CC1 = 127 gives full depth. Every Mod LFO take used CC1 = 127 with
  Modulation LFO Rate Control at 0 (`64`), so the wheel doesn't change the rate.

## Method

### Device setup

All edits went to Part 1's Temporary Tone on the test Studio Set (Part 1
receives on channel 1, and no other Part does). Addresses come from the core's
typed fields (`SnsCommonField`/`SnsPartialField` in
`crates/integral-core/src/sn_synth.rs`, `PcmsPartialField` in
`crates/integral-core/src/pcm_synth.rs`), at the bases
`ToneType::SnSynth.base_address(0)` = `19 01 00 00` and
`ToneType::PcmSynth.base_address(0)` = `19 00 00 00`. Every SysEx write was at
least 30 ms after the previous one.

The tone is an init-like one with a single steady partial: a sine wave, no
filter, flat full-sustain envelope, MFX Thru, and MFX chorus and reverb sends
at 0. The LFO under test is set to the **sine** waveform, routed to **amp
depth only** at the maximum, with every other depth at 0. Key trigger, fade,
delay, tempo sync, rate detune and analog feel are all off.

| Engine | Tone | Partial / wave | LFO under test |
|--------|------|----------------|----------------|
| SN Synth LFO | Part 1's own SN Synth preset (MSB 95, LSB 64, PC 38 raw) | Partial 1 only; OSC Wave SINE; FILTER BYPASS; AMP A 0, S 127, R 0; Analog Feel 0, Unison and RING off | `LfoShape` SIN, `LfoAmpDepth` +63, other depths 0, `LfoTempoSync` off, `LfoFadeTime` 0, `LfoKeyTrigger` off; every Mod LFO depth 0 |
| SN Synth Mod LFO | same | same | `LfoAmpDepth` 0, `ModLfoShape` SIN, `ModLfoAmpDepth` +63, `ModLfoTempoSync` off, `ModLfoRateControl` 0, **CC1 = 127** |
| PCM Synth LFO1 | PR-A 001 (MSB 87, LSB 64, PC 0 raw), loaded into Part 1 | Partial 1 only (PMT); INT 0564 Sine, mono; TVF OFF; TVA T1 0, L1–L3 127; matrix controls OFF; Analog Feel 0 | `Lfo1Waveform` SIN, `Lfo1TvaDepth` +63, other depths 0, `Lfo1Offset` 0, `Lfo1RateDetune` 0, `Lfo1DelayTime` 0, `Lfo1FadeTime` 0, `Lfo1KeyTrigger` off; LFO2 depths 0 |
| PCM Synth LFO2 | same | same | as LFO1 with the `Lfo2…` fields; `Lfo1TvaDepth` 0 |

The PCM `Lfo1Rate`/`Lfo2Rate` fields are nibblized (2 bytes). The value is
always below 128, so tempo-synced note values are not involved.

### Recording

INTEGRA-7 audio was recorded through the RØDECaster Pro II's multitrack
channel 2 (pre-fader, 48 kHz, 16-bit) with
`ffmpeg -f avfoundation -i ":2" -af "pan=mono|c0=c2"`, at an unchanged gain.
Each take starts ffmpeg, waits 1 s, plays C5 (note 72, velocity 100) and holds
it for 6 s (rates above 72), 10 s (41–72) or 20 s (40 and below). Rates 0, 4
and 8 also got a 45 s take. Every 8th value (0, 8, …, 120) plus 127 was
measured, with extra points where the curve bends.

### Analysis

1. Trim the take to the held note, drop the first 1.5 s (attack) and the last
   0.3 s.
2. Take the amplitude envelope (`|hilbert(x)|`), decimate it to 1 kHz and
   remove a linear trend.
3. Find the strongest FFT peak above 1.2 cycles per window (Hann window, 16×
   zero padding), then refine it with a least-squares sine fit within ±2% of
   the peak.

The envelope is clipped at the top (amp depth +63 drives the level past full
scale for half the cycle) and reaches silence at the trough. This makes the
waveform non-sinusoidal but leaves its fundamental where it was.

**Resolution.** A point is "well resolved" when the take holds at least 4
cycles. The 20 s cap leaves PCM rates 0–28 and SN rates 0–16 with 1.2–3.9
cycles. Most still land on the fit (see the tables), but treat them as ±10%.
Rate 0 on both SN LFOs (1.2 cycles in 41 s) is not resolved at all.

**Repeatability.** Repeated takes of the same value differ by up to 3–4%:
PCM 64 gave 1.240 and 1.276 Hz, PCM 96 gave 5.93 and 6.12 Hz, SN LFO 64 gave
1.947, 1.914, 1.884 and 1.884 Hz, and SN Mod 96 gave 4.58 and 4.77 Hz. That's
more than the fit's precision (< 0.3% at 10+ cycles), so the device itself
varies a little between notes. Per-cycle periods inside one take also snap to
a grid of about 10.7 ms. At SN 120, for example, cycles alternate between
61.2 ms and 71.9 ms. That's 512 samples at 48 kHz, so it may be the device's
LFO control rate (about 94 Hz) or a capture artefact; the glitches below share
the period. It doesn't bias the mean frequency, but individual cycles at the
top rates are uneven.

**Excluded takes.** Five takes (PCM LFO1 44, PCM LFO2 120, SN LFO 38 and 64,
SN Mod 122) had full-scale garbage samples (sample-to-sample jumps above
30 000) and a fake 93.75 or 187.5 Hz "tremolo" (48 000 / 512 and 48 000 /
256). These are recording dropouts, not device behaviour. Each was retaken,
and they're kept in the JSON with `"glitch": true`.

## Fits

### PCM Synth (LFO1 and LFO2)

```
f(rate) = 0.05438 Hz × 2^(rate / 14.172)        rate 0–127
```

Weighted least squares on log2 f (weight = cycles, capped at 20) over every
take with at least 1.5 cycles. On well-resolved points the largest residual
is −3.2% (rate 126; single takes reach +3.1%, at 96), and the rms over all
takes is 2.1%. Takes with fewer than 2.5 cycles reach −8.4% (LFO2 at 16, 1.9
cycles). Every well-resolved residual is within the take-to-take scatter, so
a lookup table would add nothing.

### SN Synth (LFO and Modulation LFO)

Continuous piecewise-linear in log2 f, with the joins found by grid search:

```
log2 f(r) = −4.38176 + 0.142372·r − 0.106941·max(r − 28.5, 0) + 0.034641·max(r − 91.5, 0)
            for 4 ≤ r ≤ 125
126 → 25.6 Hz, 127 → 26.9 Hz (measured), 0 → ≈ 0.03 Hz (unresolved)
```

| Segment | Rate span | Hz span | Doubles every |
|---------|-----------|---------|---------------|
| 1 | 4 – 28.5 | 0.071 – 0.80 | 7.0 steps |
| 2 | 28.5 – 91.5 | 0.80 – 3.75 | 28.2 steps |
| 3 | 91.5 – 125 | 3.75 – 19.1 | 14.3 steps |

The largest residual over 4–125 is 2.5% and the rms is 1.1%, within the
scatter. Three segments and two specials describe the curve, but that isn't
a closed form: Roland's table is probably a lookup with these slopes. For
the display format, a knot table (4, 28.5, 91.5, 125 and log-linear
interpolation) plus explicit entries for 0, 126 and 127 reproduces the curve
within the measurement error. A full 128-entry table would do too.

### Compact table (fit values, Hz)

| Rate | 0 | 8 | 16 | 24 | 32 | 40 | 48 | 56 | 64 | 72 | 80 | 88 | 96 | 104 | 112 | 120 | 126 | 127 |
|------|---|---|----|----|----|----|----|----|----|----|----|----|----|-----|-----|-----|-----|-----|
| PCM Synth | 0.054 | 0.080 | 0.119 | 0.176 | 0.26 | 0.39 | 0.57 | 0.84 | 1.24 | 1.84 | 2.72 | 4.02 | 5.95 | 8.80 | 13.0 | 19.2 | 25.8 | 27.1 |
| SN Synth | ≈0.03 | 0.106 | 0.233 | 0.512 | 0.870 | 1.06 | 1.29 | 1.57 | 1.91 | 2.33 | 2.83 | 3.44 | 4.67 | 6.89 | 10.2 | 15.0 | 25.6* | 26.9* |

\* measured, not fit.

## Raw measurements

Each cell lists the takes as `Hz (cycles in the analysed window)`. The
residual compares the geometric mean of the takes with at least 2.5 cycles to
the fit. Glitched takes are left out (see above).

#### PCM Synth LFO1 / LFO2

| Rate | pcm_lfo1 Hz (cycles) | pcm_lfo2 Hz (cycles) | Fit Hz | Residual |
|---|---|---|---|---|
| 0 | 0.052 (2.1) | 0.069 (1.2) | 0.054 | (< 2.5 cycles) |
| 4 | 0.064 (2.6) | – | 0.066 | -3.1% |
| 8 | 0.080 (3.2) | – | 0.080 | -0.6% |
| 12 | 0.093 (1.6) | – | 0.098 | (< 2.5 cycles) |
| 16 | 0.117 (2.0) | 0.109 (1.9) | 0.119 | (< 2.5 cycles) |
| 20 | 0.144 (2.4) | – | 0.145 | (< 2.5 cycles) |
| 24 | 0.179 (3.0), 0.175 (2.9) | – | 0.176 | +0.6% |
| 28 | 0.212 (3.5) | – | 0.214 | -0.8% |
| 32 | 0.262 (4.3) | 0.265 (4.4) | 0.260 | +1.4% |
| 36 | 0.316 (5.3) | – | 0.316 | -0.0% |
| 40 | 0.380 (6.5) | – | 0.385 | -1.1% |
| 44 | 0.469 (3.5) | – | 0.468 | +0.2% |
| 48 | 0.574 (4.3) | 0.569 (4.3) | 0.569 | +0.5% |
| 56 | 0.840 (6.4) | – | 0.841 | -0.1% |
| 64 | 1.240 (9.4), 1.276 (9.4) | 1.267 (9.4) | 1.244 | +1.4% |
| 72 | 1.853 (13.8) | – | 1.840 | +0.7% |
| 80 | 2.754 (10.6) | 2.721 (10.5) | 2.721 | +0.6% |
| 84 | 3.273 (12.7) | – | 3.309 | -1.1% |
| 88 | 4.012 (15.7) | – | 4.024 | -0.3% |
| 92 | 4.863 (19.3) | – | 4.893 | -0.6% |
| 96 | 5.934 (23.5), 6.116 (22.9) | 5.938 (23.4) | 5.950 | +0.8% |
| 100 | 7.218 (28.1) | – | 7.236 | -0.3% |
| 104 | 8.838 (34.9) | – | 8.800 | +0.4% |
| 108 | 10.786 (41.9) | – | 10.701 | +0.8% |
| 112 | 13.182 (51.2) | 13.011 (50.7) | 13.014 | +0.6% |
| 116 | 15.650 (61.9) | – | 15.826 | -1.1% |
| 120 | 19.732 (75.3), 19.379 (74.2) | 19.333 (76.6) | 19.246 | +1.2% |
| 122 | 21.433 (83.3) | – | 21.223 | +1.0% |
| 124 | 23.450 (91.7) | – | 23.404 | +0.2% |
| 125 | 24.188 (95.4) | – | 24.577 | -1.6% |
| 126 | 24.991 (101.2) | – | 25.809 | -3.2% |
| 127 | 27.016 (106.9) | 26.384 (106.3) | 27.103 | -1.5% |

#### SN Synth LFO / Modulation LFO

| Rate | sn_lfo Hz (cycles) | sn_mod Hz (cycles) | Fit Hz | Residual |
|---|---|---|---|---|
| 0 | 0.071 (1.2), 0.034 (1.2) | 0.030 (1.2) | – | – |
| 4 | 0.070 (2.8) | 0.070 (2.9) | 0.071 | -1.3% |
| 8 | 0.099 (1.6), 0.107 (4.2) | 0.106 (4.3) | 0.106 | +0.6% |
| 12 | 0.156 (2.6) | 0.159 (2.6) | 0.157 | +0.4% |
| 16 | 0.238 (3.9), 0.231 (3.9) | 0.235 (3.9) | 0.233 | +0.9% |
| 20 | 0.347 (5.8) | 0.344 (5.8) | 0.345 | +0.1% |
| 24 | 0.518 (8.6) | 0.504 (8.6) | 0.512 | -0.3% |
| 26 | 0.610 (10.5) | – | 0.624 | -2.3% |
| 28 | 0.755 (12.8) | 0.742 (12.8) | 0.760 | -1.6% |
| 30 | 0.835 (14.2) | – | 0.829 | +0.8% |
| 32 | 0.882 (14.7) | 0.868 (14.8), 0.879 (14.8) | 0.870 | +0.6% |
| 34 | 0.926 (15.5) | – | 0.914 | +1.3% |
| 36 | 0.973 (16.2) | 0.948 (16.3) | 0.960 | +0.0% |
| 38 | 1.004 (17.1) | – | 1.009 | -0.4% |
| 40 | 1.080 (18.0) | 1.043 (18.0) | 1.059 | +0.1% |
| 44 | 1.172 (8.9) | 1.160 (8.9) | 1.169 | -0.2% |
| 48 | 1.314 (9.8) | 1.281 (9.8) | 1.289 | +0.6% |
| 56 | 1.606 (11.9) | 1.567 (12.0) | 1.569 | +1.1% |
| 64 | 1.917 (14.6), 1.887 (14.5), 1.884 (14.6) | 1.900 (14.7), 1.938 (14.3) | 1.910 | -0.3% |
| 72 | 2.316 (17.7) | 2.308 (17.8) | 2.325 | -0.5% |
| 80 | 2.826 (11.2) | 2.830 (11.1) | 2.829 | -0.0% |
| 84 | 3.105 (12.3) | 3.095 (12.0) | 3.122 | -0.7% |
| 88 | 3.442 (13.5) | 3.442 (13.5) | 3.444 | -0.1% |
| 90 | 3.706 (14.1) | – | 3.617 | +2.5% |
| 92 | 3.837 (14.8) | 3.798 (14.9) | 3.845 | -0.7% |
| 94 | 4.244 (16.2) | – | 4.237 | +0.2% |
| 96 | 4.618 (17.9), 4.616 (18.0) | 4.585 (18.1), 4.759 (17.7) | 4.670 | -0.6% |
| 98 | 5.185 (19.7) | – | 5.146 | +0.8% |
| 100 | 5.751 (21.6) | 5.643 (22.1) | 5.671 | +0.5% |
| 102 | 6.260 (24.1) | – | 6.250 | +0.2% |
| 104 | 7.002 (26.1) | 6.844 (26.8) | 6.887 | +0.5% |
| 108 | 8.278 (32.2) | 8.336 (32.5) | 8.364 | -0.7% |
| 112 | 10.260 (39.0) | 10.096 (38.8) | 10.158 | +0.2% |
| 116 | 12.388 (47.9) | 12.522 (46.1) | 12.336 | +1.0% |
| 120 | 15.122 (58.5), 14.854 (57.9) | 14.949 (56.9), 15.160 (57.5) | 14.981 | +0.3% |
| 121 | 15.784 (61.0) | – | 15.727 | +0.4% |
| 122 | 16.409 (64.7) | 16.604 (64.2) | 16.509 | -0.0% |
| 123 | 17.383 (66.3) | – | 17.331 | +0.3% |
| 124 | 18.134 (71.5) | 18.203 (70.9) | 18.193 | -0.1% |
| 125 | 19.029 (75.4) | 18.658 (73.1) | 19.099 | -1.3% |
| 126 | 25.292 (100.7), 25.887 (98.8) | 25.603 (100.3) | – | – |
| 127 | 26.898 (105.5) | 26.875 (105.5) | – | – |

