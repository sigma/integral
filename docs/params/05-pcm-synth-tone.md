# PCM Synth Tone (PCMS) Parameters

Each PCM Synth Tone has settings for four partials (Partial 1--4), each with
WAVE, TVF, TVA, and LFO x 2, plus common and MFX settings. Parameters marked
with `*MC` can be controlled via Matrix Control.

## Signal Flow

```
PARTIAL 1-4 each:
  WAVE -> PITCH -> TVF -> TVA -> OUTPUT
            |        |      |
        PITCH ENV  TVF ENV TVA ENV
            |
        LFO 1 / LFO 2 / STEP LFO

All partials -> MFX -> Chorus/Reverb sends
```

---

## COMMON Tab

These parameters affect the entire tone (all partials).

| Parameter | Value | Description |
|-----------|-------|-------------|
| Tone Category | No assign, Ac.Piano, E.Piano, Organ, Other Keyboards, Accordion/Harmonica, Bell/Mallet, Ac.Guitar, E.Guitar, Dist.Guitar, Ac.Bass, E.Bass, Synth Bass, Plucked/Stroke, Strings, Brass, Wind, Flute, Sax, Recorder, Vox/Choir, Synth Lead, Synth Brass, Synth Pad/Strings, Synth Bellpad, Synth PolyKey, FX, Synth Seq/Pop, Phrase, Pulsating, Beat&Groove, Hit, Sound FX, Drums, Percussion, Combination | Selects the tone's category |
| Phrase Number | 0--243 | Number of the phrase that plays on preview |
| Phrase Octave Shift | -3--+3 | Pitch (in octave units) of the preview phrase |
| Tone Level | 0--127 | Overall volume of the tone |
| Tone Pan | L64--63R | Pan of the tone (L64=far left, 0=center, 63R=far right) |
| Tone Priority | LAST, LOUDEST | Note priority when max polyphony (128 voices) is exceeded |
| Octave Shift | -3--+3 | Pitch shift in octave units |
| Tone Coarse Tune | -48--+48 | Pitch shift in semitone steps (+/-4 octaves) |
| Tone Fine Tune | -50--+50 | Pitch shift in 1-cent steps (+/-50 cents) |
| Stretch Tune Depth | OFF, 1--3 | Applies stretched tuning (as used on acoustic pianos) |
| Analog Feel | 0--127 | Depth of 1/f modulation applied to the tone |
| Cutoff Offset | -63--+63 | Adjusts overall cutoff frequency (preserves per-partial differences) |
| Resonance Offset | -63--+63 | Adjusts overall resonance (preserves per-partial differences) |
| Attack Time Offset | -63--+63 | Adjusts overall attack time (TVA/TVF Env Time 1) |
| Release Time Offset | -63--+63 | Adjusts overall release time (TVA/TVF Env Time 4) |
| Velocity Sens Offset | -63--+63 | Adjusts overall velocity sensitivity |
| Mono/Poly | MONO, POLY | Polyphonic or monophonic playback |
| Legato Switch | OFF, ON | Enables legato (smooth transitions between notes in MONO mode) |
| Legato Retrigger | OFF, ON | Whether sounds retrigger during legato play |
| Portamento Switch | OFF, ON | Enables portamento (smooth pitch glide) |
| Portamento Mode | NORMAL, LEGATO | NORMAL: always applied; LEGATO: only when playing legato |
| Portamento Type | RATE, TIME | RATE: time depends on interval; TIME: constant time |
| Portamento Start | PITCH, NOTE | Where new portamento begins when changing notes mid-glide |
| Portamento Time | 0--127 | Time over which the pitch changes to the next note |

---

## WAVE Tab (Per-Partial)

