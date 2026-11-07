import { describe, expect, it } from "vitest";
import { ToneType, tone_type_from_bank_msb } from "../pkg/integral_wasm.js";
import { isToneTypeBank, toneBankGroups, toneBankGroupsOfType } from "./toneBanks";

const TYPES = [
  ToneType.SnSynth,
  ToneType.SnAcoustic,
  ToneType.PcmSynth,
  ToneType.SnDrum,
  ToneType.PcmDrum,
];

const banksOf = (type: ToneType | undefined) => toneBankGroupsOfType(type).flatMap((g) => g.banks);

describe("the tone banks of a tone type", () => {
  it("are the banks core says hold tones of that type, and no empty group", () => {
    for (const type of TYPES) {
      const banks = banksOf(type);
      expect(banks.length).toBeGreaterThan(0);
      for (const bank of banks) expect(tone_type_from_bank_msb(bank.msb)).toBe(type);
      for (const group of toneBankGroupsOfType(type)) expect(group.banks.length).toBeGreaterThan(0);
    }
  });

  it("partition the banks core knows a type for", () => {
    const typed = toneBankGroups()
      .flatMap((g) => g.banks)
      .filter((bank) => tone_type_from_bank_msb(bank.msb) !== undefined);
    expect(TYPES.flatMap((type) => banksOf(type))).toHaveLength(typed.length);
  });

  it("include the Expansion banks of that type", () => {
    for (const type of [ToneType.PcmSynth, ToneType.PcmDrum]) {
      expect(banksOf(type).some((bank) => bank.expansion !== null)).toBe(true);
    }
  });

  it("are all banks without a type", () => {
    expect(toneBankGroupsOfType(undefined)).toBe(toneBankGroups());
    expect(isToneTypeBank(0, undefined)).toBe(true);
  });
});
