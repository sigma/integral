import type { ReactNode } from "react";
import css from "./FaderGroup.module.css";

interface Props {
  children: ReactNode;
}

/**
 * A horizontal row of fader-based components (SynthFader, ADSREnvelope, etc.)
 * with consistent alignment. Children are separated by vertical lines when
 * wrapped in FaderGroupItem.
 */
export function FaderGroup({ children }: Props) {
  return <div className={css.group}>{children}</div>;
}

/** Separator between fader sub-groups within a FaderGroup. */
export function FaderGroupSep() {
  return <div className={css.sep} />;
}
