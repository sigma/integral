# SuperNATURAL Synth Tone (SN-S) Parameters

## Overview

SuperNATURAL Synth Tones use a virtual analog synth engine with three
partials (Partial 1--3), each with independent OSC, FILTER, AMP, and LFO
sections. Additionally there is a MOD LFO (modulation wheel), AFTERTOUCH,
MISC settings, and multi-effect (MFX) settings.

```
PARTIAL 1--3
  OSC -> FILTER -> AMP -> MFX
  LFO --------^------^
  MOD LFO
  AFTERTOUCH
  MISC
```

---

## COMMON Tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Tone Category | No assign, Ac.Piano, E.Piano, Organ, Other Keyboards, Accordion/Harmonica, Bell/Mallet, Ac.Guitar, E.Guitar, Dist.Guitar, Ac.Bass, E.Bass, Synth Bass, Plucked/Stroke, Strings, Brass, Wind, Flute, Sax, Recorder, Vox/Choir, Synth Lead, Synth Brass, Synth Pad/Strings, Synth Bellpad, Synth PolyKey, FX, Synth Seq/Pop, Phrase, Pulsating, Beat&Groove, Hit, Sound FX, Drums, Percussion, Combination | Selects the tone's category |
| Phrase Number | 0--243 | Preview phrase number |
| Phrase Octave Shift | -3--+3 | Pitch (in octave units) of the preview phrase |
| Tone Level | 0--127 | Overall volume of the tone |
| RING Switch | OFF, ON | Turns ring modulator on/off. Multiplies Partial 1 OSC by Partial 2 OSC for metallic bell-like tones. When on, Partial 1/2 PW Mod Depth, PW, and Super Saw Detune are disabled. |
| Wave Shape | 0--127 | Partial 1 is modulated by pitch of Partial 2. Higher values = greater effect. No effect if Partial 1 waveform is PW-SQR or SP-SAW. |
| Analog Feel | 0--127 | Applies 1/f fluctuation for natural analog synth instability |
| Unison Switch | OFF, ON | Layers a single sound. Number of layered notes depends on keys pressed. |
| Unison Size | 2, 4, 6, 8 | Number of notes layered per key (auto-adjusted by polyphony) |
| Mono/Poly | POLY, MONO | Polyphonic or monophonic playback |
| Legato Switch | OFF, ON | Valid only when MONO. New key while holding previous key changes pitch without retriggering. |
| Portamento Switch | OFF, ON | Enables/disables portamento effect |
| Portamento Time | 0--127 | Time taken for pitch to glide to next note. Higher = longer. |
| Portamento Mode | NORMAL, LEGATO | NORMAL: always applied. LEGATO: only when playing legato. |

---

## OSC Tab (per Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| Partial Switch | OFF, ON | Turns the partial on/off |
| Wave | SAW, SQR, PW-SQR, TRI, SINE, NOISE, SP-SAW (Super Saw), PCM | Selects the oscillator waveform |
| Wave Variation | A, B, C | Selects a variation of the current waveform. No effect for SP-SAW or PCM. |
| Wave Number | 1--450 | Selects the PCM waveform. Valid only when Wave is PCM. |
| Wave Gain | -6, 0, +6, +12 dB | Gain of the waveform in 6 dB steps. Valid only when Wave is PCM. |
| Pulse Width Mod Depth | 0--127 | Amount of LFO modulation applied to pulse width. Valid for PW-SQR. Disabled when Ring Switch is on (Partials 1/2). |
| Pulse Width | 0--127 | Width of the upper portion of the square wave. Valid for PW-SQR. Disabled when Ring Switch is on (Partials 1/2). |
| Pulse Width Shift | 0--127 | Shifts the range of PW change. Normally leave at 127. Disabled when Ring Switch is on (Partials 1/2). |
| Super Saw Detune | 0--127 | Pitch difference between the 7 layered sawtooth waves. Higher = wider detuning. Valid for SP-SAW only. Disabled when Ring Switch is on (Partials 1/2). |

### Waveform Descriptions

