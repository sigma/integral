/**
 * A mixer channel strip below its EQ: one variant set keyed by strip kind
 * (the mixer Scope kinds that draw a strip), all in one 4×36 footprint so
 * every strip's fader starts and ends on the same row.
 *
 * The proportions are the pre-Grid mixer's (#392): its level faders were
 * about 21 Cells long for a 4-Cell strip, so each strip is a fixed 36 rows,
 * a 21-row fader under the 15 rows the Comp+EQ unit's compressor needs. A
 * part strip's Controls sit just over its fader, leaving its top rows empty,
 * as the old mixer did, but for its input picker on the first row; its name
 * runs up the left of its fader. Controls in the strip's edge columns (M, S, the
 * Comp+EQ Displays) are drawn clear of the border by the renderer.
 *
 * The EQ is its own Section (`eq.ts`), stacked above the strip by the mixer
 * screen, so a strip places every field of its Scope except the EQ's.
 *
 * Two Common fields are drawn per part, as that part's own two-state
 * switch: Solo (`SoloPart` names this part) and the Drum Comp+EQ assignment
 * (`DrumCompEqPart` names this part while `DrumCompEqSwitch` is on). The
 * caller's binding maps them to on/off; the field space counts them as two
 * states. These are the strip's documented exceptions to "a field of the
 * Section's Scope".
 */

import {
  DisplayKind,
  FxRole,
  MixerField,
  MixerScopeKind,
  ParamGroup,
  mixer_field_group,
  mixer_field_in_scope,
  mixer_fx_role,
} from "../../../pkg/integral_wasm.js";
import { optionListRows } from "../../synth-ui/grid";
import { memo, mixerFields, type FieldSpace } from "../fields";
import { allElements } from "../geometry";
import type { ItemLayout, SectionBody, VariantSet } from "../types";
import { EQ_LAYOUT } from "./eq";

/** The strip kinds: every mixer Scope kind but the Studio Set common block, from core's enum. */
export const STRIP_KINDS: readonly MixerScopeKind[] = Object.values(MixerScopeKind).filter(
  (k): k is MixerScopeKind => typeof k === "number" && k !== MixerScopeKind.Common,
);

/** Common fields a part strip draws as the part's own switch: on while the field names the part. */
export const PART_TOGGLES: ReadonlySet<MixerField> = new Set([MixerField.SoloPart, MixerField.DrumCompEqPart]);

/** Fields the EQ Section places; the strip leaves them to it. */
const EQ_PLACED: ReadonlySet<MixerField> = new Set([
  ...allElements(EQ_LAYOUT.items).flatMap((el) => (el.kind === "display" ? [] : [el.field])),
  ...(EQ_LAYOUT.titleSwitch === undefined ? [] : [EQ_LAYOUT.titleSwitch]),
]);

/** Surround position fields a part strip draws: its ambience send, in place of its FX sends. */
const PART_SURROUND: ReadonlySet<MixerField> = new Set([MixerField.SurroundAmbienceSend]);

/** A Motional Surround position field: one both a part and the Ext Part have, which the Surround page places. */
function isSurroundPosition(field: MixerField): boolean {
  return mixer_field_in_scope(field, MixerScopeKind.Part) && mixer_field_in_scope(field, MixerScopeKind.Ext);
}

/**
 * The fields a strip of `kind` places: its Scope's, less the EQ's and the
 * surround positions, plus a part's toggles and ambience send.
 */
export function stripFields(kind: MixerScopeKind): FieldSpace<MixerField> {
  // Every mixer field, with core's names and ranges.
  const all = mixerFields([]);
  const part = kind === MixerScopeKind.Part;
  const toggles = part ? PART_TOGGLES : new Set<MixerField>();
  const ids = memo(() =>
    all.ids.filter(
      (field) =>
        toggles.has(field) ||
        (part && PART_SURROUND.has(field)) ||
        (mixer_field_in_scope(field, kind) && !EQ_PLACED.has(field) && !isSurroundPosition(field)),
    ),
  );
  return {
    name: `strip fields of ${MixerScopeKind[kind]}`,
    get ids() {
      return ids();
    },
    nameOf: all.nameOf,
    optionCount: (field) => (toggles.has(field) ? 2 : all.optionCount(field)),
  };
}

/** Each non-part strip's title, in place of the set's; `{unit}` is filled from its Scope. */
export const STRIP_TITLES: Readonly<Partial<Record<MixerScopeKind, string>>> = {
  [MixerScopeKind.Ext]: "EX",
  [MixerScopeKind.Master]: "Master",
  [MixerScopeKind.CompEqUnit]: "C+E{unit}",
};

/** The row every level fader starts on, and its height: it ends on the strip's last row. */
const FADER = { y: 15, h: 21 } as const;

