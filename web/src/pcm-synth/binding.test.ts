import { describe, expect, it } from "vitest";
import { PcmsPartialField } from "../../pkg/integral_wasm.js";
import { partialBinding } from "./binding";

/** The binding of `field` at `value`, and the values it writes. */
function bound(field: PcmsPartialField, value: number, index = 0) {
  const writes: number[] = [];
  return { binding: partialBinding(field, index, value, (v) => writes.push(v)), writes };
}

describe("partialBinding", () => {
  it("ranges a synced rate over its free values in Hz mode", () => {
    const { binding } = bound(PcmsPartialField.Lfo1Rate, 10);
    expect(binding.sync?.noteMode).toBe(false);
    expect(binding.sync?.labels).toEqual(["Hz", "♪"]);
    expect([binding.min, binding.max]).toEqual([0, 127]);
  });

  it("ranges it over the notes in note mode, named by core", () => {
    const { binding } = bound(PcmsPartialField.Lfo1Rate, 140);
    expect(binding.sync?.noteMode).toBe(true);
    expect([binding.min, binding.max]).toEqual([128, 149]);
    expect(binding.format?.(140)).toBe("1/4");
  });

  it("switches mode from that mode's default, and ignores a switch to the mode it is in", () => {
    const free = bound(PcmsPartialField.Lfo1Rate, 10);
    free.binding.sync!.onChange(true);
    free.binding.sync!.onChange(false);
    expect(free.writes).toEqual([140]);
    const note = bound(PcmsPartialField.Lfo1Rate, 140);
    note.binding.sync!.onChange(false);
    expect(note.writes).toEqual([92]); // INIT TONE's LFO1 Rate
  });

  it("labels the partial Delay in ms", () => {
    expect(bound(PcmsPartialField.DelayTime, 0).binding.sync?.labels).toEqual(["ms", "♪"]);
  });

  it("carries no sync for a plain field, and names an array field's element", () => {
    expect(bound(PcmsPartialField.Level, 0).binding.sync).toBeUndefined();
    expect(bound(PcmsPartialField.LfoStep, 64, 2).binding.name).toBe("Step LFO step (3 of 16)");
  });
});
