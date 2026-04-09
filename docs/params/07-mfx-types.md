# MFX Types and Parameters

The INTEGRA-7 provides 67 multi-effects types (plus Thru). Parameters marked
with `#` can be controlled via MFX Control. Parameters marked `#1`/`#2`
change two settings simultaneously.

## MFX Type Index

| ID | Name | Category | Description |
|----|------|----------|-------------|
| 0 | Thru | -- | No effect (bypass) |
| 1 | Equalizer | Filter | 4-band stereo EQ (low, mid x2, high) |
| 2 | Spectrum | Filter | Stereo spectrum (8-band filter) |
| 3 | Low Boost | Filter | Boosts lower range volume |
| 4 | Step Filter | Filter | Filter with step-sequenced cutoff (16 steps) |
| 5 | Enhancer | Filter | Controls high-frequency overtone structure |
| 6 | Auto Wah | Filter | Cyclically controlled filter (wah) |
| 7 | Humanizer | Filter | Adds vowel character (a, e, i, o, u) |
| 8 | Speaker Simulator | Filter | Simulates speaker type and mic placement |
| 9 | Phaser 1 | Modulation | Phase-shifted sound added to original |
| 10 | Phaser 2 | Modulation | Analog phaser simulation (for electric piano) |
| 11 | Phaser 3 | Modulation | Different analog phaser (for electric piano) |
| 12 | Step Phaser | Modulation | Phaser with gradual step changes |
| 13 | Multi Stage Phaser | Modulation | Deep phaser with up to 24 stages |
| 14 | Infinite Phaser | Modulation | Continuously raises/lowers modulation frequency |
| 15 | Ring Modulator | Modulation | AM modulation for bell-like sounds |
| 16 | Tremolo | Modulation | Cyclic volume modulation |
| 17 | Auto Pan | Modulation | Cyclic stereo location modulation |
| 18 | Slicer | Modulation | Successive cuts creating rhythmic backing (16 steps) |
| 19 | Rotary 1 | Modulation | Classic rotary speaker simulation |
| 20 | Rotary 2 | Modulation | Modified rotary with boosted low end (VK-7 style) |
| 21 | Rotary 3 | Modulation | Rotary speaker with overdrive |
| 22 | Chorus | Chorus | Stereo chorus with filter |
| 23 | Flanger | Chorus | Stereo flanger (metallic resonance) |
| 24 | Step Flanger | Chorus | Flanger with stepped pitch changes |
| 25 | Hexa-Chorus | Chorus | Six-phase chorus for spatial richness |
| 26 | Tremolo Chorus | Chorus | Chorus with added tremolo modulation |
| 27 | Space-D | Chorus | Multiple chorus, transparent, no modulation impression |
| 28 | Overdrive | Dynamics | Heavy overdrive distortion |
| 29 | Distortion | Dynamics | Heavy distortion (same params as Overdrive) |
| 30 | Guitar Amp Simulator | Dynamics | Guitar amplifier simulation with speaker |
| 31 | Compressor | Dynamics | Smooths volume fluctuations |
| 32 | Limiter | Dynamics | Prevents distortion from excessive levels |
| 33 | Gate | Dynamics | Cuts reverb tail based on volume |
| 34 | Delay | Delay | Stereo delay (normal/cross feedback modes) |
| 35 | Modulation Delay | Delay | Delay with added modulation |
| 36 | 3Tap Pan Delay | Delay | Three delay taps: center, left, right |
| 37 | 4Tap Pan Delay | Delay | Four delay taps with individual pan/level |
| 38 | Multi Tap Delay | Delay | Four delays with individual timing, pan, level |
| 39 | Reverse Delay | Delay | Reverse delay plus tap delay |
| 40 | Time Ctrl Delay | Delay | Delay with smooth time changes |
| 41 | LOFI Compress | Lo-Fi | Intentional sound quality degradation |
| 42 | Bit Crasher | Lo-Fi | Lo-fi via sample rate and bit depth reduction |
| 43 | Pitch Shifter | Pitch | Stereo pitch shifter |
| 44 | 2Voice Pitch Shifter | Pitch | Two independent pitch shifters |
| 45 | Overdrive -> Chorus | Combination | Overdrive followed by chorus |
| 46 | Overdrive -> Flanger | Combination | Overdrive followed by flanger |
| 47 | Overdrive -> Delay | Combination | Overdrive followed by delay |
| 48 | Distortion -> Chorus | Combination | Distortion followed by chorus |
| 49 | Distortion -> Flanger | Combination | Distortion followed by flanger |
| 50 | Distortion -> Delay | Combination | Distortion followed by delay |
| 51 | OD/DS -> TouchWah | Combination | Overdrive/distortion with touch wah |
| 52 | OD/DS -> AutoWah | Combination | Overdrive/distortion with auto wah |
| 53 | GuitarAmpSim -> Chorus | Combination | Guitar amp sim followed by chorus |
| 54 | GuitarAmpSim -> Flanger | Combination | Guitar amp sim followed by flanger |
| 55 | GuitarAmpSim -> Phaser | Combination | Guitar amp sim followed by phaser |
| 56 | GuitarAmpSim -> Delay | Combination | Guitar amp sim followed by delay |
| 57 | EP AmpSim -> Tremolo | Combination | Electric piano amp with tremolo |
| 58 | EP AmpSim -> Chorus | Combination | Electric piano amp with chorus |
| 59 | EP AmpSim -> Flanger | Combination | Electric piano amp with flanger |
| 60 | EP AmpSim -> Phaser | Combination | Electric piano amp with phaser |
| 61 | EP AmpSim -> Delay | Combination | Electric piano amp with delay |
| 62 | Enhancer -> Chorus | Combination | Enhancer followed by chorus |
| 63 | Enhancer -> Flanger | Combination | Enhancer followed by flanger |
| 64 | Enhancer -> Delay | Combination | Enhancer followed by delay |
| 65 | Chorus -> Delay | Combination | Chorus followed by delay |
| 66 | Flanger -> Delay | Combination | Flanger followed by delay |
| 67 | Chorus -> Flanger | Combination | Chorus followed by flanger |

