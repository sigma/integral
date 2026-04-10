import type { ReactNode } from "react";
import css from "./SectionPanel.module.css";

interface Props {
  label: string;
  accentColor?: string;
  children: ReactNode;
}

export function SectionPanel({ label, accentColor = "#fc8", children }: Props) {
  return (
    <div className={css.panel} style={{ borderTopColor: accentColor }}>
      <div className={css.header} style={{ borderBottomColor: accentColor }}>
        {label}
      </div>
      <div className={css.body}>{children}</div>
    </div>
  );
}
