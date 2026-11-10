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
  the bound Part (and key), for a Tone or Kit plugin. `PluginBar` replaces the
  top bar with the Binding picker and the status line.
- **App.** `App.tsx` shows a connection card until an INTEGRA-7 is
  identified. Then it shows tabs held in local state, with no router: Mixer,
  Surround, Tone Edit, Sequencer, Expansions and Library. The Sequencer tab
  and the Transport render only when a sequencer engine exists. Each page sits in
  its own `ErrorBoundary`.
- **Device service.** `integra.ts` (`IntegraService`) is the TypeScript
  side of the link to the device. It pairs MIDI I/O with the core's
  `WasmDeviceState`.

## Interface

Pages and editors (`.tsx`):

- Pages:
  - `App` (props: `openPorts`, `storage`) — the shell.
  - `MixerPage`, with `ChannelStrip`, `FxStrip` and `mixerDisplays.tsx`.
  - `SurroundPage`, with `SurroundXYPad`.
  - `ToneEditorPage` — picks the editor from the part's bank.
  - `SequencerPage`, with `Transport` and `Piano`.
  - `ExpansionsPage`.
  - `LibraryPage`, with `PackPane`, `PackExportPane`, `PackImportPane` and
    `ReinitDialog`.
- Tone editors:
  - `SnSynthEditor`, `SnAcousticEditor` and `SnDrumEditor`.
  - `PcmSynthEditor` and `PcmDrumEditor`, with `DrumKeyGrid`.
  - Shared by the editors: `MfxPanel`, `WaveDisplay` and `WavePicker`.
- Shell pieces:
  - Tab bar and selection: `TopBar`, `PartSelector`, `StudioSetSelector`,
    `ToneSelector`.
  - Saving: `SaveDialog`, `SlotRow`.
  - Connection: `DeviceSelector`, and the `DeviceStatus` views.
  - Notes: `KeyboardDrawer`.
  - Banners and markers: `ExpansionLoadBanner`, `MixerLoadWarning`,
    `NotLoadedMarker`, `PwaUpdateBanner`.
  - Error handling: `ErrorBoundary`.
- Small widgets: `NumberInput`, `SearchPopover`, `SyncUnit`,
  `TransferCurve`.

Device link and transports (`.ts`):

- `integra.ts`:
  - `IntegraService` — RQ1 requests with retries, `onDt1` listeners, mixer
    and FX setters that delegate to core, Studio Set switching and saves,
    catalog reads, Expansion loads, Pack export and import, notes.
  - Result types: `ExpansionLoadResult`, `PackImportResult`,
    `PackExportProgress`.
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
    `subscribeInstance`, `bindInstance` and `loadDefaultTone`. `ToneMismatch`
    replaces the tone editor, with the button, while the plugin reports the
    `TYPE_MISMATCH_NOTICE`. `PluginBar` disables its tone button while
    `recallOwed` is set.
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
  promises are resolved by address.
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
  a Pack import runs, the editing pages and `TopBar` are locked, but notes,
  tabs, the Sequencer and the Library stay live.
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
