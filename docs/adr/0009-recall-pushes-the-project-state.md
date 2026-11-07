# Recall pushes the project's state; the device never wins

When a DAW project opens, each plugin instance pushes its saved state to the INTEGRA-7 through the [Device service](0006-device-service-owns-the-link.md). Nothing is compared or merged first, so an edit made on the device outside the DAW is lost. The Studio Set is recalled by the Studio Set plugin pushing its own blocks into the temporary Studio Set, never by loading a stored Studio Set by number. Decided in [Recall device state from the DAW project](https://github.com/sigma/integral/issues/344).

## Considered Options

- **The device wins.** Rejected: the point of recall is that reopening a project brings its sound back.
- **Read back, diff, then ask.** Rejected: it costs an RQ1 round trip per block within a 25 DT1/s budget, and the prompt would repeat across up to 16 instances. The device sends DT1 only in reply to RQ1, so without polling the service can't tell that the device changed anyway ([#343](https://github.com/sigma/integral/issues/343)).
- **Load the Studio Set by number.** Rejected: it overwrites every Part in bulk, so it races the Tone and Kit pushes, and the stored set may have changed since the project was saved.
- **A tone by reference (Bank Select and PC) only.** Rejected: unsaved tone edits would be lost, and a User slot may hold another tone by now.

## Consequences

- **Tone and Kit plugins save the tone selection and its name** beside their host parameters. On recall they send the selection first, then the tone's values. Because the saved type is restored with the tone, a type mismatch can't arise on recall.
- **Preset and Expansion tones push only their differences.** Those tones never change, so the tone as the device reported it right after selection is a reliable baseline. The service keeps that snapshot. Recall sends the selection, then one DT1 per run of adjacent fields that differ from the baseline. An untouched preset is one message instead of about 13. When the diff would take more DT1s than the full blocks, the plugin pushes the full blocks instead.
- **User tones always push in full.** A User slot can be overwritten after the project is saved, so a diff against it would give the wrong sound. The Studio Set plugin also pushes in full: it never loads a set by number, so it has no baseline.
- **Each byte has one owner.** The Studio Set plugin pushes Common, Chorus, Reverb, Master EQ, Motional Surround, each Part's mixer fields, Part EQ and Drum COMP+EQ, but never the Part tone-select bytes (`00 06`–`00 08`). Only the Tone or Kit plugin bound to a Part sends those. Pushes from different instances therefore don't depend on order.
- **What a project doesn't cover stays as the device holds it.** A project with no Studio Set plugin leaves the Studio Set alone, and a Part with no Tone or Kit plugin keeps its tone.
- **A fresh instance adopts the device.** With no saved state it reads its bound blocks and sets its parameters silently. Dropping a plugin onto a track never changes the sound.
- **Push on load and on every new device session.** The service gives each link session an ID; it changes when the link drops, the device power-cycles or the service restarts. An instance with saved state pushes once per session ID. A socket drop within one session pushes nothing.
- **The push sends current values**, read when it is queued, not the snapshot as saved: automation the host applied or a knob moved while offline is included.
- **Pacing.** A full PCM Synth tone is about 13 DT1s (0.5 s at 40 ms each), so 16 such Parts pushed in full take about 8 s, and the Studio Set about 1.6 s more. Untouched presets bring 16 Parts under a second once the service has read each preset before (it keeps what it read for the life of the process). Cold, every distinct preset is read back first, a block at a time in its client's turns: about 2 s until the first write goes out and about 7 s until all 16 baselines exist. The service serves clients round-robin, so every Part fills in progressively. A write queued after a push always lands after it.
- **No gating.** Notes and transport run during recall, and a Part may sound wrong for its first seconds. Each instance shows "Recalling…" until its push has gone out, and "Device offline" while it waits for a session.
- **Duplicate Bindings:** both instances push; the later one wins. They already warn ([ADR 0006](0006-device-service-owns-the-link.md)).
