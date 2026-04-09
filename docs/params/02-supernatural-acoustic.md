# SuperNATURAL Acoustic Tone (SN-A) Parameters

## Overview

SuperNATURAL Acoustic Tones use advanced modeling to reproduce the behavior of
acoustic instruments. Each SN-A tone has instrument settings (INST) and
multi-effect settings (MFX). The instrument settings include a COMMON tab
(shared across all instruments) and an INST tab (instrument-specific
parameters that vary by instrument type).

---

## COMMON Tab

These parameters apply to all SN-A instruments.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Category | No assign, Ac.Piano, E.Piano, Organ, Other Keyboards, Accordion/Harmonica, Bell/Mallet, Ac.Guitar, E.Guitar, Dist.Guitar, Ac.Bass, E.Bass, Synth Bass, Plucked/Stroke, Strings, Brass, Wind, Flute, Sax, Recorder, Vox/Choir, Synth Lead, Synth Brass, Synth Pad/Strings, Synth Bellpad, Synth PolyKey, FX, Synth Seq/Pop, Phrase, Pulsating, Beat&Groove, Hit, Sound FX, Drums, Percussion, Combination | Selects the category of the tone |
| Phrase Number | 0--87 | Number of the phrase that plays when you press the PREVIEW button |
| Phrase Octave Shift | -3--+3 | Pitch (in one-octave units) of the preview phrase |
| Tone Level | 0--127 | Adjusts the volume of the tone |
| Mono/Poly | MONO, POLY | MONO: only the last-played note will sound. POLY: two or more notes can be played simultaneously. Not available for TW Organ. |
| Octave Shift | -3--+3 | Adjusts the pitch up or down in units of an octave |
| Cutoff Offset | -64--+63 | Adjusts the cutoff frequency offset. Not available for Concert Grand, Honky-tonk, or TW Organ. |
| Resonance Offset | -64--+63 | Adjusts the resonance offset. Not available for Concert Grand, Honky-tonk, or TW Organ. |
| Attack Time Offset | -64--+63 | Adjusts the TVA envelope attack time offset. Not available for Concert Grand, Honky-tonk, or TW Organ. |
| Release Time Offset | -64--+63 | Adjusts the TVA envelope release time offset. Not available for Concert Grand, Honky-tonk, or TW Organ. |
| Portamento Time Offset | -64--+63 | Time over which the pitch will change. Higher values = longer change time. Not available for TW Organ. |
| Vibrato Rate | -64--+63 | Adjusts the vibrato speed. Does not apply to Organ, Bell/Mallet, or Percussion categories. |
| Vibrato Depth | -64--+63 | Adjusts the depth of the vibrato effect. Does not apply to Organ, Bell/Mallet, or Percussion categories. |
| Vibrato Delay | -64--+63 | Adjusts the time delay until vibrato begins. Does not apply to Organ, Bell/Mallet, or Percussion categories. |

---

## INST Tab

### Instrument Bank/Number Selection

| Parameter | Value | Description |
|-----------|-------|-------------|
| INST BANK | INT, ExSN1--ExSN5 | Select the instrument bank (ExSN banks require expansion) |
| INST NUMBER | 001-- | Select the instrument number |

---

## Instrument List

### INT Bank

| Num | Name | Category |
|-----|------|----------|
| 001 | ConcertGrand | Ac.Piano |
| 002 | Grand Piano1 | Ac.Piano |
| 003 | Grand Piano2 | Ac.Piano |
| 004 | Grand Piano3 | Ac.Piano |
| 005 | Mellow Piano | Ac.Piano |
| 006 | Bright Piano | Ac.Piano |
| 007 | UprightPiano | Ac.Piano |
| 008 | Concert Mono | Ac.Piano |
| 009 | Honky-tonk | Ac.Piano |
| 010 | Pure Vintage EP1 | E.Piano |
| 011 | Pure Vintage EP2 | E.Piano |
| 012 | Pure Wurly | E.Piano |
| 013 | Pure Vintage EP3 | E.Piano |
| 014 | Old Hammer EP | E.Piano |
| 015 | Dyno Piano | E.Piano |
| 016 | Clav CB Flat | Other Keyboards |
| 017 | Clav CA Flat | Other Keyboards |
| 018 | Clav CB Medium | Other Keyboards |
| 019 | Clav CA Medium | Other Keyboards |
| 020 | Clav CB Brillia | Other Keyboards |
| 021 | Clav CA Brillia | Other Keyboards |
| 022 | Clav CB Combo | Other Keyboards |
| 023 | Clav CA Combo | Other Keyboards |
| 024 | Glockenspiel | Bell/Mallet |
| 025 | Vibraphone | Bell/Mallet |
| 026 | Marimba | Bell/Mallet |
| 027 | Xylophone | Bell/Mallet |
| 028 | Tubular Bells | Bell/Mallet |
| 029 | TW Organ | Organ |
| 030 | French Accordion | Accordion/Harmonica |
| 031 | Italian Accordion | Accordion/Harmonica |
| 032 | Harmonica | Accordion/Harmonica |
| 033 | Bandoneon | Accordion/Harmonica |
| 034 | Nylon Guitar | Ac.Guitar |
| 035 | Flamenco Guitar | Ac.Guitar |
| 036 | SteelStr Guitar | Ac.Guitar |
| 037 | Jazz Guitar | E.Guitar |
| 038 | ST Guitar Half | E.Guitar |
| 039 | ST Guitar Front | E.Guitar |
| 040 | TC Guitar Rear | E.Guitar |
| 041 | Acoustic Bass | Ac.Bass |
| 042 | Fingered Bass | E.Bass |
| 043 | Picked Bass | E.Bass |
| 044 | Fretless Bass | E.Bass |
| 045 | Violin | Strings |
| 046 | Violin 2 | Strings |
| 047 | Viola | Strings |
| 048 | Cello | Strings |
| 049 | Cello 2 | Strings |
| 050 | Contrabass | Strings |
| 051 | Harp | Plucked/Stroke |
| 052 | Timpani | Percussion |
| 053 | Strings | Strings |
| 054 | Marcato Strings | Strings |
| 055 | London Choir | Vox/Choir |
| 056 | Boys Choir | Vox/Choir |
| 057 | Trumpet | Brass |
| 058 | Trombone | Brass |
| 059 | Tb2 CupMute | Brass |
| 060 | Mute Trumpet | Brass |
| 061 | French Horn | Brass |
| 062 | Soprano Sax 2 | Sax |
| 063 | Alto Sax 2 | Sax |
| 064 | Tenor Sax 2 | Sax |
| 065 | Baritone Sax 2 | Sax |
| 066 | Oboe | Wind |
| 067 | Bassoon | Wind |
| 068 | Clarinet | Wind |
| 069 | Piccolo | Flute |
| 070 | Flute | Flute |
| 071 | Pan Flute | Flute |
| 072 | Shakuhachi | Flute |
| 073 | Sitar | Plucked/Stroke |
| 074 | Uilleann Pipes | Wind |
| 075 | Bag Pipes | Wind |
| 076 | Erhu | Strings |
| 077 | Steel Drums | Percussion |

