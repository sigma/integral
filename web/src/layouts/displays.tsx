/**
 * Displays drawn from their core bindings (#352): core says which fields a
 * Display reads, and the caller reads each slot's value for its Scope.
 */

import {
  DisplayKind,
  DisplayProp,
  ToneType,
  display_binding,
  type WasmDisplaySlot,
} from "../../pkg/integral_wasm.js";
import { BreakpointCurve } from "../synth-ui";

/** A field slot's raw value and range, as the caller resolves them for its Scope. */
export interface SlotValue {
  /** Current raw value. */
  value: number;
  /** Minimum raw value. */
  min: number;
  /** Maximum raw value. */
  max: number;
}

/** Resolves a field slot (`fixed` unset) for the Display's Scope. */
export type SlotReader = (slot: WasmDisplaySlot) => SlotValue;

const bindings = new Map<string, readonly WasmDisplaySlot[]>();

/** The slots of `kind` in the editor of `toneType`, from core; empty where it has none. */
export function displaySlots(kind: DisplayKind, toneType: ToneType): readonly WasmDisplaySlot[] {
  const id = `${kind}/${toneType}`;
  let slots = bindings.get(id);
  if (slots === undefined) {
    slots = display_binding(kind, toneType) ?? [];
    bindings.set(id, slots);
  }
  return slots;
}

/**
 * The first slot of `kind` feeding `prop` in the editor of `toneType`. Core
 * binds every prop a caller draws, so a missing one is a programming error.
 */
export function slotOf(kind: DisplayKind, toneType: ToneType, prop: DisplayProp): WasmDisplaySlot {
  const slot = displaySlots(kind, toneType).find((s) => s.prop === prop);
  if (slot === undefined) {
    throw new Error(`${DisplayKind[kind]} binds no ${DisplayProp[prop]} in the ${ToneType[toneType]} editor`);
  }
  return slot;
}

/** `field` of a slot (one of its getters), which the caller's block requires. */
export function slotField<F>(slot: WasmDisplaySlot, field: F | undefined): F {
  if (field === undefined) throw new Error(`${DisplayProp[slot.prop]} reads no field of the caller's block`);
  return field;
}

/**
 * A breakpoint envelope's curve, its times and levels read through core's
 * binding. A level the device fixes is placed on the range of the levels it
 * stores.
 */
export function BreakpointDisplay({
  kind,
  toneType,
  read,
}: {
  kind: DisplayKind;
  toneType: ToneType;
  read: SlotReader;
}) {
  const slots = displaySlots(kind, toneType);
  const normalized = (prop: DisplayProp): number[] => {
    const own = slots.filter((s) => s.prop === prop);
    const stored = own.find((s) => s.fixed === undefined);
    const range = stored === undefined ? { min: 0, max: 1 } : read(stored);
    return own.map((s) => {
      const { value, min, max } = s.fixed === undefined ? read(s) : { ...range, value: s.fixed };
      return (value - min) / (max - min || 1);
    });
  };
  return <BreakpointCurve times={normalized(DisplayProp.Times)} levels={normalized(DisplayProp.Levels)} />;
}
