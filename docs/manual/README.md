# INTEGRA-7 Owner's Manual Reference

Extracted from the official Roland INTEGRA-7 Owner's Manual. This supplements
`docs/midi/` (SysEx protocol/addresses) and `docs/params/` (parameter tables)
with architectural, operational, and reference information useful for building
a control surface.

## Contents

| File | Description |
|------|-------------|
| [01-architecture.md](01-architecture.md) | Signal flow, tone/part/studio-set hierarchy, memory model |
| [02-tone-types.md](02-tone-types.md) | Tone type details: SN-A, SN-S, SN-D, PCMS, PCMD, expansion sounds |
| [03-effects.md](03-effects.md) | Effects routing: MFX, Chorus, Reverb, COMP+EQ, Master EQ, Motional Surround interaction |
| [04-studio-sets.md](04-studio-sets.md) | Studio Set structure, part configuration, voice reserve, keyboard layering |
| [05-operational.md](05-operational.md) | Preview phrases, tone categories, USB audio, MIDI signal flow, bulk dump, GM2 mode |
| [06-specifications.md](06-specifications.md) | Technical specs, error messages, troubleshooting reference |

## Relationship to other docs

- **docs/midi/** = SysEx addresses, byte layouts, protocol rules (the *how*)
- **docs/params/** = parameter names, ranges, musical descriptions (the *what*)
- **docs/manual/** = architecture, signal flow, operational behavior (the *why*)
