/**
 * Tone bank definitions for the INTEGRA-7.
 *
 * Organized by type (SN Acoustic, SN Synth, etc.) and sub-bank (Preset, User, Expansion).
 * Reference: docs/midi/03-bank-select-tables.md
 */

export interface ToneBank {
  label: string;
  msb: number;
  lsb: number;
  count: number; // max number of tones in this bank
}

export interface ToneBankGroup {
  label: string;
  banks: ToneBank[];
}

export const TONE_BANK_GROUPS: ToneBankGroup[] = [
  {
    label: "SN Acoustic",
    banks: [
      { label: "Preset", msb: 89, lsb: 64, count: 256 },
      { label: "User", msb: 89, lsb: 0, count: 256 },
    ],
  },
  {
    label: "SN Synth",
    banks: [
      { label: "Preset", msb: 95, lsb: 64, count: 128 },
      { label: "User", msb: 95, lsb: 0, count: 128 },
    ],
  },
  {
    label: "SN Drum",
    banks: [
      { label: "Preset", msb: 88, lsb: 64, count: 26 },
      { label: "User", msb: 88, lsb: 0, count: 64 },
    ],
  },
  {
    label: "PCM Synth",
    banks: [
      { label: "Preset", msb: 87, lsb: 64, count: 128 },
      { label: "User", msb: 87, lsb: 0, count: 128 },
    ],
  },
  {
    label: "PCM Drum",
    banks: [
      { label: "Preset", msb: 86, lsb: 64, count: 14 },
      { label: "User", msb: 86, lsb: 0, count: 32 },
    ],
  },
  {
    label: "GM2",
    banks: [
      { label: "Tone", msb: 121, lsb: 0, count: 128 },
      { label: "Drum", msb: 120, lsb: 0, count: 9 },
    ],
  },
  {
    label: "Expansion",
    banks: [
      { label: "ExSN1", msb: 89, lsb: 96, count: 17 },
      { label: "ExSN2", msb: 89, lsb: 97, count: 17 },
      { label: "ExSN3", msb: 89, lsb: 98, count: 50 },
      { label: "ExSN4", msb: 89, lsb: 99, count: 12 },
      { label: "ExSN5", msb: 89, lsb: 100, count: 12 },
      { label: "ExSN6 Drum", msb: 88, lsb: 101, count: 7 },
      { label: "ExPCM Tone", msb: 97, lsb: 0, count: 128 },
      { label: "ExPCM Drum", msb: 96, lsb: 0, count: 19 },
    ],
  },
];
