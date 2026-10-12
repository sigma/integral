import type { ReactNode } from "react";
import css from "./SectionPanel.module.css";
import { familyColour, familyOfHex, useProto } from "../familyColourPrototype";

interface Props {
  label: string;
  accentColor?: string;
  children: ReactNode;
}

export function SectionPanel({ label, accentColor = "#fc8", children }: Props) {
  const proto = useProto();
  if (proto) {
    // PROTOTYPE (#160): section accent per variant.
    const c = familyColour(proto, familyOfHex(accentColor, label));
    const v = proto.variant;
    const panel =
      v === "C" ? { borderTopColor: "#2a2a4a", borderTopWidth: 1, borderLeft: `4px solid ${c}` }
      : v === "B" ? { borderTopColor: "#2a2a4a", borderTopWidth: 1 }
      : { borderTopColor: c };
    const header =
      v === "B" ? { background: `color-mix(in oklab, ${c} 30%, #1a1a30)`, color: "#fff" }
      : v === "C" ? { color: c }
      : v === "D" ? { borderBottomColor: c, color: c }
      : { borderBottomColor: c };
    return (
      <div className={css.panel} style={panel}>
        <div className={css.header} style={header}>
          {label}
        </div>
        <div className={css.body}>{children}</div>
      </div>
    );
  }
  return (
    <div className={css.panel} style={{ borderTopColor: accentColor }}>
      <div className={css.header} style={{ borderBottomColor: accentColor }}>
        {label}
      </div>
      <div className={css.body}>{children}</div>
    </div>
  );
}
