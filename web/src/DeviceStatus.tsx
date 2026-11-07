import type { DeviceIdentity } from "./midi";
import css from "./DeviceStatus.module.css";

function formatHex(value: number): string {
  return value.toString(16).toUpperCase().padStart(2, "0");
}

interface IdentifyingProps {
  portName: string;
}

export function Identifying({ portName }: IdentifyingProps) {
  return (
    <p>
      Identifying <strong>{portName}</strong>...
    </p>
  );
}

interface ConnectedProps {
  portName: string;
  identity: DeviceIdentity;
}

export function Connected({ portName, identity }: ConnectedProps) {
  return (
    <div>
      <div className={css.success}>
        {identity.isIntegra7() ? (
          <strong>Roland INTEGRA-7</strong>
        ) : (
          <strong>Unknown device</strong>
        )}{" "}
        connected on <strong>{portName}</strong>
      </div>
      <table className={css.table}>
        <tbody>
          <tr>
            <td className={css.tdLabel}>Device ID</td>
            <td className={css.tdValue}>{formatHex(identity.device_id)}H</td>
          </tr>
          <tr>
            <td className={css.tdLabel}>Family Code</td>
            <td className={css.tdValue}>{identity.familyCodeHex()}</td>
          </tr>
          <tr>
            <td className={css.tdLabel}>Family Number</td>
            <td className={css.tdValue}>{identity.familyNumberHex()}</td>
          </tr>
          <tr>
            <td className={css.tdLabel}>Revision</td>
            <td className={css.tdValue}>{identity.revisionHex()}</td>
          </tr>
        </tbody>
      </table>
    </div>
  );
}

/** Shown on a port that carries SysEx only: notes never reach the device. */
export function NotesUnavailable() {
  return (
    <p className={css.warning}>
      Notes do not reach the device through the Device service yet: the
      keyboard, the sequencer and the drum preview are silent.
    </p>
  );
}

export function Disconnected() {
  return (
    <p className={css.warning}>
      Device disconnected. Waiting for reconnection...
    </p>
  );
}

interface FailedProps {
  portName: string;
  reason: string;
  onRetry: () => void;
}

export function Failed({ portName, reason, onRetry }: FailedProps) {
  return (
    <div>
      <p className={css.error}>
        Failed on <strong>{portName}</strong>: {reason}
      </p>
      <button className={css.button} onClick={onRetry}>
        Retry
      </button>
    </div>
  );
}

export function NoDevices() {
  return <p className={css.warning}>No MIDI devices found.</p>;
}

interface MidiErrorProps {
  message: string;
}

export function MidiError({ message }: MidiErrorProps) {
  return <p className={css.error}>{message}</p>;
}
