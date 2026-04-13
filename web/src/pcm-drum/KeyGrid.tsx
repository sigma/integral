import { FIRST_KEY, LAST_KEY, noteName, isBlackKey } from "./types";
import css from "../PcmDrumEditor.module.css";

/** Colors for Comp+EQ unit routing indicators (1-6). Index 0 = PART (no color). */
const CEQ_COLORS = [
  "",        // 0 = PART — no indicator
  "#e74c3c", // C+EQ1 — red
  "#f39c12", // C+EQ2 — orange
  "#2ecc71", // C+EQ3 — green
  "#3498db", // C+EQ4 — blue
  "#9b59b6", // C+EQ5 — purple
  "#1abc9c", // C+EQ6 — teal
];

export function KeyGrid({
  selectedKey,
  onSelect,
  keyOutputAssigns,
}: {
  selectedKey: number;
  onSelect: (key: number) => void;
  /** Map of key number → outputAssign value (0=PART, 1-6=C+EQ unit). */
  keyOutputAssigns?: Map<number, number>;
}) {
  const keys: number[] = [];
  for (let k = FIRST_KEY; k <= LAST_KEY; k++) {
    keys.push(k);
  }

  return (
    <div className={css.keyGrid}>
      {keys.map((k) => {
        const selected = k === selectedKey;
        const black = isBlackKey(k);
        const oa = keyOutputAssigns?.get(k) ?? 0;
        const routeColor = CEQ_COLORS[oa] ?? "";
        const className = [
          selected ? css.keyCellSelected : css.keyCell,
          black ? css.keyCellBlack : "",
        ].filter(Boolean).join(" ");
        return (
          <div key={k} className={className} onClick={() => onSelect(k)}
            style={routeColor ? { borderLeftColor: routeColor, borderLeftWidth: 3 } : undefined}
            title={oa > 0 ? `→ C+EQ${oa}` : undefined}
          >
            <span className={css.keyNoteName}>{noteName(k)}</span>
            <span className={css.keyNumber}>{k}</span>
          </div>
        );
      })}
    </div>
  );
}
