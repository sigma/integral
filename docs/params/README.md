# INTEGRA-7 Parameter Guide Reference

Extracted from the official Roland INTEGRA-7 Parameter Guide (e01_W).

This directory documents the **musical meaning** of every parameter exposed
by the INTEGRA-7 UI — value ranges, descriptions, and dependencies between
parameters.  It complements `docs/midi/` which covers the SysEx protocol
and address map.

## Contents

| File | Description |
|------|-------------|
| [01-studio-set.md](01-studio-set.md) | Studio Set: Common, Part View (all 8 tabs), Motional Surround, Effects Routing, Studio Set Effects |
| [02-supernatural-acoustic.md](02-supernatural-acoustic.md) | SN-A Tone: Common, per-instrument parameters (20+ categories), full instrument list, performance variations |
| [03-supernatural-synth.md](03-supernatural-synth.md) | SN-S Tone: Common, OSC, Pitch, Filter, Amp, LFO, Mod LFO, Aftertouch, Misc |
| [04-supernatural-drum.md](04-supernatural-drum.md) | SN-D Kit: Common, per-instrument params, Comp/EQ, full drum inst list (INT + ExSN6) |
| [05-pcm-synth-tone.md](05-pcm-synth-tone.md) | PCM Synth Tone: Common, Wave, PMT, Pitch/Env, TVF/Env, TVA/Env, Output, LFO, Step LFO, Ctrl, Matrix Ctrl |
| [06-pcm-drum-kit.md](06-pcm-drum-kit.md) | PCM Drum Kit: Common, Wave (x4 WMT), Pitch/Env, TVF/Env, TVA/Env, Output, Comp/EQ (x6) |
| [07-mfx-types.md](07-mfx-types.md) | All 68 MFX types with per-type parameter tables |

## Relationship to docs/midi/

- **docs/midi/** = SysEx addresses, byte layouts, protocol rules (the *how*)
- **docs/params/** = parameter names, ranges, musical descriptions (the *what*)

When implementing a control surface parameter, consult both:
1. `docs/params/` to understand what the parameter does and its value range
2. `docs/midi/` to find the SysEx address and encoding

## Tone Types

| Abbreviation | Full Name | Key Characteristics |
|-------------|-----------|-------------------|
| SN-A | SuperNATURAL Acoustic | Instrument-specific parameters, behavior modeling, variation sounds |
| SN-S | SuperNATURAL Synth | 3-partial analog-style synth, 8 waveform types, ring mod |
| SN-D | SuperNATURAL Drum | Per-key instrument assignment, ambience, stereo width |
| PCMS | PCM Synth Tone | 4-partial sample-based, WMT, TVF/TVA envelopes, matrix control |
| PCMD | PCM Drum Kit | 88 keys, 4 WMT layers per key, per-key comp/EQ |