---

## Detailed Parameters by MFX Type

### 01: Equalizer

4-band stereo EQ (low, mid x2, high).

| Parameter | Value | Description |
|-----------|-------|-------------|
| Low Freq | 200, 400 Hz | Frequency of the low range |
| Low Gain `#` | -15--+15 dB | Gain of the low range |
| Mid1 Freq | 200--8000 Hz | Frequency of the middle range 1 |
| Mid1 Gain | -15--+15 dB | Gain of the middle range 1 |
| Mid1 Q | 0.5, 1.0, 2.0, 4.0, 8.0 | Width of the middle range 1 |
| Mid2 Freq | 200--8000 Hz | Frequency of the middle range 2 |
| Mid2 Gain | -15--+15 dB | Gain of the middle range 2 |
| Mid2 Q | 0.5, 1.0, 2.0, 4.0, 8.0 | Width of the middle range 2 |
| High Freq | 2000, 4000, 8000 Hz | Frequency of the high range |
| High Gain `#` | -15--+15 dB | Gain of the high range |
| Level `#` | 0--127 | Output level |

### 02: Spectrum

Stereo 8-band filter spectrum.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Band1--8 (250--8000 Hz) | -15--+15 dB | Gain of each frequency band |
| Q | 0.5, 1.0, 2.0, 4.0, 8.0 | Width of all frequency bands |
| Level `#` | 0--127 | Output level |

### 03: Low Boost

Boosts the volume of the lower range.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Boost Frequency `#` | 50--125 Hz | Center frequency for bass boost |
| Boost Gain `#` | 0--+12 dB | Amount of boost |
| Boost Width | WIDE, MID, NARROW | Width of the boosted range |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level | 0--127 | Output level |

### 04: Step Filter

Filter with 16-step sequenced cutoff frequency. MFX Control can reset
the step sequence.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Step 01--16 | 0--127 | Cutoff frequency at each step |
| Rate `#` | 0.05--10.00 Hz, note | Rate of modulation |
| Attack `#` | 0--127 | Speed of cutoff change between steps |
| Filter Type | LPF, BPF, HPF, NOTCH | Type of filter |
| Filter Slope | -12, -24, -36 dB | Attenuation per octave |
| Filter Resonance `#` | 0--127 | Filter resonance level |
| Filter Gain | 0--+12 dB | Amount of boost for filter output |
| Level | 0--127 | Output level |

### 05: Enhancer

Controls high-frequency overtone structure.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Sens `#` | 0--127 | Sensitivity of the enhancer |
| Mix `#` | 0--127 | Level of generated overtones |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level | 0--127 | Output level |

### 06: Auto Wah

Cyclically controlled filter producing wah effect.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Filter Type | LPF, BPF | LPF: wide frequency wah; BPF: narrow |
| Manual `#` | 0--127 | Center frequency of the wah effect |
| Peak | 0--127 | Wah effect amount near center frequency |
| Sens `#` | 0--127 | Sensitivity of filter control |
| Polarity | UP, DOWN | Direction of filter movement |
| Rate `#` | 0.05--10.00 Hz, note | Modulation frequency |
| Depth `#` | 0--127 | Modulation depth |
| Phase `#` | 0--180 deg | L/R phase shift of wah effect |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level | 0--127 | Output level |

### 07: Humanizer

Adds vowel character to the sound.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Drive Sw | OFF, ON | Overdrive on/off |
| Drive `#` | 0--127 | Degree of distortion |
| Vowel1 | a, e, i, o, u | First vowel |
| Vowel2 | a, e, i, o, u | Second vowel |
| Rate `#` | 0.05--10.00 Hz, note | Vowel switching frequency |
| Depth `#` | 0--127 | Effect depth |
| Manual `#` | 0--100 | Vowel 1/2 balance point |
| Input Sync Sw | OFF, ON | LFO reset on input |
| Input Sync Threshold | 0--127 | Volume level for LFO reset |
| Pan `#` | L64--63R | Stereo location |
| Level | 0--127 | Output level |

### 08: Speaker Simulator

Simulates speaker and mic placement.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Type | SMALL 1/2, MIDDLE, JC-120, BUILT-IN 1--5, BG STACK 1/2, MS STACK 1/2, METAL STACK, 2-STACK, 3-STACK | Speaker type |
| Mic Setting | 1, 2, 3 | Mic distance (1=close, 3=far) |
| Mic Level `#` | 0--127 | Microphone volume |
| Direct Level `#` | 0--127 | Direct sound volume |
| Level `#` | 0--127 | Output level |

### 09: Phaser 1

