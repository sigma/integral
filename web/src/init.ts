/**
 * What an Init resets (`docs/midi/99-undocumented.md` §7), as the web UI
 * names it: the area an "Init…" item asks for, the core machine that runs
 * it, and the words its confirmation shows. Parts are 0-indexed.
 */

import { Initialization, ToneType } from "../pkg/integral_wasm.js";
import { deviceSpec } from "./deviceSpec";
import { noteName } from "./useKeyboard";

/** An area of the Temporary memory an Init resets. */
export type InitScope =
  | { kind: "studio-set" }
  | { kind: "sound-ctrl" }
  | { kind: "part"; part: number }
  | { kind: "tone"; part: number }
  /** `partial` is the 0-based partial index for a tone, the MIDI key number for a drum kit. */
  | { kind: "partial"; part: number; toneType: ToneType; partial: number };

/** The Partial Init of `toneType` partial `partial` (an index, or a kit's key number) of Part `part`'s tone. */
export function partialInit(part: number, toneType: ToneType, partial: number): InitScope {
  return { kind: "partial", part, toneType, partial };
}

/** The core machine that sends `scope`'s Init and waits for "finished". */
export function initMachine(deviceId: number, scope: InitScope): Initialization {
  switch (scope.kind) {
    case "studio-set":
      return Initialization.studioSet(deviceId);
    case "sound-ctrl":
      return Initialization.soundCtrl(deviceId);
    case "part":
      return Initialization.part(deviceId, scope.part);
    case "tone":
      return Initialization.tone(deviceId, scope.part);
    case "partial":
      return Initialization.partial(deviceId, scope.part, scope.toneType, scope.partial);
  }
}

/** Whether a partial of `toneType` is a drum key (a kit) rather than a partial. */
function isKit(toneType: ToneType): boolean {
  return toneType === ToneType.PcmDrum || toneType === ToneType.SnDrum;
}

/**
 * Whether an Init of `running` resets all of `area`, which an `InitGuard`
 * blocks then. A Studio Set holds every Part; a Part holds its tone; a tone
 * holds its partials. The Sound Ctrl offsets sit in every Part but are
 * their own area: a Part's other settings stay live while they reset.
 */
export function covers(running: InitScope, area: InitScope): boolean {
  switch (running.kind) {
    case "studio-set":
      return true;
    case "sound-ctrl":
      return area.kind === "sound-ctrl";
    case "part":
      return area.kind !== "studio-set" && area.kind !== "sound-ctrl" && area.part === running.part;
    case "tone":
      return (area.kind === "tone" || area.kind === "partial") && area.part === running.part;
    case "partial":
      return area.kind === "partial" && area.part === running.part && area.partial === running.partial;
  }
}

/** The Init's title and what it resets, for its confirmation. */
export function describeInit(scope: InitScope): { title: string; resets: string } {
  const parts = deviceSpec().part_count;
  switch (scope.kind) {
    case "studio-set":
      return {
        title: "Init Studio Set",
        resets: `the Temporary Studio Set: its common settings and all ${parts} Parts. Every Part's tone reloads.`,
      };
    case "sound-ctrl":
      return { title: "Init Sound Ctrl", resets: `the Sound Ctrl offsets of all ${parts} Parts.` };
    case "part":
      return {
        title: `Init Part ${scope.part + 1}`,
        resets: `Part ${scope.part + 1}'s settings. Its tone reloads.`,
      };
    case "tone":
      return {
        title: `Init Part ${scope.part + 1}'s tone`,
        resets: `Part ${scope.part + 1}'s tone, which keeps its type.`,
      };
    case "partial":
      return isKit(scope.toneType)
        ? {
            title: `Init key ${noteName(scope.partial)}`,
            resets: `key ${noteName(scope.partial)} (${scope.partial}) of Part ${scope.part + 1}'s kit.`,
          }
        : {
            title: `Init Partial ${scope.partial + 1}`,
            resets: `Partial ${scope.partial + 1} of Part ${scope.part + 1}'s tone.`,
          };
  }
}