These parameters are set independently for each of the 4 partials.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Partial Switch | OFF, ON | Turns the partial on/off |
| Wave Group | INT, SRX01--SRX12 | Selects waveform group (internal or expansion) |
| Wave No.L (Mono) | OFF, 1-- | Selects the waveform number (L channel / mono) |
| Wave No.R | OFF, 1-- | Selects the waveform number (R channel, stereo only) |
| Wave Gain | -6, 0, +6, +12 dB | Gain (amplification) of the waveform |
| Wave Tempo Sync | OFF, ON | Synchronizes phrase loop to tempo (SRX waveforms with BPM) |
| FXM Switch | OFF, ON | Enables Frequency Cross Modulation |
| FXM Color | 1--4 | How FXM performs frequency modulation (lower=metallic, higher=grainy) |
| FXM Depth | 0--16 | Depth of FXM modulation `*MC` |

### Partial Delay (Per-Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| Partial Delay Mode | NORM, HOLD, OFF-N, OFF-D | How the delay before sounding works: NORM=play after delay; HOLD=don't play if key released before delay; OFF-N/OFF-D=play after key release |
| Partial Delay Time | 0--127, Note Value | Time from key press (or release) until the partial sounds |

---

## PMT Tab (Partial Mix Table)

Controls how partials are combined. Applies to partial pairs (1&2, 3&4).

| Parameter | Value | Description |
|-----------|-------|-------------|
| Structure Type 1 & 2 | 1--10 | How partial 1 and 2 are connected (see types below) |
| Structure Type 3 & 4 | 1--10 | How partial 3 and 4 are connected |
| Booster 1 & 2 | 0, +6, +12, +18 dB | Booster depth for partials 1&2 (Type 3/4 only) |
| Booster 3 & 4 | 0, +6, +12, +18 dB | Booster depth for partials 3&4 (Type 3/4 only) |

### Key Range (Per-Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| Key Range Upper | LOWER--G9 | Highest note the partial will sound |
| Key Range Lower | C-1--UPPER | Lowest note the partial will sound |
| Key Fade Upper | 0--127 | Volume fade when playing above range (0=hard cutoff) |
| Key Fade Lower | 0--127 | Volume fade when playing below range (0=hard cutoff) |

### Velocity Range (Per-Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| PMT Velocity Control | OFF, ON, RANDOM, CYCLE | Controls how partials switch by velocity |
| Velo Range Upper | LOWER--127 | Highest velocity at which the partial sounds |
| Velo Range Lower | 1--UPPER | Lowest velocity at which the partial sounds |
| Velo Fade Upper | 0--127 | Volume fade above velocity range |
| Velo Fade Lower | 0--127 | Volume fade below velocity range |
| PMT Control Switch | OFF, ON | Enable Matrix Control for partial switching |

### Structure Types

| Type | Description |
|------|-------------|
| 1 | Independent partials |
| 2 | Two stacked filters; TVA1 controls balance |
| 3 | Booster distorts waveform, then combines filters |
| 4 | Booster distorts waveform; TVA1 controls balance and booster level |
| 5 | Ring modulator creates overtones; combines filters |
| 6 | Ring modulator; TVA1 controls balance and ring depth |
| 7 | Filter partial 1, ring modulate with partial 2 |
| 8 | Filter both, ring modulate, mix partial 2, then filter result |
| 9 | Ring modulate, mix partial 2, stack filters |
| 10 | Mix both, filter, then apply booster |

---

## PITCH Tab (Per-Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| Partial Coarse Tune | -48--+48 | Pitch shift in semitones `*MC` |
| Partial Fine Tune | -50--+50 | Pitch shift in cents `*MC` |
| Random Pitch Depth | 0, 1--10, 20--100 (by 10), 200--1200 (by 100) | Random pitch deviation per key press (cents) |
| Pitch Keyfollow | -200--+200 | Pitch change per octave (+100 = normal keyboard) |
| Pitch Bend Range Up | 0--+48 | Pitch bend range up (semitones) |
| Pitch Bend Range Down | 0---48 | Pitch bend range down (semitones) |

---

