/**
 * The PCM Drum Kit editor: the kit's level, then the selected key's Partial.
 * The Partial Sections take their Scope (`key`) from the key grid, and the
 * WMT Section adds the `layer` from the layer tabs, so one layout serves
 * every key and layer and changing either only rebinds it.
 */

import {
  DisplayKind,
  DisplayProp,
  ParameterFamily,
  PcmdCommonField,
  PcmdPartialField,
  PcmdWmtField,
  pcmd_common_spec,
  pcmd_partial_count,
  pcmd_partial_spec,
  pcmd_wmt_spec,
  ToneType,
} from "../../../pkg/integral_wasm.js";
import { displaySlots } from "../displays";
import { elementFields, enumFields, memo, type FieldKey } from "../fields";
import { MFX_PANEL_FRAME } from "../fx/mfx";
import { sectionCells } from "../geometry";
import type { ControlLayout, ItemLayout, ScreenLayout, SectionLayout } from "../types";

/** A PCM Drum Partial Control's field: a field, or one value of an envelope's times or levels. */
export type PcmdPartialKey = PcmdPartialField | string;

/** The Partial fields outside the WMT layers, one key per envelope time and level. */
export const PCMD_PARTIAL_FIELDS = elementFields(
  "PcmdPartialField",
  PcmdPartialField,
  pcmd_partial_spec,
  pcmd_partial_count,
);

const elementKey = PCMD_PARTIAL_FIELDS.element;

/** The kit-wide level, as tall as a key's Level fader. */
export const PCMD_KIT_LAYOUT: SectionLayout<PcmdCommonField> = {
  title: "KIT",
  titleEdge: "top",
  w: 2,
  h: 13,
  fields: enumFields("PcmdCommonField", PcmdCommonField, pcmd_common_spec),
  items: [{ kind: "fader", field: PcmdCommonField.KitLevel, x: 0, y: 0, h: 13, label: "Kit Lv" }],
};

/** One WMT layer of the key: its Waves, gain, FXM, pitch, pan, level and velocity window. */
export const PCMD_WMT_LAYOUT: SectionLayout<PcmdWmtField> = {
  title: "WMT {layer}",
  titleEdge: "top",
  w: 12,
  h: 13,
  fields: enumFields("PcmdWmtField", PcmdWmtField, pcmd_wmt_spec),
  items: [
    { kind: "display", display: DisplayKind.WavePicker, x: 0, y: 0, w: 10, h: 5 },
    { kind: "switch", field: PcmdWmtField.Switch, x: 10, y: 0 },
    { kind: "list", field: PcmdWmtField.WaveGain, x: 10, y: 1, h: 5, label: "Gain" },
    { kind: "knob", field: PcmdWmtField.WaveFxmColor, x: 0, y: 6, label: "Color" },
    { kind: "knob", field: PcmdWmtField.WaveFxmDepth, x: 2, y: 6, label: "Depth" },
    { kind: "knob", field: PcmdWmtField.CoarseTune, x: 4, y: 6, label: "Coarse" },
    { kind: "knob", field: PcmdWmtField.FineTune, x: 6, y: 6, label: "Fine" },
    { kind: "knob", field: PcmdWmtField.Pan, x: 8, y: 6 },
    { kind: "knob", field: PcmdWmtField.Level, x: 10, y: 6 },
    { kind: "knob", field: PcmdWmtField.VelocityRangeLower, x: 0, y: 9, label: "Vel Lo" },
    { kind: "knob", field: PcmdWmtField.VelocityRangeUpper, x: 2, y: 9, label: "Vel Hi" },
    { kind: "knob", field: PcmdWmtField.VelocityFadeLower, x: 4, y: 9, label: "Fade Lo" },
    { kind: "knob", field: PcmdWmtField.VelocityFadeUpper, x: 6, y: 9, label: "Fade Hi" },
    { kind: "list", field: PcmdWmtField.AlternatePanSwitch, x: 8, y: 9, h: 4, label: "Alt Pan" },
    { kind: "switch", field: PcmdWmtField.WaveFxmSwitch, x: 10, y: 9, label: "FXM" },
    { kind: "switch", field: PcmdWmtField.RandomPanSwitch, x: 10, y: 10, label: "Rnd Pan" },
    { kind: "switch", field: PcmdWmtField.WaveTempoSync, x: 10, y: 11, label: "T.Sync" },
  ],
};

