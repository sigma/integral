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

/**
 * The space of a wasm field enum: every member, with its range from `spec`.
 * Tone Sections use this with their enum and spec lookup, e.g.
 * `enumFields("SnsPartialField", SnsPartialField, sns_partial_spec)`.
 */
export function enumFields<E extends number>(
  name: string,
  enumObject: Record<string, string | number>,
  spec: (field: E) => WasmFieldSpec,
  keep: (field: E) => boolean = () => true,
): FieldSpace<E> {
  const ids = memo(() => enumMembers<E>(enumObject).filter(keep));
  return {
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
    (field) => kinds.every((kind) => mixer_field_in_scope(field, kind)),
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

/**
 * The space of a wasm field enum whose fields may hold several values (an
 * envelope's times or levels), with a key per value: a single-valued field
 * is its enum member, value `i` of a field holding several is
 * `element(field, i)`. `count` is core's number of values per field.
 */
export interface ElementFieldSpace<E extends number> extends FieldSpace<E | string> {
  /** The key of value `index` of `field`; past the field's count, a key outside `ids`. */
  element(field: E, index: number): string;
  /** The field and value a key names; `undefined` for a key not in `ids`. */
  resolve(key: E | string): { field: E; index: number } | undefined;
}

/** The {@link ElementFieldSpace} of a wasm field enum, each field holding `count(field)` values. */
export function elementFields<E extends number>(
  name: string,
  enumObject: Record<string, string | number>,
  spec: (field: E) => WasmFieldSpec,
  count: (field: E) => number,
): ElementFieldSpace<E> {
  const memberName = (field: E): string => String(enumObject[field] ?? field);
  const keyOf = (field: E, index: number): string => `${memberName(field)}[${index}]`;
  // Counts come from core, so they are read on first use, not at import.
  const resolved = memo(() => {
    const out = new Map<E | string, { field: E; index: number }>();
    for (const field of enumMembers<E>(enumObject)) {
      const n = count(field);
      if (n === 1) out.set(field, { field, index: 0 });
      else for (let i = 0; i < n; i++) out.set(keyOf(field, i), { field, index: i });
    }
    return out;
  });
  const ids = memo(() => [...resolved().keys()]);
  return {
    name,
    get ids() {
      return ids();
    },
    nameOf: (key) => (typeof key === "string" ? key : memberName(key)),
    optionCount: (key) => {
      const r = resolved().get(key);
      if (r === undefined) return 0;
      const { min, max } = spec(r.field);
      return max - min + 1;
    },
    element: keyOf,
    resolve: (key) => resolved().get(key),
  };
}
