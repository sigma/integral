import { useEffect, useState, useCallback, useRef, useMemo } from "react";
import {
  initWasm,
  webMidi,
  findIntegraPort,
  findPortByName,
  identifyDevice,
  portMemory,
  browserStorage,
  type MidiPortPair,
  type MidiPorts,
  type DeviceIdentity,
} from "./midi";
import {
  DEVICE_SERVICE_ID,
  browserPorts,
  servedByService,
  type PageLocation,
} from "./deviceService";
import { IntegraService } from "./integra";
import { installFamilyPalette } from "./synth-ui/family";
import { PartSettingGroup, PluginKind, plugin_kind_from_wire } from "../pkg/integral_wasm.js";
import { useMixer } from "./useMixer";
import { useExpansions } from "./useExpansions";
import { useExpansionStaging } from "./useExpansionStaging";
import type { Expansion } from "./expansionCatalog";
import { DeviceSelector } from "./DeviceSelector";
import {
  Identifying,
  Connected,
  Disconnected,
  NotesUnavailable,
  Failed,
  NoDevices,
  MidiError,
} from "./DeviceStatus";
import { NavRow, browserPages } from "./NavRow";
import { Lockable } from "./Lockable";
import { PluginBar } from "./PluginBar";
import { ToneMismatch } from "./ToneMismatch";
import { usePluginInstance } from "./usePluginInstance";
import { useHostGestures } from "./hostGestures";
import { TYPE_MISMATCH_NOTICE, type PluginBridge } from "./plugin";
import { PAGE_NAMES, pluginDrumScope, pluginPages, pluginToneType, type PageTab } from "./pluginPages";
import { MixerPage } from "./MixerPage";
import { SurroundPage } from "./SurroundPage";
import { RoutingPage } from "./RoutingPage";
import { ToneEditorPage } from "./ToneEditorPage";
import { ErrorBoundary } from "./ErrorBoundary";
import { InitGuard, InitProvider } from "./InitFlow";
import { Transport } from "./Transport";
import { KeyboardDrawer } from "./KeyboardDrawer";
import { PartSettingsDrawer } from "./PartSettingsDrawer";
import { InitPartItem, SoundCtrlInitItem } from "./PartInitItems";
import { StudioSetDrawer } from "./StudioSetDrawer";
import { SequencerPage } from "./SequencerPage";
import { ExpansionsPage } from "./ExpansionsPage";
import { SystemPage } from "./SystemPage";
import { LibraryPage } from "./LibraryPage";
import { ToneCatalog } from "./toneCatalog";
import { useLibraryExport } from "./useLibraryExport";
import { useLibraryImport } from "./useLibraryImport";
import { ExpansionLoadBanner } from "./ExpansionLoadBanner";
import { useKeyboard } from "./useKeyboard";
import type { ExpansionMarking } from "./WavePicker";
import { sequencerEngine, type SequencerEngine } from "./sequencer";
import css from "./App.module.css";

type DeviceStatus =
  | { step: "idle" }
  | { step: "identifying" }
  | { step: "connected"; identity: DeviceIdentity }
  | { step: "failed"; reason: string }
  | { step: "disconnected" };

/** Where the app reaches MIDI and keeps state across page loads. These are
 *  read once, on mount. */
interface AppProps {
  /** Where the browser loaded the page from (`main.tsx`), none in the plugin
   *  or a test. A page the Device service served ({@link servedByService})
   *  reaches it on the page's own port, and connects to it without a click,
   *  whatever other ports are listed. */
  page?: PageLocation;
  /** Opens the transport's port list: by default the browser's Web MIDI and
   *  the Device service as {@link page} reaches it, the plugin transport
   *  inside the plugin (`main.tsx`). */
  openPorts?: () => Promise<MidiPorts>;
  /** Connect to the only port without a click, which is the choice there is
   *  (the plugin, whose one port is the Device service). Off in the browser,
   *  where the service is never a default (unless it served the page:
   *  {@link page}). */
  connectOnlyPort?: boolean;
  /** Keeps the last port across page loads; `localStorage` by default. */
  storage?: Storage | null;
  /**
   * The plugin hosting the page (`main.tsx`), none in a browser. A plugin's
   * editor shows only what its instance owns ({@link pluginPages}), with the
   * instance's Binding and status in place of the navigation row, and relays the
   * page's Control drags to the host as gestures.
   */
  plugin?: PluginBridge;
}

