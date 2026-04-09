# Studio Set Parameters

Extracted from the INTEGRA-7 Parameter Guide (pages 4-14). Parameters are
organized by hardware UI screen and tab structure.

---

## 1. Studio Set Common

Access: Top Screen > [MENU] > STUDIO SET COMMON > [ENTER]

### Top Screen

| Parameter | Value | Description |
|-----------|-------|-------------|
| Studio Set Number | 1--64 | Selects the studio set number. The studio set switches when you change the number and press [ENTER]. Shown only if system setting "Top Screen" is TYPE 2. |
| Tone Type / Tone Bank | (see TONE tab) | See PART VIEW TONE tab for details. |
| Tone Number | (see TONE tab) | See PART VIEW TONE tab for details. |
| MUTE | OFF, ON | Mutes (ON) or un-mutes (OFF) each part. The bar above the part number disappears for muted parts. Mute sets volume to minimum; MIDI messages are still received. |
| SOLO | OFF, 1--16 | Only the sound of the specified part will be heard. Ext part cannot be set to Solo. |

### GENERAL tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Studio Set Tempo | 20--250 | Tempo for the studio set. Used when system "Tempo Assign Source" is set to STUDIO SET. If set to SYSTEM, the system tempo is used instead. |
| Drum Comp+EQ Assign | Part1--Part16 | Specifies the part that will use the six sets of compressor + equalizer provided for drum kits. If a non-drum-kit tone is assigned to the specified part, Comp+EQ will not be available. |

### CONTROL tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Tone Control 1 Src | OFF, CC01--CC31, CC33--CC95, BEND, AFT | Specifies the MIDI message used for Tone Control 1 of the studio set. |
| Tone Control 2 Src | OFF, CC01--CC31, CC33--CC95, BEND, AFT | Specifies the MIDI message used for Tone Control 2 of the studio set. |
| Tone Control 3 Src | OFF, CC01--CC31, CC33--CC95, BEND, AFT | Specifies the MIDI message used for Tone Control 3 of the studio set. |
| Tone Control 4 Src | OFF, CC01--CC31, CC33--CC95, BEND, AFT | Specifies the MIDI message used for Tone Control 4 of the studio set. |

> **Note:** To use Tone Control 1--4 Src settings of each studio set, set
> system "Control Source Select" to STUDIO SET. To use the system-level
> System Control 1--4 Src instead, set it to SYSTEM.

### PHASE LOCK tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| CH 1--CH 16 | OFF, ON | When ON, parts on the same MIDI channel have their timing matched so they play simultaneously. A small latency is introduced between note reception and sound output. Enable only as needed. |

> **Note:** Phase Lock is not available for SuperNATURAL acoustic organ-type
> instruments.

---

## 2. Part View

Access: Top Screen > [PART VIEW]

The PART VIEW -ALL- screen shows a subset of parameters. Press [PART VIEW]
again for the full PART VIEW screen. Parameters marked "not shown in PART
VIEW -ALL-" are only visible in the detailed view.

