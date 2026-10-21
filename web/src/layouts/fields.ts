/**
 * Field spaces: the parameters a layout may name, as core lists them.
 *
 * A layout names parameters and never defines them (ADR 0005). Tone and
 * mixer fields are wasm enum members, so a misspelt one fails `tsc`; FX
 * params are names from core's per-type tables. A space is core's explicit
 * list of either, which the validator checks every Control against, so a
 * field id core doesn't define (a cast number, a stale FX name) fails a test
 * instead of rendering a default.
 */

import {
  MixerField,
  MixerScopeKind,
  chorus_params,
  chorus_type_names,
  mfx_params,
  mfx_type_names,
  mixer_field_in_scope,
  mixer_field_spec,
  reverb_params,
  reverb_type_names,
  type WasmFieldSpec,
  type WasmFxParamDef,
} from "../../pkg/integral_wasm.js";

/** What a Control names its parameter by: a wasm enum member, or an FX param name. */
export type FieldKey = number | string;

/** The parameters one Section's layout may name, from core. */
export interface FieldSpace<F extends FieldKey> {
  /** Which space this is, for validator messages ("MixerField in Part, Master"). */
  readonly name: string;
  /** Every id core defines in this space; read lazily, so a space can be built before wasm starts. */
  readonly ids: readonly F[];
  /** A field's name in messages: its enum member name, or its FX param name. */
  nameOf(field: F): string;
  /** How many values the field takes, `max - min + 1`; sizes option lists. */
  optionCount(field: F): number;
  /**
   * How many elements the field holds: more than 1 for an array field (an
   * envelope's times or levels, the Step LFO's steps), which a Control names
   * one element of by `index`. Absent, every field holds one.
   */
  elementCount?(field: F): number;
}

/** How many elements `field` of `space` holds: 1 unless core says it is an array. */
export function elementCount<F extends FieldKey>(space: FieldSpace<F>, field: F): number {
  return space.elementCount?.(field) ?? 1;
}

/**
 * `compute` on first call, then its result. Layout modules build field spaces
 * at import, before the app has started the wasm core (`initWasm`), so a space
 * asks core only when a Section first reads it.
 */
export function memo<T>(compute: () => T): () => T {
  let value: { v: T } | undefined;
  return () => (value ??= { v: compute() }).v;
}

/** The members of a wasm-bindgen numeric enum, in declaration order. */
function enumMembers<E extends number>(enumObject: Record<string, string | number>): E[] {
  return Object.values(enumObject).filter((v): v is E => typeof v === "number");
}

/** Optional parts of an {@link enumFields} space. */
export interface EnumFieldsOptions<E extends number> {
  /** Which members the space holds; default all. */
  keep?: (field: E) => boolean;
  /** Core's element count of each field, for an enum with array fields; default 1 each. */
  count?: (field: E) => number;
}

/**
 * The space of a wasm field enum: every member, with its range from `spec`.
 * Tone Sections use this with their enum and spec lookup, e.g.
 * `enumFields("SnsPartialField", SnsPartialField, sns_partial_spec)`. An enum
 * with array fields passes core's `count`, e.g. `{ count: pcms_partial_count }`.
 */
export function enumFields<E extends number>(
  name: string,
  enumObject: Record<string, string | number>,
  spec: (field: E) => WasmFieldSpec,
  { keep = () => true, count }: EnumFieldsOptions<E> = {},
): FieldSpace<E> {
  const ids = memo(() => enumMembers<E>(enumObject).filter(keep));
  return {
    ...(count && { elementCount: count }),
    name,
    get ids() {
      return ids();
    },
    nameOf: (field) => String(enumObject[field] ?? field),
    optionCount: (field) => {
      const { min, max } = spec(field);
      return max - min + 1;
    },
  };
}

/**
 * The mixer fields that exist in every one of `kinds`: a Section serving
 * several strip kinds may only name fields all of them have.
 */
export function mixerFields(kinds: readonly MixerScopeKind[]): FieldSpace<MixerField> {
  return enumFields<MixerField>(
    `MixerField in ${kinds.map((k) => MixerScopeKind[k]).join(", ")}`,
    MixerField,
    mixer_field_spec,
    { keep: (field) => kinds.every((kind) => mixer_field_in_scope(field, kind)) },
  );
}

/** Whether `field` takes exactly two values: a 2×1 switch can draw it. */
export function isTwoState<F extends FieldKey>(space: FieldSpace<F>, field: F): boolean {
  return space.optionCount(field) === 2;
}

/** An FX block whose params core lists per type. */
export type FxBlock = "mfx" | "chorus" | "reverb";

const FX_PARAMS: Record<FxBlock, (type: number) => WasmFxParamDef[]> = {
  mfx: mfx_params,
  chorus: chorus_params,
  reverb: reverb_params,
};

const FX_TYPE_NAMES: Record<FxBlock, () => string[]> = {
  mfx: mfx_type_names,
  chorus: chorus_type_names,
  reverb: reverb_type_names,
};

/** Every type of an FX block, as core enumerates them: `0..count`. */
export function fxTypes(block: FxBlock): number[] {
  return FX_TYPE_NAMES[block]().map((_, i) => i);
}

/** Core's param defs of one FX type, in its table's order. */
export function fxParamDefs(block: FxBlock, type: number): WasmFxParamDef[] {
  return FX_PARAMS[block](type);
}

/** The params of one FX type, named as core's generated table names them. */
export function fxFields(block: FxBlock, type: number): FieldSpace<string> {
  const defs = memo(() => new Map(fxParamDefs(block, type).map((d) => [d.name, d])));
  const ids = memo(() => [...defs().keys()]);
  return {
    name: `${block} type ${type}`,
    get ids() {
      return ids();
    },
    nameOf: (field) => field,
    optionCount: (field) => {
      const def = defs().get(field);
      return def === undefined ? 0 : def.max - def.min + 1;
    },
  };
}
