# PCM Drum Kit (PCMD) Parameters

Each PCM Drum Kit has 88 partials (one per key, mapped to note numbers).
Each partial has 4 wave generators (WMT) plus TVF, TVA, and pitch envelope
settings. The kit also has common settings, MFX, and up to 6 Comp+EQ units.

## Signal Flow

```
PARTIAL 1-88 each:
  WAVE (x4 WMT) -> TVF -> TVA -> OUTPUT
                     |      |
                 TVF ENV  TVA ENV
                     |
                 PITCH ENV

All partials -> MFX -> Chorus/Reverb sends
                  |
              COMP+EQ 1-6 (for one assigned part)
```

---

## COMMON Tab

These parameters affect the entire drum kit or individual drum partials.

### Kit-Level

| Parameter | Value | Description |
|-----------|-------|-------------|
| Phrase Number | 0--18 | Number of the phrase that plays on preview |
| Drum Kit Level | 0--127 | Overall volume of the drum kit |

### Per-Partial Common

| Parameter | Value | Description |
|-----------|-------|-------------|
| Partial Name | 12 characters | Name assigned to the drum partial |
| Assign Type | MULTI, SINGLE | MULTI: layer sounds on repeated keys; SINGLE: cut previous sound |
| Mute Group | OFF, 1--31 | Partials in the same group cannot sound simultaneously (e.g., open/closed hi-hat) |
| Partial Env Mode | NO-SUS, SUSTAIN | NO-SUS: natural decay; SUSTAIN: sustain while key held |
| Partial Pitch Bend Range | 0--48 | Pitch bend range in semitones (same for both directions) |
| Partial Rx Expression | OFF, ON | Whether the partial receives Expression messages |
| Partial Rx Hold-1 | OFF, ON | Whether the partial receives Hold-1 messages |
| One Shot Mode | OFF, ON | Sound plays to end of waveform regardless of key release |

---

## WAVE Tab (Per-Partial, x4 Waves)

Each drum partial has 4 wave generators (Wave 1--4). All parameters below
are per-wave.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Wave 1--4 Switch | OFF, ON | Turns the wave on/off |
| Wave Group | INT, SRX | Selects waveform group (internal or expansion) |
| Wave No.L (Mono) | OFF, 1-- | Selects the waveform (L channel / mono) |
| Wave No.R | OFF, 1-- | Selects the waveform (R channel, stereo only) |
| Wave Gain | -6, 0, +6, +12 dB | Gain of the waveform |
| Wave Tempo Sync | OFF, ON | Synchronizes phrase loop to tempo (SRX waveforms with BPM) |
| Wave FXM Switch | OFF, ON | Enables Frequency Cross Modulation |
| Wave FXM Color | 1--4 | FXM modulation character (lower=metallic, higher=grainy) |
| Wave FXM Depth | 0--16 | Depth of FXM modulation |
| Wave Coarse Tune | -48--+48 | Pitch shift in semitone steps |
| Wave Fine Tune | -50--+50 | Pitch shift in 1-cent steps |
| Wave Level | 0--127 | Volume of the waveform |
| Wave Pan | L64--63R | Pan of the waveform |
| Wave Random Pan Sw | OFF, ON | Randomize pan each key press |
| Wave Alter Pan Sw | OFF, ON, REVS | Alternate panning L/R each key press |

---

## WMT Tab (Wave Mix Table, Per-Partial)

Controls how the 4 waves within a partial are selected by velocity.

| Parameter | Value | Description |
|-----------|-------|-------------|
| WMT Velocity Control | OFF, ON, RANDOM | OFF: no velocity switching; ON: switch by velocity; RANDOM: random selection |
| Velo Fade Upper | 0--127 | Volume fade above velocity range (0=hard cutoff) |
| Velo Range Upper | LOWER--127 | Highest velocity at which the wave sounds |
| Velo Range Lower | 1--UPPER | Lowest velocity at which the wave sounds |
| Velo Fade Lower | 0--127 | Volume fade below velocity range (0=hard cutoff) |

---

## PITCH Tab (Per-Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| Partial Coarse Tune | C-1--G9 | Selects the pitch (note) at which the partial sounds |
| Partial Fine Tune | -50--+50 | Fine pitch adjustment in cents |
| Partial Random Pitch Depth | 0, 1--10, 20--100 (by 10), 200--1200 (by 100) | Random pitch deviation per key press (cents) |

---

## PITCH ENV Tab (Per-Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| Pitch Env Depth | -12--+12 | Effect depth of pitch envelope (negative inverts) |
| Pitch Env V-Sens | -63--+63 | Velocity sensitivity of pitch envelope depth |
| Pitch Env T1 V-Sens | -63--+63 | Velocity effect on Time 1 |
| Pitch Env T4 V-Sens | -63--+63 | Key release speed effect on Time 4 |
| Pitch Env Time 1--4 | 0--127 | Envelope time segments |
| Pitch Env Level 0--4 | -63--+63 | Envelope level at each point (relative to standard pitch) |

---

