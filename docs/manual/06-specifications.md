# Technical Specifications and Reference

## Specifications

### Sound Generator

| Parameter | Value |
|-----------|-------|
| Maximum Polyphony | 128 voices (varies with sound generator load) |
| Parts | 16 |
| Tone Types | SN-A, SN-S, SN-D, PCMS, PCMD (GM2 compatible included) |

### Expansion Virtual Slots

| Parameter | Value |
|-----------|-------|
| Slot Count | 4 |
| SRX Series | 12 titles (1 slot each) |
| ExSN | 6 titles (1 slot each) |
| ExPCM | 1 title (uses all 4 slots) |

### Effects

| Effect | Count | Types |
|--------|-------|-------|
| Multi-Effects (MFX) | 16 systems | 67 types |
| Part EQ | 16 systems | - |
| Drum Part COMP+EQ | 6 systems | - |
| Motional Surround | 1 system | - |
| Chorus | 1 system | 3 types |
| Reverb | 1 system | 6 types |
| Master EQ | 1 system | - |

### Connectors

| Connector | Type |
|-----------|------|
| PHONES | Stereo 1/4-inch phone |
| INPUT (Front) | L, R - 1/4-inch phone |
| INPUT (Rear) | L, R - 1/4-inch phone |
| OUTPUT A (MIX) | L, R - 1/4-inch TRS + XLR |
| OUTPUT B | L, R - 1/4-inch phone |
| OUTPUT C | L, R - 1/4-inch phone |
| OUTPUT D | L, R - 1/4-inch phone |
| DIGITAL AUDIO OUT | Coaxial (S/P DIF) |
| MIDI | IN, OUT, THRU |
| USB COMPUTER | Audio/MIDI (USB 2.0 Hi-Speed) |
| USB Memory | For USB flash drives |
| AC IN | Power cord |

### Physical

| Parameter | Value |
|-----------|-------|
| Dimensions | 481 (W) x 262 (D) x 89 (H) mm |
| Weight | 3.9 kg |
| Power Consumption | 18 W |
| Display | 256 x 80 dots graphic LCD (backlit) |
| Rack Mount | 19-inch rack compatible |

### Digital Audio

| Parameter | Value |
|-----------|-------|
| Sampling Rates | 44.1 / 48 / 96 kHz |
| Bit Depth | 24-bit (fixed) |
| Channels | 2 (stereo) |
| Digital Output | S/P DIF coaxial, same signal as A (MIX) |

## Error Messages

| Message | Meaning | Action |
|---------|---------|--------|
| Cannot Import! | Sound data cannot be imported | Select an import destination |
| File Not Selected! | No file is selected | Select a file |
| Incorrect File Name! | Invalid file name | Avoid names starting with "." or containing `\ / ; * ? " < > \|` |
| MIDI Buffer Full! | Too much MIDI data received | Reduce MIDI message volume |
| MIDI Offline! | MIDI IN connection broken | Check MIDI cable and connections |
| Permission Denied! | File/folder is read-only | Clear read-only attribute on computer |
| Program Error! | Startup failure or invalid update | Re-run system update; contact support if persistent |
| Read Error! | Cannot read from USB flash drive | Check USB connection; use Roland-branded drives |
| System Memory Damaged! | System memory corruption | Execute factory reset; contact support if persistent |
| USB Memory Full! | Insufficient USB space | Delete unneeded data |
| USB Memory Not Ready! | USB not inserted or incompletely seated | Power off, reinsert USB firmly, power on |
| Write Error! | Cannot write to USB flash drive | Check USB connection; reformat if needed |

## Troubleshooting Quick Reference

### No sound

| Check | Solution |
|-------|----------|
| Amp/speakers powered? | Turn on connected equipment |
| [VOLUME] knob at minimum? | Adjust volume |
| Connections correct? | Verify cable connections |
| MIDI receive channel match? | Match Rx Channel to transmitting device |
| Connection cable with resistor? | Use cable without resistor |
| Partial switches off? | Turn on Partial Switch |
| Master Level too low? | Check System > SOUND > Master Level |
| Effect levels at 0? | Check effect on/off and send levels |
| Expansion data not loaded? | Load required expansion into virtual slot |
| USB-MIDI Thru ON? | Turn off if playing via MIDI IN |
| Part muted? | Unmute the part |
| Rx Switch off? | Turn Rx Switch on for the part |
| Key range excluding played notes? | Check Key Range settings |

### Sound quality issues

| Problem | Check | Solution |
|---------|-------|----------|
| Distorted sound | [VOLUME] too high | Lower volume |
| Distorted sound | Output Gain too high | Check System > SOUND > Output Gain |
| Distorted sound | INPUT signal too hot | Lower external device volume (watch PEAK indicator) |
| Wrong pitch | Master Tune setting | Check System > SOUND > Master Tune |
| Wrong pitch | Pitch bend from external device | Check external device pitch bend |
| Wrong pitch | Coarse/Fine Tune set | Check part Coarse Tune and Fine Tune |
| Notes cut off | >128 voices sounding | Reduce layers; increase Voice Reserve for critical parts |
| Pan not working | Motional Surround on | Turn off Motional Surround for L/R pan |

### Delay/tempo issues

| Problem | Check | Solution |
|---------|-------|----------|
| Delay time won't change | Sync Mode = SLAVE | Change tempo on the external clock source |
| Delay time capped | Note-value delay at slow tempo | Use numeric delay value instead; increase tempo |

## MIDI Implementation Chart Summary

Key points from the MIDI Implementation Chart (full details in
[docs/midi/](../midi/README.md) -- specifically
[channel messages](../midi/02-channel-messages.md) and
[SysEx protocol](../midi/01-protocol.md)):

- **Mode:** Receives Mode 3 (Omni Off, Poly) and Mode 4 (Omni Off, Mono, M=1)
- **Note range:** 0-127
- **Velocity:** Note On and Note Off both recognized
- **Aftertouch:** Both Key (polyphonic) and Channel recognized (selectable)
- **Pitch Bend:** Recognized (selectable)
- **System Exclusive:** Transmitted only when Tx Edit Data is ON or RQ1 received

### Notable CC assignments

See [Channel Messages](../midi/02-channel-messages.md) for full details on
each CC, including receive conditions and which SysEx parameters they affect.

| CC | Function | Notes |
|----|----------|-------|
| 12 | Part L-R position | Motional Surround |
| 13 | Part F-B position | Motional Surround |
| 14 | Part Ambience Send | Motional Surround |
| 16-19 | Tone Modify 1-4 | General purpose |
| 28 | Ext Part L-R | Motional Surround |
| 29 | Ext Part F-B | Motional Surround |
| 30 | Ext Part Ambience Send | Motional Surround |
| 71 | Resonance | Maps to Part Resonance Offset |
| 72 | Release Time | Maps to Part Release Offset |
| 73 | Attack Time | Maps to Part Attack Offset |
| 74 | Cutoff | Maps to Part Cutoff Offset |
| 75 | Decay Time | Maps to Part Decay Offset |
| 76-78 | Vibrato Rate/Depth/Delay | Maps to Part Vibrato offsets |
| 80-83 | Tone Variation 1-4 | SN-A variation sounds |
| 91 | Reverb send | Maps to Part Rev Send Level |
| 93 | Chorus send | Maps to Part Cho Send Level |
