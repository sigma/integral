import type { ReactNode } from "react";
import { Relevance } from "../../pkg/integral_wasm.js";
import css from "./ScopeGate.module.css";

/**
 * Sections of one Scope (a partial, a WMT) that core may hold inactive
 * (`*_scope_relevance`): dimmed while it does, in place. Switched off, they
 * stay editable, as on the hardware; irrelevant, they go inert.
 */
export function ScopeGate({ relevance, children }: { relevance: Relevance; children: ReactNode }) {
  const active = relevance === Relevance.Active;
  return (
    <div
      className={active ? undefined : css.off}
      data-scope-inactive={active ? undefined : true}
      inert={relevance === Relevance.Irrelevant}
    >
      {children}
    </div>
  );
}