### TONE tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Tone Type | SN-A, SN-S, SN-D, PCMS, PCMD | Specifies the type of tone/drum kit assigned to each part. SN-A = SuperNATURAL Acoustic, SN-S = SuperNATURAL Synth, SN-D = SuperNATURAL Drum Kit, PCMS = PCM Synth, PCMD = PCM Drum Kit. |
| Tone Bank | PRST, USER, GM2 (GM2#), ExSN1--ExSN6, SRX01--SRX12, ExPCM | Selects the group of the tone/drum kit. ExSN1--5 selectable for SN-A, ExSN6 for SN-D, SRX01--12/ExPCM for PCMS/PCMD (if expansions are loaded). |
| Tone Number | 001-- | Selects the number of the tone/drum kit assigned to each part. |

### LEVEL/CH tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Level | 0--127 | Adjusts the volume of each part. Main purpose is balancing volume between parts. Also valid for Ext part. |
| Pan | L64--63R | Adjusts the pan of each part. L64 = far left, 0 = center, 63R = far right. Ignored if motional surround is on. |
| Cho Send Level | 0--127 | Adjusts the chorus send amount for each part. Set to 0 to disable chorus. No effect if motional surround is on. Also valid for Ext part. |
| Rev Send Level | 0--127 | Adjusts the reverb send amount for each part. Set to 0 to disable reverb. No effect if motional surround is on. Also valid for Ext part. |
| Output Assign | A, B, C, D, 1--8 | Specifies how the part's sound is output. A/B/C/D = stereo to OUTPUT jacks. 1--8 = monaural to INDIVIDUAL jacks. |
| Rx Switch | OFF, ON | Whether MIDI messages will be received. Normally leave ON; set OFF to silence a part during playback. |
| Rx Channel | 1--16 | Specifies the MIDI receive channel for each part. |
| Mono/Poly | MONO, POLY, TONE | MONO = monophonic, POLY = polyphonic, TONE = use the tone's own setting. Not shown in PART VIEW -ALL-. |
| Legato Switch | OFF, ON, TONE | Enables legato for monophonic performance (smooth note transitions like hammering-on/pulling-off). TONE = use the tone's setting. Not shown in PART VIEW -ALL-. |
| Voice Reserve | 0--63, FULL | Specifies the number of voices reserved for each part when more than 128 voices are played simultaneously. All parts cannot total more than 64. Not shown in PART VIEW -ALL-. |

**Mono/Poly and Legato availability by tone type:**

| Tone Type | Mono/Poly | Legato |
|-----------|-----------|--------|
| SN-A (Ac.Piano) | Yes | Yes |
| SN-A (Organ) | No | No |
| SN-A (Other) | Yes | No |
| SN-S | Yes | Yes |
| SN-D | No | No |
| PCMS | Yes | Yes |
| PCMD | No | No |

### EQ tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| EQ Switch | OFF, ON | Per-part EQ on/off. |
| EQ Low Freq | 200, 400 Hz | Frequency of the low range. Not shown in PART VIEW -ALL-. |
| EQ Low Gain | -15--+15 dB | Gain of the low frequency range. |
| EQ Mid Freq | 200, 250, 315, 400, 500, 630, 800, 1000, 1250, 1600, 2000, 2500, 3150, 4000, 5000, 6300, 8000 Hz | Frequency of the middle range. Not shown in PART VIEW -ALL-. |
| EQ Mid Gain | -15--+15 dB | Gain of the middle frequency range. |
| EQ Mid Q | 0.5, 1.0, 2.0, 4.0, 8.0 | Width of the middle frequency range. Higher Q = narrower range. Not shown in PART VIEW -ALL-. |
| EQ High Freq | 2000, 4000, 8000 Hz | Frequency of the high range. Not shown in PART VIEW -ALL-. |
| EQ High Gain | -15--+15 dB | Gain of the high frequency range. |

### KBD tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Key Range Lower | C-1--UPPER | Specifies the lowest note that the tone will sound for each part. |
| Key Range Upper | LOWER--G9 | Specifies the highest note that the tone will sound for each part. Lower cannot exceed Upper and vice versa. |
| Key Fade Lower | 0--127 | Determines level behavior for notes below Key Range Lower. Higher = more gradual fade. Set to 0 for hard cutoff. |
| Key Fade Upper | 0--127 | Determines level behavior for notes above Key Range Upper. Higher = more gradual fade. Set to 0 for hard cutoff. |
| Velo Range Lower | 1--UPPER | Specifies the lowest velocity at which the part will sound. Not shown in PART VIEW -ALL-. |
| Velo Range Upper | LOWER--127 | Specifies the highest velocity at which the part will sound. Not shown in PART VIEW -ALL-. |
| Velo Fade Lower | 0--127 | Level behavior for velocities below Velo Range Lower. Set to 0 for hard cutoff. Not shown in PART VIEW -ALL-. |
| Velo Fade Upper | 0--127 | Level behavior for velocities above Velo Range Upper. Set to 0 for hard cutoff. Not shown in PART VIEW -ALL-. |

**Velocity Range/Fade availability by tone type:**

| Tone Type | Available |
|-----------|-----------|
| SN-A (Ac.Piano) | Yes |
| SN-A (Organ) | No |
| SN-A (Other) | Yes |
| SN-S | Yes |
| SN-D | Yes |
| PCMS | Yes |
| PCMD | Yes |

| Parameter | Value | Description |
|-----------|-------|-------------|
| Velo Sens Offset | -63--+63 | Adjusts the velocity sensitivity. Higher = greater sensitivity. Not shown in PART VIEW -ALL-. |

### PITCH tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Octave Shift | -3--+3 | Adjusts pitch in octave units. |
| Coarse Tune | -48--+48 | Adjusts pitch in semitone steps (+/- 4 octaves). |
| Fine Tune | -50--+50 | Adjusts pitch in 1-cent steps (+/- 50 cents). One cent = 1/100th of a semitone. |
| Bend Range | 0--24, TONE | Amount of pitch change in semitones when Pitch Bend Lever is moved (same for both directions). TONE = use tone's setting. |
| Porta Switch | OFF, ON, TONE | Whether portamento will be applied. TONE = use tone's setting. Not shown in PART VIEW -ALL-. |
| Porta Time | 0--127, TONE | Time over which the pitch changes during portamento. Higher = slower pitch change. TONE = use tone's setting. Not shown in PART VIEW -ALL-. |

**Pitch parameter availability by tone type:**

| Tone Type | Coarse Tune | Fine Tune | Bend Range | Porta Switch | Porta Time |
|-----------|-------------|-----------|------------|--------------|------------|
| SN-A (Ac.Piano) | Yes | Yes | Yes | Yes | Yes |
| SN-A (Organ) | Yes | Yes | Yes | No | No |
| SN-A (Other) | Yes | Yes | Yes | Yes | Yes |
| SN-S | Yes | Yes | Yes | Yes | Yes |
| SN-D | No | No | No | No | No |
| PCMS | Yes | Yes | Yes | Yes | Yes |
| PCMD | No | No | Yes | No | No |

### OFFSET tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Cutoff Offset | -64--+63 | Adjusts the cutoff frequency for the tone/drum kit assigned to a part. |
| Reso Offset | -64--+63 | Adjusts the Resonance for the tone/drum kit assigned to a part. |
| Attack Offset | -64--+63 | Adjusts the Attack Time for the tone/drum kit assigned to a part. |
| Decay Offset | -64--+63 | Adjusts the Decay Time for the tone/drum kit assigned to a part. |
| Release Offset | -64--+63 | Adjusts the Release Time for the tone/drum kit assigned to a part. |
| Vibrato Rate | -64--+63 | Adjusts vibrato speed (pitch modulation rate). Higher = faster modulation. Not shown in PART VIEW -ALL-. |
| Vibrato Depth | -64--+63 | Adjusts vibrato depth (pitch modulation depth). Higher = deeper modulation. Not shown in PART VIEW -ALL-. |
| Vibrato Delay | -64--+63 | Adjusts the time delay before vibrato begins. Higher = longer delay. Not shown in PART VIEW -ALL-. |

**Offset parameter availability by tone type:**

| Tone Type | Cutoff | Reso | Attack | Decay | Release | Vibrato Rate/Depth/Delay |
|-----------|--------|------|--------|-------|---------|--------------------------|
| SN-A (Ac.Piano) | No | No | No | No | No | Yes |
| SN-A (Organ) | No | No | No | No | No | No |
| SN-A (Other) | Yes* | Yes* | Yes | No | Yes | Yes* |
| SN-S | Yes | Yes | Yes | Yes | Yes | Yes |
| SN-D | No | No | Yes | Yes | Yes | No |
| PCMS | Yes | Yes | Yes | Yes | Yes | Yes |
| PCMD | Yes | Yes | Yes | Yes | Yes | Yes |

> \* For some SN-A (Other) tones, the effect may be difficult to notice.
> Vibrato Rate/Depth/Delay does not apply to instruments of the Bell/Mallet
> and Percussion categories.

### SCALE tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Scale Tune Type | CUSTOM, EQUAL, JUST-MAJ, JUST-MIN, PYTHAGORE, KIRNBERGE, MEANTONE, WERCKMEIS, ARABIC | Templates that set all Scale Tune C--B settings. CUSTOM = specify tuning individually. |
| Scale Tune Key | C, C#, D, D#, E, F, F#, G, G#, A, A#, B | Specifies the tonic note for the scale tune template. |
| Scale Tune for C--B | -64--+63 | Specifies the scale tuning for each note. Not shown in PART VIEW -ALL-. |

**Scale Tune Type descriptions:**

| Value | Description |
|-------|-------------|
| CUSTOM | Specify tuning individually for Scale Tune C--B |
| EQUAL | Equal temperament |
| JUST-MAJ | Just intonation (major) |
| JUST-MIN | Just intonation (minor) |
| PYTHAGORE | Pythagorean tuning |
| KIRNBERGE | Kirnberger (type 3) |
| MEANTONE | Meantone temperament |
| WERCKMEIS | Werckmeister (type 1, number 3) |
| ARABIC | Arabic scale |

**Scale availability by tone type:**

| Tone Type | Available |
|-----------|-----------|
| SN-A (Ac.Piano) | Yes |
| SN-A (Organ) | No |
| SN-A (Other) | Yes |
| SN-S | Yes |
| SN-D | No |
| PCMS | Yes |
| PCMD | Yes |

### MIDI tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Rx Program Change (PC) | OFF, ON | Whether MIDI Program Change messages will be received. |
| Rx Bank Select (BS) | OFF, ON | Whether MIDI Bank Select messages will be received. |
| Rx Pitch Bend (PB) | OFF, ON | Whether MIDI Pitch Bend messages will be received. |
| Rx Poly Key Press (PA) | OFF, ON | Whether MIDI polyphonic key pressure messages will be received. |
| Rx Ch Press (CA) | OFF, ON | Whether MIDI Channel Pressure messages will be received. |
| Rx Modulation (MD) | OFF, ON | Whether MIDI Modulation messages will be received. |
| Rx Volume (VO) | OFF, ON | Whether MIDI Volume messages will be received. |
| Rx Pan (PN) | OFF, ON | Whether MIDI Pan messages will be received. |
| Rx Expression (EX) | OFF, ON | Whether MIDI Expression messages will be received. |
| Rx Hold-1 (HD) | OFF, ON | Whether MIDI Hold 1 messages will be received. |
| Velo Crv Type (VC) | OFF, 1--4 | Selects one of four Velocity Curve types to match the connected MIDI keyboard's touch. Set to OFF to use the keyboard's own velocity curve. |

**Velocity Curve availability by tone type:**

| Tone Type | Available |
|-----------|-----------|
| SN-A (Ac.Piano) | Yes |
| SN-A (Organ) | No |
| SN-A (Other) | Yes |
| SN-S | Yes |
| SN-D | Yes |
| PCMS | Yes |
| PCMD | Yes |

---

## 3. Motional Surround

Access: [MOTIONAL SURROUND] > [ENTER]

### COMMON tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Room Type | ROOM1, ROOM2, HALL1, HALL2 | Specifies the room type. |
| Room Size | SMALL, MEDIUM, LARGE | Specifies the room size. |
| Motional Surround Depth | 0--100 | Specifies the depth of the Motional Surround effect. |
| Ambience Level | 0--127 | Specifies the volume of ambience. |
| Ambience Time | 0--100 | Specifies the duration of ambience. |
| Ambience Density | 0--100 | Specifies the density of ambience. |
| Ambience HF Damp | 0--100 | Specifies the frequency at which the high range of the ambience will be cut. |

### PART tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Part L-R | -64--+63 | Specifies the left/right position. CC: Parts 1--16 = CC12, Ext Part = CC28. |
| Part F-B | -64--+63 | Specifies the front/rear (back) position. CC: Parts 1--16 = CC13, Ext Part = CC29. |
| Part Width | 0--32 | Specifies the width of the positioned sound. |
| Part Ambience Send Level | 0--127 | Specifies the send level to ambience. CC: Parts 1--16 = CC14, Ext Part = CC30. |

### CONTROL tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Ext Part Control Ch | 1--16, OFF | Specifies the MIDI channel used when controlling the position and ambience send level of an Ext part via MIDI. |

---

## 4. Effects Routing

Access: Top Screen > [EFFECTS]

The routing screen layout differs depending on whether motional surround is
ON or OFF.

| Parameter | Value | Description |
|-----------|-------|-------------|
| MFX Switch | OFF, ON | Whether Multi-Effect is used. Can also be set in the tone MFX tab. |
| Cho Send Level | OFF, ON | Adjusts the chorus send amount for each part. Set to 0 to disable. Also settable in LEVEL/CH tab. Ignored if motional surround is on. Also valid for Ext part. |
| Rev Send Level | OFF, ON | Adjusts the reverb send amount for each part. Set to 0 to disable. Also settable in LEVEL/CH tab. Ignored if motional surround is on. Also valid for Ext part. |
| Chorus Switch | OFF, ON | Whether chorus is used. Ignored if motional surround is on. |
| Reverb Switch | OFF, ON | Whether reverb is used. Ignored if motional surround is on. |
| Master EQ Switch | OFF, ON | Switches the Master EQ on/off. |
| Comp+EQ Switch | OFF, ON | Turns the six drum kit compressor + equalizer units on/off together. Shown only if the part specified by Drum Comp+EQ Assign is selected. |

---

## 5. Studio Set Effects

Access: EFFECTS ROUTING screen > move cursor to effect > [ENTER]

MFX and COMP+EQ can be set individually for each tone.

### COMP+EQ OUTPUT tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Comp+EQ 1 Output Assign | PART, A, B, C, D, 1--8 | Output destination for drum kit compressor + EQ unit 1. PART = input to MFX of the part. A/B/C/D = stereo to OUTPUT jacks. 1--8 = monaural to INDIVIDUAL jacks. |
| Comp+EQ 2 Output Assign | PART, A, B, C, D, 1--8 | Output destination for drum kit compressor + EQ unit 2. |
| Comp+EQ 3 Output Assign | PART, A, B, C, D, 1--8 | Output destination for drum kit compressor + EQ unit 3. |
| Comp+EQ 4 Output Assign | PART, A, B, C, D, 1--8 | Output destination for drum kit compressor + EQ unit 4. |
| Comp+EQ 5 Output Assign | PART, A, B, C, D, 1--8 | Output destination for drum kit compressor + EQ unit 5. |
| Comp+EQ 6 Output Assign | PART, A, B, C, D, 1--8 | Output destination for drum kit compressor + EQ unit 6. |

> **Note:** If motional surround is on, the output from each compressor +
> equalizer will always be the MFX of the part, regardless of the COMP+EQ
> Output Assign setting.

### CHORUS tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Chorus Switch | OFF, ON | Switches the chorus on/off. Ignored if motional surround is on. |
| Chorus Type | 00: OFF, 01: Chorus, 02: Delay, 03: GM2 Chorus | Selects the chorus type. Choose 00: OFF to disable. |
| Chorus Parameter | (varies) | Edit parameters for the selected chorus type. See Chorus Parameters reference (p. 98 of Parameter Guide). |
| Chorus Level | 0--127 | Adjusts the volume of the sound that has passed through chorus. |
| Chorus Output Assign | A, B, C, D | Selects the OUTPUT jacks to which chorus sound is routed when Chorus Output Select is MAIN or MAIN+REV. |
| Chorus Output Select | MAIN, REV, MAIN+REV | How chorus output is routed. MAIN = stereo to OUTPUT jacks. REV = monaural to reverb. MAIN+REV = both. |

### REVERB tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Reverb Switch | OFF, ON | Switches the reverb on/off. Ignored if motional surround is on. |
| Reverb Type | 00: OFF, 01: Room 1, 02: Room 2, 03: Hall 1, 04: Hall 2, 05: Plate, 06: GM2 Reverb | Selects the reverb type. Choose 00: OFF to disable. |
| Reverb Parameter | (varies) | Edit parameters for the selected reverb type. See Reverb Parameters reference (p. 98 of Parameter Guide). |
| Reverb Level | 0--127 | Adjusts the volume of the sound that has passed through reverb. |
| Reverb Output Assign | A, B, C, D | How the reverb output is routed. A/B/C/D = stereo to OUTPUT jacks. |

### MASTER EQ tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Master EQ Switch | OFF, ON | Master EQ on/off setting. |
| EQ Low Freq | 200, 400 Hz | Frequency of the low range. |
| EQ Low Gain | -15--+15 dB | Gain of the low frequency range. |
| EQ Mid Freq | 200, 250, 315, 400, 500, 630, 800, 1000, 1250, 1600, 2000, 2500, 3150, 4000, 5000, 6300, 8000 Hz | Frequency of the middle range. |
| EQ Mid Gain | -15--+15 dB | Gain of the middle frequency range. |
| EQ Mid Q | 0.5, 1.0, 2.0, 4.0, 8.0 | Width of the middle frequency range. Higher Q = narrower affected range. |
| EQ High Freq | 2000, 4000, 8000 Hz | Frequency of the high range. |
| EQ High Gain | -15--+15 dB | Gain of the high frequency range. |
