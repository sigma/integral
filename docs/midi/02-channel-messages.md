# Channel Messages

Reference for INTEGRA-7 Channel Voice Messages, Channel Mode Messages, and System Realtime Messages (Data Reception).

All messages in this section are subject to the global Rx Switch parameter (PART VIEW:LEVEL/CH) -- they are not received when this switch is OFF.

## Channel Voice Messages

### Note Off

| Field | Value | Description |
|-------|-------|-------------|
| Status | `8nH` or `9nH` | n = MIDI channel `0H`-`FH` (ch.1-16) |
| 2nd byte | `kkH` | Note number: `00H`-`7FH` (0-127) |
| 3rd byte | `vvH` | Note Off velocity: `00H`-`7FH` (0-127); `00H` when status is `9nH` |

### Note On

| Field | Value | Description |
|-------|-------|-------------|
| Status | `9nH` | n = MIDI channel `0H`-`FH` (ch.1-16) |
| 2nd byte | `kkH` | Note number: `00H`-`7FH` (0-127) |
| 3rd byte | `vvH` | Note On velocity: `01H`-`7FH` (1-127) |

### Polyphonic Key Pressure

| Field | Value | Description |
|-------|-------|-------------|
| Status | `AnH` | n = MIDI channel `0H`-`FH` (ch.1-16) |
| 2nd byte | `kkH` | Note number: `00H`-`7FH` (0-127) |
| 3rd byte | `vvH` | Pressure: `00H`-`7FH` (0-127) |

- Not received when the Rx Poly Key Press(PAFT) parameter (PART VIEW:MIDI) is OFF.

### Control Change

All Control Change messages share status byte `BnH`, where n = MIDI channel `0H`-`FH` (ch.1-16).

General notes:
- If the corresponding Controller number is selected for MFX Control Source 1-4 (TONE EDIT:MFX CTRL) or PCM Synth Tone Matrix Control 1-4 Source (TONE EDIT PCMS:MTRX CTRL1-4), the corresponding effect will occur.
- When Control Source Select (SYSTEM:CONTROL) is SYSTEM, controller numbers matching System Control Src 1-4 (SYSTEM:CONTROL) will apply to MFX Control Source 1-4 or Matrix Control 1-4 Source set to SYS CTRL1-4.
- When Control Source Select (SYSTEM:CONTROL) is STUDIO SET, controller numbers matching Tone Control Src 1-4 (STUDIO SET COMMON:CONTROL) will apply to MFX Control Source 1-4 or Matrix Control 1-4 Source set to SYS CTRL1-4.

#### Bank Select (CC 0, 32)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `00H` (MSB) / `20H` (LSB) | Controller number |
| 3rd byte | `mmH` (MSB) / `llH` (LSB) | Bank number: `00 00H`-`7F 7FH` (bank.1-16384) |

- Not received when the Rx Bank Select parameter (SYSTEM:MIDI) is OFF.
- Not received when the Rx Bank Select(BS) parameter (PART VIEW:MIDI) is OFF.

#### Modulation (CC 1)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `01H` | Controller number |
| 3rd byte | `vvH` | Modulation depth: `00H`-`7FH` (0-127) |

#### Breath Type (CC 2)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `02H` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`7FH` (0-127) |

#### Foot Type (CC 4)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `04H` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`7FH` (0-127) |

#### Portamento Time (CC 5)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `05H` | Controller number |
| 3rd byte | `vvH` | Portamento Time: `00H`-`7FH` (0-127) |

- Not received when the Porta Time parameter (PART VIEW:PITCH) is OFF.

#### Data Entry (CC 6, 38)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `06H` (MSB) / `26H` (LSB) | Controller number |
| 3rd byte | `mmH` (MSB) / `llH` (LSB) | Value of the parameter specified by RPN/NRPN |

#### Volume (CC 7)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `07H` | Controller number |
| 3rd byte | `vvH` | Volume: `00H`-`7FH` (0-127) |

- Not received when the Rx Volume(VOL) parameter (PART VIEW:MIDI) is OFF.
- The Level parameter (PART VIEW:LEVEL/CH) will change.

#### Pan (CC 10)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `0AH` | Controller number |
| 3rd byte | `vvH` | Panpot: `00H`-`40H`-`7FH` (Left - Center - Right) |

- Not received when the Rx Pan(PAN) parameter (PART VIEW:MIDI) is OFF.
- The Pan parameter (PART VIEW:LEVEL/CH) will change.

#### Expression (CC 11)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `0BH` | Controller number |
| 3rd byte | `vvH` | Expression: `00H`-`7FH` (0-127) |

