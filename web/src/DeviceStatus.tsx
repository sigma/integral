import type { Integra7Identity } from "./midi";
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
  identity: Integra7Identity;
}

export function Connected({ portName, identity }: ConnectedProps) {
  return (
    <div>
      <div className={css.success}>
        <strong>Roland INTEGRA-7</strong> connected on{" "}
        <strong>{portName}</strong>
      </div>
      <table className={css.table}>
        <tbody>
          <tr>
            <td className={css.tdLabel}>Device ID</td>
            <td className={css.tdValue}>{formatHex(identity.deviceId)}H</td>
          </tr>
          <tr>
            <td className={css.tdLabel}>Family Code</td>
            <td className={css.tdValue}>{identity.familyCode}</td>
          </tr>
          <tr>
            <td className={css.tdLabel}>Family Number</td>
            <td className={css.tdValue}>{identity.familyNumber}</td>
          </tr>
          <tr>
            <td className={css.tdLabel}>Revision</td>
            <td className={css.tdValue}>{identity.revision}</td>
          </tr>
        </tbody>
      </table>
    </div>
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