Phase-shifted sound added to original.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Mode | 4-STAGE, 8-STAGE, 12-STAGE | Number of phaser stages |
| Manual `#` | 0--127 | Base modulation frequency |
| Rate `#` | 0.05--10.00 Hz, note | Modulation frequency |
| Depth | 0--127 | Modulation depth |
| Polarity | INVERSE, SYNCHRO | L/R phase relationship |
| Resonance `#` | 0--127 | Feedback amount |
| Cross Feedback | -98--+98 % | Phaser feedback proportion |
| Mix `#` | 0--127 | Phase-shifted sound level |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level | 0--127 | Output level |

### 10: Phaser 2

Analog phaser simulation (particularly suited for electric piano).

| Parameter | Value | Description |
|-----------|-------|-------------|
| Rate `#` | 0--100 | Modulation frequency |
| Color | 1, 2 | Modulation character |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level | 0--127 | Output level |

### 11: Phaser 3

Different analog phaser (particularly suited for electric piano).

| Parameter | Value | Description |
|-----------|-------|-------------|
| Speed `#` | 0--100 | Modulation frequency |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level | 0--127 | Output level |

### 12: Step Phaser

Phaser effect that varies gradually in steps.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Mode | 4-STAGE, 8-STAGE, 12-STAGE | Number of stages |
| Manual `#` | 0--127 | Base modulation frequency |
| Rate `#` | 0.05--10.00 Hz, note | Modulation frequency |
| Depth | 0--127 | Modulation depth |
| Polarity | INVERSE, SYNCHRO | L/R phase relationship |
| Resonance `#` | 0--127 | Feedback amount |
| Cross Feedback | -98--+98 % | Feedback proportion |
| Step Rate `#` | 0.10--20.00 Hz, note | Rate of step-wise phaser change |
| Mix `#` | 0--127 | Phase-shifted sound level |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level | 0--127 | Output level |

### 13: Multi Stage Phaser

Extremely deep phaser with up to 24 stages.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Mode | 4-STAGE, 8-STAGE, 12-STAGE, 16-STAGE, 20-STAGE, 24-STAGE | Number of stages |
| Manual `#` | 0--127 | Base modulation frequency |
| Rate `#` | 0.05--10.00 Hz, note | Modulation frequency |
| Depth | 0--127 | Modulation depth |
| Resonance `#` | 0--127 | Feedback amount |
| Mix `#` | 0--127 | Phase-shifted sound level |
| Pan `#` | L64--63R | Stereo location |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level | 0--127 | Output level |

### 14: Infinite Phaser

Continuously raises/lowers the modulation frequency.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Mode | 1, 2, 3, 4 | Higher = deeper phaser effect |
| Speed `#` | -100--+100 | Speed and direction (+: up, -: down) |
| Resonance `#` | 0--127 | Feedback amount |
| Mix `#` | 0--127 | Phase-shifted sound volume |
| Pan `#` | L64--63R | Panning |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level | 0--127 | Output volume |

### 15: Ring Modulator

Applies amplitude modulation for bell-like sounds.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Frequency `#` | 0--127 | Modulation frequency |
| Sens `#` | 0--127 | Amount of frequency modulation |
| Polarity | UP, DOWN | Direction of frequency modulation |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Balance `#` | D100:0W--D0:100W | Dry/wet balance |
| Level | 0--127 | Output level |

### 16: Tremolo

Cyclic volume modulation.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Mod Wave | TRI, SQR, SIN, SAW1, SAW2 | Modulation waveform |
| Rate `#` | 0.05--10.00 Hz, note | Modulation frequency |
| Depth `#` | 0--127 | Modulation depth |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level | 0--127 | Output level |

### 17: Auto Pan

Cyclic stereo location modulation.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Mod Wave | TRI, SQR, SIN, SAW1, SAW2 | Modulation waveform |
| Rate `#` | 0.05--10.00 Hz, note | Modulation frequency |
| Depth `#` | 0--127 | Depth of panning effect |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level | 0--127 | Output level |

### 18: Slicer

Successive cuts creating rhythmic backing (16-step sequencer). MFX Control
can reset the step sequence.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Step 01--16 | L64--63R | Level at each step |
| Rate `#` | 0.05--10.00 Hz, note | Cycle rate of 16-step sequence |
| Attack `#` | 0--127 | Speed of level change between steps |
| Input Sync Sw | OFF, ON | Reset sequence on input note |
| Input Sync Threshold | 0--127 | Detection threshold |
| Mode | LEGATO, SLASH | LEGATO: smooth transitions; SLASH: brief silence between steps |
| Shuffle `#` | 0--127 | Timing shift of even-numbered steps |
| Level | 0--127 | Output level |

### 19: Rotary 1

Classic rotary speaker simulation with independent high/low frequency rotors.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Speed `#` | SLOW, FAST | Rotation speed |
| Brake `#` | OFF, ON | Gradually stop/resume rotation |
| Woofer Slow Speed | 0.05--10.00 Hz | Slow speed of low frequency rotor |
| Woofer Fast Speed | 0.05--10.00 Hz | Fast speed of low frequency rotor |
| Woofer Acceleration | 0--15 | Speed transition time (lower=slower) |
| Woofer Level | 0--127 | Low frequency rotor volume |
| Tweeter Slow Speed | 0.05--10.00 Hz | Slow speed of high frequency rotor |
| Tweeter Fast Speed | 0.05--10.00 Hz | Fast speed of high frequency rotor |
| Tweeter Acceleration | 0--15 | Speed transition time |
| Tweeter Level | 0--127 | High frequency rotor volume |
| Separation | 0--127 | Stereo spread |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level `#` | 0--127 | Output level |