## TVF Tab (Per-Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| Filter Type | OFF, LPF, BPF, HPF, PKG, LPF2, LPF3 | Type of filter (LPF2/LPF3 ignore Resonance) |
| Cutoff Frequency | 0--127 | Frequency at which the filter takes effect |
| Resonance | 0--127 | Emphasis near cutoff frequency |
| Cutoff V-Curve | FIXED, 1--7 | Velocity curve for cutoff frequency |
| Cutoff V-Sens | -63--+63 | Velocity sensitivity of cutoff frequency |
| Resonance V-Sens | -63--+63 | Velocity sensitivity of resonance |

---

## TVF ENV Tab (Per-Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| TVF Env Depth | -63--+63 | Depth of TVF envelope (negative inverts) |
| TVF Env V-Curve | FIXED, 1--7 | Velocity curve for TVF envelope |
| TVF Env V-Sens | -63--+63 | Velocity sensitivity of TVF envelope depth |
| TVF Env T1 V-Sens | -63--+63 | Velocity effect on Time 1 |
| TVF Env T4 V-Sens | -63--+63 | Key release speed effect on Time 4 |
| TVF Env Time 1--4 | 0--127 | Envelope time segments |
| TVF Env Level 0--4 | 0--127 | Envelope level at each point (relative to cutoff) |

---

## TVA Tab (Per-Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| Partial Level | 0--127 | Volume of the drum partial |
| Level V-Curve | FIXED, 1--7 | Velocity curve for volume |
| Level V-Sens | -63--+63 | Velocity sensitivity of volume |
| Partial Pan | L64--63R | Pan of the drum partial |
| Random Pan Depth | 0--63 | Random pan change per key press (only waves with Random Pan Sw ON) |
| Alternate Pan Depth | L63--63R | Alternating L/R pan per key press (only waves with Alter Pan Sw ON/REVS) |
| Relative Level | -64--+63 | Volume correction (set by key-based SysEx, normally 0) |

---

## TVA ENV Tab (Per-Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| TVA Env T1 V-Sens | -63--+63 | Velocity effect on Time 1 |
| TVA Env T4 V-Sens | -63--+63 | Key release speed effect on Time 4 |
| TVA Env Time 1--4 | 0--127 | Envelope time segments |
| TVA Env Level 1--3 | 0--127 | Envelope level at each point (relative to Partial Level) |

---

## OUTPUT Tab (Per-Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| Partial Output Assign | PART, COMP+EQ1--6 | How the partial's sound is routed (direct to part or through Comp+EQ) |
| Partial Output Level | 0--127 | Signal level of the partial |
| Partial Chorus Send Level | 0--127 | Level sent to chorus |
| Partial Reverb Send Level | 0--127 | Level sent to reverb |

---

## COMP Tab (Kit-Level, x6 Units)

Comp+EQ can only be used for the part specified by Drum Comp+EQ Assign.
6 independent compressor units.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Comp 1--6 Switch | OFF, ON | Compressor on/off |
| Comp 1--6 Attack Time | 0.05--50.0 ms | Time from input exceeding threshold until compression begins |
| Comp 1--6 Release Time | 0.05--2000 ms | Time from input falling below threshold until compression stops |
| Comp 1--6 Threshold | 0--127 | Level above which compression is applied |
| Comp 1--6 Ratio | 1:1--inf:1 | Compression ratio |
| Comp 1--6 Output Gain | 0--+24 dB | Output level |

---

## EQ Tab (Kit-Level, x6 Units)

6 independent 3-band equalizer units. Only available for the Drum Comp+EQ
Assign part.

| Parameter | Value | Description |
|-----------|-------|-------------|
| EQ 1--6 Switch | OFF, ON | Equalizer on/off |
| EQ 1--6 Low Freq | 200, 400 Hz | Low range frequency |
| EQ 1--6 Low Gain | -15--+15 dB | Low range gain |
| EQ 1--6 Mid Freq | 200, 250, 315, 400, 500, 630, 800, 1000, 1250, 1600, 2000, 2500, 3150, 4000, 5000, 6300, 8000 Hz | Mid range frequency |
| EQ 1--6 Mid Gain | -15--+15 dB | Mid range gain |
| EQ 1--6 Mid Q | 0.5, 1.0, 2.0, 4.0, 8.0 | Mid range width (higher Q = narrower) |
| EQ 1--6 High Freq | 2000, 4000, 8000 Hz | High range frequency |
| EQ 1--6 High Gain | -15--+15 dB | High range gain |

---

## MFX Tab (Kit-Level)

| Parameter | Value | Description |
|-----------|-------|-------------|
| MFX Switch | OFF, ON | Enables multi-effects processing |
| MFX Type | 0--67 | Selects one of 67 multi-effects types (see 07-mfx-types.md) |
| (per-type params) | varies | Parameters specific to the selected MFX type |
| MFX Chorus Send Level | 0--127 | Chorus send for sound passing through MFX |
| MFX Reverb Send Level | 0--127 | Reverb send for sound passing through MFX |

---

## MFX CTRL Tab (Kit-Level)

Up to 4 MFX Controls for realtime effect parameter changes.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Source (1--4) | OFF, CC01--31, CC33--95, PITCH BEND, AFTERTOUCH, SYS CTRL1--SYS CTRL4 | MIDI message used to control MFX parameter |
| Destination (1--4) | (depends on MFX type) | MFX parameter to control (see 07-mfx-types.md for # params) |
| Sens (1--4) | -63--+63 | Amount of control effect |