/** Rows of a part strip's Controls over its fader: Pan, the sends, Out, C+EQ, then M and S. */
const PART_CONTROLS_H = 9;

/** A strip's name, up the left of its level fader (the column beside it), as the old mixer had it. */
const NAME = { kind: "display", display: DisplayKind.StripName, x: 0, y: FADER.y, w: 1, h: FADER.h } as const;

/** The first row of a part strip's Controls, so they end just over its fader. */
const TOP = FADER.y - PART_CONTROLS_H;

/** The row of every strip's output picker. */
const OUT_Y = TOP + 6;

/**
 * A part strip: its input (receive channel) under its part number, the
 * counterpart of its output; then pan, the sends (or the ambience send),
 * routing, mute and solo side by side, level.
 */
const PART: SectionBody<MixerField> = {
  items: [
    { kind: "picker", field: MixerField.PartReceiveChannel, x: 0, y: 0, w: 4, label: "In" },
    { kind: "knob", field: MixerField.PartPan, x: 1, y: TOP, label: "Pan" },
    // Surround replaces the FX sends with its ambience send.
    {
      kind: "alternatives",
      group: "sends",
      x: 0,
      y: TOP + 3,
      w: 4,
      h: 3,
      options: [
        {
          name: "fx",
          items: [
            { kind: "knob", field: MixerField.PartChorusSend, x: 0, y: TOP + 3 },
            { kind: "knob", field: MixerField.PartReverbSend, x: 2, y: TOP + 3 },
          ],
        },
        { name: "surround", items: [{ kind: "knob", field: MixerField.SurroundAmbienceSend, x: 1, y: TOP + 3, label: "AMB" }] },
      ],
    },
    // The output, as the old mixer's route chip: the current output's name, opening the list.
    { kind: "picker", field: MixerField.PartOutputAssign, x: 0, y: OUT_Y, w: 4, label: "Out" },
    { kind: "switch", field: MixerField.DrumCompEqPart, x: 1, y: TOP + 7, label: false },
    { kind: "switch", field: MixerField.PartMute, x: 0, y: TOP + 8, label: false },
    { kind: "switch", field: MixerField.SoloPart, x: 2, y: TOP + 8, label: false },
    { kind: "fader", field: MixerField.PartLevel, x: 1, ...FADER, label: false, scale: true },
    NAME,
  ],
};

/** The external input: mute, centred over its level. */
const EXT: SectionBody<MixerField> = {
  items: [
    { kind: "switch", field: MixerField.ExtMute, x: 1, y: FADER.y - 1, label: false },
    { kind: "fader", field: MixerField.ExtLevel, x: 1, ...FADER, label: false, scale: true },
    NAME,
  ],
};

/** The master: its level. */
const MASTER: SectionBody<MixerField> = {
  items: [{ kind: "fader", field: MixerField.MasterLevel, x: 1, ...FADER, label: false, scale: true }, NAME],
};

/**
 * A Drum Comp+EQ unit's compressor, as in the old mixer: the switch (a
 * power toggle in the title, "C+E1 ⏻"), the envelope across the strip over attack and release on short faders, the
 * transfer curve across the strip over threshold and ratio, the routing, and
 * the output gain on a level-length fader.
 */
const COMP_EQ_UNIT: SectionBody<MixerField> = {
  titleSwitch: MixerField.CompSwitch,
  items: [
    { kind: "display", display: DisplayKind.CompEnvelope, x: 0, y: 1, w: 4, h: 2 },
    { kind: "fader", field: MixerField.CompAttack, x: 0, y: 3, h: 3, label: "Atk" },
    { kind: "fader", field: MixerField.CompRelease, x: 2, y: 3, h: 3, label: "Rel" },
    { kind: "display", display: DisplayKind.CompTransferCurve, x: 0, y: 6, w: 4, h: 3 },
    { kind: "knob", field: MixerField.CompThreshold, x: 0, y: 9, label: "Thr" },
    { kind: "knob", field: MixerField.CompRatio, x: 2, y: 9 },
    { kind: "picker", field: MixerField.CompOutputAssign, x: 0, y: OUT_Y, w: 4, label: "Out" },
    { kind: "fader", field: MixerField.CompOutputGain, x: 1, ...FADER, label: false, scale: true },
    NAME,
  ],
};

