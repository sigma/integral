import type { ReactNode } from "react";
import css from "./Lockable.module.css";

/** Controls that turn dimmed and inert while `locked`, laid out as if unwrapped. */
export function Lockable({ locked, children }: { locked: boolean; children: ReactNode }) {
  return (
    <div className={`${css.lockable} ${locked ? css.locked : ""}`} inert={locked}>
      {children}
    </div>
  );
}
