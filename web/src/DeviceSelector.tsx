import type { MidiPortPair } from "./midi";
import css from "./DeviceSelector.module.css";

interface Props {
  ports: MidiPortPair[];
  selectedId: string | null;
  onSelect: (portId: string) => void;
  onIdentify: () => void;
  showIdentifyButton: boolean;
}

export function DeviceSelector({
  ports,
  selectedId,
  onSelect,
  onIdentify,
  showIdentifyButton,
}: Props) {
  return (
    <div>
      <label className={css.label}>
        MIDI Device
        <select
          className={css.select}
          value={selectedId ?? ""}
          onChange={(e) => onSelect(e.target.value)}
        >
          {ports.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      </label>

      {showIdentifyButton && (
        <button
          className={css.button}
          onClick={onIdentify}
          disabled={!selectedId}
        >
          Identify Device
        </button>
      )}
    </div>
  );
}