## PITCH ENV Tab (Per-Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| Pitch Env Depth | -12--+12 | Effect depth of pitch envelope (negative inverts) |
| Pitch Env V-Sens | -63--+63 | Velocity sensitivity of pitch envelope depth |
| Pitch Env T1 V-Sens | -63--+63 | Velocity effect on Time 1 |
| Pitch Env T4 V-Sens | -63--+63 | Key release speed effect on Time 4 |
| Pitch Env Time KF | -100--+100 | Key follow effect on envelope times (relative to C4) |
| Pitch Env Time 1--4 | 0--127 | Envelope time segments `*MC` |
| Pitch Env Level 0--4 | -63--+63 | Envelope level at each point (relative to standard pitch) |

---

## TVF Tab (Per-Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| Filter Type | OFF, LPF, BPF, HPF, PKG, LPF2, LPF3 | Type of filter (LPF2/LPF3 ignore Resonance) |
| Cutoff Frequency | 0--127 | Frequency at which the filter takes effect `*MC` |
| Resonance | 0--127 | Emphasis near cutoff frequency `*MC` |
| Cutoff Keyfollow | -200--+200 | Cutoff change per octave (relative to C4) |
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
| TVF Env Time KF | -100--+100 | Key follow effect on envelope times (relative to C4) |
| TVF Env Time 1--4 | 0--127 | Envelope time segments `*MC` |
| TVF Env Level 0--4 | 0--127 | Envelope level at each point (relative to cutoff) |

---

## TVA Tab (Per-Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| Partial Level | 0--127 | Volume of the partial `*MC` |
| Level V-Curve | FIXED, 1--7 | Velocity curve for volume |
| Level V-Sens | -63--+63 | Velocity sensitivity of volume |
| Bias Level | -100--+100 | Angle of volume change in bias direction |
| Bias Position | C-1--G9 | Key relative to which volume is modified |
| Bias Direction | LWR, UPR, L&U, ALL | Direction of bias effect from bias position |
| Partial Pan | L64--63R | Pan of the partial `*MC` |
| Pan Keyfollow | -100--+100 | Pan change per octave (relative to C4) |
| Random Pan Depth | 0--63 | Random pan change per key press |
| Alternate Pan Depth | L63--63R | Alternating L/R pan per key press |

---

## TVA ENV Tab (Per-Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| TVA Env T1 V-Sens | -63--+63 | Velocity effect on Time 1 |
| TVA Env T4 V-Sens | -63--+63 | Key release speed effect on Time 4 |
| TVA Env Time KF | -100--+100 | Key follow effect on envelope times (relative to C4) |
| TVA Env Time 1--4 | 0--127 | Envelope time segments `*MC` |
| TVA Env Level 1--3 | 0--127 | Envelope level at each point (relative to Partial Level) |

---

## OUTPUT Tab (Per-Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| Output Level | 0--127 | Signal level of the partial |
| Chorus Send Level | 0--127 | Level sent to chorus |
| Reverb Send Level | 0--127 | Level sent to reverb |

---

## LFO1 / LFO2 Tab (Per-Partial)

Each partial has two identical LFOs. All parameters below apply to both LFO1
and LFO2 independently.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Waveform | SIN, TRI, SAW-U, SAW-D, SQR, RND, BND-U, BND-D, TRP, S&H, CHAOS, VSIN, STEP | LFO waveform shape |
| Rate | 0--127, Note Value | Modulation speed `*MC` |
| Rate Detune | 0--127 | Random variation in LFO rate per key press |
| Offset | -100, -50, 0, +50, +100 | Shifts LFO waveform relative to center value |
| Delay Time | 0--127 | Time before LFO effect begins |
| Delay Time KF | -100--+100 | Key follow effect on delay time (relative to C4) |
| Fade Mode | ON<, ON>, OFF<, OFF> | How LFO is applied over time |
| Fade Time | 0--127 | Time for LFO amplitude to reach max/min |
| Key Trigger | OFF, ON | Whether LFO resets on key press |
| Pitch Depth | -63--+63 | LFO effect on pitch (vibrato) `*MC` |
| TVF Depth | -63--+63 | LFO effect on cutoff frequency (wah) `*MC` |
| TVA Depth | -63--+63 | LFO effect on volume (tremolo) `*MC` |
| Pan Depth | -63--+63 | LFO effect on pan `*MC` |

