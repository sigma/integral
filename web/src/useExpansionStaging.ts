/**
 * Edits to the Expansion slots staged on the Expansions page, applied in
 * one load. Lives at the app level so staging survives a tab switch.
 *
 * Staging rules (ExPCM fills all four slots, an Expansion moves out of its
 * old slot) and load plans come from the core via wasm.
 */

import { useCallback, useState } from "react";
import { LoadOutcomeKind, stageExpansionSlot } from "../pkg/integral_wasm.js";
import type { UseExpansionsResult } from "./useExpansions";

/** How the last load ended, when it needs telling. */
export type ExpansionLoadNotice =
  | { kind: "failed" }
  | { kind: "timedOut" }
  | { kind: "error"; message: string };

export interface UseExpansionStagingResult {
  /** The slot (0 = A) that picking an Expansion stages into. */
  selectedSlot: number;
  selectSlot: (slot: number) => void;
  /** Staged slot values A–D, or null with nothing staged. */
  staged: readonly number[] | null;
  /** Slots that differ from what is loaded (0 = A); empty if none. */
  changedSlots: number[];
  /** Stage the Expansion of `loadValue` (or OFF) into the selected slot. */
  stage: (loadValue: number) => void;
  /**
   * Select the slot preselected for another Expansion and stage the
   * Expansion of `loadValue` into it (the tone picker's Load action).
   */
  stageInPreselectedSlot: (loadValue: number) => void;
  /** Drop the staged edits. */
  discard: () => void;
  /** Parts Apply would silence, once asked to confirm; else null. */
  confirming: number[] | null;
  /** Apply the staged edits, asking first if Parts would go silent. */
  apply: () => void;
  /** How the last load ended, if it didn't load the target. */
  notice: ExpansionLoadNotice | null;
}

export function useExpansionStaging(
  expansions: UseExpansionsResult,
): UseExpansionStagingResult {
  const { slots, plan, load, preselectSlot } = expansions;
  const [selectedSlot, selectSlot] = useState(0);
  const [stagedState, setStaged] = useState<readonly number[] | null>(null);
  const [confirming, setConfirming] = useState<number[] | null>(null);
  const [notice, setNotice] = useState<ExpansionLoadNotice | null>(null);

  // Nothing can be staged over unknown slots.
  const staged = slots === null ? null : stagedState;
  const changedSlots = staged ? (plan(staged)?.changedSlots ?? []) : [];

  const stageInto = useCallback(
    (slot: number, loadValue: number) => {
      if (slots === null) return;
      const base = Uint8Array.from(stagedState ?? slots);
      setStaged(Array.from(stageExpansionSlot(base, slot, loadValue)));
      setConfirming(null);
      setNotice(null);
    },
    [slots, stagedState],
  );

  const stage = useCallback(
    (loadValue: number) => stageInto(selectedSlot, loadValue),
    [stageInto, selectedSlot],
  );

  const stageInPreselectedSlot = useCallback(
    (loadValue: number) => {
      if (slots === null) return;
      const slot = preselectSlot(stagedState ?? slots);
      selectSlot(slot);
      stageInto(slot, loadValue);
    },
    [slots, stagedState, preselectSlot, stageInto],
  );

  const discard = useCallback(() => {
    setStaged(null);
    setConfirming(null);
  }, []);

  const apply = useCallback(() => {
    if (!staged) return;
    const p = plan(staged);
    if (!p) return;
    if (p.silencedParts.length > 0 && confirming === null) {
      setConfirming(p.silencedParts);
      return;
    }
    setStaged(null);
    setConfirming(null);
    setNotice(null);
    load(staged).then(
      (result) => {
        if (result.outcome === LoadOutcomeKind.Failed) {
          setNotice({ kind: "failed" });
        } else if (result.outcome === LoadOutcomeKind.TimedOut) {
          setNotice({ kind: "timedOut" });
        }
      },
      (e: unknown) =>
        setNotice({
          kind: "error",
          message: e instanceof Error ? e.message : String(e),
        }),
    );
  }, [staged, plan, load, confirming]);

  return {
    selectedSlot,
    selectSlot,
    staged,
    changedSlots,
    stage,
    stageInPreselectedSlot,
    discard,
    confirming,
    apply,
    notice,
  };
}
