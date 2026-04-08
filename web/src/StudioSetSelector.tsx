import { useState, useRef, useEffect, useCallback } from "react";
import css from "./StudioSetSelector.module.css";

interface Props {
  currentPC: number;
  names: Map<number, string>;
  onSelect: (pc: number) => void;
  onLoadNames: () => void;
}

export function StudioSetSelector({
  currentPC,
  names,
  onSelect,
  onLoadNames,
}: Props) {
  const [open, setOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const currentName = names.get(currentPC);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (
        containerRef.current &&
        !containerRef.current.contains(e.target as Node)
      ) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  const handleOpen = useCallback(() => {
    setOpen((prev) => {
      if (prev) return false;
      onLoadNames();
      requestAnimationFrame(() => {
        const el = dropdownRef.current?.querySelector(
          `[data-pc="${currentPC}"]`,
        );
        el?.scrollIntoView({ block: "center" });
      });
      return true;
    });
  }, [currentPC, onLoadNames]);

  const handleSelect = useCallback(
    (pc: number) => {
      setOpen(false);
      onSelect(pc);
    },
    [onSelect],
  );

  return (
    <div className={css.container} ref={containerRef}>
      <span className={css.label}>STUDIO SET</span>
      <button className={css.button} onClick={handleOpen}>
        {currentName
          ? `${currentPC + 1}: ${currentName}`
          : `${currentPC + 1}: ...`}
        <span className={css.arrow}>&#x25BC;</span>
      </button>
      {open && (
        <div className={css.dropdown} ref={dropdownRef}>
          {Array.from({ length: 64 }, (_, i) => {
            const name = names.get(i);
            const isSelected = i === currentPC;
            return (
              <button
                key={i}
                data-pc={i}
                className={`${css.item} ${isSelected ? css.selected : ""} ${!name ? css.placeholder : ""}`}
                onClick={() => handleSelect(i)}
              >
                {name ? `${i + 1}: ${name}` : `${i + 1}: ...`}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
