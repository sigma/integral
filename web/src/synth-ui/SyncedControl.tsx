import type { ReactNode } from "react";
import { SyncToggle } from "../SyncToggle";
import css from "./SyncedControl.module.css";

interface Props {
  /** The control; it edits whichever value the toggle selects. */
  children: ReactNode;
  /** Toggle labels from the core, `[free, note]`. */
  labels: string[];
  noteMode: boolean;
  onNoteModeChange: (noteMode: boolean) => void;
}

/** A control with its msec/Hz-vs-note toggle underneath. */
export function SyncedControl({ children, labels, noteMode, onNoteModeChange }: Props) {
  return (
    <div className={css.synced}>
      {children}
      <SyncToggle labels={labels} noteMode={noteMode} onChange={onNoteModeChange} />
    </div>
  );
}
