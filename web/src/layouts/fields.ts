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
  mfx_params,
  mixer_field_in_scope,
  mixer_field_spec,
  reverb_params,
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

/** An FX block whose params core lists per type. */
export type FxBlock = "mfx" | "chorus" | "reverb";

const FX_PARAMS: Record<FxBlock, (type: number) => WasmFxParamDef[]> = {
  mfx: mfx_params,
  chorus: chorus_params,
  reverb: reverb_params,
};

/** The params of one FX type, named as core's generated table names them. */
export function fxFields(block: FxBlock, type: number): FieldSpace<string> {
  const defs = memo(() => new Map(FX_PARAMS[block](type).map((d) => [d.name, d])));
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