### ExSN1 Bank

| Num | Name | Category |
|-----|------|----------|
| 001 | Santoor | Bell/Mallet |
| 002 | Yang Chin | Bell/Mallet |
| 003 | Tin Whistle | Flute |
| 004 | Ryuteki | Flute |
| 005 | Tsugaru | Plucked/Stroke |
| 006 | Sansin | Plucked/Stroke |
| 007 | Koto | Plucked/Stroke |
| 008 | Taishou Koto | Plucked/Stroke |
| 009 | Kalimba | Plucked/Stroke |
| 010 | Sarangi | Strings |

### ExSN2 Bank

| Num | Name | Category |
|-----|------|----------|
| 001 | Soprano Sax | Sax |
| 002 | Alto Sax | Sax |
| 003 | Tenor Sax | Sax |
| 004 | Baritone Sax | Sax |
| 005 | English Horn | Wind |
| 006 | Bass Clarinet | Wind |
| 007 | Flute2 | Flute |
| 008 | Soprano Recorder | Recorder |
| 009 | Alto Recorder | Recorder |
| 010 | Tenor Recorder | Recorder |
| 011 | Bass Recorder | Recorder |
| 012 | Ocarina SopC | Recorder |
| 013 | Ocarina SopF | Recorder |
| 014 | Ocarina Alto | Recorder |
| 015 | Ocarina Bass | Recorder |

### ExSN3 Bank

| Num | Name | Category |
|-----|------|----------|
| 001 | TC Guitar w/Fing | Ac.Guitar |
| 002 | 335Guitar w/Fing | Ac.Guitar |
| 003 | LP Guitar Rear | E.Guitar |
| 004 | LP Guitar Front | E.Guitar |
| 005 | 335 Guitar Half | E.Guitar |
| 006 | Acoustic Bass 2 | Ac.Bass |
| 007 | Fingered Bass 2 | E.Bass |
| 008 | Picked Bass 2 | E.Bass |

### ExSN4 Bank

| Num | Name | Category |
|-----|------|----------|
| 001 | Ukulele | Ac.Guitar |
| 002 | Nylon Guitar 2 | Ac.Guitar |
| 003 | 12th Steel Gtr | Ac.Guitar |
| 004 | Mandolin | Ac.Guitar |
| 005 | SteelFing Guitar | Ac.Guitar |
| 006 | SteelStr Guitar2 | Ac.Guitar |

### ExSN5 Bank

| Num | Name | Category |
|-----|------|----------|
| 001 | Classical Trumpet | Brass |
| 002 | Frugal Horn | Brass |
| 003 | Trumpet 2 | Brass |
| 004 | Mariachi Tp | Brass |
| 005 | Trombone 2 | Brass |
| 006 | Bass Trombone | Brass |
| 007 | Tuba | Brass |
| 008 | Straight Mute Tp | Brass |
| 009 | Cup Mute Trumpet | Brass |
| 010 | French Horn 2 | Brass |
| 011 | Mute French Horn | Brass |

---

## Per-Instrument INST Parameters

Each instrument type has its own set of parameters on the INST tab. The
parameters available depend on which instrument is selected.

### Ac.Piano

**Instruments:** INT 001--009 (ConcertGrand through Honky-tonk)

Playing dynamics smoothly change the tone character in a natural way.