### 20: Rotary 2

Modified rotary speaker with boosted low end (VK-7 style).

| Parameter | Value | Description |
|-----------|-------|-------------|
| Speed `#` | SLOW, FAST | Rotation speed |
| Woofer Slow Speed | 0.05--10.00 Hz | Slow rotation speed of woofer |
| Woofer Fast Speed | 0.05--10.00 Hz | Fast rotation speed of woofer |
| Woofer Trans Up | 0--127 | Speed-up transition rate (Slow->Fast) |
| Woofer Trans Down | 0--127 | Slow-down transition rate (Fast->Slow) |
| Woofer Level | 0--127 | Woofer volume |
| Tweeter Slow Speed | 0.05--10.00 Hz | Tweeter slow rotation speed |
| Tweeter Fast Speed | 0.05--10.00 Hz | Tweeter fast rotation speed |
| Tweeter Trans Up | 0--127 | Tweeter speed-up rate |
| Tweeter Trans Down | 0--127 | Tweeter slow-down rate |
| Tweeter Level | 0--127 | Tweeter volume |
| Spread | 0--10 | Spatial dispersion of sound |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level `#` | 0--127 | Output level |

### 21: Rotary 3

Rotary speaker with overdrive for intense organ sound.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Speed `#` | SLOW, FAST | Rotation speed |
| Brake `#` | OFF, ON | Gradually stop/resume rotation |
| OD Switch | OFF, ON | Overdrive on/off |
| OD Gain `#` | 0--127 | Overdrive input level |
| OD Drive `#` | 0--127 | Degree of distortion |
| OD Level | 0--127 | Overdrive volume |
| Woofer Slow Speed | 0.05--10.00 Hz | Woofer slow speed |
| Woofer Fast Speed | 0.05--10.00 Hz | Woofer fast speed |
| Woofer Trans Up | 0--127 | Speed-up transition rate |
| Woofer Trans Down | 0--127 | Slow-down transition rate |
| Woofer Level | 0--127 | Woofer volume |
| Tweeter Slow Speed | 0.05--10.00 Hz | Tweeter slow speed |
| Tweeter Fast Speed | 0.05--10.00 Hz | Tweeter fast speed |
| Tweeter Trans Up | 0--127 | Tweeter speed-up rate |
| Tweeter Trans Down | 0--127 | Tweeter slow-down rate |
| Tweeter Level | 0--127 | Tweeter volume |
| Spread | 0--10 | Spatial dispersion |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level `#` | 0--127 | Output level |

### 22: Chorus

Stereo chorus with filter.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Filter Type | OFF, LPF, HPF | Filter type applied to chorus |
| Cutoff Freq | 200--8000 Hz | Filter cutoff frequency |
| Pre Delay | 0.0--100.0 ms | Delay before chorus is heard |
| Rate `#` | 0.05--10.00 Hz, note | Modulation frequency |
| Depth | 0--127 | Modulation depth |
| Phase | 0--180 deg | Spatial spread |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Balance `#` | D100:0W--D0:100W | Dry/wet balance |
| Level | 0--127 | Output level |

### 23: Flanger

Stereo flanger producing metallic resonance.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Filter Type | OFF, LPF, HPF | Filter type |
| Cutoff Freq | 200--8000 Hz | Filter cutoff |
| Pre Delay | 0.0--100.0 ms | Delay before flanger |
| Rate `#` | 0.05--10.00 Hz, note | Modulation frequency |
| Depth | 0--127 | Modulation depth |
| Phase | 0--180 deg | Spatial spread |
| Feedback `#` | -98--+98 % | Feedback amount (negative inverts phase) |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Balance `#` | D100:0W--D0:100W | Dry/wet balance |
| Level | 0--127 | Output level |

### 24: Step Flanger

Flanger with stepped pitch changes.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Filter Type | OFF, LPF, HPF | Filter type |
| Cutoff Freq | 200--8000 Hz | Filter cutoff |
| Pre Delay | 0.0--100.0 ms | Delay before flanger |
| Rate `#` | 0.05--10.00 Hz, note | Modulation frequency |
| Depth | 0--127 | Modulation depth |
| Phase | 0--180 deg | Spatial spread |
| Feedback `#` | -98--+98 % | Feedback amount |
| Step Rate `#` | 0.10--20.00 Hz, note | Rate of pitch step change |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Balance `#` | D100:0W--D0:100W | Dry/wet balance |
| Level | 0--127 | Output level |

### 25: Hexa-Chorus

Six-phase chorus for rich spatial sound.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Pre Delay | 0.0--100.0 ms | Delay before chorus |
| Rate `#` | 0.05--10.00 Hz, note | Modulation frequency |
| Depth | 0--127 | Modulation depth |
| Pre Delay Deviation | 0--20 | Pre Delay difference between chorus voices |
| Depth Deviation | -20--+20 | Modulation depth difference between voices |
| Pan Deviation | 0--20 | Stereo location difference between voices |
| Balance `#` | D100:0W--D0:100W | Dry/wet balance |
| Level | 0--127 | Output level |

### 26: Tremolo Chorus

Chorus with added tremolo (cyclic volume modulation).

