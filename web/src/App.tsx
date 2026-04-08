import { useEffect, useState, useCallback, useRef } from "react";
import {
  requestMidiAccess,
  getPairedPorts,
  findIntegraPort,
  identifyDevice,
  type MidiPortPair,
  type Integra7Identity,
} from "./midi";
import { DeviceSelector } from "./DeviceSelector";
import {
  Identifying,
  Connected,
  Failed,
  NoDevices,
  MidiError,
} from "./DeviceStatus";
import css from "./App.module.css";

type DeviceStatus =
  | { step: "idle" }
  | { step: "identifying" }
  | { step: "connected"; identity: Integra7Identity }
  | { step: "failed"; reason: string };

export function App() {
  const [midiError, setMidiError] = useState<string | null>(null);
  const [ports, setPorts] = useState<MidiPortPair[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [status, setStatus] = useState<DeviceStatus>({ step: "idle" });
  const identifyGenRef = useRef(0);

  useEffect(() => {
    let cancelled = false;

    async function init() {
      let access: MIDIAccess;
      try {
        access = await requestMidiAccess();
      } catch {
        if (!cancelled) setMidiError("Web MIDI API not available. Use Chrome or Edge.");
        return;
      }
      if (cancelled) return;

      const pairs = getPairedPorts(access);
      setPorts(pairs);

      const integra = findIntegraPort(pairs);
      setSelectedId(integra?.id ?? pairs[0]?.id ?? null);
    }

    init();
    return () => { cancelled = true; };
  }, []);

  const identify = useCallback(async (port: MidiPortPair) => {
    const gen = ++identifyGenRef.current;
    setStatus({ step: "identifying" });

    const identity = await identifyDevice(port);
    if (identifyGenRef.current !== gen) return;

    if (identity) {
      setStatus({ step: "connected", identity });
    } else {
      setStatus({
        step: "failed",
        reason: "No response — device may not be an Integra-7, or is powered off.",
      });
    }
  }, []);

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

  const selectedPort = ports.find((p) => p.id === selectedId);

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
          </>
        )}
      </div>
    </main>
  );
}
