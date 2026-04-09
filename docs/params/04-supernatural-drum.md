# SuperNATURAL Drum Kit (SN-D) Parameters

## Overview

SuperNATURAL Drum Kits contain 62 drum instrument partials, each assigned to
a different note number. Each kit also has multi-effect (MFX) settings and up
to 6 sets of compressor + equalizer units for the part specified by the Drum
Comp+EQ Assign setting.

```
PARTIAL 1--62 (DRUM INST) -> COMP+EQ 1--6 -> MFX
```

---

## COMMON Tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Phrase Number | 0--16 | Preview phrase number |
| Drum Kit Level | 0--127 | Volume of the entire drum kit |
| Ambience Level | 0--127 | Volume of drum kit resonances and room resonances. Applies only to Kick, Snare, Tom, and Hi-Hat types. Some instruments are not affected (see Drum Inst List). |

---

## DRUM INST Tab (per Partial)

Each of the 62 partials has these parameters:

| Parameter | Value | Description |
|-----------|-------|-------------|
| Inst Bank | INT, ExSN6 | Selects the drum inst bank (ExSN6 requires expansion) |
| Inst Number | 000: OFF, 001-- | Selects the drum inst number (000 = silent) |
| Level | 0--127 | Volume of the drum inst |
| Pan | L64--63R | Pan position of the drum inst |
| Chorus Send Level | 0--127 | Signal sent to chorus. No effect if motional surround is on. |
| Reverb Send Level | 0--127 | Signal sent to reverb. No effect if motional surround is on. |
| Tune | -120--+120 | Pitch adjustment of the drum inst |
| Attack | 0--100% | Attack level and time. 100% = fastest attack. |
| Decay | -63--0 | Decay time adjustment. Negative values produce a muting effect. |
| Brilliance | -15--+12 | Sound brightness. Positive = brighter, negative = darker. |
| Variation | OFF, FLAM1, FLAM2, FLAM3, BUZZ1, BUZZ2, BUZZ3, ROLL | Performance variations (availability depends on instrument, see Drum Inst List) |
| Dynamic Range | 0--63 | Velocity-to-volume curve. 0 = any velocity produces max volume. |
| Stereo Width | 0--127 | Stereo width of the sound. 0 = monaural. Some instruments are not affected (see Drum Inst List). |
| Output Assign | PART, COMP+EQ1--6 | How the drum inst output is routed |

---

## COMP Tab (Compressor 1--6)

COMP + EQ can only be used for the part specified by the Drum Comp+EQ Assign
setting.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Comp 1--6 Switch | OFF, ON | Compressor on/off |
| Comp 1--6 Attack Time | 0.05--50.0 ms | Time from when input exceeds threshold until compression begins |
| Comp 1--6 Release Time | 0.05--2000 ms | Time from when input falls below threshold until compression stops |
| Comp 1--6 Threshold | 0--127 | Level above which compression is applied |
| Comp 1--6 Ratio | 1:1--inf:1 | Compression ratio |
| Comp 1--6 Output Gain | 0--+24 dB | Level of the output sound |

---

## EQ Tab (Equalizer 1--6)

COMP + EQ can only be used for the part specified by the Drum Comp+EQ Assign
setting.

| Parameter | Value | Description |
|-----------|-------|-------------|
| EQ 1--6 Switch | OFF, ON | Equalizer on/off |
| EQ 1--6 Low Freq | 200, 400 Hz | Frequency of the low range |
| EQ 1--6 Low Gain | -15--+15 dB | Gain of the low frequency range |
| EQ 1--6 Mid Freq | 200, 250, 315, 400, 500, 630, 800, 1000, 1250, 1600, 2000, 2500, 3150, 4000, 5000, 6300, 8000 Hz | Frequency of the middle range |
| EQ 1--6 Mid Gain | -15--+15 dB | Gain of the middle frequency range |
| EQ 1--6 Mid Q | 0.5, 1.0, 2.0, 4.0, 8.0 | Width of the middle frequency range. Higher Q = narrower band. |
| EQ 1--6 High Freq | 2000, 4000, 8000 Hz | Frequency of the high range |
| EQ 1--6 High Gain | -15--+15 dB | Gain of the high frequency range |

