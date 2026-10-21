/**
 * The fields SN Acoustic layouts name: Common fields by their wasm enum, and
 * an instrument's Modify Parameters by their slot (1-32), as core lists them
 * per instrument (`sna_inst_params.rs`).
 */

import { SnaCommonField, sna_common_spec, sna_inst_params } from "../../../pkg/integral_wasm.js";
import { snaInsts, type SnaInst } from "../../snaInstCatalog";
import { enumFields, type FieldSpace } from "../fields";

/**
 * The Common fields a tone-wide Section may name. Modify Parameter is one
 * field with 32 slots; the INST variant set names its slots instead.
 */
export const SNA_COMMON_FIELDS: FieldSpace<SnaCommonField> = enumFields<SnaCommonField>(
  "SnaCommonField",
  SnaCommonField,
  sna_common_spec,
  { keep: (field) => field !== SnaCommonField.ModifyParameter },
);

/** One Modify Parameter of an instrument, from core's list. */
export interface ModifyParamDef {
  /** Its slot, 1-32: Modify Parameter `slot`. */
  slot: number;
  /** Core's name for it. */
  name: string;
  /** Minimum raw value. */
  min: number;
  /** Maximum raw value. */
  max: number;
  /** Default raw value. */
  defaultValue: number;
}

/** The Modify Parameters of the instrument at `variation`, `number`; none for an unlisted one. */
export function modifyParams(variation: number, number: number): ModifyParamDef[] {
  const raw: unknown = sna_inst_params(variation, number);
  if (!Array.isArray(raw)) return [];
  return (raw as [number, string, number, number, number][]).map(([slot, name, min, max, defaultValue]) => ({
    slot,
    name,
    min,
    max,
    defaultValue,
  }));
}

/** The instrument at picker number `key` (1-based list position); `undefined` past the list. */
export function instOf(key: number): SnaInst | undefined {
  return snaInsts()[key - 1];
}

/** Instrument `key`'s Modify Parameters by slot; none past the list. */
export function modifyDefs(key: number): Map<number, ModifyParamDef> {
  const inst = instOf(key);
  return new Map(inst ? modifyParams(inst.variation, inst.instNumber).map((d) => [d.slot, d]) : []);
}

/** Whether `def` takes two values, so a switch draws it. */
export function isTwoState(def: ModifyParamDef): boolean {
  return def.max - def.min === 1;
}

/** The Modify Parameter slots instrument `key` uses, named and ranged by core. */
export function modifyFields(key: number): FieldSpace<number> {
  const defs = modifyDefs(key);
  return {
    name: `Modify Parameters of ${instOf(key)?.name ?? `instrument ${key}`}`,
    ids: [...defs.keys()],
    nameOf: (slot) => defs.get(slot)?.name ?? `slot ${slot}`,
    optionCount: (slot) => {
      const def = defs.get(slot);
      return def === undefined ? 0 : def.max - def.min + 1;
    },
  };
}
