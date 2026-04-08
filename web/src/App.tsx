import { useEffect, useState, useCallback, useRef } from "react";
import {
  requestMidiAccess,
  getPairedPorts,
  findIntegraPort,
  identifyDevice,
  type MidiPortPair,
  type Integra7Identity,
} from "./midi";

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

  // Initialize MIDI access on mount
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
    if (identifyGenRef.current !== gen) return; // stale

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
    <main style={styles.main}>
      <h1 style={styles.title}>Integral</h1>
      <p style={styles.subtitle}>Integra-7 Control Surface</p>

      <div style={styles.card}>
        <h2 style={styles.cardTitle}>MIDI Connection</h2>

        {midiError ? (
          <p style={styles.error}>{midiError}</p>
        ) : ports.length === 0 ? (
          <p style={styles.warning}>No MIDI devices found.</p>
        ) : (
          <>
            <label style={styles.label}>
              MIDI Device
              <select
                style={styles.select}
                value={selectedId ?? ""}
                onChange={(e) => handleSelect(e.target.value)}
              >
                {ports.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>
            </label>

            {status.step === "idle" && (
              <button
                style={styles.button}
                onClick={handleIdentify}
                disabled={!selectedId}
              >
                Identify Device
              </button>
            )}

            {status.step === "identifying" && selectedPort && (
              <p>
                Identifying <strong>{selectedPort.name}</strong>...
              </p>
            )}

            {status.step === "connected" && selectedPort && (
              <div>
                <div style={styles.success}>
                  <strong>Roland INTEGRA-7</strong> connected on{" "}
                  <strong>{selectedPort.name}</strong>
                </div>
                <table style={styles.table}>
                  <tbody>
                    <tr>
                      <td style={styles.tdLabel}>Device ID</td>
                      <td style={styles.tdValue}>
                        {status.identity.deviceId
                          .toString(16)
                          .toUpperCase()
                          .padStart(2, "0")}
                        H
                      </td>
                    </tr>
                    <tr>
                      <td style={styles.tdLabel}>Family Code</td>
                      <td style={styles.tdValue}>{status.identity.familyCode}</td>
                    </tr>
                    <tr>
                      <td style={styles.tdLabel}>Family Number</td>
                      <td style={styles.tdValue}>{status.identity.familyNumber}</td>
                    </tr>
                    <tr>
                      <td style={styles.tdLabel}>Revision</td>
                      <td style={styles.tdValue}>{status.identity.revision}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            )}

            {status.step === "failed" && selectedPort && (
              <div>
                <p style={styles.error}>
                  Failed on <strong>{selectedPort.name}</strong>: {status.reason}
                </p>
                <button style={styles.button} onClick={handleIdentify}>
                  Retry
                </button>
              </div>
            )}
          </>
        )}
      </div>
    </main>
  );
}

const styles: Record<string, React.CSSProperties> = {
  main: {
    fontFamily:
      '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
    maxWidth: 520,
    margin: "60px auto",
    padding: "0 20px",
    color: "#e0e0e0",
  },
  title: {
    fontSize: 32,
    fontWeight: 700,
    margin: 0,
    color: "#ffffff",
  },
  subtitle: {
    fontSize: 14,
    color: "#888",
    marginTop: 4,
    marginBottom: 32,
  },
  card: {
    background: "#1a1a2e",
    borderRadius: 12,
    padding: 24,
    border: "1px solid #2a2a4a",
  },
  cardTitle: {
    fontSize: 16,
    fontWeight: 600,
    margin: "0 0 16px 0",
    color: "#b0b0d0",
    textTransform: "uppercase" as const,
    letterSpacing: 1,
  },
  label: {
    display: "block",
    fontSize: 13,
    color: "#888",
    marginBottom: 12,
  },
  select: {
    display: "block",
    width: "100%",
    padding: "8px 12px",
    marginTop: 6,
    fontSize: 14,
    background: "#0f0f1a",
    color: "#e0e0e0",
    border: "1px solid #333",
    borderRadius: 6,
  },
  button: {
    marginTop: 12,
    padding: "10px 20px",
    fontSize: 14,
    fontWeight: 600,
    background: "#4a6cf7",
    color: "#fff",
    border: "none",
    borderRadius: 6,
    cursor: "pointer",
  },
  success: {
    marginTop: 16,
    background: "#0a2a0a",
    border: "1px solid #2d6a2d",
    borderRadius: 8,
    padding: "12px 16px",
    marginBottom: 16,
    color: "#6ddb6d",
  },
  error: {
    color: "#ff6b6b",
  },
  warning: {
    color: "#ffaa44",
  },
  table: {
    width: "100%",
    borderCollapse: "collapse" as const,
    fontSize: 13,
  },
  tdLabel: {
    padding: "4px 8px 4px 0",
    color: "#888",
    whiteSpace: "nowrap" as const,
  },
  tdValue: {
    padding: "4px 0",
    fontFamily: "monospace",
    color: "#c0c0e0",
  },
};
