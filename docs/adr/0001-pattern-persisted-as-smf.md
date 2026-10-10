# The Sequencer Pattern is persisted as a Standard MIDI File

We save the Sequencer's Pattern as a Standard MIDI File (type 1, PPQ 96, one `MTrk` per Track in Part order), not as a bespoke JSON schema. The same bytes go into `localStorage` (base64) and into `.mid` downloads, any DAW can open them, and the format is ready if we import MIDI files later. Parameter automation fits without a format change, because SMF carries CC and SysEx events natively. Decided in [Where do the Pattern model and scheduler live — Rust core or TS?](https://github.com/sigma/integral/issues/43).

## Consequences

- A Step can only hold what SMF expresses natively: pitch, velocity and note length. Anything else needs an `FF 7F` sequencer-specific meta event, which is a last resort.
- A Track's length is encoded as the time of its End-of-Track event, and Track identity comes from `MTrk` order. The channel nibble is written for DAW compatibility but ignored on load: at play time the channel always comes from the Part's live receive channel.
- Only files we wrote ourselves must round-trip exactly. Importing arbitrary MIDI files is out of scope for now.