| Parameter | Value | Description |
|-----------|-------|-------------|
| Pre Delay | 0.0--100.0 ms | Delay before chorus |
| Chorus Rate `#` | 0.05--10.00 Hz, note | Chorus modulation frequency |
| Chorus Depth | 0--127 | Chorus modulation depth |
| Tremolo Rate `#` | 0.05--10.00 Hz, note | Tremolo modulation frequency |
| Tremolo Separation | 0--127 | Tremolo spread |
| Tremolo Phase | 0--180 deg | Tremolo spatial spread |
| Balance `#` | D100:0W--D0:100W | Dry/wet balance |
| Level | 0--127 | Output level |

### 27: Space-D

Multiple chorus with transparent effect (no perceived modulation).

| Parameter | Value | Description |
|-----------|-------|-------------|
| Pre Delay | 0.0--100.0 ms | Delay before chorus |
| Rate `#` | 0.05--10.00 Hz, note | Modulation frequency |
| Depth | 0--127 | Modulation depth |
| Phase | 0--180 deg | Spatial spread |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Balance `#` | D100:0W--D0:100W | Dry/wet balance |
| Level | 0--127 | Output level |

### 28: Overdrive

Heavy overdrive distortion with optional amp simulator.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Drive `#` | 0--127 | Degree of distortion (also affects volume) |
| Tone `#` | 0--127 | Sound quality |
| Amp Sw | OFF, ON | Amp simulator on/off |
| Amp Type | SMALL, BUILT-IN, 2-STACK, 3-STACK | Type of guitar amp |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Pan `#` | L64--63R | Stereo location |
| Level | 0--127 | Output level |

### 29: Distortion

Heavy distortion (same parameters as 28: Overdrive).

### 30: Guitar Amp Simulator

Guitar amplifier simulation with preamp, speaker, and mic.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Amp Sw | OFF, ON | Amp on/off |
| Amp Type | JC-120, CLEAN TWIN, MATCH DRIVE, BG LEAD, MS1959I, MS1959II, MS1959I+II, SLDN LEAD, METAL5150, METAL LEAD, OD-1, OD-2 TURBO, DISTORTION, FUZZ | Amp model |
| Amp Volume `#` | 0--127 | Amp volume/distortion |
| Amp Master `#` | 0--127 | Pre-amp master volume |
| Amp Gain | LOW, MIDDLE, HIGH | Pre-amp distortion amount |
| Amp Bass | 0--127 | Bass EQ |
| Amp Middle | 0--127 | Mid EQ (not available for Match Drive) |
| Amp Treble | 0--127 | Treble EQ |
| Amp Presence | 0--127 | Ultra-high frequency tone |
| Amp Bright | OFF, ON | Sharper/brighter sound (JC-120, Clean Twin, BG Lead only) |
| Speaker Sw | OFF, ON | Speaker simulation on/off |
| Speaker Type | SMALL 1/2, MIDDLE, JC-120, BUILT-IN 1--5, BG STACK 1/2, MS STACK 1/2, METAL STACK, 2-STACK, 3-STACK | Speaker type |
| Mic Setting | 1, 2, 3 | Mic distance |
| Mic Level | 0--127 | Mic volume |
| Direct Level | 0--127 | Direct sound volume |
| Pan `#` | L64--63R | Stereo location |
| Level `#` | 0--127 | Output level |

### 31: Compressor

Smooths volume fluctuations.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Attack `#` | 0--127 | Speed at which compression starts |
| Threshold `#` | 0--127 | Volume at which compression begins |
| Post Gain | 0--+18 dB | Output gain |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level `#` | 0--127 | Output level |

### 32: Limiter

Prevents distortion from excessive volume levels.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Release `#` | 0--127 | Time after falling below threshold until compression stops |
| Threshold `#` | 0--127 | Volume at which compression begins |
| Ratio | 1.5:1, 2:1, 4:1, 100:1 | Compression ratio |
| Post Gain | 0--+18 dB | Output gain |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level `#` | 0--127 | Output level |

### 33: Gate

Cuts reverb tail based on volume for artificial decay effects.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Threshold `#` | 0--127 | Volume at which gate begins to close |
| Mode | GATE, DUCK | GATE: closes when volume decreases; DUCK: closes when volume increases |
| Attack | 0--127 | Time for gate to fully open |
| Hold | 0--127 | Time before gate starts closing |
| Release | 0--127 | Time for gate to fully close |
| Balance `#` | D100:0W--D0:100W | Dry/wet balance |
| Level | 0--127 | Output level |

### 34: Delay

Stereo delay with normal or cross feedback modes.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Delay Left | 0--1300 ms, note | Left delay time |
| Delay Right | 0--1300 ms, note | Right delay time |
| Phase Left | NORMAL, INVERSE | Left delay phase |
| Phase Right | NORMAL, INVERSE | Right delay phase |
| Feedback Mode | NORMAL, CROSS | Feedback routing |
| Feedback `#` | -98--+98 % | Feedback amount |
| HF Damp | 200--8000 Hz, BYPASS | High-frequency damping of feedback |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Balance `#` | D100:0W--D0:100W | Dry/wet balance |
| Level | 0--127 | Output level |

### 35: Modulation Delay

Delay with modulation on the delayed sound.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Delay Left | 0--1300 ms, note | Left delay time |
| Delay Right | 0--1300 ms, note | Right delay time |
| Feedback Mode | NORMAL, CROSS | Feedback routing |
| Feedback `#` | -98--+98 % | Feedback amount |
| HF Damp | 200--8000 Hz, BYPASS | High-frequency damping |
| Rate `#` | 0.05--10.00 Hz, note | Modulation frequency |
| Depth | 0--127 | Modulation depth |
| Phase | 0--180 deg | Spatial spread |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Balance `#` | D100:0W--D0:100W | Dry/wet balance |
| Level | 0--127 | Output level |

