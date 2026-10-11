import { useEffect, useState, useCallback, useRef, useMemo } from "react";
import {
  initWasm,
  requestMidiAccess,
  getPairedPorts,
  findIntegraPort,
  identifyDevice,
  type MidiPortPair,
  type DeviceIdentity,
} from "./midi";
import { IntegraService } from "./integra";
import { useMixer } from "./useMixer";
import { useExpansions } from "./useExpansions";
import { useExpansionStaging } from "./useExpansionStaging";
import type { Expansion } from "./expansionCatalog";
import { DeviceSelector } from "./DeviceSelector";
import {
  Identifying,
  Connected,
  Disconnected,
  Failed,
  NoDevices,
  MidiError,
} from "./DeviceStatus";
import { TopBar } from "./TopBar";
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
import { ExpansionLoadBanner } from "./ExpansionLoadBanner";
import { useKeyboard } from "./useKeyboard";
import { sequencerEngine, type SequencerEngine } from "./sequencer";
import css from "./App.module.css";

type PageTab =
  | "mixer"
  | "surround"
  | "toneEdit"
  | "sequencer"
  | "expansions"
  | "library";

type DeviceStatus =
  | { step: "idle" }
  | { step: "identifying" }
  | { step: "connected"; identity: DeviceIdentity }
  | { step: "failed"; reason: string }
  | { step: "disconnected" };

export function App() {
  const [midiError, setMidiError] = useState<string | null>(null);
  const [ports, setPorts] = useState<MidiPortPair[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [status, setStatus] = useState<DeviceStatus>({ step: "idle" });
  const identifyGenRef = useRef(0);
  const midiAccessRef = useRef<MIDIAccess | null>(null);
  const [sequencer, setSequencer] = useState<SequencerEngine | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function init() {
      try {
        await initWasm();
      } catch {
        if (!cancelled) setMidiError("Failed to load WASM module.");
        return;
      }
      // Load the Pattern before any device connects.
      setSequencer(sequencerEngine());

      let access: MIDIAccess;
      try {
        access = await requestMidiAccess();
      } catch {
        if (!cancelled)
          setMidiError("Web MIDI API not available. Use Chrome or Edge.");
        return;
      }
      if (cancelled) return;

      midiAccessRef.current = access;

      const pairs = getPairedPorts(access);
      setPorts(pairs);

      const integra = findIntegraPort(pairs);
      setSelectedId(integra?.id ?? pairs[0]?.id ?? null);

      // Monitor port connect/disconnect events.
      access.onstatechange = () => {
        if (cancelled) return;
        const updatedPairs = getPairedPorts(access);
        setPorts(updatedPairs);
      };
    }

    init();
    return () => {
      cancelled = true;
      if (midiAccessRef.current) {
        midiAccessRef.current.onstatechange = null;
      }
    };
  }, []);

  const identify = useCallback(
    async (port: MidiPortPair) => {
      const gen = ++identifyGenRef.current;
      setStatus({ step: "identifying" });

      const identity = await identifyDevice(port);
      if (identifyGenRef.current !== gen) return;

      if (identity) {
        setStatus({ step: "connected", identity });
      } else {
        setStatus({
          step: "failed",
          reason:
            "No response — device may not be an Integra-7, or is powered off.",
        });
      }
    },
    [],
  );

  const handleSelect = useCallback(
    (portId: string) => {
      setSelectedId(portId);
      setStatus({ step: "idle" });
      const port = ports.find((p) => p.id === portId);
      if (port) identify(port);
    },
    [ports, identify],
  );

  const handleIdentify = useCallback(() => {
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
  const [activeTab, setActiveTab] = useState<PageTab>("mixer");
  // One tone catalog for the top bar and the Library.
  const catalog = useMemo(
    () => (service ? new ToneCatalog(service) : null),
    [service],
  );
  const libraryExport = useLibraryExport(service);
  // The tone picker's Load action: stage, then let the user Apply.
  const loadExpansion = useCallback(
    (expansion: Expansion) => {
      stageInPreselectedSlot(expansion.loadValue);
      setActiveTab("expansions");
    },
    [stageInPreselectedSlot],
  );

  // Show connected UI when connected to an Integra-7
  if (service && catalog) {
    const loading = expansions.running;
    return (
      <div className={css.shell}>
        {loading && <ExpansionLoadBanner load={loading} />}
        {/* A running Expansion load holds every edit: the app is inert. */}
        <div
          className={`${css.body} ${loading ? css.bodyBusy : ""}`}
          inert={loading !== null}
        >
          <nav className={css.tabBar}>
            <div className={css.tabs}>
              <button
                className={`${css.tab} ${activeTab === "mixer" ? css.tabActive : ""}`}
                onClick={() => setActiveTab("mixer")}
              >
                Mixer
              </button>
              <button
                className={`${css.tab} ${activeTab === "surround" ? css.tabActive : ""}`}
                onClick={() => setActiveTab("surround")}
              >
                Surround
              </button>
              <button
                className={`${css.tab} ${activeTab === "toneEdit" ? css.tabActive : ""}`}
                onClick={() => setActiveTab("toneEdit")}
              >
                Tone Edit
              </button>
              {sequencer && (
                <button
                  className={`${css.tab} ${activeTab === "sequencer" ? css.tabActive : ""}`}
                  onClick={() => setActiveTab("sequencer")}
                >
                  Sequencer
                </button>
              )}
              <button
                className={`${css.tab} ${activeTab === "expansions" ? css.tabActive : ""}`}
                onClick={() => setActiveTab("expansions")}
              >
                Expansions
              </button>
              <button
                className={`${css.tab} ${activeTab === "library" ? css.tabActive : ""}`}
                onClick={() => setActiveTab("library")}
              >
                Library
              </button>
              {sequencer && <Transport engine={sequencer} />}
            </div>
            <TopBar
              mixer={mixer}
              service={service}
              catalog={catalog}
              expansions={expansions}
              onLoadExpansion={loadExpansion}
            />
          </nav>
          {activeTab === "mixer" && (
            <ErrorBoundary section="Mixer">
              <MixerPage mixer={mixer} expansions={expansions} />
            </ErrorBoundary>
          )}
          {activeTab === "surround" && (
            <ErrorBoundary section="Surround">
              <SurroundPage mixer={mixer} />
            </ErrorBoundary>
          )}
          {activeTab === "toneEdit" && (
            <ErrorBoundary section="Tone Editor">
              <ToneEditorPage mixer={mixer} service={service} onBack={() => setActiveTab("mixer")} />
            </ErrorBoundary>
          )}
          {activeTab === "expansions" && (
            <ErrorBoundary section="Expansions">
              <ExpansionsPage expansions={expansions} staging={expansionStaging} />
            </ErrorBoundary>
          )}
          {activeTab === "library" && (
            <ErrorBoundary section="Library">
              <LibraryPage
                mixer={mixer}
                catalog={catalog}
                libraryExport={libraryExport}
              />
            </ErrorBoundary>
          )}
          {activeTab === "sequencer" && sequencer ? (
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