### Fade Mode Details

| Mode | Behavior |
|------|----------|
| ON< | Gradually apply LFO after key press (delay vibrato) |
| ON> | Apply LFO immediately, then gradually decrease |
| OFF< | Gradually apply LFO after key release |
| OFF> | Apply LFO while key held, gradually decrease after release |

---

## STEP LFO Tab (Per-Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| Step Type | TYP1, TYP2 | TYP1: abrupt level changes; TYP2: linear interpolation |
| LFO Step 1--16 | -36--+36 | Step data (at Pitch Depth +63, each +1 = +50 cents) |

---

## CTRL Tab (Per-Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| Env Mode | NOSUS, SUST | NOSUS: sound decays naturally; SUST: sound sustains while key held |
| Rx Bender | OFF, ON | Whether the partial receives Pitch Bend messages |
| Rx Expression | OFF, ON | Whether the partial receives Expression messages |
| Rx Hold-1 | OFF, ON | Whether the partial receives Hold-1 messages |
| Redamper Sw | OFF, ON | Whether sound sustains on Hold-1 after key release |

---

## MTRX CTRL 1--4 Tab (Common, affects all partials)

Up to 4 Matrix Controls can be used per tone. Each has the following
parameters:

| Parameter | Value | Description |
|-----------|-------|-------------|
| Source | OFF, CC01--CC31, CC33--CC95, PITCH BEND, AFTERTOUCH, SYS CTRL1--4, VELOCITY, KEYFOLLOW, TEMPO, LFO1, LFO2, PITCH ENV, TVF ENV, TVA ENV | MIDI message used to control the parameter |
| Dest 1--4 | OFF, PITCH, CUTOFF, RESONANCE, LEVEL, PAN, OUTPUT LEVEL, CHORUS SEND, REVERB SEND, LFO1/LFO2 PITCH DEPTH, LFO1/LFO2 TVF DEPTH, LFO1/LFO2 TVA DEPTH, LFO1/LFO2 PAN DEPTH, LFO1/LFO2 RATE, PIT ENV A-TIME, PIT ENV D-TIME, PIT ENV R-TIME, TVF ENV A-TIME, TVF ENV D-TIME, TVF ENV R-TIME, TVA ENV A-TIME, TVA ENV D-TIME, TVA ENV R-TIME, PMT, FXM DEPTH | Parameter to be controlled (up to 4 per Matrix Control) |
| Sens 1--4 | -63--+63 | Amount of control effect (0 = no effect) |
| Switch 1--4 | OFF, ON, REVS | Per-partial enable (REVS = effect applied in reverse) |

---

## MFX Tab (Common)

| Parameter | Value | Description |
|-----------|-------|-------------|
| MFX Switch | OFF, ON | Enables multi-effects processing |
| MFX Type | 0--67 | Selects one of 67 multi-effects types (see 07-mfx-types.md) |
| (per-type params) | varies | Parameters specific to the selected MFX type |
| MFX Chorus Send Level | 0--127 | Chorus send for sound passing through MFX |
| MFX Reverb Send Level | 0--127 | Reverb send for sound passing through MFX |

---

## MFX CTRL Tab (Common)

Up to 4 MFX Controls for realtime effect parameter changes.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Source (1--4) | OFF, CC01--31, CC33--95, PITCH BEND, AFTERTOUCH, SYS CTRL1--SYS CTRL4 | MIDI message used to control MFX parameter |
| Destination (1--4) | (depends on MFX type) | MFX parameter to control (see 07-mfx-types.md for # params) |
| Sens (1--4) | -63--+63 | Amount of control effect |