/** The control surface: connects to a device, then shows its pages. */
export function App({
  page,
  openPorts = () => browserPorts(webMidi, page),
  connectOnlyPort = false,
  storage = browserStorage(),
  plugin,
}: AppProps = {}) {
  const [{ memory, connectPorts, loadedFrom }] = useState(() => ({
    memory: portMemory(storage),
    connectPorts: openPorts,
    loadedFrom: page,
  }));
  // The remembered port, while it has yet to appear and no port was chosen.
  const awaitedPortRef = useRef<string | null>(null);
  const [midiError, setMidiError] = useState<string | null>(null);
  const [ports, setPorts] = useState<MidiPortPair[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [status, setStatus] = useState<DeviceStatus>({ step: "idle" });
  const identifyGenRef = useRef(0);
  const [sequencer, setSequencer] = useState<SequencerEngine | null>(null);
  // The plugin instance this page edits, once the plugin has said what it is
  // and the core can name its kind.
  const instance = usePluginInstance(plugin ?? null);
  const [wasmReady, setWasmReady] = useState(false);
  const pluginKind =
    wasmReady && instance.info ? plugin_kind_from_wire(instance.info.plugin) : undefined;

  const identify = useCallback(
    async (port: MidiPortPair) => {
      const gen = ++identifyGenRef.current;
      setStatus({ step: "identifying" });

      const identity = await identifyDevice(port);
      if (identifyGenRef.current !== gen) return;

      if (identity) {
        // The next open reconnects here without a click. Only a port an
        // INTEGRA-7 answered on is remembered: a dead pick keeps the old one.
        if (identity.isIntegra7()) memory.remember(port.name);
        setStatus({ step: "connected", identity });
      } else {
        setStatus({
          step: "failed",
          reason:
            port.failure?.() ??
            "No response — device may not be an Integra-7, or is powered off.",
        });
      }
    },
    [memory],
  );

  useEffect(() => {
    let cancelled = false;

    async function init() {
      try {
        await initWasm();
      } catch {
        if (!cancelled) setMidiError("Failed to load WASM module.");
        return;
      }
      setWasmReady(true);
      installFamilyPalette(document.documentElement);
      // Load the Pattern before any device connects.
      setSequencer(sequencerEngine());

      let access: MidiPorts;
      try {
        access = await connectPorts();
      } catch {
        if (!cancelled)
          setMidiError("Web MIDI API not available. Use Chrome or Edge.");
        return;
      }
      if (cancelled) {
        access.close?.();
        return;
      }
      opened = access;

      const pairs = access.pairs();
      setPorts(pairs);

      // Reopening (a reloaded tab, a plugin editor the host recreated)
      // reconnects to the last port; a first open waits for a click.
      // A remembered port that shows up later (a network or virtual port
      // still coming up) is connected to when it appears.
      const remembered = memory.recall();
      const last = findPortByName(pairs, remembered);
      if (last) {
        setSelectedId(last.id);
        identify(last);
      } else {
        awaitedPortRef.current = remembered;
        // A page the Device service served connects to it, whatever else
        // is listed; the picker still switches ports.
        const service =
          loadedFrom !== undefined && servedByService(loadedFrom)
            ? pairs.find((p) => p.id === DEVICE_SERVICE_ID)
            : undefined;
        const integra = findIntegraPort(pairs);
        setSelectedId(service?.id ?? integra?.id ?? pairs[0]?.id ?? null);
        if (service) identify(service);
        else if (connectOnlyPort && pairs.length === 1) identify(pairs[0]!);
      }

      // Monitor port connect/disconnect events.
      stopWatching = access.onChange(() => {
        if (cancelled) return;
        const updatedPairs = access.pairs();
        setPorts(updatedPairs);
        const arrived = findPortByName(updatedPairs, awaitedPortRef.current);
        if (arrived) {
          awaitedPortRef.current = null;
          setSelectedId(arrived.id);
          identify(arrived);
        }
      });
    }

    let stopWatching: (() => void) | null = null;
    let opened: MidiPorts | null = null;
    init();
    return () => {
      cancelled = true;
      stopWatching?.();
      opened?.close?.();
    };
  }, [connectPorts, connectOnlyPort, loadedFrom, memory, identify]);

  const handleSelect = useCallback(
    (portId: string) => {
      awaitedPortRef.current = null;
      setSelectedId(portId);
      setStatus({ step: "idle" });
      const port = ports.find((p) => p.id === portId);
      if (port) identify(port);
    },
    [ports, identify],
  );

  const handleIdentify = useCallback(() => {
    awaitedPortRef.current = null;
    const port = ports.find((p) => p.id === selectedId);
    if (port) identify(port);
  }, [ports, selectedId, identify]);

  // Detect port disconnect/reconnect.
  useEffect(() => {
    if (!selectedId) return;
    const portPresent = ports.some((p) => p.id === selectedId);

    if (!portPresent && (status.step === "connected" || status.step === "identifying")) {
      // Port disappeared while connected — mark disconnected.
      setStatus({ step: "disconnected" });
    } else if (portPresent && status.step === "disconnected") {
      // Port reappeared — auto-reconnect.
      const port = ports.find((p) => p.id === selectedId);
      if (port) identify(port);
    }
  }, [ports, selectedId, status.step, identify]);

  const selectedPort = ports.find((p) => p.id === selectedId);

  // A port whose link can drop while it stays listed (the Device service)
  // says so itself: mark the device gone, and look for it again when the
  // link is back, or comes up after a failed first try.
  const stepRef = useRef(status.step);
  useEffect(() => {
    stepRef.current = status.step;
  }, [status.step]);
  useEffect(() => {
    if (!selectedPort?.watch) return;
    return selectedPort.watch((up) => {
      if (!up) {
        setStatus((s) =>
          s.step === "connected" || s.step === "identifying"
            ? { step: "disconnected" }
            : s,
        );
      } else if (stepRef.current === "disconnected" || stepRef.current === "failed") {
        identify(selectedPort);
      }
    });
  }, [selectedPort, identify]);

  // A pair that holds its link open lets go of it when another is chosen.
  useEffect(() => () => selectedPort?.release?.(), [selectedPort]);

  // Create IntegraService when connected to an Integra-7
  const service = useMemo(() => {
    if (
      status.step !== "connected" ||
      !status.identity.isIntegra7() ||
      !selectedPort
    ) {
      return null;
    }
    return new IntegraService(selectedPort, status.identity.device_id);
  }, [status, selectedPort]);

  // Clean up service on change
  useEffect(() => {
    return () => {
      service?.destroy();
    };
  }, [service]);

  // A reconnect only changes where Sequencer sends go.
  useEffect(() => {
    sequencer?.setService(service);
  }, [sequencer, service]);

  const mixer = useMixer(service);
  const expansions = useExpansions(service);
  const expansionStaging = useExpansionStaging(expansions);
  const { stageInPreselectedSlot } = expansionStaging;
  const selectedChannel = mixer.state.parts[mixer.state.selectedPart]?.receiveChannel ?? 0;
  const keyboard = useKeyboard(service, selectedChannel, sequencer);
  useEffect(() => {
    sequencer?.setTrack(mixer.state.selectedPart);
  }, [sequencer, mixer.state.selectedPart]);
  useHostGestures(
    service,
    plugin ?? null,
    pluginKind,
    instance.info?.binding ?? null,
    instance.info?.hostEdits ?? false,
  );
  const [activeTab, setActiveTab] = useState<PageTab>("mixer");
  // The Studio Set settings drawer: ⚙ in the Studio Set group opens it. It and
  // the Part settings drawer take the same place: opening one closes the other.
  const [studioSetSettingsOpen, setStudioSetSettingsOpen] = useState(false);
  // The Part settings drawer stays open across pages, following the selected Part.
  const [partSettingsOpen, setPartSettingsOpen] = useState(false);
  const partSettings = useMemo(
    () => ({
      open: partSettingsOpen,
      onToggle: () => {
        setPartSettingsOpen((open) => !open);
        setStudioSetSettingsOpen(false);
      },
    }),
    [partSettingsOpen],
  );
  const openStudioSetSettings = useCallback(() => {
    setStudioSetSettingsOpen(true);
    setPartSettingsOpen(false);
  }, []);
  const closeStudioSetSettings = useCallback(() => setStudioSetSettingsOpen(false), []);
  // The drawer's link to the System page, where System's side of each pair is edited.
  const openSystemFromDrawer = useCallback(() => {
    setStudioSetSettingsOpen(false);
    setActiveTab("system");
  }, []);
  // A plugin shows the pages its kind owns; a browser, all of them.
  const pages = useMemo(
    () => (plugin ? (pluginKind === undefined ? [] : pluginPages(pluginKind)) : browserPages(sequencer !== null)),
    [plugin, pluginKind, sequencer],
  );
  const tab = pages.includes(activeTab) ? activeTab : pages[0];
  // One tone catalog for the navigation row and the Library.
  const catalog = useMemo(
    () => (service ? new ToneCatalog(service) : null),
    [service],
  );
  const libraryExport = useLibraryExport(service);
  const libraryImport = useLibraryImport(service, mixer, catalog);
  // The tone picker's Load action: stage, then let the user Apply.
  const loadExpansion = useCallback(
    (expansion: Expansion) => {
      stageInPreselectedSlot(expansion.loadValue);
      setActiveTab("expansions");
    },
    [stageInPreselectedSlot],
  );
  // The Wave pickers' not-loaded markers and Load action.
  const { isUnloaded } = expansions;
  const waveMarking = useMemo<ExpansionMarking>(
    () => ({ isUnloaded, onLoad: loadExpansion }),
    [isUnloaded, loadExpansion],
  );

  // The plugin's Binding, and what its editor edits within it.
  const pluginBinding = instance.info?.binding ?? null;
  // A refresh the plugin finished (ADR 0009: it adopted the device's values)
  // re-reads what this page shows, which its own reads left at the old ones
  // (#562). A refresh the plugin refused or dropped never raises the count,
  // so the views are left alone.
  const refreshed = instance.info?.refreshed ?? 0;
  const seenRefresh = useRef<number | null>(null);
  const reloadMixer = mixer.reload;
  useEffect(() => {
    const first = seenRefresh.current === null;
    const risen = (seenRefresh.current ?? 0) < refreshed;
    seenRefresh.current = refreshed;
    if (first || !risen) return;
    if (pluginKind === PluginKind.StudioSet) {
      // The mixer is its view, and its Binding: read whole, once.
      reloadMixer();
    } else if (pluginKind !== undefined && pluginBinding?.part !== undefined && service) {
      // One Part's tone is its view: read the Part's dump (the tone select
      // with it); the editor below re-reads its blocks on the same count.
      service.device.reReadPart(pluginBinding.part);
    }
  }, [refreshed, pluginKind, pluginBinding, reloadMixer, service]);
  const mismatch = instance.info?.notices.find((notice) => notice.notice === TYPE_MISMATCH_NOTICE);
  const pluginBar =
    plugin && instance.info && pluginKind !== undefined ? (
      <PluginBar
        kind={pluginKind}
        info={instance.info}
        refusal={instance.refusal}
        onBind={instance.bind}
        // Only once the mixer is read: before, the Parts hold defaults.
        part={
          pluginBinding?.part === undefined || mixer.state.loading
            ? undefined
            : mixer.state.parts[pluginBinding.part]
        }
        catalog={catalog}
        expansions={expansions}
        onChangeTone={mixer.changePartTone}
        onLoadExpansion={loadExpansion}
        onRefresh={instance.refresh}
      />
    ) : plugin && wasmReady && instance.info ? (
      // The plugin is newer than this page, which cannot show it.
      <div className={css.unbound} role="alert">
        This page does not know the plugin kind {instance.info.plugin}. Update the plugin and the page together.
      </div>
    ) : null;

  // Show connected UI when connected to an Integra-7
  if (service && catalog && (!plugin || pluginKind !== undefined)) {
    const loading = expansions.running;
    // A running Pack import borrows a Part and the Temporary Studio Set:
    // what edits them is inert until it ends. Notes, the pages' buttons and
    // the transport stay live.
    const importing = libraryImport.busy;
    const shell = (
      <div className={css.shell}>
        {loading && <ExpansionLoadBanner load={loading} />}
        {selectedPort?.carriesNotes === false && <NotesUnavailable />}
        {/* A running Expansion load holds every edit: the app is inert. */}
        <div
          className={`${css.body} ${loading ? css.bodyBusy : ""}`}
          inert={loading !== null}
        >
          {plugin ? (
            // The plugin keeps its tab row and bar (web first).
            <nav className={css.tabBar}>
              <div className={css.tabs}>
                {pages.length > 1 &&
                  pages.map((page) => (
                    <button
                      key={page}
                      className={`${css.tab} ${tab === page ? css.tabActive : ""}`}
                      onClick={() => setActiveTab(page)}
                    >
                      {PAGE_NAMES[page]}
                    </button>
                  ))}
              </div>
              {pluginBar}
            </nav>
          ) : (
            <NavRow
              mixer={mixer}
              service={service}
              catalog={catalog}
              expansions={expansions}
              onLoadExpansion={loadExpansion}
              pages={pages}
              tab={tab}
              onTab={setActiveTab}
              transport={sequencer && <Transport engine={sequencer} />}
              locked={importing}
              partSettings={partSettings}
              onStudioSetSettings={openStudioSetSettings}
            />
          )}
          {/* The pages, and beside them the Part settings drawer while it is open. */}
          <div className={css.workspace}>
            <div className={css.pages}>
              {/* What a Studio Set Init resets waits for its "finished" too. */}
              <Lockable locked={importing}><InitGuard area={{ kind: "studio-set" }}>
                {tab === "mixer" && (
                  <ErrorBoundary section="Mixer">
                    <MixerPage mixer={mixer} expansions={expansions} />
                  </ErrorBoundary>
                )}
                {tab === "surround" && (
                  <ErrorBoundary section="Surround">
                    <SurroundPage mixer={mixer} />
                  </ErrorBoundary>
                )}
                {tab === "routing" && (
                  <ErrorBoundary section="Routing">
                    <RoutingPage mixer={mixer} onOpen={setActiveTab} />
                  </ErrorBoundary>
                )}
                {tab === "toneEdit" && (
                  <ErrorBoundary section="Tone Editor">
                    {plugin && pluginKind !== undefined ? (
                      pluginBinding?.part === undefined ? (
                        <div className={css.unbound}>Choose a Part above to edit its tone.</div>
                      ) : mismatch ? (
                        // The plugin is inert: its own report is the one source.
                        <ToneMismatch
                          text={mismatch.text}
                          defaultTone={instance.info?.defaultTone ?? null}
                          pending={instance.info?.defaultTonePending ?? false}
                          onLoad={instance.loadDefaultTone}
                        />
                      ) : (
                        <ToneEditorPage
                          mixer={mixer}
                          service={service}
                          marking={waveMarking}
                          part={pluginBinding.part}
                          only={pluginToneType(pluginKind)}
                          drum={pluginDrumScope(pluginKind, pluginBinding)}
                          reread={refreshed}
                        />
                      )
                    ) : (
                      <ToneEditorPage
                        mixer={mixer}
                        service={service}
                        marking={waveMarking}
                        onBack={() => setActiveTab("mixer")}
                        initItems
                      />
                    )}
                  </ErrorBoundary>
                )}
                {tab === "system" && (
                  <ErrorBoundary section="System">
                    <SystemPage
                      service={service}
                      studioSetSettings={mixer.state.studioSetSettings}
                      onOpenStudioSetSettings={openStudioSetSettings}
                    />
                  </ErrorBoundary>
                )}
                {tab === "expansions" && (
                  <ErrorBoundary section="Expansions">
                    <ExpansionsPage expansions={expansions} staging={expansionStaging} />
                  </ErrorBoundary>
                )}
              </InitGuard></Lockable>
              {tab === "library" && (
                <ErrorBoundary section="Library">
                  <LibraryPage
                    service={service}
                    mixer={mixer}
                    catalog={catalog}
                    libraryExport={libraryExport}
                    libraryImport={libraryImport}
                  />
                </ErrorBoundary>
              )}
              {/* The plugin's notes go to the Device service's note port, not through its editor. */}
              {plugin ? null : tab === "sequencer" && sequencer ? (
                <ErrorBoundary section="Sequencer">
                  <SequencerPage engine={sequencer} mixer={mixer} keyboard={keyboard} />
                </ErrorBoundary>
              ) : (
                <KeyboardDrawer keyboard={keyboard} />
              )}
            </div>
            {!plugin && partSettingsOpen && (
              <Lockable locked={importing}>
                <ErrorBoundary section="Part settings">
                  <PartSettingsDrawer
                    mixer={mixer}
                    service={service}
                    onClose={() => setPartSettingsOpen(false)}
                    onOpenSystem={() => setActiveTab("system")}
                    headerActions={<InitPartItem part={mixer.state.selectedPart} />}
                    groupActions={{ [PartSettingGroup.Offset]: <SoundCtrlInitItem /> }}
                  />
                </ErrorBoundary>
              </Lockable>
            )}
            {!plugin && studioSetSettingsOpen && (
              <Lockable locked={importing}>
                <ErrorBoundary section="Studio Set settings">
                  <StudioSetDrawer
                    mixer={mixer}
                    service={service}
                    onClose={closeStudioSetSettings}
                    onOpenSystem={openSystemFromDrawer}
                  />
                </ErrorBoundary>
              </Lockable>
            )}
          </div>
        </div>
      </div>
    );
    // Init items anywhere in the shell share one flow (#288).
    return (
      <InitProvider service={service} mixer={mixer}>
        {shell}
      </InitProvider>
    );
  }

  const card = (
    <main className={css.main}>
      <h1 className={css.title}>Integral</h1>
      <p className={css.subtitle}>Integra-7 Control Surface</p>

      <div className={css.card}>
        <h2 className={css.cardTitle}>MIDI Connection</h2>
        {/* Why a plugin has no device to show yet, and what it is bound to. */}
        {pluginBar}

        {midiError ? (
          <MidiError message={midiError} />
        ) : ports.length === 0 ? (
          <NoDevices />
        ) : (
          <>
            <DeviceSelector
              ports={ports}
              selectedId={selectedId}
              onSelect={handleSelect}
              onIdentify={handleIdentify}
              showIdentifyButton={status.step === "idle"}
            />

            {status.step === "identifying" && selectedPort && (
              <Identifying portName={selectedPort.name} />
            )}

            {status.step === "connected" && selectedPort && (
              <Connected
                portName={selectedPort.name}
                identity={status.identity}
              />
            )}


            {status.step === "failed" && selectedPort && (
              <Failed
                portName={selectedPort.name}
                reason={status.reason}
                onRetry={handleIdentify}
              />
            )}

            {status.step === "disconnected" && <Disconnected />}
          </>
        )}
      </div>
    </main>
  );
  // The shell's InitProvider, kept mounted: an Init that lost the link says so here.
  return (
    <InitProvider service={service} mixer={mixer}>
      {card}
    </InitProvider>
  );
}
