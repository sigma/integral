import { useEffect, useState, useCallback, useRef, useMemo, type ReactNode } from "react";
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
import { browserPorts } from "./deviceService";
import { IntegraService } from "./integra";
import { installFamilyPalette } from "./synth-ui/family";
import { plugin_kind_from_wire } from "../pkg/integral_wasm.js";
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
import { TopBar } from "./TopBar";
import { PluginBar } from "./PluginBar";
import { usePluginInstance } from "./usePluginInstance";
import { useHostGestures } from "./hostGestures";
import type { PluginBridge } from "./plugin";
import { pluginDrumScope, pluginPages, pluginToneType, type PageTab } from "./pluginPages";
import { MixerPage } from "./MixerPage";
import { SurroundPage } from "./SurroundPage";
import { ToneEditorPage } from "./ToneEditorPage";
import { ErrorBoundary } from "./ErrorBoundary";
import { Transport } from "./Transport";
import { KeyboardDrawer } from "./KeyboardDrawer";
import { SequencerPage } from "./SequencerPage";
import { ExpansionsPage } from "./ExpansionsPage";
import { LibraryPage } from "./LibraryPage";
import { ToneCatalog } from "./toneCatalog";
import { useLibraryExport } from "./useLibraryExport";
import { useLibraryImport } from "./useLibraryImport";
import { ExpansionLoadBanner } from "./ExpansionLoadBanner";
import { useKeyboard } from "./useKeyboard";
import type { ExpansionMarking } from "./WavePicker";
import { sequencerEngine, type SequencerEngine } from "./sequencer";
import css from "./App.module.css";

/** The pages' tab labels. */
const TAB_LABELS: Record<PageTab, string> = {
  mixer: "Mixer",
  surround: "Surround",
  toneEdit: "Tone Edit",
  sequencer: "Sequencer",
  expansions: "Expansions",
  library: "Library",
};

/** The pages of the app in a browser, the Sequencer once it is up. */
function browserPages(sequencer: boolean): readonly PageTab[] {
  return [
    "mixer",
    "surround",
    "toneEdit",
    ...(sequencer ? (["sequencer"] as const) : []),
    "expansions",
    "library",
  ];
}

type DeviceStatus =
  | { step: "idle" }
  | { step: "identifying" }
  | { step: "connected"; identity: DeviceIdentity }
  | { step: "failed"; reason: string }
  | { step: "disconnected" };

/** Controls that turn dimmed and inert while `locked`, laid out as if unwrapped. */
function Lockable({ locked, children }: { locked: boolean; children: ReactNode }) {
  return (
    <div className={`${css.lockable} ${locked ? css.locked : ""}`} inert={locked}>
      {children}
    </div>
  );
}

/** The browser's ports: Web MIDI's, and the Device service beside them. */
const openBrowserPorts = () => browserPorts(webMidi);

/** Where the app reaches MIDI and keeps state across page loads. Both are
 *  read once, on mount. */
interface AppProps {
  /** Opens the transport's port list: the browser's Web MIDI and the Device
   *  service by default, the plugin transport inside the plugin
   *  (`main.tsx`). */
  openPorts?: () => Promise<MidiPorts>;
  /** Connect to the only port without a click, which is the choice there is
   *  (the plugin, whose one port is the Device service). Off in the browser,
   *  where the service is never a default. */
  connectOnlyPort?: boolean;
  /** Keeps the last port across page loads; `localStorage` by default. */
  storage?: Storage | null;
  /**
   * The plugin hosting the page (`main.tsx`), none in a browser. A plugin's
   * editor shows only what its instance owns ({@link pluginPages}), with the
   * instance's Binding and status in place of the top bar, and relays the
   * page's Control drags to the host as gestures.
   */
  plugin?: PluginBridge;
}

/** The control surface: connects to a device, then shows its pages. */
export function App({
  openPorts = openBrowserPorts,
  connectOnlyPort = false,
  storage = browserStorage(),
  plugin,
}: AppProps = {}) {
  const [{ memory, connectPorts }] = useState(() => ({
    memory: portMemory(storage),
    connectPorts: openPorts,
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
        const integra = findIntegraPort(pairs);
        setSelectedId(integra?.id ?? pairs[0]?.id ?? null);
        if (connectOnlyPort && pairs.length === 1) identify(pairs[0]!);
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
  }, [connectPorts, connectOnlyPort, memory, identify]);

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
  // A plugin shows the pages its kind owns; a browser, all of them.
  const pages = useMemo(
    () => (plugin ? (pluginKind === undefined ? [] : pluginPages(pluginKind)) : browserPages(sequencer !== null)),
    [plugin, pluginKind, sequencer],
  );
  const tab = pages.includes(activeTab) ? activeTab : pages[0];
  // One tone catalog for the top bar and the Library.
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
  const pluginBar =
    plugin && instance.info && pluginKind !== undefined ? (
      <PluginBar kind={pluginKind} info={instance.info} refusal={instance.refusal} onBind={instance.bind} />
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
    // what edits them is inert until it ends. Notes and the tabs stay live.
    const importing = libraryImport.busy;
    return (
      <div className={css.shell}>
        {loading && <ExpansionLoadBanner load={loading} />}
        {selectedPort?.carriesNotes === false && <NotesUnavailable />}
        {/* A running Expansion load holds every edit: the app is inert. */}
        <div
          className={`${css.body} ${loading ? css.bodyBusy : ""}`}
          inert={loading !== null}
        >
          <nav className={css.tabBar}>
            <div className={css.tabs}>
              {pages.length > 1 &&
                pages.map((page) => (
                  <button
                    key={page}
                    className={`${css.tab} ${tab === page ? css.tabActive : ""}`}
                    onClick={() => setActiveTab(page)}
                  >
                    {TAB_LABELS[page]}
                  </button>
                ))}
              {!plugin && sequencer && <Transport engine={sequencer} />}
            </div>
            {plugin ? (
              pluginBar
            ) : (
              <Lockable locked={importing}>
                <TopBar
                  mixer={mixer}
                  service={service}
                  catalog={catalog}
                  expansions={expansions}
                  onLoadExpansion={loadExpansion}
                />
              </Lockable>
            )}
          </nav>
          <Lockable locked={importing}>
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
            {tab === "toneEdit" && (
              <ErrorBoundary section="Tone Editor">
                {plugin && pluginKind !== undefined ? (
                  pluginBinding?.part === undefined ? (
                    <div className={css.unbound}>Choose a Part above to edit its tone.</div>
                  ) : (
                    <ToneEditorPage
                      mixer={mixer}
                      service={service}
                      marking={waveMarking}
                      part={pluginBinding.part}
                      only={pluginToneType(pluginKind)}
                      drum={pluginDrumScope(pluginKind, pluginBinding)}
                    />
                  )
                ) : (
                  <ToneEditorPage
                    mixer={mixer}
                    service={service}
                    marking={waveMarking}
                    onBack={() => setActiveTab("mixer")}
                  />
                )}
              </ErrorBoundary>
            )}
            {tab === "expansions" && (
              <ErrorBoundary section="Expansions">
                <ExpansionsPage expansions={expansions} staging={expansionStaging} />
              </ErrorBoundary>
            )}
          </Lockable>
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
      </div>
    );
  }

  return (
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
}
