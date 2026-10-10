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
Writing notes into the Step under the Cursor by playing them on the Keyboard
while the Sequencer is armed.
_Avoid_: Step recording

**Cursor**:
The Step where Step input writes next. Its Track is always the selected Part's
Track.
_Avoid_: Playhead (that is where playback is)

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

## Librarian

**Pack**:
One file holding any mix of Studio Sets and tones of any type. Exporting a
single tone gives a Pack of one. Exporting a Studio Set also carries the User
tones its Parts use.
_Avoid_: Bank (that is Bank Select), tone set, library

**User slot**:
One numbered place in the device's user memory for a tone of one type or a
Studio Set. A range of User slots is a way to pick what goes into a Pack, not
a thing of its own.
_Avoid_: Patch, program

**Free slot**:
A User slot whose name is its type's INIT name. An import fills free slots
first and never overwrites another slot without the user choosing it.
_Avoid_: Empty slot (a slot always holds something)