/** What sets an envelope Section apart: its title, its Display and the knobs over its curve. */
interface EnvelopeSpec {
  title: string;
  display: DisplayKind;
  /** The knobs over the curve, left to right. */
  knobs: readonly { field: PcmdPartialField; label: string }[];
}

/** Rows of an envelope Section: its knobs, its curve, its level faders and its time faders. */
const ENV_ROWS = { knobs: 0, curve: 3, levels: 5, times: 9 } as const;

/** Height of an envelope's faders, in Cells. */
const ENV_FADER_H = 4;

/**
 * One fader per stored value of `prop` in `display`'s core binding: the
 * n-th value (0-based) on column `2n + offset`, named `name` and its number
 * counted from `first`. Values the device fixes (a TVA's L0 and L4) get no
 * fader; only the curve draws them.
 */
function envelopeFaders(
  display: DisplayKind,
  prop: DisplayProp,
  name: string,
  first: number,
  offset: number,
  y: number,
): ControlLayout<PcmdPartialKey>[] {
  return displaySlots(display, ToneType.PcmDrum)
    .filter((s) => s.prop === prop)
    .flatMap((slot, n) => {
      const field = slot.pcmdPartial;
      if (field === undefined) return [];
      const label = `${name}${n + first}`;
      return [{ kind: "fader", field: elementKey(field, slot.index), x: 2 * n + offset, y, h: ENV_FADER_H, label }];
    });
}

/**
 * An envelope Section, after the old BreakpointEnvelope: knobs, the curve
 * across the Section, then levels L0–L4 on even columns and times T1–T4
 * between them, so each time sits under the two levels it joins. The
 * faders come from the Display's core binding.
 */
function envelopeLayout(spec: EnvelopeSpec): SectionLayout<PcmdPartialKey> {
  const knobs: ControlLayout<PcmdPartialKey>[] = spec.knobs.map((k, i) => ({
    kind: "knob",
    field: k.field,
    x: 2 * i,
    y: ENV_ROWS.knobs,
    label: k.label,
  }));
  // The faders come from core's Display binding, read on first use, not at import.
  const items = memo((): ItemLayout<PcmdPartialKey>[] => [
    ...knobs,
    { kind: "display", display: spec.display, x: 0, y: ENV_ROWS.curve, w: 10, h: 2 },
    ...envelopeFaders(spec.display, DisplayProp.Levels, "L", 0, 0, ENV_ROWS.levels),
    ...envelopeFaders(spec.display, DisplayProp.Times, "T", 1, 1, ENV_ROWS.times),
  ]);
  return {
    title: spec.title,
    titleEdge: "top",
    w: 10,
    h: 13,
    fields: PCMD_PARTIAL_FIELDS,
    get items() {
      return items();
    },
  };
}

export const PCMD_PITCH_ENV_LAYOUT = envelopeLayout({
  title: "PITCH ENV",
  display: DisplayKind.PitchEnvelope,
  knobs: [
    { field: PcmdPartialField.PitchEnvDepth, label: "Depth" },
    { field: PcmdPartialField.PitchEnvVelocitySens, label: "Vel Sns" },
    { field: PcmdPartialField.PitchEnvT1VelocitySens, label: "T1 Vel" },
    { field: PcmdPartialField.PitchEnvT4VelocitySens, label: "T4 Vel" },
  ],
});

