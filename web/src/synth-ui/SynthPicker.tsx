/**
 * A picker on the Grid, 6×1: the field's name, then its current option's in a
 * button that opens a searchable list of the field's options (`SearchPopover`).
 * For fields with too many named values for an option list: a Wave, an Inst.
 */

// shortcut: entries from an Expansion that isn't loaded (SN Drum ExSN6 Insts 169-294) aren't marked; the options carry no Expansion.

import { useRef, useState } from "react";
import { SearchPopover, type PickOption } from "../SearchPopover";
import { FOOTPRINT, NAME, cellsPx, lineStyle } from "./grid";
import css from "./SynthPicker.module.css";

/** Props of {@link SynthPicker}. */
export interface SynthPickerProps {
  /** Current raw value. */
  value: number;
  /** Every option, named as core formats it. */
  options: readonly PickOption[];
  /** Writes the value picked. */
  onChange: (value: number) => void;
  /** Shown before the current option; none when absent. */
  label?: string;
  /** Full name, for the tooltip and the search box. Defaults to `label`. */
  title?: string;
}

/** A picker, filling its 6×1 footprint at scale 1. */
export function SynthPicker({ value, options, onChange, label, title = label }: SynthPickerProps) {
  const [open, setOpen] = useState(false);
  const anchor = useRef<HTMLButtonElement>(null);
  const current = options.find((o) => o.value === value)?.label ?? String(value);
  return (
    <div className={css.picker} style={cellsPx(FOOTPRINT.picker)} title={title}>
      {label && (
        <span className={css.label} style={lineStyle(NAME)}>
          {label}
        </span>
      )}
      <button
        type="button"
        ref={anchor}
        className={css.current}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={title ? `${title}: ${current}` : current}
        onClick={() => setOpen((v) => !v)}
      >
        {current}
      </button>
      {open && (
        <SearchPopover
          options={options}
          current={value}
          anchor={anchor}
          onPick={(v) => {
            onChange(v);
            setOpen(false);
          }}
          onClose={() => setOpen(false)}
          placeholder={`Search ${label ?? title ?? ""}…`}
          noun="options"
        />
      )}
    </div>
  );
}
