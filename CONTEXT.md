# Integral

Control surface for the Roland INTEGRA-7. This glossary fixes the words used
across code, issues and docs.

## Sequencer

**Sequencer**:
The built-in pattern player of the web UI. It drives the device's parts from
an internal clock and keeps playing across page and tone changes.
_Avoid_: Arpeggiator, looper

**Track**:
One row of the Pattern. Track N is fixed to Part N and plays on that Part's
receive channel.
_Avoid_: Lane, channel

**Step**:
One 16th-note slot in a Track. It holds zero or more notes (a chord).
_Avoid_: Tick, beat

**Pattern**:
All Tracks together, each up to 64 Steps long with its own length.
_Avoid_: Sequence, song, clip

**Step input**:
Writing notes into the Step under the cursor by playing them on the Keyboard.
_Avoid_: Step recording

**Live play**:
Playing the Keyboard to sound the selected Part without writing to the
Pattern.
_Avoid_: Jamming, preview

**Keyboard**:
The note source of the web UI: the on-screen piano and the computer-keyboard
note mapping, treated as one.
_Avoid_: Piano, controller

**Phrase preview**:
The device's own built-in demo phrase for a Part, started by SysEx. Unrelated
to the Sequencer.
