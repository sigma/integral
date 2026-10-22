import { PowerButton } from "./PowerButton";
import css from "./PartialPowerButton.module.css";

interface Props {
  /** Read bottom to top, e.g. "Partial 1". */
  label: string;
  isOn: boolean;
  onToggle: () => void;
}

/** Full-height power toggle at the head of a partial row. */
export function PartialPowerButton({ label, isOn, onToggle }: Props) {
  return (
    <div className={css.column}>
      <PowerButton isOn={isOn} onToggle={onToggle} className={css.button}>
        <span className={css.label}>{label}</span>
      </PowerButton>
    </div>
  );
}
