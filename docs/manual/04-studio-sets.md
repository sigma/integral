# Studio Set Structure and Features

## Overview

A Studio Set is the top-level organizational unit. It contains:
- 16 parts (each assigned a tone or drum kit)
- 1 external part (Ext Part) for audio input or USB audio
- Effects routing (Chorus, Reverb, Master EQ, Motional Surround)
- Per-part settings (level, pan, EQ, MIDI, keyboard range, etc.)

Switching studio sets changes all sounds and settings at once, making them
ideal for song presets or performance configurations.

> **See also:**
> - [Studio Set SysEx address map](../midi/05-studio-set.md) — complete parameter addresses (base `18 00 00 00`)
> - [Studio Set parameter descriptions](../params/01-studio-set.md) — musical meaning of every parameter
> - [Bank Select for Studio Sets](../midi/03-bank-select-tables.md#studio-sets) — MSB `85`, LSB `0`, PC `1-64`

## Studio Set Common Settings

### GENERAL tab

- **Tempo:** System tempo for the studio set
- **Drum COMP+EQ Assign Part:** Which part receives the 6 COMP+EQ sets

### CONTROL tab

- **Tone Controls 1-4:** Assigns MIDI messages to control tone parameters
  system-wide

### PHASE LOCK tab

- **Phase Lock:** Minimizes timing discrepancies between notes played by
  parts on the same MIDI channel

## Part Configuration

Each of the 16 parts has the following parameter groups:

### TONE tab

| Parameter | Description |
|-----------|-------------|
| Type | Tone type (SN-A, SN-S, SN-D, PCMS, PCMD) |
| Bank | Tone bank (Preset, User, SRX, ExSN, etc.) |
| Tone Number | Specific tone within the bank |

### LEVEL tab

| Parameter | Description |
|-----------|-------------|
| Level | Part volume (for balance between parts) |
| Pan | L64 (far left) to 63R (far right), 0 = center. No effect when Motional Surround is ON |
| Cho Send Level | Signal level sent to chorus (0 = no chorus) |
| Rev Send Level | Signal level sent to reverb (0 = no reverb) |
| Output Assign | Direct output routing (A-D, INDIVIDUAL 1-8). Ignored when Motional Surround is ON |
| Rx Switch | Whether the part receives MIDI data |
| Rx Channel | MIDI receive channel for the part |

### PITCH tab

| Parameter | Description |
|-----------|-------------|
| Octave Shift | +/- 3 octaves |
| Coarse Tune | +/- 4 octaves in semitones |
| Fine Tune | +/- 50 cents |
| Bend Range | Pitch bend range in semitones (up to 2 octaves) |
| Porta Switch | Portamento on/off |
| Porta Time | Portamento glide time |

### OFFSET tab

Sound modification offsets applied on top of the tone's own settings:

- Cutoff Offset, Resonance Offset
- Attack Offset, Decay Offset, Release Offset
- Vibrato Rate, Vibrato Depth, Vibrato Delay

These can be initialized via "SOUND CTRL INIT" without affecting other
part settings.

### EQ tab

Per-part 3-band equalizer:
- Low: Frequency + Gain
- Mid: Frequency + Gain + Q (bandwidth)
- High: Frequency + Gain
- EQ Switch: on/off

### KBD tab (Keyboard)

Keyboard split/layer configuration per part
([SysEx offsets `00 1D`-`00 24`](../midi/05-studio-set.md#7-studio-set-part-per-part)):

| Parameter | Description |
|-----------|-------------|
| Key Range Upper/Lower | Note range the part responds to |
| Key Fade Upper/Lower | Gradual volume decrease outside range (0 = hard cut) |
| Velo Range Upper/Lower | Velocity range for sounding |
| Velo Fade Upper/Lower | Gradual volume decrease outside velocity range |
| Velo Sens Offset | Velocity sensitivity adjustment |
| Mono/Poly | Monophonic or polyphonic playback |
| Legato Switch | Legato behavior when Mono/Poly = MONO |

By setting different Key Range and Velo Range values across parts on the
same MIDI channel, you can create keyboard splits and velocity-switched
layers.

### SCALE tab

- Tuning system selection (equal temperament, just intonation major/minor,
  Arabian scales, etc.)
- Key (tonic) selection for non-equal temperaments
- Per-note pitch adjustment in cents

### MIDI tab

Per-part MIDI receive filters
([SysEx offsets `00 39`-`00 43`](../midi/05-studio-set.md#7-studio-set-part-per-part),
see also [Channel Messages reference](../midi/02-channel-messages.md)):

| Parameter | Receives |
|-----------|----------|
| PC | Program Change |
| BS | Bank Select |
| BEND | Pitch Bend |
| PAFT | Polyphonic Key Pressure |
| CAFT | Channel Pressure |
| MOD | Modulation |
| VOL | Volume |
| PAN | Pan |
| EXP | Expression |
| HOLD | Hold 1 |
| VELO CRV | Velocity curve type (4 types or OFF) |

## Voice Reserve

The INTEGRA-7 has 128-voice maximum polyphony. When more than 128 voices
are needed simultaneously, the **Voice Reserve** parameter (per part)
determines how many voices are guaranteed for each part.

- Set higher values for parts where note dropout is unacceptable
  (e.g., piano, pads)
- Total voice reserve across all parts should not exceed 128
- Notes beyond the total 128 limit will steal from parts with lower
  voice reserve

> **Implementation note:** Voice Reserve parameters are at SysEx offsets
> `00 18` through `00 27` in
> [Studio Set Common](../midi/05-studio-set.md#1-studio-set-common).
> Range is 0-64, where 64 = FULL. The [params guide](../params/01-studio-set.md)
> notes that the total across all 16 parts cannot exceed 64 (not 128 as the
> max polyphony might suggest -- the SysEx range caps at 64 per part).

## External Part (Ext Part)

The Ext Part routes external audio through the studio set's effects chain:

| Parameter | Description |
|-----------|-------------|
| Level | Volume of the external input |
| Cho Send Level | Chorus send (0 = no chorus) |
| Rev Send Level | Reverb send (0 = no reverb) |

The external audio source is selected via the system parameter
**Ext Part Source Select**: either "USB AUDIO" or "INPUT" (front/rear jacks).

When Motional Surround is ON, the Ext Part can also be positioned in the
surround field using its own L-R/F-B controls (CC28/CC29/CC30).

## Saving Studio Sets

- Edits are in the Temporary Area and are lost on power-off or studio set change
- Save via Studio Set Write to User Memory
- Writing overwrites the destination slot
- When exporting a studio set to USB, the tones it uses are also exported

## Initialization

Three levels of initialization available:
1. **Part Init:** Resets one part's settings
2. **Sound Ctrl Init:** Resets only the offset parameters (cutoff, resonance,
   attack, decay, release, vibrato) for one part
3. **Studio Set Init:** Resets the entire studio set