| Parameter | Value | Description |
|-----------|-------|-------------|
| String Resonance | 0--127 | When keys are pressed, the strings for already-pressed keys vibrate sympathetically. Increasing the value increases the effect. |
| Key Off Resonance | 0--127 | Adjusts resonances such as key-off sound (subtle sounds heard when you release a key). Higher values increase volume. |
| Hammer Noise | -2, -1, 0, +1, +2 | Adjusts the sound of the hammer striking the string. Higher values increase the hammer sound. |
| StereoWidth | 0--63 | The higher the value, the wider the sound is spread out. |
| Nuance | Type1, Type2, Type3 | Changes the tone's subtle nuances by altering the phase of left/right sounds. Difficult to hear on headphones. No effect for Concert Mono. |
| Tone Character | -5 to +5 | Higher values produce a harder sound; lower values produce a more mellow sound. |

### E.Piano

**Instruments:** INT 010--015 (Pure Vintage EP1 through Dyno Piano)

Playing dynamics smoothly change the tone character. A key-off noise typical
of the instrument is heard when you release the key (PureWurly excepted).

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts the amount of hum noise and key-off noise. Higher settings raise the volume. |

### Other Keyboards (Clav)

**Instruments:** INT 016--023 (Clav CB Flat through Clav CA Combo)

Playing dynamics smoothly change the tone character. A key-off noise typical
of the instrument is heard when you release the key.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts the amount of hum noise and key-off noise. Higher settings raise the volume. |

### Bell/Mallet

**Instruments:** INT 024--028 (Glockenspiel through Tubular Bells), ExSN1 001--002 (Santoor, Yang Chin)

- You can play a roll by operating the Modulation controller (CC01) while playing a note.
- You can produce a glissando effect by operating the pitch bend lever while holding a note, or by playing legato with Portamento SW (CC65) on.
- If Bend Range is set to Tone, pitch bend produces glissando. If Bend Range is not Tone, this effect requires Bend Mode (CC19) on.
- CC18 simulates muting (INT 024--028) or slide roll (ExSN1 001--002).

| Parameter | Value | Description |
|-----------|-------|-------------|
| Mallet Hardness (CC16) | -64--+63 | Adjusts the hardness of the mallet. Higher settings produce a harder mallet sound. |
| Roll Speed (CC17) | -64--+63 | Adjusts the speed of the roll effect. |
| Variation | Refer to Variation table | Performance variation sounds |

### Organ (TW Organ)

**Instruments:** INT 029 (TW Organ)

The sound is unaffected by playing strength. Uses nine harmonic bars to create
the sound, just like a tone wheel organ.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Harmonic Bar 16' | 0--8 | Adjusts the level of 16' footage |
| Harmonic Bar 5-1/3' | 0--8 | Adjusts the level of 5-1/3' footage |
| Harmonic Bar 8' | 0--8 | Adjusts the level of 8' footage (core pitch) |
| Harmonic Bar 4' | 0--8 | Adjusts the level of 4' footage |
| Harmonic Bar 2-2/3' | 0--8 | Adjusts the level of 2-2/3' footage |
| Harmonic Bar 2' | 0--8 | Adjusts the level of 2' footage |
| Harmonic Bar 1-3/5' | 0--8 | Adjusts the level of 1-3/5' footage |
| Harmonic Bar 1-1/3' | 0--8 | Adjusts the level of 1-1/3' footage |
| Harmonic Bar 1' | 0--8 | Adjusts the level of 1' footage. Unavailable if Percussion Switch is on. |
| Leakage Level | 0--127 | Noise level at which the signal of unrelated tone wheels is mixed in |
| Percussion Switch | OFF, ON | If on, a crisp attack is added to the beginning of notes |
| Percussion Soft | NORM, SOFT | NORM: percussion at normal volume, harmonic bars reduced. SOFT: percussion reduced, harmonic bars at normal volume. |
| Percussion Soft Level | 0--15 | Volume of percussion when set to SOFT |
| Percussion Normal Level | 0--15 | Volume of percussion when set to NORM |
| Percussion Slow | FAST, SLOW | FAST: percussion disappears immediately (sharp attack). SLOW: percussion disappears slowly (gentle attack). |
| Percussion Slow Time | 0--127 | Decay time when set to SLOW |
| Percussion Fast Time | 0--127 | Decay time when set to FAST |
| Percussion Harmonic | 2ND, 3RD | 2ND: pitch matches 4' harmonic bar. 3RD: pitch matches 2-2/3' harmonic bar. |
| Percussion Recharge Time | 0--15 | Reproduces analog circuitry characteristics that soften percussion on rapid key presses |
| Percussion Harmonic Bar Level | 0--127 | How much volume is reduced when Percussion Soft is NORM |
| Key On Click Level | 0--31 | Level of key-click when a key is pressed |
| Key Off Click Level | 0--31 | Level of key-click when a key is released |

### Accordion/Harmonica (Accordion)

**Instruments:** INT 030 (French Accordion), INT 031 (Italian Accordion), INT 033 (Bandoneon)

- Keyboard dynamics create volume changes as if using bellows.
- If Bend Range is Tone, pitch bend up produces tremolo effect (moving bellows in small steps).
- If Bend Range is not Tone, Bend Mode (CC19) controls this effect.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts the amount of key noise heard when pressing or releasing a key |

