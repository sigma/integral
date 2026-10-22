/**
 * A searchable list of named options in a popover: a search box, family
 * chips and the filtered options. A short list can leave the search box out
 * (`searchable`), and lists its options alone. The Wave picker and the picker Control
 * both open it; the options and their families come from core.
 */

import { useEffect, useMemo, useRef, useState, type ReactNode, type RefObject } from "react";
import { createPortal } from "react-dom";
import css from "./SearchPopover.module.css";

/** One option a popover lists. */
export interface PickOption {
  /** Raw value picked. */
  value: number;
  /** Its name, matched by the search. */
  label: string;
  /** The family its chip filters by; none when it has no family. */
  family?: string;
}

/** Families shown as chips: the largest ones, with at least this many options. */
const MIN_CHIP_SIZE = 4;
const MAX_CHIPS = 24;
const POPOVER_WIDTH = 280;

/** `n` as four digits, as the device numbers Waves. */
export function pad4(n: number): string {
  return String(n).padStart(4, "0");
}

/** Props of {@link SearchPopover}. */
export interface SearchPopoverProps {
  /** The options searched, in order. */
  options: readonly PickOption[];
  /** Listed first whatever the search or chip, and not counted (a Wave list's OFF). */
  pinned?: readonly PickOption[];
  /** The value picked now; it is marked. */
  current: number;
  /** The element the popover opens under. */
  anchor: RefObject<HTMLElement | null>;
  /** An option was picked. */
  onPick: (value: number) => void;
  /** Escape, or a press outside. */
  onClose: () => void;
  /** The search box's placeholder. */
  placeholder: string;
  /** What the options are, after their count ("Waves"). */
  noun: string;
  /** Show each option's value before its name, as four digits. */
  numbered?: boolean;
  /** The families, in the order chips of equal size take; default the options' order. */
  families?: readonly string[];
  /** Show the search box and the count; default `true`. Without it, the current option takes focus. */
  searchable?: boolean;
}

/** Search box, family chips and the (filtered) options. */
export function SearchPopover({
  options,
  pinned = [],
  current,
  anchor,
  onPick,
  onClose,
  placeholder,
  noun,
  numbered = false,
  families,
  searchable = true,
}: SearchPopoverProps) {
  const [query, setQuery] = useState("");
  const [family, setFamily] = useState<string | null>(null);
  const chips = useMemo(() => {
    const counts = new Map<string, number>();
    for (const o of options) {
      if (o.family !== undefined) counts.set(o.family, (counts.get(o.family) ?? 0) + 1);
    }
    return (families ?? [...counts.keys()])
      .map((name) => ({ name, count: counts.get(name) ?? 0 }))
      .filter((c) => c.count >= MIN_CHIP_SIZE)
      .sort((a, b) => b.count - a.count)
      .slice(0, MAX_CHIPS);
  }, [options, families]);
  const needle = query.trim().toLowerCase();
  const shown = options.filter(
    (o) => (family === null || o.family === family) && o.label.toLowerCase().includes(needle),
  );
  const item = (o: PickOption) => (
    <button
      type="button"
      key={o.value}
      role="option"
      aria-selected={o.value === current}
      autoFocus={!searchable && o.value === current}
      className={`${css.item} ${o.value === current ? css.itemCurrent : ""}`}
      onClick={() => onPick(o.value)}
    >
      {numbered && <span className={css.dim}>{pad4(o.value)}</span>}
      {o.label}
    </button>
  );

  return (
    <Floating anchor={anchor} onClose={onClose}>
      {searchable && (
        <input
          autoFocus
          className={css.search}
          placeholder={placeholder}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      )}
      {chips.length > 0 && (
        <div className={css.chips}>
          {chips.map((c) => (
            <button
              type="button"
              key={c.name}
              className={`${css.chip} ${family === c.name ? css.chipOn : ""}`}
              aria-pressed={family === c.name}
              onClick={() => setFamily(family === c.name ? null : c.name)}
            >
              {c.name} <span className={css.dim}>{c.count}</span>
            </button>
          ))}
        </div>
      )}
      {searchable && (
        <div className={css.dim}>
          {shown.length} {noun}
        </div>
      )}
      <div className={css.list} role="listbox">
        {pinned.map(item)}
        {shown.map(item)}
      </div>
    </Floating>
  );
}

/**
 * A popover anchored under `anchor`, in a portal so a panel's overflow
 * doesn't clip it. Closes on an outside pointer press or Escape.
 */
export function Floating({
  anchor,
  onClose,
  children,
}: {
  anchor: RefObject<HTMLElement | null>;
  onClose: () => void;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const onDown = (e: PointerEvent) => {
      const target = e.target;
      if (!(target instanceof Node)) return;
      if (ref.current?.contains(target) || anchor.current?.contains(target)) {
        return;
      }
      onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("pointerdown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("pointerdown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [anchor, onClose]);

  const rect = anchor.current?.getBoundingClientRect();
  const top = (rect?.bottom ?? 0) + 2;
  const left = Math.max(0, Math.min(rect?.left ?? 0, window.innerWidth - POPOVER_WIDTH - 8));
  return createPortal(
    <div
      ref={ref}
      className={css.floating}
      style={{
        top,
        left,
        width: POPOVER_WIDTH,
        maxHeight: window.innerHeight - top - 16,
      }}
    >
      {children}
    </div>,
    document.body,
  );
}
