# web/src

## Overview

The React app: the Integral control surface for the INTEGRA-7. The Rust core
compiled to wasm (`../pkg/integral_wasm.js`) owns device state, specs, the
SysEx send queue and layouts data. This directory is a thin view over it,
plus the I/O the browser has to do.

- **Entry.** `main.tsx` checks for a plugin bridge (`windowBridge(window)`).
  Inside the plugin webview
  ([ADR 0004](../../docs/adr/0004-plugin-ui-in-a-webview.md)) it sizes the
  window after wasm loads and hands `App` the plugin transport and the
  bridge (`plugin`). In a browser it mounts `PwaUpdateBanner`, and `App`
  uses Web MIDI.
- **Plugin editor.** Given a bridge, `App` shows only what the plugin's
  instance owns (ADR 0007, `pluginPages.ts`): the Mixer and Surround pages
  for the Studio Set plugin, the tone editor of the plugin's type, scoped to
  the bound Part (and key), for a Tone or Kit plugin, as tabs ("Tone Edit"
  for the tone editor). `PluginBar` replaces the navigation row with the
  Binding picker and the status line.
- **App.** `App.tsx` shows a connection card until an INTEGRA-7 is
  identified. Then it shows the navigation row (`NavRow`,
  [#584](https://github.com/sigma/integral/issues/584)) and the page held in
  local state, with no router. The row nests the scopes left to right, each
  group its selector and controls, then its pages:
  - **Device:** System, Expansions, Library.
  - **Studio Set:** selector, SAVE; Mixer, Surround, Routing, Sequencer and
    the Transport.
  - **Part:** selector, receive channel, ⚙ (the Part settings drawer),
    PREVIEW.
  - **Tone:** selector, SAVE; Edit (the tone editor).

  A narrow screen wraps the row between groups. The Sequencer page and the
  Transport render only when a sequencer engine exists. Each page sits in its
  own `ErrorBoundary`. A new page goes in its scope's list in `SCOPE_PAGES`, a
  new control in its scope's `controls` in `NavRow`.

  ⚙ opens the Part settings drawer (`PartSettingsDrawer`,
  [PRD #588](https://github.com/sigma/integral/issues/588)) at the right of
  the pages. It stacks the groups of core's Part settings registry for the
  selected Part, follows the selection and stays open across pages. Its
  header holds Init Part…, and its OFFSET group Sound Ctrl Init…, which
  resets the offsets of every Part (`PartInitItems.tsx`,
  [#591](https://github.com/sigma/integral/issues/591)). The groups a Part
  Init resets are guarded by it (VOICE is Studio Set Common: it isn't), and
  OFFSET by Sound Ctrl Init too. The Mixer guards each Part's strip by its
  Part Init, but not its EQ. Its SCALE group reads the System Scale Tune
  Switch (`useSystemCommon`) and links to the System page.
- **Device service.** `integra.ts` (`IntegraService`) is the TypeScript
  side of the link to the device. It pairs MIDI I/O with the core's
  `WasmDeviceState`.

## Interface

Pages and editors (`.tsx`):

- Pages:
  - `App` (props: `openPorts`, `storage`) — the shell.
  - `MixerPage`, with `ChannelStrip`, `FxStrip` and `mixerDisplays.tsx`.
  - `SurroundPage`, with `SurroundXYPad`.
  - `RoutingPage` — the effective routing (`MixerState.routing`, from core's
    `effectiveRouting()`): a row per source, a column per output, the stage
    switches, and chips linking to each stage's editor.
  - `ToneEditorPage` — picks the editor from the part's bank. The web page's
    header has Tone Init… for an editable tone; a plugin's editor has none.
  - `SequencerPage`, with `Transport` and `Piano`.
  - `SystemPage` — System Common in its core groups, read on open and on
    Refresh (`useSystemCommon`).
  - `ExpansionsPage`.
  - `LibraryPage`, with `PackPane`, `PackExportPane`, `PackImportPane` and
    `ReinitDialog`.
- Tone editors:
  - `SnSynthEditor`, `SnAcousticEditor` and `SnDrumEditor`.
  - `PcmSynthEditor` and `PcmDrumEditor`, with `DrumKeyGrid` and
    `DrumKeyHeader` (the selected key's header, shared with `SnDrumEditor`).
  - Partial Init (#598): with `initItems` (the browser's `ToneEditorPage`
    passes it; a plugin's doesn't, web first), each SN Synth and PCM Synth
    partial's switch strip shows an `InitItem` in its title (`GridSection`'s
    `titleAction`), and each kit's `DrumKeyHeader` one for the selected key,
    by its MIDI key number. An `InitGuard` holds that partial's boxes (and a
    PCM Synth partial's strip, whose PMT switch the Init may reset), or that
    key's Sections, busy. `ToneEditorPage` keeps a kit's selected key
    in a ref (`lastKey`, down to `useDrumKey`) while the Part holds the same
    tone, so the editor the Init remounts opens on the same key.
  - Shared by the editors: `MfxPanel`, `WaveDisplay` and `WavePicker`.
- Shell pieces:
  - Navigation and selection: `NavRow` (with `NavScope`), the scopes'
    controls in `ScopeControls`, `PartSelector`, `StudioSetSelector`,
    `ToneSelector`.
  - Locking: `Lockable`.
  - Saving: `SaveDialog`, `SlotRow`.
  - Initialization (`InitFlow.tsx`, #288): `InitProvider` (in `App`, around
    the shell and the connection card alike, so it outlives a lost link)
    runs one Init at a time. An area's "Init…" item calls
    `useInit().request(scope)`, which confirms first, naming what resets;
    `InitItem` (`scope`, optional `className` and label) is that item, "Init…",
    disabled while an Init runs.
    `InitGuard` (`area`) keeps the area's controls dimmed and inert while an
    Init that resets all of it runs. After "finished", or a timeout, the
    provider reads back
    through the existing paths: `mixer.reload` for a Studio Set,
    `reReadPartBlock` for a Part (its settings past the dump too),
    `reReadPart` for a Tone or Sound Ctrl, and `useInit().rereads(part)`,
    which `ToneEditorPage` keys its editor on. A timeout, a refusal or a
    lost link ends the busy state and says so.
  - Connection: `DeviceSelector`, and the `DeviceStatus` views.
  - Notes: `KeyboardDrawer`.
  - Drawers: `Drawer`, the shell (title, header actions, close, stacked
    groups); `PartSettingsDrawer` in it.
  - Banners and markers: `ExpansionLoadBanner`, `MixerLoadWarning`,
    `NotLoadedMarker`, `PwaUpdateBanner`.
  - Error handling: `ErrorBoundary`.
- Small widgets: `NumberInput`, `SearchPopover`, `SyncUnit`,
  `TransferCurve`.

Device link and transports (`.ts`):

- `integra.ts`:
  - `IntegraService` — RQ1 requests with retries, `onDt1` listeners, mixer
    and FX setters that delegate to core, Studio Set switching and saves,
    catalog reads, Expansion loads, Pack export and import, Inits
    (`initialize`), notes. `destroy` ends a Librarian run or an Init still
    going with `LINK_LOST_ERROR`.
  - Result types: `ExpansionLoadResult`, `PackImportResult`,
    `PackExportProgress`.
- `init.ts`: `InitScope` (the area an Init resets), `initMachine` (its core
  `Initialization`), `covers` and `describeInit`.
- `midi.ts`:
  - `initWasm` — memoized wasm init.
  - Port interfaces: `MidiPortPair`, `MidiPorts`, `MidiListener`.
  - Web MIDI implementation: `webMidi`, `webMidiPorts`, `webMidiPair`,
    `requestMidiAccess`, `getPairedPorts`.
  - Port choice: `findIntegraPort`, `findPortByName`.
  - Port memory: `portMemory`, `browserStorage`, `LAST_PORT_KEY`.
  - Identity: `identifyDevice`, `DeviceIdentity`, `IDENTIFY_TIMEOUT_MS`
    (2000).
  - `PortMemory` (the port-memory interface).
- `deviceService.ts`:
  - The Device service as a port (ADR 0006): `deviceServicePair` is a
    `MidiPortPair` over the service's localhost WebSocket (one binary frame
    per SysEx message each way; the URL comes from core through
    `deviceServiceUrl`). Lazy: it connects on `open()`, then reconnects with
    a wait growing from `RECONNECT_MIN_MS` to `RECONNECT_MAX_MS`. `watch`
    reports the link going down and up, `failure` gives `NOT_RUNNING_HINT`
    (not running, or this page's origin is not allowed) or `LINK_DOWN_HINT`
    (the service closed the page with 1013: it lost the device), and
    `release` stops reconnecting when another pair is chosen. It carries
    notes too (`carriesNotes`): a channel voice message is its own frame,
    which the service plays ahead of its SysEx queue, and a send with a
    future `timestamp` is held in the page until then (`holdUntil`, on
    `workerTimers`, which a background tab does not throttle).
  - `withDeviceService`, `browserPorts`: the service listed beside Web MIDI's
    pairs (alone where the browser has no Web MIDI), `DEVICE_SERVICE_NAME`.
- `timedSend.ts`: `holdUntil` holds a send with a future `timestamp` until
  then, for the pairs with no scheduler underneath (the Device service's, in
  the browser and the plugin), on `Timers`: `mainThreadTimers`, or
  `workerTimers` (a dedicated Worker's, which a background tab does not
  throttle; the page's own where there is no Worker).
- `devPorts.ts`: `DEV_SERVER_PORT` (5181) and `PREVIEW_PORT` (4181), pinned in
  `vite.config.ts` and equal to core's, which the service admits as origins.
- `plugin.ts`:
  - The plugin transport: `windowBridge`, `PluginBridge`, `pluginPorts`. It
    offers the Device service as the one pair (`DEVICE_SERVICE_ID` and
    `DEVICE_SERVICE_NAME`, as in the browser). It carries notes too
    (`carriesNotes`): the plugin sends a channel voice message to the
    service as a `Note` request, played ahead of its SysEx queue. A timed
    send is held on the page's own timers (the webview's CSP refuses a
    `blob:` worker, and the editor sends nothing timed). While the plugin has no port to offer, `failure` gives the
    reason it sent, else `SERVICE_UNAVAILABLE_HINT`.
  - The IPC wire format: `encodePageMessage`, `decodePluginMessage`,
    `PLUGIN_EVENT`, and the wire types `PageMessage` and `PluginMessage`.
  - The instance: `InstanceInfo` (the plugin kind, Binding, status, notices
    and default tone the plugin reports, and whether a recall is owed),
    `subscribeInstance`, `bindInstance`, `loadDefaultTone` and
    `refreshFromDevice`. `ToneMismatch`
    replaces the tone editor, with the button, while the plugin reports the
    `TYPE_MISMATCH_NOTICE`. `PluginBar` disables its tone button while
    `recallOwed` is set, and its "Refresh from device" button unless
    `canRefresh` (bound, `CONNECTED_STATUS`, `hostEdits`, no `RECALLING_NOTICE`, no
    `recallOwed`).
    `usePluginInstance` is the hook.
  - `OPEN_TIMEOUT_MS` (2000) and `ScreenArea`.
  - Window sizing: `pluginGrid`, `pluginWindowSize`, `resizePluginWindow`.

- `pluginPages.ts`: `pluginPages` (the pages a plugin kind shows),
  `pluginToneType`, `DrumScope` (`pluginDrumScope`, `drumScope`,
  `lockedKey`), `kitKeys` and `acceptsBinding` (core's rule, through wasm).
- `hostGestures.ts`: `HostGestures` turns a drag's writes into `beginEdit`,
  `performEdit` and `endEdit` (begin on the first write, end on the pointer's
  release; a write with no pointer held is its own gesture). `useHostGestures`
  collects the writes from the core (`takeHostWrites`, which names the host
  parameter each sets for the Binding) every `GESTURE_POLL_MS`.

Catalogs from the core spec:

- `deviceSpec.ts` (`deviceSpec`)
- `toneBanks.ts`
- `toneCatalog.ts` (`ToneCatalog`, the shared tone-name cache)
- `expansionCatalog.ts`
- `snaInstCatalog.ts`
- `waveCatalog.ts`
- `librarian.ts` (library types, slot selection, `downloadFile`)
- `userSlots.ts`
- `categories.ts` and `categoryIcons.ts`

Binding helpers:

- `fieldSpec.ts` (`FieldEntry`, `switchOptions`)
- `paramDefaults.ts` (`paramMeta`, `paramFormatter`, `mixerBinding`)
- `fxBinding.ts` (`useFxBind`)
- `cached.ts` (`cached`)

State and hooks:

- `types.ts` — the `MixerState` mirror and its defaults.
- Mixer and editing: `useMixer`, `useMfxHandlers`, `useDrumKey`,
  `useKeyPrefetch`.
- Expansions: `useExpansions`, `useExpansionStaging`.
- System: `useSystemCommon`; every reader of one service shares its reads
  and edits.
- Librarian: `useLibraryExport`, `useLibraryImport`, `useUserSlots`,
  `useBankEntries`.
- Notes: `useKeyboard`.
- Controls: `useDragControl` (pointer drag for knobs and faders).
- Sequencer: `sequencer.ts` (`sequencerEngine`, `useSequencer`).

Subdirectories:

- [`layouts`](./layouts/README.md) — the Grid layout system: format,
  validator, routing, renderers, and the per-screen layouts.
- [`synth-ui`](./synth-ui/README.md) — hardware-style Controls and
  Displays, Grid units, and the family palette.
- [`pcm-synth`](./pcm-synth/README.md) — PCM Synth screen, bindings, and the
  Structure routing diagram.
- [`sn-synth`](./sn-synth/README.md) — SN Synth COMMON, partial boxes and
  scopes.
- [`pcm-drum`](./pcm-drum/README.md) — PCM Drum Kit mirrors and field
  tables.
- [`sn-drum`](./sn-drum/README.md) — SN Drum Kit mirrors and field tables.
- `test/` — shared test helpers (fake MIDI access, storage, DOM, grid). Not
  production code.
- `wasm/` — untracked `wasm-pack` output. Not documented here.

## Invariants

- **Core is the source of truth.** Device constants (part count, name
  lengths, key ranges, FX and output names), tone banks, Expansions,
  instruments, library types, field specs, relevance and Display bindings
  come from wasm. Never hardcode them here. The only literal tables are the
  tone categories: `CATEGORIES` (sourced from `docs/sounds/categories.json`),
  `ABBREVS` in `categories.ts`, and `ICON_MAP` / `STRIP_ICONS` in
  `categoryIcons.ts`.
- `initWasm()` must resolve before anything reads core. `App` shows an error
  if it fails.
- **SysEx pacing lives in Rust.** `WasmDeviceState` owns throttling, queueing
  and echo suppression. `IntegraService` polls `device.drain()` every 5 ms
  and sends what it returns. RQ1 timeouts (2000 ms, 2 retries) start only
  once the bytes leave the queue.
- **Incoming DT1** goes to core first (`handleExpansionDt1`, then
  `handleDt1`). TypeScript listeners are notified after that. Pending RQ1
  promises are resolved by address. `handleDt1` also takes the Part tone
  selects of dumps the Device service relayed — a tone changed on the
  device, or selected by another client of the service, reaches the mixer
  without a reload, in the browser and the plugin.
- **Transports.** Device I/O goes through `MidiPorts`. In a browser that is
  Web MIDI: ports paired by identical name, inputs opened explicitly. In the
  plugin, bytes are relayed as JSON over the wry IPC, and the page still
  paces its sends. The wire format must stay in step with
  [`crates/integral-vst/src/webview/ipc.rs`](../../crates/integral-vst/src/webview/ipc.rs).
  Tests on both sides enforce it.
- **Connection.**
  - The last port is remembered by name, not id, and only after an
    INTEGRA-7 answered on it.
  - A generation counter drops stale identify results.
  - `IntegraService` is created only for an identified INTEGRA-7.
- **Locking.** While an Expansion load runs, the whole body is inert. While
  a Pack import runs, the editing pages and the row's Studio Set, Part and
  Tone controls are locked, but notes, the row's page buttons, the Transport,
  the Sequencer and the Library stay live.
- **PWA updates.** The service worker is registered only outside the plugin,
  in prompt mode. A new deploy takes over only on the reload the banner
  offers, so a running page never mixes JS and wasm from two builds.
- **Saves.** Save targets are snapshotted when the dialog opens. Save and
  Reinit dialogs can't be closed mid-run.
- **Sequencer.** Core's `WasmSequencer` owns the pattern and timing. A Worker
  ticks every 25 ms and sends what is due in the next 100 ms, with
  timestamps. A Web Lock is held while it plays.

## Side Effects

- Web MIDI: `navigator.requestMIDIAccess({ sysex: true })`, port open, send,
  listen (`midi.ts`). In the plugin: `window.ipc.postMessage`, the
  `integral-plugin` window event, and window resize requests (`plugin.ts`).
- Device writes, all through `IntegraService`: mixer and tone edits, saves,
  User-slot clears, Expansion loads, Pack imports, notes and Panic.
  `IntegraService` logs every outgoing SysEx to the console.
- `localStorage`: `integral.lastPort` (`midi.ts`) and `integral.pattern`, the
  sequencer pattern as base64 SMF (`sequencer.ts`).
- Service worker registration (`PwaUpdateBanner.tsx`, browser only).
- File downloads through Blob URLs: Packs, undo files, `pattern.mid`. File
  reads: Pack open and drop, `.mid` upload.
- Timers and listeners:
  - The SysEx drain interval and RQ1 timeouts.
  - `requestAnimationFrame` for mixer sync, the sequencer playhead, and the
    tone and Studio Set selectors.
  - The sequencer Worker and its Web Lock.
  - Global key, blur, focus and pointer listeners (keyboard input,
    Expansion re-reads).
- CSS custom properties on `document.documentElement`
  (`installFamilyPalette`).

## Verification

- Run: `just test-web` (Vitest + jsdom against the packed wasm)
- Run: `just lint-web` (`tsc -b`) and `just lint-css` (Stylelint over
  `src/**/*.css`)
