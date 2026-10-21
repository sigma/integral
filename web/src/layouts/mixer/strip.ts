/**
 * A mixer channel strip below its EQ: one variant set keyed by strip kind
 * (the mixer Scope kinds that draw a strip), all in one 4×22 footprint so
 * every strip's fader starts on the same row.
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

import { DisplayKind, MixerField, MixerScopeKind, mixer_field_in_scope } from "../../../pkg/integral_wasm.js";
import { memo, mixerFields, type FieldSpace } from "../fields";
import { allElements } from "../geometry";
import type { SectionBody, VariantSet } from "../types";
import { EQ_LAYOUT } from "./eq";

/** The strip kinds: every mixer Scope kind but the Studio Set common block, from core's enum. */
export const STRIP_KINDS: readonly MixerScopeKind[] = Object.values(MixerScopeKind).filter(
  (k): k is MixerScopeKind => typeof k === "number" && k !== MixerScopeKind.Common,
);

/** Common fields a part strip draws as the part's own switch: on while the field names the part. */
export const PART_TOGGLES: ReadonlySet<MixerField> = new Set([MixerField.SoloPart, MixerField.DrumCompEqPart]);

/** Fields the EQ Section places; the strip leaves them to it. */
const EQ_PLACED: ReadonlySet<MixerField> = new Set(
  allElements(EQ_LAYOUT.items).flatMap((el) => (el.kind === "display" ? [] : [el.field])),
);

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

/** The row every full-height level fader starts on, and its height. */
const FADER = { y: 10, h: 12 } as const;

/** A part strip: channel and pan, the sends (or the ambience send), routing, mute and solo, level. */
const PART: SectionBody<MixerField> = {
  items: [
    { kind: "knob", field: MixerField.PartReceiveChannel, x: 0, y: 0, label: "Ch" },
    { kind: "knob", field: MixerField.PartPan, x: 2, y: 0, label: "Pan" },
    // Surround replaces the FX sends with its ambience send.
    {
      kind: "alternatives",
      group: "sends",
      x: 0,
      y: 3,
      w: 4,
      h: 3,
      options: [
        {
          name: "fx",
          items: [
            { kind: "knob", field: MixerField.PartChorusSend, x: 0, y: 3 },
            { kind: "knob", field: MixerField.PartReverbSend, x: 2, y: 3 },
          ],
        },
        { name: "surround", items: [{ kind: "knob", field: MixerField.SurroundAmbienceSend, x: 1, y: 3, label: "AMB" }] },
      ],
    },
    { kind: "knob", field: MixerField.PartOutputAssign, x: 0, y: 6, label: "Out" },
    { kind: "switch", field: MixerField.DrumCompEqPart, x: 2, y: 7, label: false },
    { kind: "switch", field: MixerField.PartMute, x: 0, y: 9, label: false },
    { kind: "switch", field: MixerField.SoloPart, x: 2, y: 9, label: false },
    { kind: "fader", field: MixerField.PartLevel, x: 1, ...FADER, label: false },
  ],
};

/** The external input: mute and level. */
const EXT: SectionBody<MixerField> = {
  items: [
    { kind: "switch", field: MixerField.ExtMute, x: 0, y: 9, label: false },
    { kind: "fader", field: MixerField.ExtLevel, x: 1, ...FADER, label: false },
  ],
};

/** The master: its level. */
const MASTER: SectionBody<MixerField> = {
  items: [{ kind: "fader", field: MixerField.MasterLevel, x: 1, ...FADER, label: false }],
};

/**
 * A Drum Comp+EQ unit's compressor: the switch, the transfer curve over
 * threshold and ratio, the envelope over attack and release, the routing,
 * and the output gain on a shorter fader that ends with the others.
 */
const COMP_EQ_UNIT: SectionBody<MixerField> = {
  items: [
    { kind: "switch", field: MixerField.CompSwitch, x: 1, y: 0, label: false },
    { kind: "display", display: DisplayKind.CompTransferCurve, x: 0, y: 1, w: 4, h: 3 },
    { kind: "knob", field: MixerField.CompThreshold, x: 0, y: 4, label: "Thr" },
    { kind: "knob", field: MixerField.CompRatio, x: 2, y: 4 },
    { kind: "display", display: DisplayKind.CompEnvelope, x: 0, y: 7, w: 4, h: 2 },
    { kind: "knob", field: MixerField.CompAttack, x: 0, y: 9, label: "Atk" },
    { kind: "knob", field: MixerField.CompRelease, x: 2, y: 9, label: "Rel" },
    { kind: "knob", field: MixerField.CompOutputAssign, x: 1, y: 12, label: "Out" },
    { kind: "fader", field: MixerField.CompOutputGain, x: 1, y: 15, h: 7, label: "Gain" },
  ],
};

/** The strip below each mixer EQ, by strip kind. */
export const STRIP_SET: VariantSet<MixerScopeKind, MixerField> = {
  name: "mixer strip",
  // A part strip's title is its part number.
  title: "{part}",
  titleEdge: "top",
  w: 4,
  h: 22,
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
