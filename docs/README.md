# docs

## Overview

Design and reference documentation for Integral, the INTEGRA-7 control
surface. It holds the extracted Roland MIDI Implementation, device data
references, architecture decisions, specs, agent workflow notes and research.
The only code under `docs/` is two research scripts in `research/`; everything
else is Markdown and data.

## Subdirectories & Boundaries

- research: [./research/README.md](./research/README.md) - Research notes and
  tables behind design choices and data maps. Holds `mfx-param-gen.mjs`
  (generates the MFX parameter tables in `crates/integral-core/src/mfx_params.rs`)
  and `xv-mfx-fit-check.py` (vets XV MFX 41-90 layout mappings against a
  SysEx corpus).
- midi: [./midi/README.md](./midi/README.md) - INTEGRA-7 MIDI Implementation
  v1.00, extracted. The authority for SysEx addresses, checksums and bank
  select.
- params: [./params/README.md](./params/README.md) - Parameter Guide (e01_W) extract: musical meaning,
  ranges and dependencies of each parameter.
- svd: [./svd/README.md](./svd/README.md) - Reverse-engineered `.SVD` backup file format.
- manual: [./manual/README.md](./manual/README.md) - Owner's Manual extract: architecture and
  operation.
- sounds: [./sounds/README.md](./sounds/README.md) - Factory preset tone lists (JSON), from the Sound
  List.
- waves: [./waves/README.md](./waves/README.md) - Wave name lists (JSON), one per Wave Group.
- sn-acoustic-insts: [./sn-acoustic-insts/README.md](./sn-acoustic-insts/README.md) -
  SN Acoustic instrument list (`insts.json`).
- sn-drum-insts: [./sn-drum-insts/README.md](./sn-drum-insts/README.md) -
  SN Drum Kit Inst list (`insts.json`).
- adr: `./adr/` - Architecture Decision Records (`NNNN-title.md`).
- specs: `./specs/` - Feature specs (e.g. `wireless-transport.md`).
- design: `./design/` - Design notes (e.g. `output-routing.md`).
- agents: `./agents/` - Agent workflow: issue tracker, triage labels, domain
  docs.