- Not received when the Rx Expression(EXP) parameter (PART VIEW:MIDI) is OFF.
- Not received when the Partial Rx Expression parameter (TONE EDIT PCMS:CTRL or TONE EDIT PCMD:COMMON) is OFF.

#### Motional Surround Control 1 (CC 12)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `0CH` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`40H`-`7FH` (Left - Center - Right) |

- Not received when the Rx Modulation(MOD) parameter (PART VIEW:MIDI) is OFF.
- The Part L-R parameter (MOTIONAL SURROUND EDIT:PART) will change.
- Valid when the Motional Surround is ON.

#### Motional Surround Control 2 (CC 13)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `0DH` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`40H`-`7FH` (Back - Center - Front) |

- The Part F-B parameter (MOTIONAL SURROUND EDIT:PART) will change.
- Valid when the Motional Surround is ON.

#### Motional Surround Control 3 (CC 14)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `0EH` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`7FH` (0-127) |

- The Part Ambience Send Level parameter (MOTIONAL SURROUND EDIT:PART) will change.
- Valid when the Motional Surround is ON.

#### General Purpose Controller 1 / Tone Modify 1 (CC 16)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `10H` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`7FH` (0-127) |

#### General Purpose Controller 2 / Tone Modify 2 (CC 17)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `11H` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`7FH` (0-127) |

#### General Purpose Controller 3 / Tone Modify 3 (CC 18)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `12H` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`7FH` (0-127) |

#### General Purpose Controller 4 / Tone Modify 4 (CC 19)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `13H` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`7FH` (0-127) |

#### Motional Surround External Part Control 1 (CC 28)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `1CH` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`40H`-`7FH` (Left - Center - Right) |

- The PartEx L-R parameter (MOTIONAL SURROUND EDIT:PART) will change.
- Valid when the Motional Surround is ON.

#### Motional Surround External Part Control 2 (CC 29)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `1DH` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`40H`-`7FH` (Back - Center - Front) |

- The PartEx F-B parameter (MOTIONAL SURROUND EDIT:PART) will change.
- Valid when the Motional Surround is ON.

#### Motional Surround External Part Control 3 (CC 30)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `1EH` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`7FH` (0-127) |

- The PartEx Ambience Send Level parameter (MOTIONAL SURROUND EDIT:PART) will change.
- Valid when the Motional Surround is ON.

#### Hold 1 (CC 64)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `40H` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`7FH` (0-127); 0-63 = OFF, 64-127 = ON |

- Not received when the Rx Hold-1(HOLD) parameter (PART VIEW:MIDI) is OFF.
- Not received when the Partial Rx Hold-1 parameter (TONE EDIT PCMS:CTRL or TONE EDIT PCMD:COMMON) is OFF.

#### Portamento (CC 65)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `41H` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`7FH` (0-127); 0-63 = OFF, 64-127 = ON |

- The Porta Switch parameter (PART VIEW:PITCH) will change.

#### Sostenuto (CC 66)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `42H` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`7FH` (0-127); 0-63 = OFF, 64-127 = ON |

#### Soft (CC 67)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `43H` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`7FH` (0-127) |

#### Legato Foot Switch (CC 68)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `44H` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`7FH` (0-127); 0-63 = OFF, 64-127 = ON |

- The Legato Switch parameter (PART VIEW:LEVEL/CH) will change.

#### Hold 2 (CC 69)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `45H` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`7FH` (0-127) |

- A hold movement isn't done.

#### Resonance (CC 71)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `47H` | Controller number |
| 3rd byte | `vvH` | Resonance (relative change): `00H`-`40H`-`7FH` (-64 - 0 - +63) |

- The Reso Offset parameter (PART VIEW:OFFSET) will change.

#### Release Time (CC 72)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `48H` | Controller number |
| 3rd byte | `vvH` | Release Time (relative change): `00H`-`40H`-`7FH` (-64 - 0 - +63) |

- The Release Offset parameter (PART VIEW:OFFSET) will change.

#### Attack Time (CC 73)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `49H` | Controller number |
| 3rd byte | `vvH` | Attack Time (relative change): `00H`-`40H`-`7FH` (-64 - 0 - +63) |

- The Attack Offset parameter (PART VIEW:OFFSET) will change.

#### Cutoff (CC 74)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `4AH` | Controller number |
| 3rd byte | `vvH` | Cutoff (relative change): `00H`-`40H`-`7FH` (-64 - 0 - +63) |

- The Cutoff Offset parameter (PART VIEW:OFFSET) will change.