/** The strip below each mixer EQ, by strip kind. */
export const STRIP_SET: VariantSet<MixerScopeKind, MixerField> = {
  name: "mixer strip",
  // A part strip's title is its part number.
  title: "{part}",
  titleEdge: "top",
  w: 4,
  h: FADER.y + FADER.h,
  keys: STRIP_KINDS,
  fieldsOf: stripFields,
  variants: new Map([
    [MixerScopeKind.Part, PART],
    [MixerScopeKind.Ext, EXT],
    [MixerScopeKind.Master, MASTER],
    [MixerScopeKind.CompEqUnit, COMP_EQ_UNIT],
  ]),
  generate: (kind) => {
    throw new Error(`no strip layout for ${MixerScopeKind[kind]}`);
  },
};

/** The FX columns' strips: the Chorus's and the Reverb's. */
export type FxStripKind = "chorus" | "reverb";

/** Each FX strip's block: core's group of its fields. */
const FX_GROUP: Readonly<Record<FxStripKind, ParamGroup>> = { chorus: ParamGroup.Chorus, reverb: ParamGroup.Reverb };

/** An FX strip's fields, by their role in its block. */
export interface FxStripFields {
  /** The block's switch, drawn in the title. */
  switch: MixerField;
  /** The block's type, a list. */
  type: MixerField;
  /** The block's level, on the strip's level fader. */
  level: MixerField;
  /** Where its output goes, in registry order: a picker each. */
  outputs: readonly MixerField[];
}

/** Each FX strip's fields, read from core once. */
const FX_FIELDS: Readonly<Record<FxStripKind, () => FxStripFields>> = {
  chorus: memo(() => readFxStripFields("chorus")),
  reverb: memo(() => readFxStripFields("reverb")),
};

/** An FX strip's fields: its block's Studio Set common fields, by the role core gives each (`mixer_fx_role`). */
export function fxStripFields(kind: FxStripKind): FxStripFields {
  return FX_FIELDS[kind]();
}

/** Reads {@link fxStripFields} from core. */
function readFxStripFields(kind: FxStripKind): FxStripFields {
  const fields = mixerFields([MixerScopeKind.Common]).ids.filter((f) => mixer_field_group(f) === FX_GROUP[kind]);
  const withRole = (role: FxRole) => fields.filter((f) => mixer_fx_role(f) === role);
  const one = (role: FxRole): MixerField => {
    const [field, ...rest] = withRole(role);
    if (field === undefined || rest.length > 0) throw new Error(`${kind}: not one ${FxRole[role]} field`);
    return field;
  };
  return { switch: one(FxRole.Switch), type: one(FxRole.Type), level: one(FxRole.Level), outputs: withRole(FxRole.Output) };
}

/** The labels of an FX strip's output pickers, in `outputs` order. */
const OUTPUT_LABELS = ["Out", "To"] as const;

/** An FX strip's field space: its block's fields, from core's Common Scope. */
function fxStripSpace(kind: FxStripKind): FieldSpace<MixerField> {
  const { switch: sw, type, level, outputs } = fxStripFields(kind);
  const mine = new Set([sw, type, level, ...outputs]);
  const all = mixerFields([MixerScopeKind.Common]);
  return {
    name: `${kind} strip fields`,
    get ids() {
      return all.ids.filter((f) => mine.has(f));
    },
    nameOf: all.nameOf,
    optionCount: all.optionCount,
  };
}

/**
 * An FX strip, laid out like a Comp+EQ unit's: the switch in the title
 * ("FX1 ⏻"), the type
 * list sitting on the output pickers, which sit on the part strips' Out row,
 * then the level fader on every strip's fader rows, the type's name up its
 * side. The list is as long as core's type count.
 */
function fxStrip(kind: FxStripKind): SectionBody<MixerField> {
  const { switch: sw, type, level, outputs } = fxStripFields(kind);
  const listH = optionListRows(fxStripSpace(kind).optionCount(type), true);
  return {
    titleSwitch: sw,
    items: [
      { kind: "list", field: type, x: 1, y: OUT_Y - listH, h: listH, label: "Type" },
      ...outputs.map(
        (field, i): ItemLayout<MixerField> => ({ kind: "picker", field, x: 0, y: OUT_Y + i, w: 4, label: OUTPUT_LABELS[i] }),
      ),
      { kind: "fader", field: level, x: 1, ...FADER, label: false, scale: true },
      NAME,
    ],
  };
}

/** The strip under each FX column's parameters, by FX: the strip footprint, so its fader is level with theirs. */
export const FX_STRIP_SET: VariantSet<FxStripKind, MixerField> = {
  name: "mixer FX strip",
  // Its Scope's `fx` is its key's index: FX1 the Chorus, FX2 the Reverb.
  title: "FX{fx}",
  titleEdge: STRIP_SET.titleEdge,
  w: STRIP_SET.w,
  h: STRIP_SET.h,
  keys: ["chorus", "reverb"],
  fieldsOf: fxStripSpace,
  variants: new Map(),
  generate: fxStrip,
};