| Waveform | Description |
|----------|-------------|
| SAW | Sawtooth wave -- contains all integer harmonics |
| SQR | Square wave -- contains odd-numbered harmonics |
| PW-SQR | Pulse width square wave -- overtone structure varies with pulse width |
| TRI | Triangle wave -- contains even-numbered harmonics |
| SINE | Pure sine wave -- single frequency, basis of all sound |
| NOISE | White noise -- all frequencies, suited for percussion and SFX |
| SP-SAW | Super Saw -- 7 detuned sawtooth waves for thick pad/string sounds |
| PCM | PCM sample playback |

---

## PITCH Tab (per Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| OSC Pitch | -24--+24 | Adjusts pitch in semitone steps |
| OSC Detune | -50--+50 | Adjusts pitch in cent steps |
| Pitch Env Attack Time | 0--127 | Time from key-on until pitch reaches highest/lowest point |
| Pitch Env Decay Time | 0--127 | Time from pitch peak until it returns to played key pitch |
| Pitch Env Depth | -63--+63 | How much the pitch envelope affects pitch |
| Octave Shift | -3--+3 | Octave of the tone |
| Pitch Bend Range Up | 0--+24 | Pitch change when bend lever is fully right (semitones) |
| Pitch Bend Range Down | 0---24 | Pitch change when bend lever is fully left (semitones) |

---

## FILTER Tab (per Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| FILTER Mode | BYPASS, LPF1, LPF2, LPF3, LPF4, HPF, BPF, PKG | Filter type selection |
| FILTER Slope | -12, -24 dB | Steepness of the filter rolloff |
| FILTER Cutoff | 0--127 | Cutoff frequency |
| FILTER Cutoff KF | -100--+100 | Keyboard follow -- how cutoff varies by key position |
| FILTER Env V-Sens | -63--+63 | How velocity affects filter envelope depth |
| FILTER Resonance | 0--127 | Emphasizes overtones near the cutoff frequency |
| FILTER Env Attack | 0--127 | Time from key-on until cutoff reaches its peak |
| FILTER Env Decay | 0--127 | Time from peak until cutoff decays to sustain level |
| FILTER Env Sustain | 0--127 | Cutoff level maintained until key release |
| FILTER Env Release | 0--127 | Time from key release until cutoff reaches minimum |
| FILTER Env Depth | -63--+63 | Direction and depth of cutoff frequency change |
| HPF Cutoff | 0--127 | Cutoff frequency of independent -6 dB high-pass filter |

### Filter Mode Descriptions

| Mode | Description |
|------|-------------|
| BYPASS | No filtering |
| LPF1--LPF4 | Low-pass filter variants (different resonance character) |
| HPF | High-pass filter |
| BPF | Band-pass filter |
| PKG | Peaking filter |

---

## AMP Tab (per Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| AMP Level | 0--127 | Partial volume |
| AMP Level V-Sens | -63--+63 | How velocity affects volume |
| AMP Pan | L64--63R | Stereo position of the partial |
| AMP Level Keyfollow | -100 to +100 (steps of 10) | Volume change by key position, centered on C4 |
| AMP Env Attack | 0--127 | Time from key-on until maximum volume |
| AMP Env Decay | 0--127 | Time from max volume until sustain level |
| AMP Env Sustain | 0--127 | Volume level maintained until key release |
| AMP Env Release | 0--127 | Time from key release until silence |

---

## LFO Tab (per Partial)

| Parameter | Value | Description |
|-----------|-------|-------------|
| LFO Shape | TRI, SIN, SAW, SQR, S&H, RND | LFO waveform |
| LFO Rate | 0--127 | LFO speed (when Tempo Sync is OFF) |
| LFO Tempo Sync Sw | OFF, ON | When ON, LFO rate syncs to tempo as a note value |
| LFO Tempo Sync Note | 16, 12, 8, 4, 2, 1, 3/4, 2/3, 1/2, 3/8, 1/3, 1/4, 3/16, 1/6, 1/8, 3/32, 1/12, 1/16, 1/24, 1/32 | LFO rate as note value (when Tempo Sync is ON) |
| LFO Fade Time | 0--127 | Time from note-on until LFO reaches maximum amplitude |
| LFO Key Trigger | OFF, ON | When on, LFO cycle restarts on each key press |
| LFO Pitch Depth | -63--+63 | LFO modulation of pitch (vibrato) |
| LFO FILTER Depth | -63--+63 | LFO modulation of cutoff frequency (wah) |
| LFO AMP Depth | -63--+63 | LFO modulation of volume (tremolo) |
| LFO Pan Depth | -63--+63 | LFO modulation of pan position (auto-panning) |