---

## MFX Tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| MFX Switch | OFF, ON | Switches the multi-effect on/off |
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

---

## SuperNATURAL Drum Inst List

The table below shows each drum instrument with its type, support for Stereo
Width, Ambience Level, and available Variation types.

Legend:
- **Width**: supports Stereo Width parameter
- **Ambience**: supports Ambience Level parameter
- **Variation**: available performance variation types (Flam/Buzz, Flam/Buzz/Roll, or -)

### INT Bank

| No. | Name | Type | Width | Ambience | Variation |
|-----|------|------|-------|----------|-----------|
| 1 | Studio Kick | Kick | Yes | Yes | Flam/Buzz |
| 2 | Pop Kick | Kick | Yes | Yes | Flam/Buzz |
| 3 | Jazz Kick | Kick | Yes | Yes | Flam/Buzz |
| 4 | Rock Kick | Kick | Yes | Yes | Flam/Buzz |
| 5 | Studio Kick 2 | Kick | Yes | Yes | Flam/Buzz |
| 6 | Rock Kick 2 | Kick | Yes | Yes | Flam/Buzz |
| 7 | Orch Bass Drum | Kick | Yes | Yes | Flam/Buzz |
| 8 | Studio Sn | Snare | Yes | Yes | Flam/Buzz/Roll |
| 9 | Studio Sn Rim | Snare | Yes | Yes | Flam/Buzz/Roll |
| 10 | Studio Sn XStk | Snare | Yes | Yes | Flam/Buzz |
| 11 | Pop Sn | Snare | Yes | Yes | Flam/Buzz/Roll |
| 12 | Pop Sn Rim | Snare | Yes | Yes | Flam/Buzz/Roll |
| 13 | Pop Sn XStk | Snare | Yes | Yes | Flam/Buzz |
| 14 | Jazz Sn | Snare | Yes | Yes | Flam/Buzz/Roll |
| 15 | Jazz Sn Rim | Snare | Yes | Yes | Flam/Buzz/Roll |
| 16 | Jazz Sn XStk | Snare | Yes | Yes | Flam/Buzz |
| 17 | Rock Sn | Snare | Yes | Yes | Flam/Buzz/Roll |
| 18 | Rock Sn Rim | Snare | Yes | Yes | Flam/Buzz/Roll |
| 19 | Rock Sn XStk | Snare | Yes | Yes | Flam/Buzz |
| 20 | Tight Sn | Snare | Yes | Yes | Flam/Buzz/Roll |
| 21 | Tight Sn Rim | Snare | Yes | Yes | Flam/Buzz/Roll |
| 22 | Tight Sn XStk | Snare | Yes | Yes | Flam/Buzz |
| 23 | Studio Sn 2 | Snare | Yes | Yes | Flam/Buzz/Roll |
| 24 | Studio Sn 2 Rim | Snare | Yes | Yes | Flam/Buzz/Roll |
| 25 | Studio Sn 2 XStk | Snare | Yes | Yes | Flam/Buzz |
| 26 | Rock Sn 2 | Snare | Yes | Yes | Flam/Buzz/Roll |
| 27 | Rock Sn 2 Rim | Snare | Yes | Yes | Flam/Buzz/Roll |
| 28 | Rock Sn 2 XStk | Snare | Yes | Yes | Flam/Buzz |
| 29 | Brush Sn Slap | Snare | Yes | Yes | Flam/Buzz/Roll |
| 30 | Brush Sn Tap | Snare | Yes | Yes | Flam/Buzz/Roll |
| 31 | Brush Sn Slide | Snare | Yes | Yes | Flam/Buzz |
| 32 | Brush Sn Swirl 1 | Snare | Yes | Yes | - |
| 33 | Brush Sn Swirl 2 | Snare | Yes | Yes | - |
| 34 | Snare CrossStk | Snare | Yes | Yes | Flam/Buzz |
| 35 | Orch Snare | Snare | Yes | Yes | Flam/Buzz/Roll |
| 36 | Orch Snare XStk | Snare | Yes | Yes | Flam/Buzz |
| 37 | Pop Tom Hi | Tom | Yes | Yes | Flam/Buzz |
| 38 | Pop Tom Mid | Tom | Yes | Yes | Flam/Buzz |
| 39 | Pop Tom Flr | Tom | Yes | Yes | Flam/Buzz |
| 40 | Rock Tom Hi | Tom | Yes | Yes | Flam/Buzz |
| 41 | Rock Tom Mid | Tom | Yes | Yes | Flam/Buzz |
| 42 | Rock Tom Floor | Tom | Yes | Yes | Flam/Buzz |
| 43 | Jazz Tom Hi | Tom | Yes | Yes | Flam/Buzz |
| 44 | Jazz Tom Mid | Tom | Yes | Yes | Flam/Buzz |
| 45 | Jazz Tom Floor | Tom | Yes | Yes | Flam/Buzz |
| 46 | Brush Tom Hi | Tom | Yes | Yes | Flam/Buzz |
| 47 | Brush Tom Mid | Tom | Yes | Yes | Flam/Buzz |
| 48 | Brush Tom Floor | Tom | Yes | Yes | Flam/Buzz |
| 49 | Med HH Close | Hi-Hat | Yes | Yes | Flam/Buzz |
| 50 | Med HH Open | Hi-Hat | Yes | Yes | Flam/Buzz |
| 51 | Med HH Pedal | Hi-Hat | Yes | Yes | Flam/Buzz |
| 52 | Standard HH Cl | Hi-Hat | Yes | Yes | Flam/Buzz |
| 53 | Standard HH Op | Hi-Hat | Yes | Yes | Flam/Buzz |
| 54 | Standard HH Pdl | Hi-Hat | Yes | Yes | Flam/Buzz |
| 55 | Jazz HH Close | Hi-Hat | Yes | Yes | Flam/Buzz |
| 56 | Jazz HH Open | Hi-Hat | Yes | Yes | Flam/Buzz |
| 57 | Jazz HH Pedal | Hi-Hat | Yes | Yes | Flam/Buzz |
| 58 | Brush HH Close | Hi-Hat | Yes | Yes | Flam/Buzz |
| 59 | Brush HH Open | Hi-Hat | Yes | Yes | Flam/Buzz |
| 60 | Standard Rd Edge | Ride | Yes | - | Flam/Buzz |
| 61 | Standard Rd Bell | Ride | Yes | - | Flam/Buzz |
| 62 | Std Rd Edge/Bell | Ride | Yes | - | Flam/Buzz |
| 63 | Medium Ride Edge | Ride | Yes | - | Flam/Buzz |
| 64 | Medium Ride Bell | Ride | Yes | - | Flam/Buzz |
| 65 | Med Rd Edge/Bell | Ride | Yes | - | Flam/Buzz |
| 66 | Flat 18"Ride | Ride | Yes | - | Flam/Buzz |
| 67 | Brush 18"Ride | Ride | Yes | - | Flam/Buzz |
| 68 | Brush 20"Ride | Ride | Yes | - | Flam/Buzz |
| 69 | Standard 16"Cr R | Crash | Yes | - | Flam/Buzz/Roll |
| 70 | Standard 16"Cr L | Crash | Yes | - | Flam/Buzz/Roll |
| 71 | Standard 18"Cr R | Crash | Yes | - | Flam/Buzz/Roll |
| 72 | Standard 18"Cr L | Crash | Yes | - | Flam/Buzz/Roll |
| 73 | Jazz 16"Cr R | Crash | Yes | - | Flam/Buzz/Roll |
| 74 | Jazz 16"Cr L | Crash | Yes | - | Flam/Buzz/Roll |
| 75 | Heavy 18"Cr R | Crash | Yes | - | Flam/Buzz/Roll |
| 76 | Heavy 18"Cr L | Crash | Yes | - | Flam/Buzz/Roll |
| 77 | Brush 16"Cr R | Crash | Yes | - | Flam/Buzz |
| 78 | Brush 16"Cr L | Crash | Yes | - | Flam/Buzz |
| 79 | Brush 18"Cr R | Crash | Yes | - | Flam/Buzz |
| 80 | Brush 18"Cr L | Crash | Yes | - | Flam/Buzz |
| 81 | Splash Cymbal 1 | Crash | Yes | - | Flam/Buzz |
| 82 | Splash Cymbal 2 | Crash | Yes | - | Flam/Buzz |
| 83 | Brush Splash Cym | Crash | Yes | - | Flam/Buzz |
| 84 | China Cymbal | Crash | Yes | - | Flam/Buzz |
| 85 | Orch Cymbal | Crash | Yes | - | Flam/Buzz |
| 86 | Orch Mallet Cym | Crash | Yes | - | Flam/Buzz/Roll |
| 87 | Gong | Crash | Yes | - | Flam/Buzz |
| 88 | Timpani F2 | Percussion | Yes | - | Flam/Buzz |
| 89 | Timpani F#2 | Percussion | Yes | - | Flam/Buzz |
| 90 | Timpani G2 | Percussion | Yes | - | Flam/Buzz |
| 91 | Timpani G#2 | Percussion | Yes | - | Flam/Buzz |
| 92 | Timpani A2 | Percussion | Yes | - | Flam/Buzz |
| 93 | Timpani A#2 | Percussion | Yes | - | Flam/Buzz |
| 94 | Timpani B2 | Percussion | Yes | - | Flam/Buzz |
| 95 | Timpani C3 | Percussion | Yes | - | Flam/Buzz |
| 96 | Timpani C#3 | Percussion | Yes | - | Flam/Buzz |
| 97 | Timpani D3 | Percussion | Yes | - | Flam/Buzz |
| 98 | Timpani D#3 | Percussion | Yes | - | Flam/Buzz |
| 99 | Timpani E3 | Percussion | Yes | - | Flam/Buzz |
| 100 | Timpani F3 | Percussion | Yes | - | Flam/Buzz |
| 101 | Tambourine 1 | Percussion | Yes | - | Flam/Buzz/Roll |
| 102 | Tambourine 2 | Percussion | - | - | Flam/Buzz |
| 103 | Cowbell 1 | Percussion | Yes | - | Flam/Buzz |
| 104 | Cowbell 2 | Percussion | - | - | Flam/Buzz |
| 105 | Vibra-slap | Percussion | - | - | Flam/Buzz |
| 106 | High Bongo 1 | Percussion | Yes | - | Flam/Buzz/Roll |
| 107 | Low Bongo 1 | Percussion | Yes | - | Flam/Buzz |
| 108 | High Bongo 2 | Percussion | - | - | Flam/Buzz |
| 109 | Low Bongo 2 | Percussion | - | - | Flam/Buzz |
| 110 | MuteHi Conga 1 | Percussion | Yes | - | Flam/Buzz |
| 111 | OpenHi Conga 1 | Percussion | Yes | - | Flam/Buzz/Roll |
| 112 | Low Conga 1 | Percussion | Yes | - | Flam/Buzz/Roll |
| 113 | MuteHi Conga 2 | Percussion | - | - | Flam/Buzz |
| 114 | OpenHi Conga 2 | Percussion | - | - | Flam/Buzz |
| 115 | Low Conga 2 | Percussion | - | - | Flam/Buzz |
| 116 | High Timbale | Percussion | Yes | - | Flam/Buzz |
| 117 | Low Timbale | Percussion | Yes | - | Flam/Buzz |
| 118 | High Agogo 1 | Percussion | Yes | - | Flam/Buzz |
| 119 | Low Agogo 1 | Percussion | Yes | - | Flam/Buzz |
| 120 | High Agogo 2 | Percussion | - | - | Flam/Buzz |
| 121 | Low Agogo 2 | Percussion | - | - | Flam/Buzz |
| 122 | Cabasa 1 | Percussion | Yes | - | Flam/Buzz |
| 123 | Cabasa 2 | Percussion | - | - | Flam/Buzz |
| 124 | Maracas 1 | Percussion | Yes | - | Flam/Buzz |
| 125 | Maracas 2 | Percussion | - | - | Flam/Buzz |
| 126 | Short Whistle | Percussion | - | - | Flam/Buzz |
| 127 | Long Whistle | Percussion | - | - | Flam/Buzz |
| 128 | Short Guiro | Percussion | - | - | Flam/Buzz |
| 129 | Long Guiro | Percussion | - | - | Flam/Buzz |
| 130 | Claves 1 | Percussion | Yes | - | Flam/Buzz |
| 131 | Claves 2 | Percussion | - | - | Flam/Buzz |
| 132 | Hi WoodBlock 1 | Percussion | Yes | - | Flam/Buzz |
| 133 | Low WoodBlock 1 | Percussion | Yes | - | Flam/Buzz |
| 134 | Hi WoodBlock 2 | Percussion | - | - | Flam/Buzz |
| 135 | Low WoodBlock 2 | Percussion | - | - | Flam/Buzz |
| 136 | Mute Cuica 1 | Percussion | Yes | - | Flam/Buzz |
| 137 | Open Cuica 1 | Percussion | Yes | - | Flam/Buzz |
| 138 | Mute Cuica 2 | Percussion | - | - | Flam/Buzz |
| 139 | Open Cuica 2 | Percussion | - | - | Flam/Buzz |
| 140 | Mute Triangle 1 | Percussion | - | - | Flam/Buzz/Roll |
| 141 | Open Triangle 1 | Percussion | - | - | Flam/Buzz/Roll |
| 142 | Mute Triangle 2 | Percussion | - | - | Flam/Buzz |
| 143 | Open Triangle 2 | Percussion | - | - | Flam/Buzz |
| 144 | Shaker | Percussion | - | - | Flam/Buzz |
| 145 | Sleigh Bell 1 | Percussion | Yes | - | Flam/Buzz |
| 146 | Sleigh Bell 2 | Percussion | - | - | Flam/Buzz |
| 147 | Wind Chimes | Percussion | Yes | - | Flam/Buzz |
| 148 | Castanets 1 | Percussion | Yes | - | Flam/Buzz/Roll |
| 149 | Castanets 2 | Percussion | - | - | Flam/Buzz |
| 150 | Mute Surdo 1 | Percussion | Yes | - | Flam/Buzz |
| 151 | Open Surdo 1 | Percussion | Yes | - | Flam/Buzz |
| 152 | Mute Surdo 2 | Percussion | - | - | Flam/Buzz |
| 153 | Open Surdo 2 | Percussion | - | - | Flam/Buzz |
| 154 | Sticks | Other | - | - | Flam/Buzz |
| 155 | Square Click | Other | - | - | Flam/Buzz |
| 156 | Metro Click | Other | - | - | Flam/Buzz |
| 157 | Metro Bell | Other | - | - | Flam/Buzz |
| 158 | Hand Clap | Other | - | - | Flam/Buzz |
| 159 | High Q | SFX | - | - | Flam/Buzz |
| 160 | Slap | SFX | - | - | Flam/Buzz |
| 161 | Scratch Push | SFX | - | - | Flam/Buzz |
| 162 | Scratch Pull | SFX | - | - | Flam/Buzz |
| 163 | Gt Fret Noise | SFX | - | - | Flam/Buzz |
| 164 | Gt Cutting Up Nz | SFX | - | - | Flam/Buzz |
| 165 | Gt Cutting Dw Nz | SFX | - | - | Flam/Buzz |
| 166 | AcBass Noise | SFX | - | - | Flam/Buzz |
| 167 | Flute Key Click | SFX | - | - | Flam/Buzz |
| 168 | Applause | SFX | Yes | - | - |

