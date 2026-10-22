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
tones its Parts use. A `.syx` dump of one tone or one Studio Set reads as a
Pack of one.
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

**Foreign patch**:
A Patch or Rhythm Set dumped by another Roland synth (JV, XP or XV series)
that the Librarian converts into a PCM Synth tone or a PCM Drum Kit. Importing
Foreign patches yields an ordinary Pack.
_Avoid_: Legacy patch, external patch (external reads as Expansion)

**Conversion report**:
The per-tone list of everything a Foreign patch conversion dropped or
approximated: unmapped Waves, effect types, controller routings, parameters.
_Avoid_: Log, warnings

**Clean**:
A converted tone whose Conversion report holds nothing but settings the
Studio Set restores (Tempo, Chorus, Reverb, outputs: `NotApplied` items).
Those items are still listed.

**Lossy**:
A converted tone whose Conversion report holds anything else: a Wave, effect,
routing or parameter the INTEGRA-7 tone lost. Counted by those items only.

## Expansions

**Expansion**:
One of the sound libraries stored inside the device (SRX-01 to SRX-12, ExSN1
to ExSN6, ExPCM). Its tones and waves play only while it is loaded.
_Avoid_: Board, card, expansion pack (a Pack is a Librarian file), SRX as the
generic word

**Expansion slot**:
One of the four places, A to D, that an Expansion is loaded into. Most
Expansions take one Expansion slot; ExPCM takes all four. An Expansion is in
at most one Expansion slot at a time.
_Avoid_: Virtual slot, slot on its own (that reads as User slot)

**Loaded**:
Said of an Expansion that sits in an Expansion slot. Loading puts it there
and unloads whatever it replaces. Tones of an Expansion that is not loaded
play silence.
_Avoid_: Installed, enabled, active (every Expansion is always installed)

## Waves

**Wave**:
One PCM sample stored in the device, picked by number by a PCM Synth
partial, a PCM Drum Kit WMT, or an SN Synth OSC in PCM mode.
_Avoid_: Sample, waveform, multisample

**Wave Group**:
The numbered set a Wave belongs to: INT (built in), one SRX Expansion, or SN
Synth (the waves an SN Synth OSC picks from). A PCM partial or WMT picks from
INT or an SRX Wave Group; an SN Synth OSC only from SN Synth. An SRX Wave
Group plays only while its Expansion is loaded.
_Avoid_: SRX on its own, bank (a bank holds Tones)

**Wave Number**:
A Wave's position within its Wave Group, from 1. Wave Number 0 means no Wave
(OFF).

**Stereo pair**:
Two adjacent Waves of one Wave Group whose names differ only by L and R. A
naming convention read from the names, not something the device stores: L and
R are picked as two separate Wave Numbers.

## Editing

**Parameter family**:
The role a parameter plays in shaping the sound, whatever editor or effect it
sits in: Pitch / Source, Filter / Tone, Amp / Level, Modulation, Time / Space,
or Common / Global. Controls of one family look alike everywhere.
A parameter's family is the sound dimension it changes. Anything that
generates movement (LFOs, step sequencers, envelope followers) is Modulation,
whatever it targets. Common / Global holds controls with no sound dimension:
on/off switches and structure selectors.
_Avoid_: Section (a section is a group of controls on screen), category (a
Category classifies Tones)

## Layout

**Grid**:
The one lattice of Cells that every control surface screen is laid out on.
Sections placed side by side share it, so their rows and columns line up.
_Avoid_: Layout grid per section (there is only one Grid)

**Cell**:
One unit square of the Grid. Every Section and Control covers a whole number
of Cells.
_Avoid_: Slot (a User slot is device memory)

**Section**:
A titled group of Controls on screen, such as EQ, MFX or one mixer strip. It
covers a rectangle of whole Cells, its border and title included. Its border
runs a few px inside its outer Cells; a Control or Display in one of those
Cells is drawn clear of the border on that edge, so it loses those px, never
a Cell.
_Avoid_: Box, panel

**Switch strip**:
The narrow Section heading a row of Sections that one switch turns on or off
(a synth partial's row). Its title runs down its side, read bottom to top, and
its one Control is that switch as a power toggle. Turning it off dims the
row; the strip stays live.
_Avoid_: Partial header

**Control**:
One interactive element (knob, switch, power toggle, fader, picker) bound to one
parameter. A picker chooses one of a parameter's named options from a list. It
covers a rectangle of whole Cells and carries its own name and value readout.
An array field, one field holding several values (an envelope's times or
levels, the Step LFO's steps), gets one Control per element: each element is a
parameter of its own.
A Control is **inactive** when the device state makes its parameter
irrelevant; it keeps its Cells, dimmed or hidden, and never reflows.
_Avoid_: Widget

**Display**:
An element that binds to several parameters or none (envelope, scope, EQ
curve, structure diagram, surround pad). It covers a rectangle of whole Cells
like a Control and fills it. It may be interactive: what makes it a Display
rather than a Control is binding more than one parameter, or none.
_Avoid_: Graph, visualiser

**Silkscreen**:
Everything drawn that is neither a Control nor a Display: group titles,
bracket lines, divider rules, signal arrows. It sits on Cell edges and Cell
centres but covers no Cells. A Section's own Silkscreen moves with the
Section; signal arrows between Sections belong to the screen.
_Avoid_: Decoration, label (a Control's name belongs to the Control)

**Zoom**:
The one scale factor of a whole page: its Grid and everything else on the
page. It fits the Grid's width to the width the page is given, rounded to
the nearest 5%, but never drops below the Grid's drawn size: a wider Grid
scrolls instead. Nothing reflows.
_Avoid_: Breakpoint, responsive layout

**Lane**:
A run of Cells that no Section, Control or Display covers, left so signal
arrows between Sections have room to run.
_Avoid_: Gutter (Sections have none)

**Scope**:
The indices (part, partial, drum key, layer) that bind one instance of a
Section to its parameters. One layout serves every instance; a repeat or a
selection supplies the Scope.
_Avoid_: Index, instance number

**Layout**:
The definition of where things go on the Grid. A Section's layout places its
Controls, Displays and Silkscreen. A screen's layout places its Sections and
Lanes and the signal arrows between Sections. A layout names parameters and
never defines them.
_Avoid_: Skin, template

**Variant set**:
The layouts of one Section, one per key (MFX type, structure type,
instrument, strip kind). Every variant fits the same rectangle, so changing
the key never moves a neighbouring Section.
_Avoid_: Skin

## Connection

**Wireless link**:
A session with the INTEGRA-7 through its USB wireless LAN adapter, with no
MIDI interface in between. The device serves one wireless link at a time;
other clients wait until it ends.
_Avoid_: Wi-Fi MIDI, network MIDI (that is RTP-MIDI through an interface)

**Device service**:
The one process that owns the link to the INTEGRA-7 (a Wireless link, an
RTP-MIDI port or a USB port) and the one device state. Plugin instances reach
the device through it. It shows the link to other apps as an ordinary MIDI
port, present only while the link is up.
_Avoid_: Bridge (its former name, when it only held a Wireless link), proxy,
daemon
