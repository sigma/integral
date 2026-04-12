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

### How to use these docs together

When implementing a feature in the control surface, start here for context,
then follow the cross-reference links to the specific technical docs:

| You want to... | Start here | Then go to |
|----------------|-----------|------------|
| Understand the signal flow | [01-architecture](01-architecture.md) | [midi/05-studio-set](../midi/05-studio-set.md) for addresses |
| Select/switch tones | [02-tone-types](02-tone-types.md) | [midi/03-bank-select-tables](../midi/03-bank-select-tables.md) for MSB/LSB/PC |
| Configure effects | [03-effects](03-effects.md) | [midi/11-fx-parameters](../midi/11-fx-parameters.md) for type-dependent params, [params/07-mfx-types](../params/07-mfx-types.md) for all 67 MFX types |
| Edit a studio set | [04-studio-sets](04-studio-sets.md) | [midi/05-studio-set](../midi/05-studio-set.md) for part addresses, [params/01-studio-set](../params/01-studio-set.md) for param meanings |
| Edit an SN-A tone | [02-tone-types](02-tone-types.md#supernatural-acoustic-tone-sn-a) | [params/02-supernatural-acoustic](../params/02-supernatural-acoustic.md) for instrument list, [midi/09-supernatural-acoustic-tone](../midi/09-supernatural-acoustic-tone.md) for addresses |
| Edit an SN-S tone | [02-tone-types](02-tone-types.md#supernatural-synth-tone-sn-s) | [params/03-supernatural-synth](../params/03-supernatural-synth.md) for OSC/filter params, [midi/08-supernatural-synth-tone](../midi/08-supernatural-synth-tone.md) for addresses |
| Edit a PCMS tone | [02-tone-types](02-tone-types.md#pcm-synth-tone-pcms) | [params/05-pcm-synth-tone](../params/05-pcm-synth-tone.md) for wave/TVF/TVA params, [midi/06-pcm-synth-tone](../midi/06-pcm-synth-tone.md) for addresses |
| Send MIDI CC | [06-specifications](06-specifications.md#notable-cc-assignments) | [midi/02-channel-messages](../midi/02-channel-messages.md) for full CC reference |
| Query tone/set names | [05-operational](05-operational.md) | [midi/99-undocumented](../midi/99-undocumented.md) for catalog query protocol |

### Key behavioral insights from the manual

The manual docs capture several important behavioral constraints that are
not obvious from the raw parameter tables in `docs/midi/` and `docs/params/`:

1. **Motional Surround exclusivity** ([03-effects](03-effects.md#motional-surround-interaction)):
   When Motional Surround is ON, Chorus and Reverb are disabled, Pan is
   ignored, and Output Assign is overridden. A control surface must
   disable/hide those controls when surround is active.

2. **Temporary Area semantics** ([01-architecture](01-architecture.md#important-behavior)):
   All editing happens in the Temporary Area. Changes are lost on power-off
   or studio set change unless explicitly written to User Memory. The control
   surface should warn users about unsaved changes.

3. **COMP+EQ single-part restriction** ([03-effects](03-effects.md#compeq-drum-compressor--equalizer)):
   Although drum kits can be on multiple parts, COMP+EQ processing only
   applies to the single designated Drum COMP+EQ Assign Part. The SysEx
   parameter is at Studio Set Common offset `00 44`.

4. **Phase Lock latency** ([04-studio-sets](04-studio-sets.md#phase-lock-tab)):
   Enabling Phase Lock introduces audible latency. It is also not available
   for SN-A organ-type instruments.

5. **Expansion slot volatility** ([01-architecture](01-architecture.md#important-behavior)):
   Expansion data in virtual slots is volatile and reloaded on startup.
   Tones referencing unloaded expansions will not be audible.

6. **Voice Reserve cap** ([04-studio-sets](04-studio-sets.md#voice-reserve)):
   Despite 128-voice max polyphony, Voice Reserve per part caps at 64
   (FULL), and the total across all parts cannot exceed 64.

7. **SN-A Mono/Poly/Legato availability varies by instrument**
   ([params/01-studio-set](../params/01-studio-set.md)): Organ-type SN-A
   instruments do not support Mono/Poly or Legato settings at all.

8. **CC-to-offset parameter mapping** ([06-specifications](06-specifications.md#notable-cc-assignments)):
   CCs 71-78 map directly to Part offset parameters (Cutoff, Resonance,
   Attack, Decay, Release, Vibrato Rate/Depth/Delay). These are relative
   changes centered at 64, not absolute values.

### Contradictions and clarifications

No significant contradictions were found between the manual, MIDI
implementation, and parameter guide docs. Minor clarifications:

- The manual says Voice Reserve total should "not exceed 128" but the SysEx
  range per part is 0-64 (FULL) and the parameter guide says total cannot
  exceed 64. The SysEx constraint is authoritative.
- The manual refers to "3 types" of Chorus. The MIDI implementation lists 4
  values (0=OFF, 1=Chorus, 2=Delay, 3=GM2 Chorus). OFF is a type selector
  value, not a distinct effect type -- both are correct depending on how
  you count.
