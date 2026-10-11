/**
 * The Library page's view of the Librarian: its types, how sources and
 * slots are named, slot selection and Pack file names. The Pack format,
 * import planning and the device I/O live in the core (via wasm) and in
 * `IntegraService`.
 */

import {
  librarianTypes,
  type ExportSource,
  type LibrarianType,
} from "../pkg/integral_wasm.js";
import { toneSlotLabel, userToneBank, type ToneBank } from "./toneBanks";
import { studioSetSlotLabel } from "./userSlots";

/** A Librarian type with what the UI shows of it. */
export interface LibraryType extends LibrarianType {
  /** Short tag for tabs and badges (e.g. "SN-S"). */
  short: string;
  /** The User tone bank, or null for Studio Sets. */
  bank: ToneBank | null;
}

let cachedTypes: LibraryType[] | null = null;

/** Every Librarian type, Studio Set first (from the core device spec). */
export function libraryTypes(): LibraryType[] {
  cachedTypes ??= librarianTypes().map((t) => {
    const bank = t.studioSet ? null : userToneBank(t.bankMsb);
    return { ...t, short: bank?.badge ?? t.label, bank };
  });
  return cachedTypes;
}

/** The Librarian type named `kind` (its CLI name). */
export function libraryType(kind: string): LibraryType {
  const type = libraryTypes().find((t) => t.kind === kind);
  if (!type) throw new Error(`Unknown Librarian type ${kind}`);
  return type;
}

/** The Studio Set type. */
export function studioSetType(): LibraryType {
  const type = libraryTypes().find((t) => t.studioSet);
  if (!type) throw new Error("The device spec has no Studio Set memory");
  return type;
}

/** The type of a tone selected by Bank Select MSB `msb`, if it has one. */
export function toneTypeOf(msb: number): LibraryType | undefined {
  const bank = userToneBank(msb);
  return bank ? libraryTypes().find((t) => t.bank?.msb === bank.msb) : undefined;
}

/** Slot number as shown on the device, for User slot `slot` (0-indexed). */
export function slotLabel(type: LibraryType, slot: number): string {
  return type.studioSet ? studioSetSlotLabel(slot) : toneSlotLabel(slot);
}

/** Whether a slot holding `name` is a Free slot of `type`. */
export function isFree(type: LibraryType, name: string): boolean {
  return type.initNames.includes(name.trimEnd());
}

/** Identity of an export source. */
export function sourceKey(source: ExportSource): string {
  switch (source.source) {
    case "user":
      return `user:${source.kind}:${source.slot}`;
    case "partTone":
      return `part:${source.part}`;
    case "studioSet":
      return "studioSet";
  }
}

/** Where an export source reads from, e.g. "SN-S 0012" or "Part 3". */
export function sourceLabel(source: ExportSource): string {
  switch (source.source) {
    case "user": {
      const type = libraryType(source.kind);
      return `${type.short} ${slotLabel(type, source.slot)}`;
    }
    case "partTone":
      return `Part ${source.part + 1} (Temporary)`;
    case "studioSet":
      return "Temporary Studio Set";
  }
}

/** The type an export source reads. */
export function sourceType(source: ExportSource): LibraryType {
  return source.source === "studioSet" ? studioSetType() : libraryType(source.kind);
}

/** Selected User slots, and where a ⇧-click range starts. */
export interface SlotSelection {
  slots: ReadonlySet<number>;
  anchor: number | null;
}

/** Nothing selected. */
export const NO_SELECTION: SlotSelection = { slots: new Set(), anchor: null };

/**
 * The selection after clicking `slot`: a click selects it alone, `range`
 * (⇧) selects from the anchor to it, `toggle` (⌘/Ctrl) adds or removes it,
 * and both add the range to the selection.
 */
export function selectSlot(
  selection: SlotSelection,
  slot: number,
  { range, toggle }: { range: boolean; toggle: boolean },
): SlotSelection {
  const { anchor } = selection;
  if (range && anchor !== null) {
    const slots = new Set(toggle ? selection.slots : []);
    for (let s = Math.min(anchor, slot); s <= Math.max(anchor, slot); s++) {
      slots.add(s);
    }
    return { slots, anchor };
  }
  if (toggle) {
    const slots = new Set(selection.slots);
    if (!slots.delete(slot)) slots.add(slot);
    return { slots, anchor: slot };
  }
  return { slots: new Set([slot]), anchor: slot };
}

/** Length of an 8.3 file name's base. */
const FILE_BASE_LENGTH = 8;

/** Extension of a Pack file. */
export const PACK_EXTENSION = ".SVD";

function isFileNameChar(ch: string): boolean {
  return (ch >= "A" && ch <= "Z") || (ch >= "0" && ch <= "9") || ch === "_" || ch === "-";
}

/** `input` as an 8.3 uppercase file name base: letters, digits, `_`, `-`. */
export function packFileBase(input: string): string {
  let base = "";
  for (const ch of input.toUpperCase()) {
    if (base.length === FILE_BASE_LENGTH) break;
    if (isFileNameChar(ch)) base += ch;
  }
  return base;
}

/** Save `bytes` as a download named `name`. */
export function downloadFile(name: string, bytes: Uint8Array): void {
  const url = URL.createObjectURL(
    new Blob([bytes.slice()], { type: "application/octet-stream" }),
  );
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  // Let the download start before releasing the data.
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
