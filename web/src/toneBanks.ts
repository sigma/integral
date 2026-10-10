/**
 * Tone bank definitions for the INTEGRA-7.
 *
 * Each bank may span multiple LSBs (e.g., SN Acoustic Preset uses LSB 64-65,
 * with 128 tones per LSB). The catalog query returns tones per single MSB/LSB,
 * so we query each LSB separately and merge.
 *
 * Reference: docs/midi/03-bank-select-tables.md
 */

import { user_tone_bank } from "../pkg/integral_wasm.js";

/** Program numbers per LSB (MIDI Program Change range). */
const TONES_PER_LSB = 128;

export interface ToneBank {
  label: string;
  msb: number;
  /** Array of LSB values to query. Each LSB holds up to 128 tones. */
  lsbs: number[];
}

export interface ToneBankGroup {
  label: string;
  banks: ToneBank[];
}

export const TONE_BANK_GROUPS: ToneBankGroup[] = [
  {
    label: "SN Acoustic",
    banks: [
      { label: "Preset", msb: 89, lsbs: [64, 65] },
      { label: "User", msb: 89, lsbs: [0, 1] },
    ],
  },
  {
    label: "SN Synth",
    banks: [
      { label: "Preset", msb: 95, lsbs: [64, 65, 66, 67, 68, 69, 70, 71, 72] },
      { label: "User", msb: 95, lsbs: [0, 1, 2, 3] },
    ],
  },
  {
    label: "SN Drum",
    banks: [
      { label: "Preset", msb: 88, lsbs: [64] },
      { label: "User", msb: 88, lsbs: [0] },
    ],
  },
  {
    label: "PCM Synth",
    banks: [
      { label: "Preset", msb: 87, lsbs: [64, 65, 66, 67, 68, 69, 70] },
      { label: "User", msb: 87, lsbs: [0, 1] },
    ],
  },
  {
    label: "PCM Drum",
    banks: [
      { label: "Preset", msb: 86, lsbs: [64] },
      { label: "User", msb: 86, lsbs: [0] },
    ],
  },
  {
    label: "GM2",
    banks: [
      { label: "Tone", msb: 121, lsbs: [0, 1] },
      { label: "Drum", msb: 120, lsbs: [0] },
    ],
  },
  {
    label: "Expansion",
    banks: [
      { label: "ExSN1", msb: 89, lsbs: [96] },
      { label: "ExSN2", msb: 89, lsbs: [97] },
      { label: "ExSN3", msb: 89, lsbs: [98] },
      { label: "ExSN4", msb: 89, lsbs: [99] },
      { label: "ExSN5", msb: 89, lsbs: [100] },
      { label: "ExSN6 Drum", msb: 88, lsbs: [101] },
      { label: "ExPCM Tone", msb: 97, lsbs: [0, 1, 2, 3] },
      { label: "ExPCM Drum", msb: 96, lsbs: [0] },
    ],
  },
];

/** Slot number as shown on the device (e.g. "0129") for a tone in `bank`. */
export function toneSlotNumber(bank: ToneBank, lsb: number, pc: number): string {
  const index = bank.lsbs.indexOf(lsb) * TONES_PER_LSB + pc;
  return String(index + 1).padStart(4, "0");
}

/**
 * The user bank a tone selected by `msb` saves into (from the core device
 * spec), or null for unknown banks.
 */
export function userToneBank(msb: number): ToneBank | null {
  const bank = user_tone_bank(msb);
  if (!bank) return null;
  const result = { label: bank.label, msb: bank.msb, lsbs: Array.from(bank.lsbs) };
  bank.free();
  return result;
}
