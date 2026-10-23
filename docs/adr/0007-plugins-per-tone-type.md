# The DAW plugin family: one Studio Set plugin, plus a plugin per tone type

The DAW integration ships a **Studio Set plugin** for what the whole Studio Set shares, a **Tone plugin** per synth tone type (SN Synth, SN Acoustic, PCM Synth) bound to one Part, and a **Kit plugin** per drum kit type (SN Drum, PCM Drum) bound to one Part and either a key or the whole kit. There are no Part plugins that change with the tone type, no per-effect plugins, and no MFX outside its sound. Decided in [Which plugins make up the family?](https://github.com/sigma/integral/issues/342). No vendor ships anything finer than one instance per device ([#337](https://github.com/sigma/integral/issues/337)), so this has no precedent.

## Considered Options

- **One Part plugin for every tone type.** Rejected: a plugin's parameter list is fixed when it is built ([#338](https://github.com/sigma/integral/issues/338)). It would need the union of every type, about 1,000 parameters, most of them inert for any one tone. A drum kit would push it past 12,000.
- **Effect plugins for MFX, Chorus and Reverb.** Rejected: the device mixes them in hardware and returns them only on its analog outputs ([#340](https://github.com/sigma/integral/issues/340)), so they never sit in the DAW's signal path. Each MFX is an insert inside its Part and has no output of its own.
- **One Drum Key plugin besides a Kit plugin.** Rejected for one Kit plugin whose binding chooses the role, so a kit type is one plugin, not two.

## Consequences

- **Parameter lists per type:** SN Synth 233, SN Acoustic 96, PCM Synth 705; a PCM Drum key 143. The Studio Set plugin is about 1,300. A kit-bound Kit plugin leaves its key parameters inert, and a key-bound one leaves its kit-wide parameters inert.
- **The Studio Set plugin owns the mixer** (every Part's level, pan, sends, Part EQ, Output Assign, mute, solo and receive channel), Ext, Chorus, Reverb, the Master EQ, Motional Surround and the Drum COMP+EQ assignment and outputs. These are the web UI's Mixer and Surround pages.
- **The Tone or Kit plugin owns which tone its Part plays:** it picks among tones of its type. The Studio Set plugin shows each Part's tone but doesn't change it.
- **Binding is explicit:** chosen in the plugin and saved in the project. A new instance defaults to the first target the Device service sees as unbound. Duplicates work with a warning ([ADR 0006](0006-device-service-owns-the-link.md)).
- **Type mismatch:** when the bound Part holds another type, the plugin shows the mismatch and stays inert, offering to load its type's default tone. Its automation lanes survive and resume once the type matches.
- **Placement:** Tone and Kit plugins are pass-through audio effects. In Ableton Live a Tone plugin follows External Instrument on the Part's track, which sends notes to the Device service's port. A key-bound Kit plugin sits in a Drum Rack pad chain. The Studio Set plugin sits on the master track. The [#345](https://github.com/sigma/integral/issues/345) spike tests this.