### Accordion/Harmonica (Harmonica)

**Instruments:** INT 032 (Harmonica)

- If Bend Range is Tone, pitch bend up produces a wah effect (as if cupping the harmonica).
- If Bend Range is not Tone, Bend Mode (CC19) controls this effect.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts the amount of breath noise |
| Growl Sens (CC18) | 0--127 | Adjusts the distinctive growl nuance of the breath noise |

### Ac.Guitar

**Instruments:** INT 034--036 (Nylon Guitar, Flamenco Guitar, SteelStr Guitar), ExSN3 001--002, ExSN4 001--003, ExSN4 005--006

- Rapid legato playing within 2 semitones produces slide or hammering-on effects.
- If Strum Mode is off, arpeggios with Hold pedal produce guitar-like arpeggios.
- If Strum Mode is on, chords with Hold pedal produce guitar-like strumming.
- Note numbers 34 and lower produce ghost notes.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts the volume of string grazing or picking noise. No effect on Ukulele. |
| Strum Speed (CC17) | -64--+63 | Adjusts timing deviation in strumming. Higher values = greater deviation. More significant at lower velocities. |
| Strum Mode (CC19) | OFF, ON | When on, playing multiple keys simultaneously produces strumming with alternating up/down strokes. |
| Sub String Tune | -64--+63 | Adjusts pitch of sympathetic strings. Valid only for 12th Steel Gtr. |
| Variation | Refer to Variation table | Performance variation sounds |

### Ac.Guitar (Mandolin)

**Instruments:** ExSN4 004 (Mandolin)

- If Strum Mode is off, arpeggios with Hold pedal produce mandolin-like arpeggios.
- If Strum Mode is on, chords with Hold pedal produce mandolin-like strumming.
- Note numbers 46 and lower produce ghost notes.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts the volume of string grazing or picking noise |
| Tremolo Speed (CC17) | -64--+63 | Adjusts the speed of the tremolo effect |
| Strum Mode (CC19) | OFF, ON | When on, strumming is produced with alternating up/down strokes |
| Variation | Refer to Variation table | Performance variation sounds |

### E.Guitar

**Instruments:** INT 037--040 (Jazz Guitar through TC Guitar Rear), ExSN3 003--005

- Rapid legato playing within 2 semitones produces slide or hammering-on effects.
- If Strum Mode is off, arpeggios with Hold pedal produce guitar-like arpeggios.
- If Strum Mode is on, chords with Hold pedal produce guitar-like strumming.
- Note numbers 34 and lower produce ghost notes.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts the volume of string grazing or picking noise |
| Strum Speed (CC17) | -64--+63 | Adjusts timing deviation in strumming. Higher values = greater deviation. |
| Strum Mode (CC19) | OFF, ON | When on, strumming is produced with alternating up/down strokes |
| Picking Harmonics | OFF, ON | When on, strongly played notes have a picking harmonic effect. No effect on Jazz Guitar. |
| Variation | Refer to Variation table | Performance variation sounds |

### Ac.Bass

**Instruments:** INT 041 (Acoustic Bass), ExSN3 006 (Acoustic Bass 2)

- Rapid legato playing within 2 semitones produces slide or hammering-on effects.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts the volume of string grazing or picking noise |
| Variation | Refer to Variation table | Performance variation sounds |

### E.Bass

**Instruments:** INT 042--044 (Fingered Bass, Picked Bass, Fretless Bass), ExSN3 007--008

- Rapid legato playing within 2 semitones produces slide or hammering-on effects.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts the volume of string grazing or picking noise |
| Variation | Refer to Variation table | Performance variation sounds |

### Strings (Solo)

**Instruments:** INT 045--050 (Violin through Contrabass)

- Playing multiple keys simultaneously limits vibrato so chords sound natural.
- Open-string note ranges produce an open-string sound without vibrato (when Part View Vibrato Depth is 0 for that range).

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts the amount of string grazing noise |
| Variation | Refer to Variation table | Performance variation sounds |

### Strings (Ensemble)

**Instruments:** INT 053 (Strings), INT 054 (Marcato Strings)

- Attack and release adjust automatically for phrase speed (faster = crisper).

| Parameter | Value | Description |
|-----------|-------|-------------|
| Hold Legato Mode (CC19) | OFF, ON | When on, held notes go silent when new keys are played (legato hold behavior) |
| Variation | Refer to Variation table | Performance variation sounds |

### Strings (Erhu)

**Instruments:** INT 076 (Erhu)

- Legato playing produces distinctive ornamental sounds when played strongly.
- Portamento SW on produces erhu-typical portamento.
- Open-string note ranges (up to note 62) produce open-string sound without vibrato.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts the amount of string grazing noise |
| Variation | Refer to Variation table | Performance variation sounds |

### Strings (Sarangi)

**Instruments:** ExSN1 010 (Sarangi)

- Legato playing produces distinctive ornamental sounds when played strongly.
- Portamento SW on produces sarangi-typical portamento.
- CC80 64--127 plays tambura phrase; 0--63 silences it.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Resonance Level (CC16) | -64--+63 | Adjusts sympathetic resonance. Higher = more resonance. |
| Tambura Level | -64--+63 | Adjusts volume of the tambura sound effect (CC80) |
| Tambura Pitch | -12--+12 | Adjusts pitch of the tambura sound effect (CC80) |