### LFO Shape Descriptions

| Shape | Description |
|-------|-------------|
| TRI | Triangle wave |
| SIN | Sine wave |
| SAW | Sawtooth wave |
| SQR | Square wave |
| S&H | Sample and Hold (changes once per cycle) |
| RND | Random wave |

---

## MOD LFO Tab (per Partial)

The Modulation LFO is applied when the modulation lever is moved. It is
separate from the always-on LFO.

| Parameter | Value | Description |
|-----------|-------|-------------|
| ModLFO Shape | TRI, SIN, SAW, SQR, S&H, RND | Modulation LFO waveform |
| ModLFO Rate | 0--127 | Modulation LFO speed (when Tempo Sync is OFF) |
| ModLFO TempoSyncSw | OFF, ON | When ON, rate syncs to tempo |
| ModLFO TempoSyncNote | 16, 12, 8, 4, 2, 1, 3/4, 2/3, 1/2, 3/8, 1/3, 1/4, 3/16, 1/6, 1/8, 3/32, 1/12, 1/16, 1/24, 1/32 | Rate as note value (when Tempo Sync is ON) |
| ModLFO Pitch Depth | -63--+63 | LFO modulation of pitch (vibrato via mod wheel) |
| ModLFO FILTER Depth | -63--+63 | LFO modulation of cutoff frequency (wah via mod wheel) |
| ModLFO AMP Depth | -63--+63 | LFO modulation of volume (tremolo via mod wheel) |
| ModLFO Pan Depth | -63--+63 | LFO modulation of pan position (auto-panning via mod wheel) |
| ModLFO Rate Control | -63--+63 | How the mod lever changes the Modulation LFO rate. Positive = speeds up, negative = slows down. |

---

## AFTERTOUCH Tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Cutoff Aftertouch Sens | -63--+63 | How aftertouch affects cutoff frequency. Positive = raise, negative = lower. |
| Level Aftertouch Sens | -63--+63 | How aftertouch affects volume. Positive = increase, negative = decrease. |

---

## MISC Tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Attack Time Interval Sens | 0--127 | Shortens FILTER and AMP attack time based on note-on spacing. Effective for rapid notes with slow-attack sounds. 0 = no effect. |
| Release Time Interval Sens | 0--127 | Shortens FILTER and AMP release time if note-on to next note-off interval is brief. Effective for staccato with slow-release sounds. 0 = no effect. |
| Portamento Time Interval Sens | 0--127 | Shortens portamento time based on note-on spacing. 0 = no effect. |
| Envelope Loop Mode | OFF, FREE-RUN, TEMPO-SYNC | OFF: normal envelope. FREE-RUN: Attack-Decay loops until note-off. TEMPO-SYNC: loops at specified sync note rate. |
| Envelope Loop Sync Note | (Note values) | Loop rate when TEMPO-SYNC is selected. If Attack+Decay is shorter than the rate, Sustain Level is maintained. |
| Chromatic Portamento | OFF, ON | When on, portamento operates in semitone steps |

---

## MFX Tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| MFX Switch | OFF, ON | Enables/disables multi-effect |
| MFX Type | 0--67 | Selects from 67 available MFX types (see MFX Parameters) |
| (MFX type parameters) | (varies) | Edit parameters for the selected MFX type |
| MFX Chorus Send Level | 0--127 | Amount of chorus for MFX output. No effect if motional surround is on. |
| MFX Reverb Send Level | 0--127 | Amount of reverb for MFX output. No effect if motional surround is on. |

---

## MFX CTRL Tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Source (1--4) | OFF, CC01--31, CC33--95, PITCH BEND, AFTERTOUCH, SYS CTRL1--4 | MIDI message used to control MFX parameters |
| Destination (1--4) | (depends on MFX type) | MFX parameter to be controlled |
| Sens (1--4) | -63--+63 | Amount of control effect. Positive = increase, negative = decrease. 0 = no effect. |
