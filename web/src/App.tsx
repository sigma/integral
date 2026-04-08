import { useEffect, useState, useCallback } from "react";
import {
  requestMidiAccess,
  getPairedPorts,
  findIntegraPort,
  identifyDevice,
  type MidiPortPair,
  type Integra7Identity,
} from "./midi";

type ConnectionState =
  | { status: "idle" }
  | { status: "no-midi" }
  | { status: "ready"; ports: MidiPortPair[]; selectedId: string | null }
  | { status: "identifying"; port: MidiPortPair }
  | { status: "connected"; port: MidiPortPair; identity: Integra7Identity }
  | { status: "failed"; port: MidiPortPair; reason: string };

export function App() {
  const [state, setState] = useState<ConnectionState>({ status: "idle" });

  // Initialize MIDI access on mount
  useEffect(() => {
    let cancelled = false;

    async function init() {
      let access: MIDIAccess;
      try {
        access = await requestMidiAccess();
      } catch {
        if (!cancelled) setState({ status: "no-midi" });
        return;
      }

      if (cancelled) return;

      const ports = getPairedPorts(access);
      const integraPort = findIntegraPort(ports);

      setState({
        status: "ready",
        ports,
        selectedId: integraPort?.id ?? ports[0]?.id ?? null,
      });

      // Auto-identify if we found an Integra port
      if (integraPort) {
        setState({ status: "identifying", port: integraPort });
        const identity = await identifyDevice(integraPort);
        if (cancelled) return;

        if (identity) {
          setState({ status: "connected", port: integraPort, identity });
        } else {
          setState({
            status: "failed",
            port: integraPort,
            reason: "No response — is the device powered on?",
          });
        }
      }
    }

    init();
    return () => {
      cancelled = true;
    };
  }, []);

  const handleSelect = useCallback(
    (portId: string) => {
      if (state.status !== "ready") return;
      setState({ ...state, selectedId: portId });
    },
    [state],
  );

  const handleConnect = useCallback(async () => {
    if (state.status !== "ready" || !state.selectedId) return;
    const port = state.ports.find((p) => p.id === state.selectedId);
    if (!port) return;

    setState({ status: "identifying", port });
    const identity = await identifyDevice(port);

    if (identity) {
      setState({ status: "connected", port, identity });
    } else {
      setState({
        status: "failed",
        port,
        reason: "No response — device may not be an Integra-7, or is powered off.",
      });
    }
  }, [state]);

  const handleRetry = useCallback(async () => {
    if (state.status !== "failed" && state.status !== "connected") return;
    const port = state.port;

    setState({ status: "identifying", port });
    const identity = await identifyDevice(port);

    if (identity) {
      setState({ status: "connected", port, identity });
    } else {
      setState({
        status: "failed",
        port,
        reason: "No response — device may not be an Integra-7, or is powered off.",
      });
    }
  }, [state]);

  return (
    <main style={styles.main}>
      <h1 style={styles.title}>Integral</h1>
      <p style={styles.subtitle}>Integra-7 Control Surface</p>

      <div style={styles.card}>
        <h2 style={styles.cardTitle}>MIDI Connection</h2>
        {renderContent(state, handleSelect, handleConnect, handleRetry)}
      </div>
    </main>
  );
}

function renderContent(
  state: ConnectionState,
  onSelect: (id: string) => void,
  onConnect: () => void,
  onRetry: () => void,
) {
  switch (state.status) {
    case "idle":
      return <p>Requesting MIDI access...</p>;

    case "no-midi":
      return (
        <p style={styles.error}>
          Web MIDI API not available. Use Chrome or Edge, and ensure SysEx permissions are
          granted.
        </p>
      );

    case "ready":
      if (state.ports.length === 0) {
        return <p style={styles.warning}>No MIDI devices found.</p>;
      }
      return (
        <div>
          <label style={styles.label}>
            MIDI Device
            <select
              style={styles.select}
              value={state.selectedId ?? ""}
              onChange={(e) => onSelect(e.target.value)}
            >
              {state.ports.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </label>
          <button
            style={styles.button}
            onClick={onConnect}
            disabled={!state.selectedId}
          >
            Identify Device
          </button>
        </div>
      );

    case "identifying":
      return (
        <p>
          Identifying <strong>{state.port.name}</strong>...
        </p>
      );

    case "connected":
      return (
        <div>
          <div style={styles.success}>
            <strong>Roland INTEGRA-7</strong> connected on{" "}
            <strong>{state.port.name}</strong>
          </div>
          <table style={styles.table}>
            <tbody>
              <tr>
                <td style={styles.tdLabel}>Device ID</td>
                <td style={styles.tdValue}>
                  {state.identity.deviceId.toString(16).toUpperCase().padStart(2, "0")}H
                </td>
              </tr>
              <tr>
                <td style={styles.tdLabel}>Family Code</td>
                <td style={styles.tdValue}>{state.identity.familyCode}</td>
              </tr>
              <tr>
                <td style={styles.tdLabel}>Family Number</td>
                <td style={styles.tdValue}>{state.identity.familyNumber}</td>
              </tr>
              <tr>
                <td style={styles.tdLabel}>Revision</td>
                <td style={styles.tdValue}>{state.identity.revision}</td>
              </tr>
            </tbody>
          </table>
          <button style={styles.buttonSecondary} onClick={onRetry}>
            Re-identify
          </button>
        </div>
      );

    case "failed":
      return (
        <div>
          <p style={styles.error}>
            Failed on <strong>{state.port.name}</strong>: {state.reason}
          </p>
          <button style={styles.button} onClick={onRetry}>
            Retry
          </button>
        </div>
      );
  }
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
  buttonSecondary: {
    marginTop: 12,
    padding: "8px 16px",
    fontSize: 13,
    background: "transparent",
    color: "#4a6cf7",
    border: "1px solid #4a6cf7",
    borderRadius: 6,
    cursor: "pointer",
  },
  success: {
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
