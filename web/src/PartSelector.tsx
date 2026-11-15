import { deviceSpec } from "./deviceSpec";
import css from "./PartSelector.module.css";

interface Props {
  selectedPart: number;
  onSelect: (part: number) => void;
}

/** The selected Part, picked from the device's Parts. */
export function PartSelector({ selectedPart, onSelect }: Props) {
  return (
    <select
      className={css.select}
      aria-label="Part"
      value={selectedPart}
      onChange={(e) => onSelect(Number(e.target.value))}
    >
      {Array.from({ length: deviceSpec().part_count }, (_, i) => (
        <option key={i} value={i}>
          {i + 1}
        </option>
      ))}
    </select>
  );
}