export const PCMD_FILTER_ENV_LAYOUT = envelopeLayout({
  title: "FILTER ENV",
  display: DisplayKind.FilterEnvelope,
  knobs: [
    { field: PcmdPartialField.TvfEnvDepth, label: "Depth" },
    { field: PcmdPartialField.TvfEnvVelocitySens, label: "Vel Sns" },
    { field: PcmdPartialField.TvfEnvT1VelocitySens, label: "T1 Vel" },
    { field: PcmdPartialField.TvfEnvT4VelocitySens, label: "T4 Vel" },
    { field: PcmdPartialField.TvfEnvVelocityCurve, label: "Env Crv" },
  ],
});

export const PCMD_AMP_ENV_LAYOUT = envelopeLayout({
  title: "AMP",
  display: DisplayKind.AmpEnvelope,
  knobs: [
    { field: PcmdPartialField.TvaLevelVelocityCurve, label: "Vel Crv" },
    { field: PcmdPartialField.TvaLevelVelocitySens, label: "Vel Sns" },
    { field: PcmdPartialField.TvaEnvT1VelocitySens, label: "T1 Vel" },
    { field: PcmdPartialField.TvaEnvT4VelocitySens, label: "T4 Vel" },
  ],
});

/** The filter: its type, the response scope, cutoff and resonance with their velocity sensitivities. */
export const PCMD_FILTER_LAYOUT: SectionLayout<PcmdPartialKey> = {
  title: "FILTER",
  titleEdge: "top",
  w: 10,
  h: 9,
  fields: PCMD_PARTIAL_FIELDS,
  items: [
    { kind: "list", field: PcmdPartialField.TvfFilterType, x: 0, y: 0, h: 8, label: "Type" },
    { kind: "display", display: DisplayKind.FilterScope, x: 2, y: 0, w: 8, h: 3 },
    { kind: "knob", field: PcmdPartialField.TvfCutoffFrequency, x: 2, y: 3, label: "Cutoff" },
    { kind: "knob", field: PcmdPartialField.TvfResonance, x: 4, y: 3, label: "Reso" },
    { kind: "knob", field: PcmdPartialField.TvfCutoffVelocityCurve, x: 6, y: 3, label: "Vel Crv" },
    { kind: "knob", field: PcmdPartialField.TvfCutoffVelocitySens, x: 2, y: 6, label: "Cut Vel" },
    { kind: "knob", field: PcmdPartialField.TvfResonanceVelocitySens, x: 4, y: 6, label: "Res Vel" },
  ],
};

/** How the key plays: assignment, mute group, envelope mode, output, pitch, level and what it receives. */
export const PCMD_MISC_LAYOUT: SectionLayout<PcmdPartialKey> = {
  title: "MISC",
  titleEdge: "top",
  w: 10,
  h: 12,
  fields: PCMD_PARTIAL_FIELDS,
  items: [
    { kind: "switch", field: PcmdPartialField.AssignType, x: 0, y: 0, label: "Assign" },
    { kind: "switch", field: PcmdPartialField.EnvMode, x: 2, y: 0, label: "Env Mode" },
    { kind: "switch", field: PcmdPartialField.OneShotMode, x: 4, y: 0, label: "1-Shot" },
    { kind: "switch", field: PcmdPartialField.ReceiveExpression, x: 6, y: 0, label: "RxExpr" },
    { kind: "switch", field: PcmdPartialField.ReceiveHold1, x: 8, y: 0, label: "RxHold" },
    { kind: "knob", field: PcmdPartialField.MuteGroup, x: 0, y: 1, label: "Mute Grp" },
    { kind: "knob", field: PcmdPartialField.PitchBendRange, x: 2, y: 1, label: "PB Rng" },
    { kind: "knob", field: PcmdPartialField.Level, x: 4, y: 1 },
    { kind: "knob", field: PcmdPartialField.CoarseTune, x: 6, y: 1, label: "Coarse" },
    { kind: "knob", field: PcmdPartialField.FineTune, x: 8, y: 1, label: "Fine" },
    { kind: "knob", field: PcmdPartialField.RandomPitchDepth, x: 0, y: 4, label: "Rnd Pit" },
    { kind: "list", field: PcmdPartialField.OutputAssign, x: 2, y: 4, h: 8, label: "Output" },
    { kind: "list", field: PcmdPartialField.WmtVelocityControl, x: 4, y: 4, h: 4, label: "Vel Ctrl" },
  ],
};

