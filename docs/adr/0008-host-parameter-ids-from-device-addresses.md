# Host parameter IDs come from device addresses, relative to the Binding

A DAW saves automation against each host parameter's string ID, so changing an ID breaks every saved project. The DAW plugins therefore derive each ID from where the parameter lives on the device: block kind, byte offset within the block, and element index for array fields (for example `pcms/partial2/0x2A`). An MFX, Chorus or Reverb parameter uses its effect type and parameter index (`mfx/<type>/<index>`, [#347](https://github.com/sigma/integral/issues/347)). The firmware fixes these addresses. Core's field enums already use the byte offset as their discriminant, so the ID needs no new data, and renaming a Rust variant or a display name can't break a project. Decided in [Expose every parameter to the host](https://github.com/sigma/integral/issues/343).

## Considered Options

- **Rust enum variant names.** Rejected: they read well, but a refactor that renames a variant silently orphans saved automation.
- **A hand-kept numeric table.** Rejected: it is a second source of truth beside the field enums.
- **Absolute addresses, Part included.** Rejected for Tone and Kit plugins: changing the [Binding](../../CONTEXT.md) from one Part to another would orphan every lane.

## Consequences

- **Tone and Kit plugin IDs are relative to the Binding.** They carry no Part number, and a key-bound Kit plugin's IDs carry no key number. When the Binding changes, automation follows it to the new target.
- **Studio Set plugin IDs include the Part index** (`part07/level`): a Part is part of what each of those parameters is.
- **nih-plug hashes IDs to 31 bits without checking for collisions** ([#338](https://github.com/sigma/integral/issues/338)), so a unit test hashes every plugin's full list and fails on a collision.
- **A snapshot test of each plugin's (ID, min, max) list** makes any change show up in review. Hosts store normalised values, so correcting a shipped field's range moves its saved automation. Such a correction is a breaking change for the release notes; there is no migration.
- **Order in the host's list is fixed by device address** and grouped by block (Common, Partial 1–4, PMT, MFX per type), whatever MFX type is active.
