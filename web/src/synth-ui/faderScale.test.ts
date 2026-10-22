import { describe, expect, it } from "vitest";
import { MixerField, mixer_field_spec } from "../../pkg/integral_wasm.js";
import { faderScale } from "./faderScale";

describe("faderScale", () => {
  it("marks a 0–127 level on the MIDI ruler, 0 dB where core puts it", () => {
    const zeroDb = mixer_field_spec(MixerField.PartLevel).zeroDb!;
    const marks = faderScale({ min: 0, max: 127, format: (v) => `#${v}`, zeroDb });
    expect(marks.map((m) => m.value)).toEqual([127, 100, 80, 60, 40, 20, 0]);
    expect(marks.filter((m) => m.zero)).toEqual([{ value: zeroDb, zero: true, label: "0dB" }]);
    // The others read as the field formats them.
    expect(marks[0]!.label).toBe("#127");
  });

  it("draws no 0 dB mark on a 0–127 level core gives none", () => {
    const marks = faderScale({ min: 0, max: 127, format: (v) => `#${v}`, nominal: 0 });
    expect(marks.map((m) => m.value)).toEqual([127, 100, 80, 60, 40, 20, 0]);
    expect(marks.filter((m) => m.zero)).toEqual([]);
    expect(marks[1]!.label).toBe("#100");
  });

  it("marks another range in quarters, labelled by its formatter, 0 dB at its nominal value", () => {
    const marks = faderScale({ min: 0, max: 24, format: (v) => `+${v}dB`, nominal: 0 });
    expect(marks.map((m) => [m.value, m.label, m.zero])).toEqual([
      [24, "+24dB", false],
      [18, "+18dB", false],
      [12, "+12dB", false],
      [6, "+6dB", false],
      [0, "+0dB", true],
    ]);
  });
});
