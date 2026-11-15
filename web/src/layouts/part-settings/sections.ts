/**
 * The Part settings drawer's Sections, one per group of core's Part
 * settings registry (KBD, PITCH, OFFSET, MIDI, VOICE), stacked in one
 * column of {@link PART_SETTINGS_W} Cells. A group without a layout here
 * (SCALE, until #600) is not drawn.
 */

import {
  PartSetting as F,
  PartSettingGroup,
  part_setting_group,
  part_setting_group_label,
  part_setting_spec,
} from "../../../pkg/integral_wasm.js";
import { FOOTPRINT } from "../../synth-ui/grid";
import { cached } from "../../cached";
import { enumFields, type FieldSpace } from "../fields";
import type { SectionBody, SectionLayout } from "../types";

/** Content width of every group's Section, in Cells: eight small knobs. */
export const PART_SETTINGS_W = 8 * FOOTPRINT.knobSm.w;

/** The Part settings of `group`, as core's registry lists them. */
export function partSettingFields(group: PartSettingGroup): FieldSpace<F> {
  return enumFields<F>(`PartSetting in ${PartSettingGroup[group]}`, F, part_setting_spec, {
    keep: (field) => part_setting_group(field) === group,
  });
}

/** A group's Controls and its content height in Cells. */
type GroupBody = SectionBody<F> & { h: number };

/** A knob at knob place `i` of row `y`. */
const knob = (field: F, i: number, y: number, label?: string) =>
  ({ kind: "knob", field, x: i * FOOTPRINT.knobSm.w, y, ...(label && { label }) }) as const;

/** The second row's top: under one row of knobs. */
const ROW_2 = FOOTPRINT.knobSm.h;

/** Key range and fades, then velocity's, then how the Part plays. */
const KBD: GroupBody = {
  h: ROW_2 + 4,
  items: [
    knob(F.KeyRangeLower, 0, 0, "Key Lo"),
    knob(F.KeyRangeUpper, 1, 0, "Key Hi"),
    knob(F.KeyFadeLower, 2, 0, "Fade Lo"),
    knob(F.KeyFadeUpper, 3, 0, "Fade Hi"),
    knob(F.VelocityRangeLower, 4, 0, "Velo Lo"),
    knob(F.VelocityRangeUpper, 5, 0, "Velo Hi"),
    knob(F.VelocityFadeLower, 6, 0, "Fade Lo"),
    knob(F.VelocityFadeUpper, 7, 0, "Fade Hi"),
    knob(F.VelocitySensOffset, 0, ROW_2, "Velo Sens"),
    { kind: "list", field: F.MonoPoly, x: 2, y: ROW_2, w: 3, h: 4 },
    { kind: "list", field: F.Legato, x: 5, y: ROW_2, w: 3, h: 4 },
    { kind: "picker", field: F.VelocityCurve, x: 8, y: ROW_2, w: FOOTPRINT.dropdownW, label: "Curve" },
  ],
  // Divides the key range from the velocity range.
  silkscreen: [{ kind: "rule", points: [[8, 0], [8, ROW_2]] }],
};

/** Transpose, bend range, then portamento. */
const PITCH: GroupBody = {
  h: 4,
  items: [
    knob(F.OctaveShift, 0, 0),
    knob(F.CoarseTune, 1, 0),
    knob(F.FineTune, 2, 0),
    knob(F.BendRange, 3, 0, "Bend"),
    { kind: "list", field: F.PortamentoSwitch, x: 8, y: 0, w: 3, h: 4, label: "Porta" },
    { kind: "knob", field: F.PortamentoTime, x: 11, y: 0, label: "Time" },
  ],
};

/** The Sound Ctrl offsets: filter, envelope, then vibrato. */
const OFFSET: GroupBody = {
  h: FOOTPRINT.knobSm.h,
  items: [
    knob(F.CutoffOffset, 0, 0),
    knob(F.ResonanceOffset, 1, 0, "Reso"),
    knob(F.AttackOffset, 2, 0),
    knob(F.DecayOffset, 3, 0),
    knob(F.ReleaseOffset, 4, 0),
    knob(F.VibratoRate, 5, 0),
    knob(F.VibratoDepth, 6, 0),
    knob(F.VibratoDelay, 7, 0),
  ],
};

/** The receive filters, eight switches to a row. */
const RX: readonly [F, string][] = [
  [F.RxProgramChange, "PC"],
  [F.RxBankSelect, "BS"],
  [F.RxPitchBend, "Bend"],
  [F.RxPolyPressure, "P.Aft"],
  [F.RxChannelPressure, "C.Aft"],
  [F.RxModulation, "Mod"],
  [F.RxVolume, "Vol"],
  [F.RxPan, "Pan"],
  [F.RxExpression, "Exp"],
  [F.RxHold1, "Hold"],
];
const SWITCHES_PER_ROW = PART_SETTINGS_W / FOOTPRINT.switch.w;
const MIDI: GroupBody = {
  h: Math.ceil(RX.length / SWITCHES_PER_ROW) * FOOTPRINT.switch.h,
  items: RX.map(([field, label], i) => ({
    kind: "switch",
    field,
    x: (i % SWITCHES_PER_ROW) * FOOTPRINT.switch.w,
    y: Math.floor(i / SWITCHES_PER_ROW) * FOOTPRINT.switch.h,
    label,
  })),
};

/** Voice Reserve. */
const VOICE: GroupBody = {
  h: FOOTPRINT.knobSm.h,
  items: [knob(F.VoiceReserve, 0, 0, "Reserve")],
};

const BODIES: ReadonlyMap<PartSettingGroup, GroupBody> = new Map([
  [PartSettingGroup.Kbd, KBD],
  [PartSettingGroup.Pitch, PITCH],
  [PartSettingGroup.Offset, OFFSET],
  [PartSettingGroup.Midi, MIDI],
  [PartSettingGroup.Voice, VOICE],
]);

/**
 * `group`'s Section, titled as core names the group; none for a group the
 * drawer doesn't draw yet. Built on first call, as it reads core.
 */
export const partSettingsLayout = cached((group: PartSettingGroup): SectionLayout<F> | undefined => {
  const body = BODIES.get(group);
  if (!body) return undefined;
  const { h, ...rest } = body;
  return {
    title: part_setting_group_label(group),
    titleEdge: "top",
    w: PART_SETTINGS_W,
    h,
    fields: partSettingFields(group),
    ...rest,
  };
});

/** Every group's Section the drawer draws, for the validator. */
export function partSettingsLayouts(): SectionLayout<F>[] {
  return [...BODIES.keys()].map((group) => partSettingsLayout(group)!);
}
