/**
 * A mixer channel strip on the Grid: its EQ Section, its strip Section (a
 * variant of `STRIP_SET` by strip kind) and its name plate. Each binds the
 * strip's Scope: a part, the external input, the master or a Drum Comp+EQ
 * unit. Values come from the mixer state, and every write goes through
 * `setMixerField` in that Scope.
 */

import {
  MixerField,
  MixerScopeKind,
  comp_eq_unit_is_active,
  mixer_is_active,
  mixer_output_is_active,
} from "../pkg/integral_wasm.js";
import { CATEGORIES, lookupToneCategory } from "./categories";
import { categoryIcon, stripIcon } from "./categoryIcons";
import css from "./ChannelStrip.module.css";
import { GridSection, variantLayout, type Binding, type Scope, type SectionLayout } from "./layouts";
import { EQ_LAYOUT } from "./layouts/mixer/eq";
import { PART_TOGGLES, STRIP_SET, STRIP_TITLES } from "./layouts/mixer/strip";
import { renderMixerDisplay } from "./mixerDisplays";
import { NotLoadedMarker } from "./NotLoadedMarker";
import { paramFormatter, paramMeta } from "./paramDefaults";
import type { CompEqUnit, EqState, MixerState, PartState } from "./types";

/** Writes a mixer field in a Scope (`useMixer`'s `setMixerField`). */
export type SetMixerField = (kind: MixerScopeKind, index: number, field: MixerField, value: number) => void;

/** Which strip: its Scope kind and index (the part or Comp+EQ unit; 0 otherwise). */
export interface StripScope {
  kind: MixerScopeKind;
  index: number;
}

/** Props shared by a strip's Sections. */
interface StripProps extends StripScope {
  /** The mixer state the strip reads. */
  state: MixerState;
  /** Writes a field in the strip's Scope. */
  setField: SetMixerField;
}

/** The `EqState` member holding each EQ knob's value. */
const EQ_STATE_KEY: Partial<Record<MixerField, Exclude<keyof EqState, "enabled">>> = {
  [MixerField.EqLowFreq]: "lowFreq",
  [MixerField.EqLowGain]: "lowGain",
  [MixerField.EqMidFreq]: "midFreq",
  [MixerField.EqMidGain]: "midGain",
  [MixerField.EqMidQ]: "midQ",
  [MixerField.EqHighFreq]: "highFreq",
  [MixerField.EqHighGain]: "highGain",
};

/** The `CompEqUnit` member holding each compressor field's value. */
const COMP_STATE_KEY: Partial<Record<MixerField, Exclude<keyof CompEqUnit, "compSwitch" | "eqSwitch">>> = {
  [MixerField.CompAttack]: "compAttack",
  [MixerField.CompRelease]: "compRelease",
  [MixerField.CompThreshold]: "compThreshold",
  [MixerField.CompRatio]: "compRatio",
  [MixerField.CompOutputGain]: "compOutputGain",
};

/** The `PartState` member holding each part field's value. */
const PART_STATE_KEY: Partial<Record<MixerField, keyof PartState>> = {
  [MixerField.PartLevel]: "level",
  [MixerField.PartPan]: "pan",
  [MixerField.PartMute]: "muted",
  [MixerField.PartChorusSend]: "chorusSend",
  [MixerField.PartReverbSend]: "reverbSend",
  [MixerField.PartOutputAssign]: "outputAssign",
  [MixerField.PartReceiveChannel]: "receiveChannel",
};

/** Fields read in dB; core's labels carry the sign only. */
const DB_FIELDS: ReadonlySet<MixerField> = new Set([
  MixerField.EqLowGain,
  MixerField.EqMidGain,
  MixerField.EqHighGain,
  MixerField.CompOutputGain,
]);

/** What a strip's toggle buttons read, on and off alike: their LED shows the state. */
const SWITCH_TEXT: Partial<Record<MixerField, string>> = {
  [MixerField.PartMute]: "M",
  [MixerField.ExtMute]: "M",
  [MixerField.SoloPart]: "S",
  [MixerField.DrumCompEqPart]: "C+EQ",
  [MixerField.CompSwitch]: "COMP",
};

