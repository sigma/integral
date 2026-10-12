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

describe("SequencerEngine Cursor", () => {
  it("stays inside a shorter Track after switching Part", () => {
    const engine = new SequencerEngine();
    engine.edit((seq) => seq.setTrackLength(1, 8));
    engine.setCursor(12);
    engine.setTrack(1);
    expect(engine.getSnapshot().cursor).toBe(7);
  });

  it("stays inside the Track after lowering its length", () => {
    const engine = new SequencerEngine();
    engine.setCursor(12);
    engine.edit((seq) => seq.setTrackLength(0, 8));
    expect(engine.getSnapshot().cursor).toBe(7);
  });

  it("clamps a click past the Track's length to its last Step", () => {
    const engine = new SequencerEngine();
    engine.edit((seq) => seq.setTrackLength(0, 8));
    engine.setCursor(20);
    expect(engine.getSnapshot().cursor).toBe(7);
  });

  it("never writes Step input past the Track's length", () => {
    const engine = new SequencerEngine();
    engine.setCursor(12);
    engine.setArmed(true);
    engine.edit((seq) => seq.setTrackLength(0, 8));
    engine.noteDown(60);
    engine.noteUp(60);
    expect(engine.getSnapshot().tracks[0]?.steps.map((s) => s.step)).toEqual([7]);
  });
});
