import type { ReactNode } from "react";
import css from "./PowerButton.module.css";

interface Props {
  /** Whether the power is on: the button lights. */
  isOn: boolean;
  /** Flips the power. */
  onToggle: () => void;
  /** What clicking does; default "Power off" or "Power on". */
  title?: string;
  /** Lays the button out for its host (e.g. a partial's column, a Section title). */
  className?: string;
  /** Drawn after the power symbol, e.g. a label. */
  children?: ReactNode;
}

/** A power-symbol toggle, lit while on. */
export function PowerButton({ isOn, onToggle, title, className, children }: Props) {
  return (
    <button
      type="button"
      className={`${css.button} ${isOn ? css.on : ""} ${className ?? ""}`}
      onClick={onToggle}
      title={title ?? (isOn ? "Power off" : "Power on")}
      aria-pressed={isOn}
    >
      <svg className={css.icon} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
        <path d="M12 3v9M6.3 6.3a8 8 0 1 0 11.4 0" />
      </svg>
      {children}
    </button>
  );
}