#### Decay Time (CC 75)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `4BH` | Controller number |
| 3rd byte | `vvH` | Decay Time (relative change): `00H`-`40H`-`7FH` (-64 - 0 - +63) |

- The Decay Offset parameter (PART VIEW:OFFSET) will change.

#### Vibrato Rate (CC 76)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `4CH` | Controller number |
| 3rd byte | `vvH` | Vibrato Rate (relative change): `00H`-`40H`-`7FH` (-64 - 0 - +63) |

- The Vibrato Rate parameter (PART VIEW:OFFSET) will change.

#### Vibrato Depth (CC 77)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `4DH` | Controller number |
| 3rd byte | `vvH` | Vibrato Depth (relative change): `00H`-`40H`-`7FH` (-64 - 0 - +63) |

- The Vibrato Depth parameter (PART VIEW:OFFSET) will change.

#### Vibrato Delay (CC 78)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `4EH` | Controller number |
| 3rd byte | `vvH` | Vibrato Delay (relative change): `00H`-`40H`-`7FH` (-64 - 0 - +63) |

- The Vibrato Delay parameter (PART VIEW:OFFSET) will change.

#### General Purpose Controller 5 / Tone Variation 1 (CC 80)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `50H` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`7FH` (0-127) |

#### General Purpose Controller 6 / Tone Variation 2 (CC 81)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `51H` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`7FH` (0-127) |

#### General Purpose Controller 7 / Tone Variation 3 (CC 82)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `52H` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`7FH` (0-127) |

#### General Purpose Controller 8 / Tone Variation 4 (CC 83)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `53H` | Controller number |
| 3rd byte | `vvH` | Control value: `00H`-`7FH` (0-127) |

#### Portamento Control (CC 84)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `54H` | Controller number |
| 3rd byte | `kkH` | Source note number: `00H`-`7FH` (0-127) |

- A Note-on received immediately after a Portamento Control message will change continuously in pitch, starting from the pitch of the Source Note Number.
- If a voice is already sounding for a note number identical to the Source Note Number, this voice will continue sounding (legato) and will, when the next Note-on is received, smoothly change to the pitch of that Note-on.
- The rate of the pitch change is determined by the Portamento Time value.

#### Reverb Send Level (CC 91)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `5BH` | Controller number |
| 3rd byte | `vvH` | Reverb Send Level: `00H`-`7FH` (0-127) |

- The Rev Send Level parameter (PART VIEW:LEVEL/CH) will change.

#### Chorus Send Level (CC 93)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `5DH` | Controller number |
| 3rd byte | `vvH` | Chorus Send Level: `00H`-`7FH` (0-127) |

- The Cho Send Level parameter (PART VIEW:LEVEL/CH) will change.

#### RPN MSB/LSB (CC 100, 101)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `65H` (MSB, CC 101) / `64H` (LSB, CC 100) | Controller number |
| 3rd byte | `mmH` (MSB) / `llH` (LSB) | Parameter number specified by RPN |

When using RPNs, first send RPN (CC 100 and CC 101, in any order) to select the parameter, then send Data Entry (CC 6 and CC 38) to set the value. Once RPN messages are received, subsequent Data Entry messages on the same MIDI channel are recognized as targeting that RPN. Transmitting RPN Null after setting parameters is recommended to prevent unintended changes.

##### RPN Parameter Table

| RPN MSB, LSB | Data Entry MSB, LSB | Parameter | Notes |
|--------------|---------------------|-----------|-------|
| `00H`, `00H` | `mmH`, `llH` | Pitch Bend Sensitivity | mm: `00H`-`18H` (0-24 semitones); ll: ignored (processed as `00H`). Up to 2 octaves in semitone steps. The Bend Range parameter (PART VIEW:PITCH) will change. |
| `00H`, `01H` | `mmH`, `llH` | Channel Fine Tuning | mm, ll: `20 00H` - `40 00H` - `60 00H` (-4096x100/8192 - 0 - +4096x100/8192 cent). The Fine Tune parameter (PART VIEW:PITCH) will change. |
| `00H`, `02H` | `mmH`, `llH` | Channel Coarse Tuning | mm: `10H`-`40H`-`70H` (-48 - 0 - +48 semitones); ll: ignored (processed as `00H`). The Coarse Tune parameter (PART VIEW:PITCH) will change. |
| `7FH`, `7FH` | `---`, `---` | RPN Null | RPN and NRPN will be set as "unspecified." Once set, previously set parameter values will not change. mm, ll: ignored. |

### Program Change

