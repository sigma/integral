/**
 * User slots as listed by the save dialogs and the Library: one row per
 * slot of a type's User bank, named from the catalog.
 */

import type { SaveSlot } from "./SlotRow";
import type { ToneEntry } from "./toneCatalog";
import { toneSlotIndex, toneSlotLabel, type ToneBank } from "./toneBanks";

/** A User slot with its Bank Select. */
export interface UserSlot extends SaveSlot {
  /** The slot, 0-indexed within its type. */
  slot: number;
  msb: number;
  lsb: number;
  pc: number;
}

/** Studio Set slot number as shown on the device (e.g. "64"). */
export function studioSetSlotLabel(slot: number): string {
  return String(slot + 1);
}

/** The slots of a user tone bank, in slot order, from its catalog entries. */
export function toneUserSlots(entries: ToneEntry[], bank: ToneBank): UserSlot[] {
  return entries
    .map((e) => {
      const slot = toneSlotIndex(bank, e.lsb, e.pc);
      return {
        key: `${e.lsb}:${e.pc}`,
        number: toneSlotLabel(slot),
        name: e.name,
        slot,
        msb: e.msb,
        lsb: e.lsb,
        pc: e.pc,
      };
    })
    .sort((a, b) => a.slot - b.slot);
}

/** The Studio Set slots, in slot order, from the Studio Set names by PC. */
export function studioSetUserSlots(names: Map<number, string>, msb: number): UserSlot[] {
  return [...names.entries()]
    .sort(([a], [b]) => a - b)
    .map(([pc, name]) => ({
      key: String(pc),
      number: studioSetSlotLabel(pc),
      name,
      slot: pc,
      msb,
      lsb: 0,
      pc,
    }));
}