### Plucked/Stroke (Harp)

**Instruments:** INT 051 (Harp)

- Glissando Mode (CC19) on causes only notes in a specific scale to sound, enabling idiomatic harp glissando on white keys.
- CC18 simulates hand-damping of strings.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Glissando Mode (CC19) | OFF, ON | When on, glissando on keyboard produces harp string sweep effect |
| Play Scale | 7th, Major, Minor, Hrm-Mi, Dim, Whole | Specifies the scale used when Glissando Mode is on |
| Scale Key | C, Db, D, Eb, E, F, Gb, G, Ab, A, Bb, B | Specifies the key of the glissando scale |
| Variation | Refer to Variation table | Performance variation sounds |

### Plucked/Stroke (Sitar)

**Instruments:** INT 073 (Sitar)

- Strongly playing legato from higher to lower note produces ornamental effects.
- Rapid legato within 2 semitones produces slide effect.
- Note numbers 47 and below produce sitar sound effects.
- CC80 64--127 plays tambura phrase; 0--63 silences it.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Resonance Level (CC16) | -64--+63 | Adjusts sympathetic resonance. Higher = more resonance. |
| Tambura Level | -64--+63 | Adjusts volume of the tambura sound effect (CC80) |
| Tambura Pitch | -12--+12 | Adjusts pitch of the tambura sound effect (CC80) |

### Plucked/Stroke (Shamisen)

**Instruments:** ExSN1 005 (Tsugaru), ExSN1 006 (Sansin)

- Playing strongly produces bend-up effect typical of shamisen.
- Rapid legato within 2 semitones produces slide effect.
- CC81 on produces ghost note on upstroke when key is released.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Resonance Level (CC16) | -64--+63 | Adjusts sympathetic resonance |
| Bend Depth (CC17) | -64--+63 | Adjusts amount of pitch change at attack when playing strongly |
| Buzz Key Switch | OFF, ON | When on, note number 42 and lower produce vocal interjections or sound effects |
| Variation | Refer to Variation table | Performance variation sounds |

### Plucked/Stroke (Koto)

**Instruments:** ExSN1 007 (Koto)

- Glissando Mode (CC19) on causes only notes in the specified scale to sound.
- CC18 simulates hand-damping of strings.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Tremolo Speed (CC17) | -64--+63 | Adjusts speed of tremolo effect controlled by CC80 |
| Glissando Mode (CC19) | OFF, ON | When on, the selected Play Scale applies to glissando |
| Play Scale | Chroma, Hira (Hirajyoshi) | Specifies the scale when Glissando Mode is on |
| Scale Key | C, Db, D, Eb, E, F, Gb, G, Ab, A, Bb, B | Specifies the key for Play Scale |
| Buzz Key Switch | OFF, ON | When on, note number 42 and lower produce vocal interjections or sound effects |
| Variation | Refer to Variation table | Performance variation sounds |

### Plucked/Stroke (Taishou Koto)

**Instruments:** ExSN1 008 (Taishou Koto)

- Operating Modulation (CC01) while playing produces tremolo.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts the level of key-on noise |
| Tremolo Speed (CC17) | -64--+63 | Adjusts the speed of the tremolo effect |

### Plucked/Stroke (Kalimba)

**Instruments:** ExSN1 009 (Kalimba)

Playing dynamics smoothly change the tone character.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Resonance Level (CC16) | -64--+63 | Adjusts sympathetic resonance |
| Variation | Refer to Variation table | Performance variation sounds |

### Vox/Choir

**Instruments:** INT 055 (London Choir), INT 056 (Boys Choir)

Wide range of expression by combining dynamics with variation sounds.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Hold Legato Mode (CC19) | OFF, ON | When on, held notes go silent when new keys are played |
| Variation | Refer to Variation table | Performance variation sounds |

### Brass

**Instruments:** INT 057--061 (Trumpet through French Horn), ExSN5 001--011

- Bend Range set to Tone enables discontinuous pitch changes (up) and falls (down) via pitch bend.
- If Bend Range is not Tone, Bend Mode (CC19) toggles between this effect and conventional pitch bend.
- Legato with Portamento SW on creates trombone-like glissando.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts the amount of breath noise |
| Crescendo Depth (CC17) | -64--+63 | Adjusts automatic crescendo amount. Most noticeable when playing softly. |
| Growl Sens (CC18) | 0--127 | Adjusts the growl nuance when blowing |
| Variation | Refer to Variation table | Performance variation sounds (Mariachi Tp only) |

### Wind

**Instruments:** INT 066--068 (Oboe, Bassoon, Clarinet), ExSN2 005--006 (English Horn, Bass Clarinet)

- Bend Range set to Tone enables glissando (up) and fall (down) via pitch bend.
- If Bend Range is not Tone, Bend Mode (CC19) toggles the effect.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts the amount of breath noise |
| Growl Sens (CC18) | 0--127 | Adjusts the growl nuance when blowing |
| Play Scale | Chroma, Major, Minor, 7th, Dim, Whole | Produces discontinuous pitch changes per specified scale |
| Scale Key | C, Db, D, Eb, E, F, Gb, G, Ab, A, Bb, B | Key of the specified Play Scale |
| Variation | Refer to Variation table | Performance variation sounds |

