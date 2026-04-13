import { FIRST_KEY, LAST_KEY, noteName, isBlackKey } from "./types";
import css from "../PcmDrumEditor.module.css";

export function KeyGrid({
  selectedKey,
  onSelect,
}: {
  selectedKey: number;
  onSelect: (key: number) => void;
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
        const className = [
          selected ? css.keyCellSelected : css.keyCell,
          black ? css.keyCellBlack : "",
        ].filter(Boolean).join(" ");
        return (
          <div key={k} className={className} onClick={() => onSelect(k)}>
            <span className={css.keyNoteName}>{noteName(k)}</span>
            <span className={css.keyNumber}>{k}</span>
          </div>
        );
      })}
    </div>
  );
}
