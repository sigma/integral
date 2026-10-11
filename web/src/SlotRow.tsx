import type { MouseEvent, ReactNode } from "react";
import css from "./SlotRow.module.css";

/** A user slot, as listed for saving into or picking from. */
export interface SaveSlot {
  /** Stable React key. */
  key: string;
  /** Slot number as shown on the device (e.g. "0129", "64"). */
  number: string;
  /** Name currently stored in the slot. */
  name: string;
}

interface Props {
  slot: SaveSlot;
  active: boolean;
  disabled?: boolean;
  /** Extra classes, e.g. to dim a free slot. */
  className?: string;
  onClick: (e: MouseEvent<HTMLButtonElement>) => void;
  /** Shown after the name. */
  children?: ReactNode;
}

/** One row of a slot list: the slot number and the name it holds. */
export function SlotRow({ slot, active, disabled, className, onClick, children }: Props) {
  return (
    <button
      className={`${css.slot} ${active ? css.slotActive : ""} ${className ?? ""}`}
      onClick={onClick}
      disabled={disabled}
    >
      <span className={css.slotNumber}>{slot.number}</span>
      <span className={css.slotName}>{slot.name}</span>
      {children}
    </button>
  );
}