### Wind (Pipes)

**Instruments:** INT 074 (Uilleann Pipes), INT 075 (Bag Pipes)

- Legato playing produces ornamental sounds when played strongly.
- CC80 64--127 sounds a drone; 0--63 silences it.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Drone Level | -64--+63 | Adjusts volume of the drone sound (CC80) |
| Drone Pitch | -12--+12 | Adjusts pitch of the drone sound (CC80) |
| Variation | Refer to Variation table | Performance variation sounds |

### Flute

**Instruments:** INT 069--071 (Piccolo, Flute, Pan Flute), ExSN2 007 (Flute2)

- Bend Range set to Tone enables glissando (up) and fall (down) via pitch bend.
- Legato playing produces ornamental sounds when played strongly.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts the amount of breath noise |
| Growl Sens (CC18) | 0--127 | Adjusts the growl nuance when blowing |
| Play Scale | Chroma, Major, Minor, 7th, Dim, Whole | Produces discontinuous pitch changes. No effect on Pan Flute. |
| Scale Key | C, Db, D, Eb, E, F, Gb, G, Ab, A, Bb, B | Key of the specified Play Scale. No effect on Pan Flute. |
| Variation | Refer to Variation table | Performance variation sounds |

### Flute (Shakuhachi/Tin Whistle/Ryuteki)

**Instruments:** INT 072 (Shakuhachi), ExSN1 003 (Tin Whistle), ExSN1 004 (Ryuteki)

- Legato playing produces connected notes as if played in a single breath.
- Legato playing produces ornamental sounds when played strongly.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts the amount of breath noise |
| Growl Sens (CC18) | 0--127 | Adjusts the growl nuance when blowing |
| Variation | Refer to Variation table | Performance variation sounds |

### Sax

**Instruments:** INT 062--065 (Soprano Sax 2 through Baritone Sax 2), ExSN2 001--004 (Soprano Sax through Baritone Sax)

- Bend Range set to Tone enables glissando (up) and fall (down) via pitch bend.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts breath noise and key noise |
| Growl Sens (CC18) | 0--127 | Adjusts the growl nuance when blowing |
| Play Scale | Chroma, Major, Minor, 7th, Dim, Whole | Produces discontinuous pitch changes per specified scale |
| Scale Key | C, Db, D, Eb, E, F, Gb, G, Ab, A, Bb, B | Key of the specified Play Scale |
| Glide | Porta, Gliss | Whether portamento or glissando is applied when portamento switch is on |
| Variation | Refer to Variation table | Performance variation sounds |

### Recorder

**Instruments:** ExSN2 008--011 (Soprano Recorder through Bass Recorder)

Legato playing produces smoothly connected notes as if played in a single breath.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts the amount of breath noise |
| Growl Sens (CC18) | 0--127 | Adjusts the growl nuance when blowing |
| Variation | Refer to Variation table | Performance variation sounds |

### Recorder (Ocarina)

**Instruments:** ExSN2 012--015 (Ocarina SopC through Ocarina Bass)

- Legato playing produces connected notes as if played in a single breath.
- Legato playing produces ornamental sounds when played strongly.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Noise Level (CC16) | -64--+63 | Adjusts the amount of breath noise |
| Growl Sens (CC18) | 0--127 | Adjusts the growl nuance when blowing |
| Variation | Refer to Variation table | Performance variation sounds |

### Percussion (Timpani)

**Instruments:** INT 052 (Timpani)

- Modulation (CC01) while playing produces a roll.
- CC18 simulates hand-muting the timpani.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Roll Speed (CC17) | -64--+63 | Adjusts the speed of the roll effect |
| Variation | Refer to Variation table | Performance variation sounds |

### Percussion (Steel Drums)

**Instruments:** INT 077 (Steel Drums)

- Modulation (CC01) while playing produces a roll.
- Pitch bend or legato with Portamento SW on produces glissando.
- CC18 simulates hand/mallet muting.

| Parameter | Value | Description |
|-----------|-------|-------------|
| Resonance Level (CC16) | -64--+63 | Adjusts sympathetic resonance |
| Roll Speed (CC17) | -64--+63 | Adjusts the speed of the roll effect |
| Variation | Refer to Variation table | Performance variation sounds |

---

## Performance Variation Sounds

Each instrument supports up to 4 variation sounds, selected via the Variation
parameter. "-" means no variation is available for that slot.

