import type { ReactNode } from "react";
import { ADDON_LINE } from "./grid";
import css from "./ControlAddon.module.css";

/** A Control's line under its readout (the tempo-sync toggle); one `ADDON_LINE` tall on the Grid. */
export function ControlAddon({ children, grid }: { children: ReactNode; grid: boolean }) {
  return (
    <div className={css.addon} style={grid ? { height: ADDON_LINE } : undefined}>
      {children}
    </div>
  );
}