### 36: 3Tap Pan Delay

Three delay taps: center, left, and right.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Delay Left/Right/Center | 0--2600 ms, note | Delay times for each tap |
| Center Feedback `#` | -98--+98 % | Center feedback amount |
| HF Damp | 200--8000 Hz, BYPASS | High-frequency damping |
| Left/Right/Center Level | 0--127 | Volume of each delay tap |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Balance `#` | D100:0W--D0:100W | Dry/wet balance |
| Level | 0--127 | Output level |

### 37: 4Tap Pan Delay

Four delay taps with individual pan and level.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Delay 1--4 Time | 0--2600 ms, note | Delay time for each tap |
| Delay 1 Feedback `#` | -98--+98 % | Feedback amount for tap 1 |
| HF Damp | 200--8000 Hz, BYPASS | High-frequency damping |
| Delay 1--4 Level | 0--127 | Volume of each delay tap |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Balance `#` | D100:0W--D0:100W | Dry/wet balance |
| Level | 0--127 | Output level |

### 38: Multi Tap Delay

Four delays with individual timing, pan, and level.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Delay 1--4 Time | 0--2600 ms, note | Delay time for each tap |
| Delay 1 Feedback `#` | -98--+98 % | Feedback for tap 1 |
| HF Damp | 200--8000 Hz, BYPASS | High-frequency damping |
| Delay 1--4 Pan | L64--63R | Pan for each tap |
| Delay 1--4 Level | 0--127 | Level for each tap |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Balance `#` | D100:0W--D0:100W | Dry/wet balance |
| Level | 0--127 | Output level |

### 39: Reverse Delay

Reversed delay sound plus tap delay.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Threshold | 0--127 | Volume at which reverse delay triggers |
| Rev Delay Time | 0--1300 ms, note | Reverse delay time |
| Rev Delay Feedback `#` | -98--+98 % | Reverse delay feedback |
| Rev Delay HF Damp | 200--8000 Hz, BYPASS | Reverse delay HF damping |
| Rev Delay Pan | L64--63R | Reverse delay panning |
| Rev Delay Level | 0--127 | Reverse delay volume |
| Delay 1--3 Time | 0--1300 ms, note | Tap delay times |
| Delay 3 Feedback `#` | -98--+98 % | Tap delay feedback |
| Delay HF Damp | 200--8000 Hz, BYPASS | Tap delay HF damping |
| Delay 1 Pan, Delay 2 Pan | L64--63R | Tap delay panning |
| Delay 1 Level, Delay 2 Level | 0--127 | Tap delay volumes |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Balance `#` | D100:0W--D0:100W | Dry/wet balance |
| Level | 0--127 | Output level |

### 40: Time Ctrl Delay

Delay with smooth time changes (pitch changes with delay time).

| Parameter | Value | Description |
|-----------|-------|-------------|
| Delay Time `#` | 0--1300 ms, note | Delay time |
| Acceleration | 0--15 | Speed of delay time change (affects pitch) |
| Feedback `#` | -98--+98 % | Feedback amount |
| HF Damp | 200--8000 Hz, BYPASS | High-frequency damping |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Balance `#` | D100:0W--D0:100W | Dry/wet balance |
| Level | 0--127 | Output level |

### 41: LOFI Compress

Intentional sound quality degradation.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Pre Filt Type | 1--6 | Type of pre-filter (1: compressor off) |
| LoFi Type | 1--9 | Degree of degradation (higher=worse quality) |
| PostFilt Type | OFF, LPF, HPF | Post-filter type |
| PostFilt Cof | 200--8000 Hz | Post-filter cutoff frequency |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Balance `#` | D100:0W--D0:100W | Dry/wet balance |
| Level `#` | 0--127 | Output level |

### 42: Bit Crasher

Lo-fi via sample rate and bit depth reduction.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Sample Rate `#` | 0--127 | Adjusts sample rate |
| Bit Down `#` | 0--20 | Adjusts bit depth |
| Filter `#` | 0--127 | Filter depth |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level | 0--127 | Output level |

### 43: Pitch Shifter

Stereo pitch shifter.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Coarse `#1` | -24--+12 semi | Pitch shift in semitones |
| Fine `#1` | -100--+100 cent | Pitch shift in 2-cent steps |
| Delay Time | 0--1300 ms, note | Delay before pitch-shifted sound |
| Feedback `#` | -98--+98 % | Feedback amount |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Balance `#` | D100:0W--D0:100W | Dry/wet balance |
| Level | 0--127 | Output level |

### 44: 2Voice Pitch Shifter

Two independent pitch shifters.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Pitch1 Coarse `#1` | -24--+12 semi | Pitch Shift 1 in semitones |
| Pitch1 Fine `#1` | -100--+100 cent | Pitch Shift 1 in 2-cent steps |
| Pitch1 Delay | 0--1300 ms, note | Delay for Pitch Shift 1 |
| Pitch1 Feedback `#` | -98--+98 % | Feedback for Pitch Shift 1 |
| Pitch1 Pan `#` | L64--63R | Pan for Pitch Shift 1 |
| Pitch1 Level | 0--127 | Volume for Pitch Shift 1 |
| Pitch2 Coarse `#2` | -24--+12 semi | Pitch Shift 2 in semitones |
| Pitch2 Fine `#2` | -100--+100 cent | Pitch Shift 2 in 2-cent steps |
| Pitch2 Delay | 0--1300 ms, note | Delay for Pitch Shift 2 |
| Pitch2 Feedback `#` | -98--+98 % | Feedback for Pitch Shift 2 |
| Pitch2 Pan `#` | L64--63R | Pan for Pitch Shift 2 |
| Pitch2 Level | 0--127 | Volume for Pitch Shift 2 |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Balance `#` | D100:0W--D0:100W | Dry/wet balance |
| Level | 0--127 | Output level |