/** A strip's EQ, as the EQ Section reads it. */
function eqState(state: MixerState, { kind, index }: StripScope): EqState {
  if (kind === MixerScopeKind.Master) return state.masterEq;
  if (kind === MixerScopeKind.CompEqUnit) {
    const u = state.drumCompEq.units[index]!;
    return {
      enabled: u.eqSwitch,
      lowFreq: u.eqLowFreq,
      lowGain: u.eqLowGain,
      midFreq: u.eqMidFreq,
      midGain: u.eqMidGain,
      midQ: u.eqMidQ,
      highFreq: u.eqHighFreq,
      highGain: u.eqHighGain,
    };
  }
  return state.parts[index]!.eq;
}

/** `eq`'s raw value of EQ field `field`, or `undefined` for another field. */
function eqValue(eq: EqState, field: MixerField): number | undefined {
  if (field === MixerField.EqSwitch) return Number(eq.enabled);
  const key = EQ_STATE_KEY[field];
  return key === undefined ? undefined : eq[key];
}

/**
 * The raw value of `field` on strip `scope` in `state`. A part's toggles read
 * on while their Common field names the part.
 */
export function stripValue(state: MixerState, scope: StripScope, field: MixerField): number {
  const { kind, index } = scope;
  const eq = kind === MixerScopeKind.Ext ? undefined : eqValue(eqState(state, scope), field);
  if (eq !== undefined) return eq;
  const found = ((): number | boolean | undefined => {
    switch (kind) {
      case MixerScopeKind.Part: {
        if (field === MixerField.PartAmbienceSend) return state.surround.parts[index]?.ambienceSend;
        if (field === MixerField.SoloPart) return state.soloPart === index + 1;
        if (field === MixerField.DrumCompEqPart) return state.drumCompEq.enabled && state.drumCompEq.part === index;
        const key = PART_STATE_KEY[field];
        const value = key === undefined ? undefined : state.parts[index]![key];
        return typeof value === "number" || typeof value === "boolean" ? value : undefined;
      }
      case MixerScopeKind.Ext:
        if (field === MixerField.ExtLevel) return state.extLevel;
        return field === MixerField.ExtMute ? state.extMuted : undefined;
      case MixerScopeKind.Master:
        return field === MixerField.MasterLevel ? state.masterLevel : undefined;
      case MixerScopeKind.CompEqUnit: {
        if (field === MixerField.CompOutputAssign) return state.drumCompEq.outputAssigns[index];
        const u = state.drumCompEq.units[index]!;
        if (field === MixerField.CompSwitch) return u.compSwitch;
        const key = COMP_STATE_KEY[field];
        return key === undefined ? undefined : u[key];
      }
      default:
        return undefined;
    }
  })();
  if (found === undefined) throw new Error(`${MixerField[field]} is not on a ${MixerScopeKind[kind]} strip`);
  return Number(found);
}

/**
 * Writes `value` to `field` on strip `scope`. A part's toggles write their
 * Common field: Solo names the part or none; the Comp+EQ assignment names
 * the part and switches the Comp+EQ on, or switches it off.
 */
function stripWrite(setField: SetMixerField, { kind, index }: StripScope, field: MixerField, value: number): void {
  if (kind !== MixerScopeKind.Part || !PART_TOGGLES.has(field)) {
    setField(kind, index, field, value);
    return;
  }
  const common = (f: MixerField, v: number) => setField(MixerScopeKind.Common, 0, f, v);
  if (field === MixerField.SoloPart) {
    common(MixerField.SoloPart, value ? index + 1 : 0);
  } else if (value) {
    common(MixerField.DrumCompEqPart, index);
    common(MixerField.DrumCompEqSwitch, 1);
  } else {
    common(MixerField.DrumCompEqSwitch, 0);
  }
}

/** Binds a strip's fields, from core's names and ranges, to `state`, writing through `setField`. */
function stripBinding({ state, setField, ...scope }: StripProps): (field: MixerField) => Binding {
  return (field) => {
    const meta = paramMeta(field);
    const fmt = paramFormatter(field);
    const unit = DB_FIELDS.has(field) ? "dB" : "";
    const text = SWITCH_TEXT[field];
    const toggle = scope.kind === MixerScopeKind.Part && PART_TOGGLES.has(field);
    return {
      ...meta,
      // A part's toggle is two-state, whatever its Common field's range.
      ...(toggle ? { min: 0, max: 1, defaultValue: 0 } : {}),
      // Pan's arc grows from the centre, its default.
      center: field === MixerField.PartPan ? meta.defaultValue : undefined,
      value: stripValue(state, scope, field),
      format: (v) => `${fmt(v)}${unit}`,
      options: text === undefined ? undefined : [{ value: 0, label: text }, { value: 1, label: text }],
      onChange: (v) => stripWrite(setField, scope, field, v),
    };
  };
}