### ExSN6 Bank

| No. | Name | Type | Width | Ambience | Variation |
|-----|------|------|-------|----------|-----------|
| 1 | Laughing 1 | SFX | Yes | - | - |
| 2 | Laughing 2 | SFX | Yes | - | - |
| 3 | Laughing 3 | SFX | Yes | - | - |
| 4 | Scream 1 | SFX | Yes | - | - |
| 5 | Scream 2 | SFX | Yes | - | - |
| 6 | Scream 3 | SFX | Yes | - | - |
| 7 | Punch 1 | SFX | Yes | - | - |
| 8 | Punch 2 | SFX | Yes | - | - |
| 9 | Punch 3 | SFX | Yes | - | - |
| 10 | Heart Beat 1 | SFX | Yes | - | - |
| 11 | Heart Beat 2 | SFX | Yes | - | - |
| 12 | Heart Beat 3 | SFX | Yes | - | - |
| 13 | Foot Steps 1 | SFX | Yes | - | - |
| 14 | Foot Steps 2 | SFX | Yes | - | - |
| 15 | Foot Steps 3 | SFX | Yes | - | - |
| 16 | Foot Step 1 A | SFX | Yes | - | - |
| 17 | Foot Step 1 B | SFX | Yes | - | - |
| 18 | Foot Step 2 A | SFX | Yes | - | - |
| 19 | Foot Step 2 B | SFX | Yes | - | - |
| 20 | Foot Step 3 A | SFX | Yes | - | - |
| 21 | Foot Step 3 B | SFX | Yes | - | - |
| 22 | Door Creaking 1 | SFX | Yes | - | - |
| 23 | Door Creaking 2 | SFX | Yes | - | - |
| 24 | Door Creaking 3 | SFX | Yes | - | - |
| 25 | Door Slam 1 | SFX | Yes | - | - |
| 26 | Door Slam 2 | SFX | Yes | - | - |
| 27 | Door Slam 3 | SFX | Yes | - | - |
| 28 | Scratch | SFX | Yes | - | - |
| 29 | MetalScratch | SFX | Yes | - | - |
| 30 | Matches | SFX | Yes | - | - |
| 31 | Car Engine 1 | SFX | Yes | - | - |
| 32 | Car Engine 2 | SFX | Yes | - | - |
| 33 | Car Engine 3 | SFX | Yes | - | - |
| 34 | Car Stop 1 L>R | SFX | Yes | - | - |
| 35 | Car Stop 1 R>L | SFX | Yes | - | - |
| 36 | Car Stop 2 L>R | SFX | Yes | - | - |
| 37 | Car Stop 2 R>L | SFX | Yes | - | - |
| 38 | Car Stop 3 L>R | SFX | Yes | - | - |
| 39 | Car Stop 3 R>L | SFX | Yes | - | - |
| 40 | CarPassing 1 L>R | SFX | Yes | - | - |
| 41 | CarPassing 1 R>L | SFX | Yes | - | - |
| 42 | CarPassing 2 L>R | SFX | Yes | - | - |
| 43 | CarPassing 2 R>L | SFX | Yes | - | - |
| 44 | CarPassing 3 L>R | SFX | Yes | - | - |
| 45 | CarPassing 3 R>L | SFX | Yes | - | - |
| 46 | CarPassing 4 | SFX | - | - | - |
| 47 | CarPassing 5 | SFX | - | - | - |
| 48 | CarPassing 6 | SFX | - | - | - |
| 49 | Car Crash 1 L>R | SFX | Yes | - | - |
| 50 | Car Crash 1 R>L | SFX | Yes | - | - |
| 51 | Car Crash 2 L>R | SFX | Yes | - | - |
| 52 | Car Crash 2 R>L | SFX | Yes | - | - |
| 53 | Car Crash 3 L>R | SFX | Yes | - | - |
| 54 | Car Crash 3 R>L | SFX | Yes | - | - |
| 55 | Crash 1 | SFX | Yes | - | - |
| 56 | Crash 2 | SFX | Yes | - | - |
| 57 | Crash 3 | SFX | Yes | - | - |
| 58 | Siren 1 | SFX | Yes | - | - |
| 59 | Siren 2 L>R | SFX | Yes | - | - |
| 60 | Siren 2 R>L | SFX | Yes | - | - |
| 61 | Siren 3 | SFX | Yes | - | - |
| 62 | Train 1 | SFX | Yes | - | - |
| 63 | Train 2 | SFX | Yes | - | - |
| 64 | Jetplane 1 L>R | SFX | Yes | - | - |
| 65 | Jetplane 1 R>L | SFX | Yes | - | - |
| 66 | Jetplane 2 L>R | SFX | Yes | - | - |
| 67 | Jetplane 2 R>L | SFX | Yes | - | - |
| 68 | Jetplane 3 L>R | SFX | Yes | - | - |
| 69 | Jetplane 3 R>L | SFX | Yes | - | - |
| 70 | Helicopter 1 L | SFX | Yes | - | - |
| 71 | Helicopter 1 R | SFX | Yes | - | - |
| 72 | Helicopter 2 L | SFX | Yes | - | - |
| 73 | Helicopter 2 R | SFX | Yes | - | - |
| 74 | Helicopter 3 L | SFX | Yes | - | - |
| 75 | Helicopter 3 R | SFX | Yes | - | - |
| 76 | Starship 1 L>R | SFX | Yes | - | - |
| 77 | Starship 1 R>L | SFX | Yes | - | - |
| 78 | Starship 2 L>R | SFX | Yes | - | - |
| 79 | Starship 2 R>L | SFX | Yes | - | - |
| 80 | Starship 3 L>R | SFX | Yes | - | - |
| 81 | Starship 3 R>L | SFX | Yes | - | - |
| 82 | Gun Shot 1 | SFX | Yes | - | - |
| 83 | Gun Shot 2 | SFX | Yes | - | - |
| 84 | Gun Shot 3 | SFX | Yes | - | - |
| 85 | Machine Gun 1 | SFX | Yes | - | - |
| 86 | Machine Gun 2 | SFX | Yes | - | - |
| 87 | Machine Gun 3 | SFX | Yes | - | - |
| 88 | Laser Gun 1 | SFX | Yes | - | - |
| 89 | Laser Gun 2 | SFX | Yes | - | - |
| 90 | Laser Gun 3 | SFX | Yes | - | - |
| 91 | Explosion 1 | SFX | Yes | - | - |
| 92 | Explosion 2 | SFX | Yes | - | - |
| 93 | Explosion 3 | SFX | Yes | - | - |
| 94 | Dog 1 | SFX | Yes | - | - |
| 95 | Dog 2 | SFX | Yes | - | - |
| 96 | Dog 3 | SFX | Yes | - | - |
| 97 | Dog 4 | SFX | Yes | - | - |
| 98 | Horse 1 L>R | SFX | Yes | - | - |
| 99 | Horse 1 R>L | SFX | Yes | - | - |
| 100 | Horse 2 L>R | SFX | Yes | - | - |
| 101 | Horse 2 R>L | SFX | Yes | - | - |
| 102 | Horse 3 L>R | SFX | Yes | - | - |
| 103 | Horse 3 R>L | SFX | Yes | - | - |
| 104 | Birds 1 | SFX | Yes | - | - |
| 105 | Birds 2 | SFX | Yes | - | - |
| 106 | Rain 1 | SFX | Yes | - | - |
| 107 | Rain 2 | SFX | Yes | - | - |
| 108 | Thunder 1 | SFX | Yes | - | - |
| 109 | Thunder 2 | SFX | Yes | - | - |
| 110 | Thunder 3 | SFX | Yes | - | - |
| 111 | Wind | SFX | Yes | - | - |
| 112 | Seashore | SFX | Yes | - | - |
| 113 | Stream 1 | SFX | Yes | - | - |
| 114 | Stream 2 | SFX | Yes | - | - |
| 115 | Bubbles 1 | SFX | Yes | - | - |
| 116 | Bubbles 2 | SFX | Yes | - | - |
| 117 | Burst 1 | SFX | Yes | - | - |
| 118 | Burst 2 | SFX | Yes | - | - |
| 119 | Burst 3 | SFX | Yes | - | - |
| 120 | Burst 4 | SFX | - | - | - |
| 121 | Glass Burst 1 | SFX | Yes | - | - |
| 122 | Glass Burst 2 | SFX | Yes | - | - |
| 123 | Glass Burst 3 | SFX | Yes | - | - |
| 124 | Telephone 1 | SFX | Yes | - | - |
| 125 | Telephone 2 | SFX | Yes | - | - |
| 126 | Telephone 3 | SFX | Yes | - | - |