| Field | Value | Description |
|-------|-------|-------------|
| Status | `CnH` | n = MIDI channel `0H`-`FH` (ch.1-16) |
| 2nd byte | `ppH` | Program number: `00H`-`7FH` (prog.1-128) |

- Not received when the Rx Program Change parameter (SYSTEM:MIDI) is OFF.
- Not received when the Rx Program Change(PC) parameter (PART VIEW:MIDI) is OFF.

### Channel Pressure

| Field | Value | Description |
|-------|-------|-------------|
| Status | `DnH` | n = MIDI channel `0H`-`FH` (ch.1-16) |
| 2nd byte | `vvH` | Channel Pressure: `00H`-`7FH` (0-127) |

- Not received when the Rx Ch Press(CAFT) parameter (PART VIEW:MIDI) is OFF.

### Pitch Bend Change

| Field | Value | Description |
|-------|-------|-------------|
| Status | `EnH` | n = MIDI channel `0H`-`FH` (ch.1-16) |
| 2nd byte | `llH` | Pitch Bend value LSB |
| 3rd byte | `mmH` | Pitch Bend value MSB |
| | | mm, ll: `00 00H` - `40 00H` - `7F 7FH` (-8192 - 0 - +8191) |

- Not received when the Rx Pitch Bend(BEND) parameter (PART VIEW:MIDI) is OFF.
- Not received when the Partial Rx Bender parameter (TONE EDIT PCMS:CTRL) is OFF.

## Channel Mode Messages

All Channel Mode messages share status byte `BnH`, where n = MIDI channel `0H`-`FH` (ch.1-16).

Not received when the Rx Switch parameter (PART VIEW:LEVEL/CH) is OFF.

### All Sounds Off (CC 120)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `78H` | Controller number |
| 3rd byte | `00H` | |

- When received, all notes currently sounding on the corresponding channel will be turned off.

### Reset All Controllers (CC 121)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `79H` | Controller number |
| 3rd byte | `00H` | |

When received, the following controllers are set to their reset values:

| Controller | Reset Value |
|------------|-------------|
| Pitch Bend Change | +/-0 (center) |
| Polyphonic Key Pressure | 0 (off) |
| Channel Pressure | 0 (off) |
| Modulation | 0 (off) |
| Breath Type | 0 (min) |
| Foot Type | 0 (min) |
| Expression | 127 (max); however the controller will be at minimum |
| Hold 1 | 0 (off) |
| Sostenuto | 0 (off) |
| Soft | 0 (off) |
| Hold 2 | 0 (off) |
| RPN | unset; previously set data will not change |
| NRPN | unset; previously set data will not change |

### All Notes Off (CC 123)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `7BH` | Controller number |
| 3rd byte | `00H` | |

- When received, all notes on the corresponding channel will be turned off. However, if Hold 1 or Sostenuto is ON, the sound will continue until these are turned off.

### OMNI OFF (CC 124)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `7CH` | Controller number |
| 3rd byte | `00H` | |

- The same processing will be carried out as when All Notes Off is received.

### OMNI ON (CC 125)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel |
| 2nd byte | `7DH` | Controller number |
| 3rd byte | `00H` | |

- The same processing will be carried out as when All Notes Off is received. OMNI ON will not be turned on.

### MONO (CC 126)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel `0H`-`FH` (ch.1-16) |
| 2nd byte | `7EH` | Controller number |
| 3rd byte | `mmH` | Mono number: `00H`-`10H` (0-16) |

- The same processing will be carried out as when All Notes Off is received.
- The Mono/Poly parameter (PART VIEW:LEVEL/CH) will change.

### POLY (CC 127)

| Field | Value | Description |
|-------|-------|-------------|
| Status | `BnH` | n = MIDI channel `0H`-`FH` (ch.1-16) |
| 2nd byte | `7FH` | Controller number |
| 3rd byte | `00H` | |

- The same processing will be carried out as when All Notes Off is received.
- The Mono/Poly parameter (PART VIEW:LEVEL/CH) will change.

## System Realtime Messages

### Timing Clock

| Field | Value |
|-------|-------|
| Status | `F8H` |

- Received when Sync Mode parameter (SYSTEM:SYNC/TEMPO) is set to SLAVE.

### Active Sensing

| Field | Value |
|-------|-------|
| Status | `FEH` |

- When Active Sensing is received, the unit will begin monitoring the intervals of all further messages. While monitoring, if the interval between messages exceeds 420 ms, the same processing will be carried out as when All Sounds Off, All Notes Off, and Reset All Controllers are received, and message interval monitoring will be halted.