---

## Combination Effects (45--67)

These effects chain two processors in series. The parameters combine those of
the individual effects. Only key differences from the standalone versions
are noted.

### 45: Overdrive -> Chorus

| Parameter | Value | Description |
|-----------|-------|-------------|
| Drive `#` | 0--127 | Overdrive distortion amount |
| Overdrive Pan `#` | L64--63R | Overdrive output pan |
| Chorus Pre Delay | 0.0--100.0 ms | Chorus pre-delay |
| Chorus Rate `#` | 0.05--10.00 Hz, note | Chorus modulation rate |
| Chorus Depth | 0--127 | Chorus modulation depth |
| Chorus Balance `#` | D100:0W--D0:100W | Chorus dry/wet balance |
| Level | 0--127 | Output level |

### 46: Overdrive -> Flanger

| Parameter | Value | Description |
|-----------|-------|-------------|
| Drive `#` | 0--127 | Overdrive distortion |
| Overdrive Pan `#` | L64--63R | Overdrive pan |
| Flanger Pre Delay | 0.0--100.0 ms | Flanger pre-delay |
| Flanger Rate `#` | 0.05--10.00 Hz, note | Flanger modulation rate |
| Flanger Depth | 0--127 | Flanger depth |
| Flanger Feedback `#` | -98--+98 % | Flanger feedback |
| Flanger Balance `#` | D100:0W--D0:100W | Flanger dry/wet |
| Level | 0--127 | Output level |

### 47: Overdrive -> Delay

| Parameter | Value | Description |
|-----------|-------|-------------|
| Drive `#` | 0--127 | Overdrive distortion |
| Overdrive Pan `#` | L64--63R | Overdrive pan |
| Delay Time | 0--2600 ms, note | Delay time |
| Delay Feedback `#` | -98--+98 % | Delay feedback |
| Delay HF Damp | 200--8000 Hz, BYPASS | HF damping |
| Delay Balance `#` | D100:0W--D0:100W | Delay dry/wet |
| Level | 0--127 | Output level |

### 48: Distortion -> Chorus

Same as 45 (Overdrive -> Chorus) but uses distortion instead of overdrive.

### 49: Distortion -> Flanger

Same as 46 (Overdrive -> Flanger) but uses distortion.

### 50: Distortion -> Delay

Same as 47 (Overdrive -> Delay) but uses distortion.

### 51: OD/DS -> TouchWah

Overdrive/distortion with touch wah and amp simulator.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Drive Switch | OFF, ON | OD/distortion on/off |
| Drive Type | OVERDRIVE, DISTORTION | Type of distortion |
| Drive `#` | 0--127 | Distortion amount |
| Tone `#` | 0--127 | Sound quality |
| Amp Sw | OFF, ON | Amp simulator on/off |
| Amp Type | SMALL, BUILT-IN, 2-STACK, 3-STACK | Amp type |
| Touch Wah Switch | OFF, ON | Wah on/off |
| Touch Wah Filter Type | LPF, BPF | LPF: wide wah; BPF: narrow wah |
| Touch Wah Polarity | DOWN, UP | Filter direction |
| Touch Wah Sens `#` | 0--127 | Filter sensitivity |
| Touch Wah Manual `#` | 0--127 | Center wah frequency |
| Touch Wah Peak `#` | 0--127 | Wah frequency width |
| Touch Wah Balance `#` | D100:0W--D0:100W | Wah dry/wet |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level | 0--127 | Output level |

### 52: OD/DS -> AutoWah

Same drive/amp section as 51, with auto wah instead of touch wah.

| Parameter | Value | Description |
|-----------|-------|-------------|
| (Drive section) | (same as 51) | See 51: OD/DS -> TouchWah |
| Auto Wah Switch | OFF, ON | Auto wah on/off |
| Auto Wah Filter Type | LPF, BPF | Wah filter type |
| Auto Wah Manual `#` | 0--127 | Center wah frequency |
| Auto Wah Peak `#` | 0--127 | Wah frequency width |
| Auto Wah Rate `#` | 0.05--10.00 Hz, note | Wah modulation rate |
| Auto Wah Depth `#` | 0--127 | Wah modulation depth |
| Auto Wah Balance `#` | D100:0W--D0:100W | Wah dry/wet |
| Low Gain | -15--+15 dB | Low frequency gain |
| High Gain | -15--+15 dB | High frequency gain |
| Level | 0--127 | Output level |

### 53--56: GuitarAmpSim -> Chorus/Flanger/Phaser/Delay

Guitar amp simulator (same preamp params as type 30) followed by the
named effect. Each includes:

| Parameter | Value | Description |
|-----------|-------|-------------|
| Amp Sw | OFF, ON | Amp on/off |
| Amp Type | (same as type 30) | Amp model |
| Amp Volume `#` | 0--127 | Amp volume/distortion |
| Amp Master `#` | 0--127 | Pre-amp master |
| Amp Gain | LOW, MIDDLE, HIGH | Pre-amp distortion |
| Amp Bass/Middle/Treble | 0--127 | Tone controls |
| Speaker Sw | OFF, ON | Speaker sim on/off |
| Speaker Type | (same as type 30) | Speaker model |
| (Effect Switch) `#` | OFF, ON | Enable the chained effect |
| (Effect params) | (varies) | Standard params for Chorus/Flanger/Phaser/Delay |
| Level | 0--127 | Output level |

### 57--61: EP AmpSim -> Tremolo/Chorus/Flanger/Phaser/Delay

Electric piano amp simulator followed by the named effect.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Type | OLDCASE, NEWCASE, WURLY | EP amp type (70s, 80s, 60s style) |
| Bass `#` | -50--+50 | Low-frequency boost/cut |
| Treble `#` | -50--+50 | High-frequency boost/cut |
| OD Switch | OFF, ON | Overdrive on/off |
| OD Gain | 0--127 | Overdrive input level |
| OD Drive | 0--127 | Distortion degree |
| (Effect Switch) `#` | OFF, ON | Enable the chained effect |
| (Effect params) | (varies) | Standard params for the effect |
| Speaker Type | LINE, OLD, NEW, WURLY, TWIN | Speaker type (LINE=no speaker sim) |
| Level | 0--127 | Output level |

### 62--64: Enhancer -> Chorus/Flanger/Delay

| Parameter | Value | Description |
|-----------|-------|-------------|
| Enhancer Sens `#` | 0--127 | Enhancer sensitivity |
| Enhancer Mix `#` | 0--127 | Level of generated overtones |
| (Effect params) | (varies) | Standard Chorus/Flanger/Delay params |
| Level | 0--127 | Output level |

### 65: Chorus -> Delay

| Parameter | Value | Description |
|-----------|-------|-------------|
| Chorus Pre Delay | 0.0--100.0 ms | Chorus pre-delay |
| Chorus Rate `#` | 0.05--10.00 Hz, note | Chorus rate |
| Chorus Depth | 0--127 | Chorus depth |
| Chorus Balance `#` | D100:0W--D0:100W | Chorus dry/wet |
| Delay Time | 0--2600 ms, note | Delay time |
| Delay Feedback `#` | -98--+98 % | Delay feedback |
| Delay HF Damp | 200--8000 Hz, BYPASS | Delay HF damping |
| Delay Balance `#` | D100:0W--D0:100W | Delay dry/wet |
| Level | 0--127 | Output level |

### 66: Flanger -> Delay

| Parameter | Value | Description |
|-----------|-------|-------------|
| Flanger Pre Delay | 0.0--100.0 ms | Flanger pre-delay |
| Flanger Rate `#` | 0.05--10.00 Hz, note | Flanger rate |
| Flanger Depth | 0--127 | Flanger depth |
| Flanger Feedback `#` | -98--+98 % | Flanger feedback |
| Flanger Balance `#` | D100:0W--D0:100W | Flanger dry/wet |
| Delay Time | 0--2600 ms, note | Delay time |
| Delay Feedback `#` | -98--+98 % | Delay feedback |
| Delay HF Damp | 200--8000 Hz, BYPASS | Delay HF damping |
| Delay Balance `#` | D100:0W--D0:100W | Delay dry/wet |
| Level | 0--127 | Output level |

### 67: Chorus -> Flanger

| Parameter | Value | Description |
|-----------|-------|-------------|
| Chorus Pre Delay | 0.0--100.0 ms | Chorus pre-delay |
| Chorus Rate `#` | 0.05--10.00 Hz, note | Chorus modulation rate |
| Chorus Depth | 0--127 | Chorus depth |
| Chorus Balance `#` | D100:0W--D0:100W | Chorus dry/wet |
| Flanger Pre Delay | 0.0--100.0 ms | Flanger pre-delay |
| Flanger Rate `#` | 0.05--10.00 Hz, note | Flanger rate |
| Flanger Depth | 0--127 | Flanger depth |
| Flanger Feedback `#` | -98--+98 % | Flanger feedback |
| Flanger Balance `#` | D100:0W--D0:100W | Flanger dry/wet |
| Level | 0--127 | Output level |

---

## Note Values for Tempo-Synced Parameters

When a parameter accepts "note" as a value, these note durations are
available (relative to the current tempo):

| Symbol | Duration |
|--------|----------|
| 64th note | Sixty-fourth note |
| 64th triplet | Sixty-fourth-note triplet |
| 32nd note | Thirty-second note |
| 32nd triplet | Thirty-second-note triplet |
| dotted 32nd | Dotted thirty-second note |
| 16th note | Sixteenth note |
| 16th triplet | Sixteenth-note triplet |
| dotted 16th | Dotted sixteenth note |
| 8th note | Eighth note |
| 8th triplet | Eighth-note triplet |
| dotted 8th | Dotted eighth note |
| quarter | Quarter note |
| quarter triplet | Quarter-note triplet |
| dotted quarter | Dotted quarter note |
| half | Half note |
| half triplet | Half-note triplet |
| dotted half | Dotted half note |
| whole | Whole note |
| whole triplet | Whole-note triplet |
| dotted whole | Dotted whole note |
| double | Double note |
| double triplet | Double-note triplet |

---

## MFX Control (Step Reset)

For types **04: Step Filter** and **18: Slicer**, the MFX Control Destination
can be set to "Step Reset" to restart the 16-step sequence from step 1
using any assigned MIDI controller.
