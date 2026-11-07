/**
 * Tone banks of the INTEGRA-7, read from the core device spec via wasm.
 *
 * Each bank may span multiple LSBs (e.g., SN Acoustic Preset uses LSB 64-65,
 * with 128 tones per LSB). The catalog query returns tones per single MSB/LSB,
 * so we query each LSB separately and merge.
 */

import {
  tone_bank_groups,
  tone_type_from_bank_msb,
  user_tone_bank,
  type ToneType,
  type WasmToneBank,
} from "../pkg/integral_wasm.js";
import type { PartState } from "./types";
import { expansionByLoadValue, type Expansion } from "./expansionCatalog";

/** Program numbers per LSB (MIDI Program Change range). */
const TONES_PER_LSB = 128;

export interface ToneBank {
  label: string;
  msb: number;
  /** Array of LSB values to query. Each LSB holds up to 128 tones. */
  lsbs: number[];
  /** Label of the group holding the bank (e.g. "SN Acoustic"). */
  group: string;
  /** Short tag for the bank's tones in mixed lists (e.g. "SN-A"). */
  badge: string;
  /** The Expansion providing the bank, or null for built-in banks. */
  expansion: Expansion | null;
}

export interface ToneBankGroup {
  label: string;
  banks: ToneBank[];
}

/** Copy a wasm bank into a plain object and free the wasm one. */
function fromWasm(bank: WasmToneBank): ToneBank {
  const exp = bank.expansion;
  const result: ToneBank = {
    label: bank.label,
    msb: bank.msb,
    lsbs: Array.from(bank.lsbs),
    group: bank.group,
    badge: bank.badge,
    expansion: exp ? expansionByLoadValue(exp.loadValue) : null,
  };
  exp?.free();
  bank.free();
  return result;
}

let cachedGroups: ToneBankGroup[] | null = null;

/** All tone bank groups, in display order (from the core device spec). */
export function toneBankGroups(): ToneBankGroup[] {
  if (!cachedGroups) {
    cachedGroups = tone_bank_groups().map((group) => {
      const banks: ToneBank[] = [];
      for (let i = 0; i < group.bankCount; i++) {
        const bank = group.getBank(i);
        if (bank) banks.push(fromWasm(bank));
      }
      const result = { label: group.label, banks };
      group.free();
      return result;
    });
  }
  return cachedGroups;
}

/** Whether tones of Bank Select `msb` are of `type` (core's rule), or `type` is unrestricted. */
export function isToneTypeBank(
  msb: number,
  type: ToneType | undefined,
): boolean {
  return type === undefined || tone_type_from_bank_msb(msb) === type;
}

/**
 * The groups of {@link toneBankGroups} cut down to the banks of tone type
 * `type` (Expansion banks included), without the groups left empty. All of
 * them when `type` is undefined.
 */
export function toneBankGroupsOfType(
  type: ToneType | undefined,
): ToneBankGroup[] {
  const groups = toneBankGroups();
  if (type === undefined) return groups;
  return groups
    .map((group) => ({
      ...group,
      banks: group.banks.filter((bank) => isToneTypeBank(bank.msb, type)),
    }))
    .filter((group) => group.banks.length > 0);
}

/** The bank holding the tone at (msb, lsb), or null for unknown banks. */
export function findToneBank(msb: number, lsb: number): ToneBank | null {
  for (const group of toneBankGroups()) {
    for (const bank of group.banks) {
      if (bank.msb === msb && bank.lsbs.includes(lsb)) return bank;
    }
  }
  return null;
}

/** Index (0-based) of the tone at (lsb, pc) among the slots of `bank`. */
export function toneSlotIndex(bank: ToneBank, lsb: number, pc: number): number {
  return bank.lsbs.indexOf(lsb) * TONES_PER_LSB + pc;
}

/** Tone slot number as shown on the device (e.g. "0129") for slot `index`. */
export function toneSlotLabel(index: number): string {
  return String(index + 1).padStart(4, "0");
}

/**
 * The user bank a tone selected by `msb` saves into (from the core device
 * spec), or null for unknown banks.
 */
export function userToneBank(msb: number): ToneBank | null {
  const bank = user_tone_bank(msb);
  return bank ? fromWasm(bank) : null;
}

/** How the bar shows a Part's tone: its source, number and name. */
export function toneDisplay(part: PartState): string {
  if (part.toneName) {
    return `${toneSourceLabel(part)} : ${part.tonePC + 1} : ${part.toneName}`;
  }
  return `Bank ${part.toneBankMsb}-${part.toneBankLsb} : PC ${part.tonePC + 1}`;
}

/** The part's tone bank group, or its Expansion for Expansion banks. */
function toneSourceLabel(part: PartState): string {
  const bank = findToneBank(part.toneBankMsb, part.toneBankLsb);
  if (!bank) return `Bank ${part.toneBankMsb}`;
  return bank.expansion?.name ?? bank.group;
}
