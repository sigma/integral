import type { ReactNode } from "react";
import css from "./ScopeGate.module.css";

/**
 * Sections of one Scope (a partial) that core may hold inactive
 * (`*_scope_is_active`): dimmed and inert while it does, in place.
 */
export function ScopeGate({ active, children }: { active: boolean; children: ReactNode }) {
  return (
    <div className={active ? undefined : css.off} data-scope-inactive={active ? undefined : true} inert={!active}>
      {children}
    </div>
  );
}