/** The scope a strip's titles read: its part or unit. */
function titleScope({ kind, index }: StripScope): Scope {
  return kind === MixerScopeKind.CompEqUnit ? { unit: index } : { part: index };
}

/** The EQ Section's accent: the family of its gain knobs. */
const eqFamily = () => paramMeta(MixerField.EqMidGain).family;

/** A strip's EQ Section. */
export function StripEq(props: StripProps) {
  return <GridSection layout={EQ_LAYOUT} bind={stripBinding(props)} family={eqFamily()} />;
}

/** Each strip kind's layout, titled for its kind. */
const STRIP_LAYOUTS = new Map<MixerScopeKind, SectionLayout<MixerField>>(
  STRIP_SET.keys.map((kind) => {
    const layout = variantLayout(STRIP_SET, kind);
    return [kind, { ...layout, title: STRIP_TITLES[kind] ?? layout.title }];
  }),
);

/** The strip Section's accent: the family of its level fader. */
const stripFamily = () => paramMeta(MixerField.PartLevel).family;

/** Whether core counts `field` active on strip `scope` in `state`. */
function stripIsActive(state: MixerState, { kind, index }: StripScope, field: MixerField): boolean {
  if (!mixer_output_is_active(field, state.surround.enabled)) return false;
  if (kind === MixerScopeKind.Part) return mixer_is_active(field, index, state.parts[index], state.drumCompEq);
  if (kind === MixerScopeKind.CompEqUnit) return comp_eq_unit_is_active(field, state.drumCompEq.units[index]);
  return true;
}

/** A strip's Section: everything but its EQ. */
export function ChannelStrip(props: StripProps) {
  const { state, kind } = props;
  const bind = stripBinding(props);
  return (
    <GridSection
      layout={STRIP_LAYOUTS.get(kind)!}
      bind={bind}
      scope={titleScope(props)}
      family={stripFamily()}
      isActive={(field) => stripIsActive(state, props, field)}
      // Surround replaces the FX sends with its ambience send.
      alternative={(group) => (group === "sends" && state.surround.enabled ? "surround" : undefined)}
      renderDisplay={(display, size) => renderMixerDisplay(display, size, (field) => bind(field).value)}
    />
  );
}

/** A part's tone name, or its bank and program while the name is unknown. */
function toneLabel(part: PartState): string {
  return part.toneName || `${part.toneBankMsb}-${part.toneBankLsb}-${part.tonePC + 1}`;
}

/** The icon and role each non-part strip's name plate shows. */
const ROLE: Partial<Record<MixerScopeKind, { icon: string; label: (index: number) => string }>> = {
  [MixerScopeKind.Ext]: { icon: "ext", label: () => "External Input" },
  [MixerScopeKind.Master]: { icon: "master", label: () => "Master" },
  [MixerScopeKind.CompEqUnit]: { icon: "comp-eq", label: (i) => `Compressor ${i + 1}` },
};

/** Props of {@link StripName}. */
interface StripNameProps extends StripScope {
  state: MixerState;
  /** The Expansion of a part's tone, when known not to be loaded. */
  unloadedExpansion?: string | null;
}

/** A strip's name plate: a part's tone and category, or the strip's role. */
export function StripName({ state, kind, index, unloadedExpansion }: StripNameProps) {
  let icon = "";
  let name: string;
  let category: string | undefined;
  if (kind === MixerScopeKind.Part) {
    const p = state.parts[index]!;
    // The tone's own category covers user tones; the factory table fills in until it's read.
    const catId = p.toneCategory || lookupToneCategory(p.toneBankMsb, p.toneBankLsb, p.tonePC);
    if (catId !== 0 && CATEGORIES[catId]) {
      icon = categoryIcon(catId) ?? "";
      category = CATEGORIES[catId];
    }
    name = toneLabel(p);
  } else {
    const role = ROLE[kind];
    icon = role ? stripIcon(role.icon) : "";
    name = role ? role.label(index) : "";
  }
  return (
    <div className={css.namePlate} title={name}>
      <span className={css.toneName}>
        {/* Ahead of the tone name, so a long name can't clip it. */}
        {unloadedExpansion && <NotLoadedMarker expansion={unloadedExpansion} />}
        {name}
      </span>
      <div className={css.categoryRow}>
        {icon && <div className={css.categoryIcon} dangerouslySetInnerHTML={{ __html: icon }} />}
        {category && <span className={css.categoryLabel}>{category}</span>}
      </div>
    </div>
  );
}