| Bank | Num | Name | Var 1 | Var 2 | Var 3 | Var 4 |
|------|-----|------|-------|-------|-------|-------|
| INT | 001 | ConcertGrand | - | - | - | - |
| INT | 002 | Grand Piano1 | - | - | - | - |
| INT | 003 | Grand Piano2 | - | - | - | - |
| INT | 004 | Grand Piano3 | - | - | - | - |
| INT | 005 | Mellow Piano | - | - | - | - |
| INT | 006 | Bright Piano | - | - | - | - |
| INT | 007 | UprightPiano | - | - | - | - |
| INT | 008 | Concert Mono | - | - | - | - |
| INT | 009 | Honky-tonk | - | - | - | - |
| INT | 010 | Pure Vintage EP1 | - | - | - | - |
| INT | 011 | Pure Vintage EP2 | - | - | - | - |
| INT | 012 | Pure Wurly | - | - | - | - |
| INT | 013 | Pure Vintage EP3 | - | - | - | - |
| INT | 014 | Old Hammer EP | - | - | - | - |
| INT | 015 | Dyno Piano | - | - | - | - |
| INT | 016 | Clav CB Flat | - | - | - | - |
| INT | 017 | Clav CA Flat | - | - | - | - |
| INT | 018 | Clav CB Medium | - | - | - | - |
| INT | 019 | Clav CA Medium | - | - | - | - |
| INT | 020 | Clav CB Brillia | - | - | - | - |
| INT | 021 | Clav CA Brillia | - | - | - | - |
| INT | 022 | Clav CB Combo | - | - | - | - |
| INT | 023 | Clav CA Combo | - | - | - | - |
| INT | 024 | Glockenspiel | Dead Stroke | - | - | - |
| INT | 025 | Vibraphone | Dead Stroke | Tremolo Sw | - | - |
| INT | 026 | Marimba | Dead Stroke | - | - | - |
| INT | 027 | Xylophone | Dead Stroke | - | - | - |
| INT | 028 | Tubular Bells | Dead Stroke | - | - | - |
| INT | 029 | TW Organ | - | - | - | - |
| INT | 030 | French Accordion | - | - | - | - |
| INT | 031 | Italian Accordion | - | - | - | - |
| INT | 032 | Harmonica | - | - | - | - |
| INT | 033 | Bandoneon | - | - | - | - |
| INT | 034 | Nylon Guitar | Mute | Harmonics | - | - |
| INT | 035 | Flamenco Guitar | Rasugueado | Harmonics | - | - |
| INT | 036 | SteelStr Guitar | Mute | Harmonics | - | - |
| INT | 037 | Jazz Guitar | FingerPicking | Octave Tone | - | - |
| INT | 038 | ST Guitar Half | Mute | Harmonics | - | - |
| INT | 039 | ST Guitar Front | Mute | Harmonics | - | - |
| INT | 040 | TC Guitar Rear | Mute | Harmonics | - | - |
| INT | 041 | Acoustic Bass | Staccato | Harmonics | - | - |
| INT | 042 | Fingered Bass | Slap | Harmonics | - | - |
| INT | 043 | Picked Bass | Bridge Mute | Harmonics | - | - |
| INT | 044 | Fretless Bass | Staccato | Harmonics | - | - |
| INT | 045 | Violin | Staccato | Pizzicato | Tremolo | - |
| INT | 046 | Violin 2 | Staccato | Pizzicato | Tremolo | - |
| INT | 047 | Viola | Staccato | Pizzicato | Tremolo | - |
| INT | 048 | Cello | Staccato | Pizzicato | Tremolo | - |
| INT | 049 | Cello 2 | Staccato | Pizzicato | Tremolo | - |
| INT | 050 | Contrabass | Staccato | Pizzicato | Tremolo | - |
| INT | 051 | Harp | Nail | - | - | - |
| INT | 052 | Timpani | Flam | Accent Roll | - | - |
| INT | 053 | Strings | Staccato | Pizzicato | Tremolo | - |
| INT | 054 | Marcato Strings | Staccato | Pizzicato | Tremolo | - |
| INT | 055 | London Choir | Voice Woo | - | - | - |
| INT | 056 | Boys Choir | Voice Woo | - | - | - |
| INT | 057 | Trumpet | Staccato | Fall | - | - |
| INT | 058 | Trombone | Staccato | Fall | - | - |
| INT | 059 | Tb2 CupMute | Staccato | Fall | - | - |
| INT | 060 | Mute Trumpet | Staccato | Fall | - | - |
| INT | 061 | French Horn | Staccato | - | - | - |
| INT | 062 | Soprano Sax 2 | Staccato | Fall | SubTone | - |
| INT | 063 | Alto Sax 2 | Staccato | Fall | SubTone | - |
| INT | 064 | Tenor Sax 2 | Staccato | Fall | SubTone | - |
| INT | 065 | Baritone Sax 2 | Staccato | Fall | SubTone | - |
| INT | 066 | Oboe | Staccato | - | - | - |
| INT | 067 | Bassoon | Staccato | - | - | - |
| INT | 068 | Clarinet | Staccato | - | - | - |
| INT | 069 | Piccolo | Staccato | - | - | - |
| INT | 070 | Flute | Staccato | - | - | - |
| INT | 071 | Pan Flute | Staccato | Flutter | - | - |
| INT | 072 | Shakuhachi | Staccato | Ornament | - | - |
| INT | 073 | Sitar | - | - | - | - |
| INT | 074 | Uilleann Pipes | - | Ornament | - | - |
| INT | 075 | Bag Pipes | - | Ornament | - | - |
| INT | 076 | Erhu | Staccato | Ornament | - | - |
| INT | 077 | Steel Drums | Mute | - | - | - |
| ExSN1 | 001 | Santoor | Mute | Tremolo | - | - |
| ExSN1 | 002 | Yang Chin | Mute | Tremolo | - | - |
| ExSN1 | 003 | Tin Whistle | Cut | Ornament | - | - |
| ExSN1 | 004 | Ryuteki | Staccato | Ornament | - | - |
| ExSN1 | 005 | Tsugaru | Strum | Up Picking | Auto Bend | - |
| ExSN1 | 006 | Sansin | Strum | Up Picking | Auto Bend | - |
| ExSN1 | 007 | Koto | Tremolo | Ornament | - | - |
| ExSN1 | 008 | Taishou Koto | - | - | - | - |
| ExSN1 | 009 | Kalimba | Buzz | - | - | - |
| ExSN1 | 010 | Sarangi | - | - | - | - |
| ExSN2 | 001 | Soprano Sax | Staccato | Fall | SubTone | - |
| ExSN2 | 002 | Alto Sax | Staccato | Fall | SubTone | - |
| ExSN2 | 003 | Tenor Sax | Staccato | Fall | SubTone | - |
| ExSN2 | 004 | Baritone Sax | Staccato | Fall | SubTone | - |
| ExSN2 | 005 | English Horn | Staccato | - | - | - |
| ExSN2 | 006 | Bass Clarinet | Staccato | - | - | - |
| ExSN2 | 007 | Flute2 | Staccato | - | - | - |
| ExSN2 | 008 | Soprano Recorder | Staccato | - | - | - |
| ExSN2 | 009 | Alto Recorder | Staccato | - | - | - |
| ExSN2 | 010 | Tenor Recorder | Staccato | - | - | - |
| ExSN2 | 011 | Bass Recorder | Staccato | - | - | - |
| ExSN2 | 012 | Ocarina SopC | Staccato | Ornament | - | - |
| ExSN2 | 013 | Ocarina SopF | Staccato | Ornament | - | - |
| ExSN2 | 014 | Ocarina Alto | Staccato | Ornament | - | - |
| ExSN2 | 015 | Ocarina Bass | Staccato | Ornament | - | - |
| ExSN3 | 001 | TC Guitar w/Fing | FingerPicking | Octave Tone | - | - |
| ExSN3 | 002 | 335Guitar w/Fing | FingerPicking | Octave Tone | - | - |
| ExSN3 | 003 | LP Guitar Rear | Mute | Harmonics | - | - |
| ExSN3 | 004 | LP Guitar Front | Mute | Harmonics | - | - |
| ExSN3 | 005 | 335 Guitar Half | Mute | Harmonics | - | - |
| ExSN3 | 006 | Acoustic Bass 2 | Staccato | Harmonics | - | - |
| ExSN3 | 007 | Fingered Bass 2 | Slap | Harmonics | - | - |
| ExSN3 | 008 | Picked Bass 2 | Bridge Mute | Harmonics | - | - |
| ExSN4 | 001 | Ukulele | - | - | - | - |
| ExSN4 | 002 | Nylon Guitar 2 | Mute | Harmonics | - | - |
| ExSN4 | 003 | 12th Steel Gtr | Mute | Harmonics | - | - |
| ExSN4 | 004 | Mandolin | Mute | Harmonics | - | - |
| ExSN4 | 005 | SteelFing Guitar | FingerPicking | Octave Tone | - | - |
| ExSN4 | 006 | SteelStr Guitar2 | Mute | Harmonics | - | - |
| ExSN5 | 001 | Classical Trumpet | Staccato | Fall | - | - |
| ExSN5 | 002 | Frugal Horn | Staccato | Fall | - | - |
| ExSN5 | 003 | Trumpet 2 | Staccato | Fall | - | - |
| ExSN5 | 004 | Mariachi Tp | Staccato | Fall | - | - |
| ExSN5 | 005 | Trombone 2 | Staccato | Fall | - | - |
| ExSN5 | 006 | Bass Trombone | Staccato | Fall | - | - |
| ExSN5 | 007 | Tuba | Staccato | - | - | - |
| ExSN5 | 008 | Straight Mute Tp | Staccato | Fall | - | - |
| ExSN5 | 009 | Cup Mute Trumpet | Staccato | Fall | - | - |
| ExSN5 | 010 | French Horn 2 | Staccato | - | - | - |
| ExSN5 | 011 | Mute French Horn | Staccato | - | - | - |

---

## MFX Tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| MFX Switch | OFF, ON | Switches the multi-effect on/off |
| MFX Type | 0--67 | Selects from 67 available MFX types (see MFX Parameters) |
| (MFX type parameters) | (varies) | Edit parameters for the selected MFX type |
| MFX Chorus Send Level | 0--127 | Amount of chorus for MFX output. No effect if motional surround is on. |
| MFX Reverb Send Level | 0--127 | Amount of reverb for MFX output. No effect if motional surround is on. |

## MFX CTRL Tab

| Parameter | Value | Description |
|-----------|-------|-------------|
| Source (1--4) | OFF, CC01--31, CC33--95, PITCH BEND, AFTERTOUCH, SYS CTRL1--4 | MIDI message used to control MFX parameters |
| Destination (1--4) | (depends on MFX type) | MFX parameter to be controlled |
| Sens (1--4) | -63--+63 | Amount of control effect. Positive = increase, negative = decrease. 0 = no effect. |
