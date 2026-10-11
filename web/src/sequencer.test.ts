import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SequencerEngine } from "./sequencer";

/** jsdom has no Worker or blob URLs; the engine only needs them to exist. */
beforeEach(() => {
  vi.stubGlobal(
    "Worker",
    class {
      onmessage: (() => void) | null = null;
      postMessage(): void {}
    },
  );
  vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:tick");
  localStorage.clear();
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("SequencerEngine with no device", () => {
  it("autosaves edits so a reload restores them", () => {
    new SequencerEngine().edit((seq) => seq.setStep(2, 3, Uint8Array.of(60, 64), 90, 2));

    const steps = new SequencerEngine().getSnapshot().tracks[2]?.steps;
    expect(steps).toEqual([
      {
        step: 3,
        notes: [
          { pitch: 60, velocity: 90, length: 2 },
          { pitch: 64, velocity: 90, length: 2 },
        ],
      },
    ]);
  });

  it("exports a .mid file it can load back", () => {
    const engine = new SequencerEngine();
    engine.edit((seq) => seq.setStep(0, 0, Uint8Array.of(48), 100, 1));
    const bytes = engine.toSmf();

    expect(new TextDecoder().decode(bytes.subarray(0, 4))).toBe("MThd");
    const other = new SequencerEngine();
    other.loadSmf(bytes);
    expect(other.getSnapshot().tracks).toEqual(engine.getSnapshot().tracks);
  });
});
