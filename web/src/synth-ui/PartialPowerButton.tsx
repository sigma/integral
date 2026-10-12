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
      <button
        type="button"
        className={`${css.button} ${isOn ? css.on : ""}`}
        onClick={onToggle}
        title={isOn ? "Power off" : "Power on"}
        aria-pressed={isOn}
      >
        <svg className={css.icon} viewBox="0 0 24 24" fill="none" stroke="currentColor"
          strokeWidth="2.5" strokeLinecap="round">
          <path d="M12 3v9M6.3 6.3a8 8 0 1 0 11.4 0" />
        </svg>
        <span className={css.label}>{label}</span>
      </button>
    </div>
  );
}
