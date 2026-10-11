import type { ReactNode } from "react";
import type { LibraryType } from "./librarian";
import css from "./PackPane.module.css";

interface RowProps {
  type: LibraryType;
  /** Where the entry comes from, e.g. "SN-S 0012". */
  label: string;
  name: string;
  /** Before the badge, e.g. a "+" mark or a checkbox. */
  lead?: ReactNode;
  className?: string;
  onClick?: () => void;
  /** After the name: status, destination, remove button. */
  children?: ReactNode;
}

/** One entry of a Pack: its type, where it comes from and its name. */
export function PackRow({ type, label, name, lead, className, onClick, children }: RowProps) {
  return (
    <div className={`${css.row} ${className ?? ""}`} onClick={onClick}>
      <span className={css.lead}>{lead}</span>
      <span className={css.badge}>{type.short}</span>
      <span className={css.label}>{label}</span>
      <span className={css.name}>{name}</span>
      {children}
    </div>
  );
}

/** A progress bar with a caption. */
export function ProgressBar({
  done,
  total,
  children,
}: {
  done: number;
  total: number;
  children: ReactNode;
}) {
  const pct = total > 0 ? (done / total) * 100 : 100;
  return (
    <div className={css.progress}>
      <div className={css.bar}>
        <div className={css.barFill} style={{ width: `${pct}%` }} />
      </div>
      <span className={css.hint}>{children}</span>
    </div>
  );
}
