import type { ReactNode } from "react";
import type { ParameterFamily } from "../../pkg/integral_wasm.js";
import { accentColour } from "./family";
import css from "./SectionPanel.module.css";

interface Props {
  label: string;
  /** Top border and header underline colour, when `family` is absent. Default: "#fc8". */
  accentColor?: string;
  /** Parameter family: colours the top border and header underline. */
  family?: ParameterFamily;
  children: ReactNode;
}

export function SectionPanel({
  label,
  accentColor = "#fc8",
  family,
  children,
}: Props) {
  const accent = accentColour(family, accentColor);
  return (
    <div className={css.panel} style={{ borderTopColor: accent }}>
      <div className={css.header} style={{ borderBottomColor: accent }}>
        {label}
      </div>
      <div className={css.body}>{children}</div>
    </div>
  );
}