/** The key's pan, effect sends and output level. */
export const PCMD_OUTPUT_LAYOUT: SectionLayout<PcmdPartialKey> = {
  title: "LEVEL / PAN",
  titleEdge: "top",
  w: 4,
  h: 9,
  fields: PCMD_PARTIAL_FIELDS,
  items: [
    { kind: "knob", field: PcmdPartialField.Pan, x: 0, y: 0 },
    { kind: "knob", field: PcmdPartialField.ChorusSend, x: 0, y: 3, label: "FX1" },
    { kind: "knob", field: PcmdPartialField.ReverbSend, x: 0, y: 6, label: "FX2" },
    { kind: "fader", field: PcmdPartialField.OutputLevel, x: 2, y: 0, h: 9, label: "Level" },
  ],
};

/** Which family accents each Section's border, by placement id. */
export const PCMD_FAMILIES: Readonly<Record<string, ParameterFamily>> = {
  wmt: ParameterFamily.Pitch,
  pitchEnv: ParameterFamily.Pitch,
  filter: ParameterFamily.Filter,
  filterEnv: ParameterFamily.Filter,
  ampEnv: ParameterFamily.Amp,
  misc: ParameterFamily.Common,
  output: ParameterFamily.Amp,
};

/** Right edge of the key Sections, in Cells. */
const KEY_W = 34;
/** Height of the key Sections, in Cells. */
const KEY_H = 28;
/** The Lane between the key's output and the MFX, in Cells. */
const LANE_W = 2;
const MFX = sectionCells(MFX_PANEL_FRAME);

/**
 * The PCM Drum editor's Grid: the key's wave and pitch over its filter and
 * amp, and the MFX every key feeds across a Lane for its arrow.
 */
export const PCMD_SCREEN: ScreenLayout = {
  name: "PCM Drum",
  w: KEY_W + LANE_W + MFX.w,
  h: Math.max(KEY_H, MFX.h),
  sections: [
    { id: "kit", section: PCMD_KIT_LAYOUT, x: 0, y: 0 },
    { id: "wmt", section: PCMD_WMT_LAYOUT, x: 2, y: 0 },
    { id: "pitchEnv", section: PCMD_PITCH_ENV_LAYOUT, x: 14, y: 0 },
    { id: "misc", section: PCMD_MISC_LAYOUT, x: 24, y: 0 },
    { id: "filter", section: PCMD_FILTER_LAYOUT, x: 0, y: 14 },
    { id: "filterEnv", section: PCMD_FILTER_ENV_LAYOUT, x: 10, y: 14 },
    { id: "ampEnv", section: PCMD_AMP_ENV_LAYOUT, x: 20, y: 14 },
    { id: "output", section: PCMD_OUTPUT_LAYOUT, x: 30, y: 14 },
    { id: "mfx", section: MFX_PANEL_FRAME, x: KEY_W + LANE_W, y: 0 },
  ],
  lanes: [{ x: KEY_W, y: 0, w: LANE_W, h: Math.max(KEY_H, MFX.h) }],
  // The kit's keys all feed the MFX.
  silkscreen: [{ kind: "arrow", from: { section: "output", side: "right" }, to: { section: "mfx", side: "left" } }],
};

/** Every PCM Drum Section layout, for the validator. */
export const PCMD_SECTIONS: readonly SectionLayout<FieldKey>[] = [
  PCMD_KIT_LAYOUT,
  PCMD_WMT_LAYOUT,
  PCMD_PITCH_ENV_LAYOUT,
  PCMD_FILTER_ENV_LAYOUT,
  PCMD_AMP_ENV_LAYOUT,
  PCMD_FILTER_LAYOUT,
  PCMD_MISC_LAYOUT,
  PCMD_OUTPUT_LAYOUT,
];
