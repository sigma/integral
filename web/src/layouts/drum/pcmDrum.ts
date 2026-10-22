/**
 * The PCM Drum Kit editor: the kit's level, beside the key grid, and the
 * selected key's Partial.
 * Every Section takes its Scope (`key`) from the key grid; each WMT row adds
 * its `layer`, so one layout serves every key and WMT and changing the key
 * only rebinds them.
 */

import {
  DisplayKind,
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
import { ENV_CELLS, envelopeItems } from "../envelope";
import { enumFields, memo, type FieldKey } from "../fields";
import { MFX_PANEL_FRAME } from "../fx/mfx";
import { deviceSpec } from "../../deviceSpec";
import { placeRow, rowCells, rowSectionLookup, sectionCells, type SectionRow } from "../geometry";
import { freeLanes } from "../region";
import { switchStrip } from "../switchStrip";
import type { ScreenLayout, SectionLayout } from "../types";

/** The Partial fields outside the WMT layers; an envelope's times and levels are array fields. */
export const PCMD_PARTIAL_FIELDS = enumFields("PcmdPartialField", PcmdPartialField, pcmd_partial_spec, {
  count: pcmd_partial_count,
});

/** The kit-wide level. The editor sets it beside the key grid, off the key Sections' screen. */
export const PCMD_KIT_LAYOUT: SectionLayout<PcmdCommonField> = {
  title: "KIT",
  titleEdge: "top",
  w: 2,
  h: 13,
  fields: enumFields("PcmdCommonField", PcmdCommonField, pcmd_common_spec),
  items: [{ kind: "fader", field: PcmdCommonField.KitLevel, x: 0, y: 0, h: 13, label: "Kit Lv" }],
};

/** The WMT fields, from core. */
const PCMD_WMT_FIELDS = enumFields("PcmdWmtField", PcmdWmtField, pcmd_wmt_spec);

/**
 * One WMT of the key, the row its switch strip heads: its Waves and gain,
 * the FXM, Tempo Sync and pan switches under them, then its pitch, pan,
 * level and FXM knobs over its velocity window.
 */
export const PCMD_WMT_LAYOUT: SectionLayout<PcmdWmtField> = {
  title: "WAVE",
  titleEdge: "top",
  w: 27,
  h: 7,
  fields: PCMD_WMT_FIELDS,
  // The picker, switches and lists keep a free Cell from the left and bottom borders;
  // Alt Pan is 3 Cells wide, so REVERSE reads whole.
  items: [
    { kind: "display", display: DisplayKind.WavePicker, x: 1, y: 0, w: 8, h: 5 },
    { kind: "switch", field: PcmdWmtField.WaveFxmSwitch, x: 1, y: 5, label: "FXM" },
    { kind: "switch", field: PcmdWmtField.WaveTempoSync, x: 3, y: 5, label: "T.Sync" },
    { kind: "switch", field: PcmdWmtField.RandomPanSwitch, x: 5, y: 5, label: "Rnd Pan" },
    { kind: "list", field: PcmdWmtField.WaveGain, x: 10, y: 0, h: 5, label: "Gain" },
    { kind: "list", field: PcmdWmtField.AlternatePanSwitch, x: 12, y: 0, w: 3, h: 4, label: "Alt Pan" },
    { kind: "knob", field: PcmdWmtField.CoarseTune, x: 15, y: 0, label: "Coarse" },
    { kind: "knob", field: PcmdWmtField.FineTune, x: 17, y: 0, label: "Fine" },
    { kind: "knob", field: PcmdWmtField.Pan, x: 19, y: 0, label: "Pan" },
    { kind: "knob", field: PcmdWmtField.Level, x: 21, y: 0, label: "Level" },
    { kind: "knob", field: PcmdWmtField.WaveFxmColor, x: 23, y: 0, label: "Color" },
    { kind: "knob", field: PcmdWmtField.WaveFxmDepth, x: 25, y: 0, label: "Depth" },
    { kind: "knob", field: PcmdWmtField.VelocityRangeLower, x: 15, y: 3, label: "Vel Lo" },
    { kind: "knob", field: PcmdWmtField.VelocityRangeUpper, x: 17, y: 3, label: "Vel Hi" },
    { kind: "knob", field: PcmdWmtField.VelocityFadeLower, x: 19, y: 3, label: "Fade Lo" },
    { kind: "knob", field: PcmdWmtField.VelocityFadeUpper, x: 21, y: 3, label: "Fade Hi" },
  ],
};

/**
 * A WMT's on/off, the switch strip (`switchStrip`) at the head of its row,
 * as in the synth editors; a `layer` Scope binds it.
 */
export const PCMD_WMT_SWITCH_LAYOUT: SectionLayout<PcmdWmtField> = switchStrip({
  title: "WMT {layer}",
  field: PcmdWmtField.Switch,
  fields: PCMD_WMT_FIELDS,
  h: sectionCells(PCMD_WMT_LAYOUT).h,
});

/** What sets an envelope Section apart: its title, its Display and the knobs over its curve. */
interface EnvelopeSection {
  title: string;
  display: DisplayKind;
  /** The knobs over the curve, left to right. */
  knobs: readonly { field: PcmdPartialField; label: string }[];
}

/**
 * An envelope Section, after the old BreakpointEnvelope: one envelope block
 * (`envelopeItems`), its faders from the Display's core binding.
 */
function envelopeLayout({ title, display, knobs }: EnvelopeSection): SectionLayout<PcmdPartialField> {
  // The faders come from core's Display binding, read on first use, not at import.
  const items = memo(() =>
    envelopeItems({ display, toneType: ToneType.PcmDrum, fieldOf: (s) => s.pcmdPartial, knobs }, 0, 0),
  );
  return {
    title,
    titleEdge: "top",
    ...ENV_CELLS,
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
export const PCMD_FILTER_LAYOUT: SectionLayout<PcmdPartialField> = {
  title: "FILTER",
  titleEdge: "top",
  w: 10,
  h: 9,
  fields: PCMD_PARTIAL_FIELDS,
  items: [
    // The list and scope keep a free Cell from the left and right borders.
    { kind: "list", field: PcmdPartialField.TvfFilterType, x: 1, y: 0, h: 8, label: "Type" },
    { kind: "display", display: DisplayKind.FilterScope, x: 3, y: 0, w: 6, h: 3 },
    { kind: "knob", field: PcmdPartialField.TvfCutoffFrequency, x: 3, y: 3, label: "Cutoff" },
    { kind: "knob", field: PcmdPartialField.TvfResonance, x: 5, y: 3, label: "Reso" },
    { kind: "knob", field: PcmdPartialField.TvfCutoffVelocityCurve, x: 7, y: 3, label: "Vel Crv" },
    { kind: "knob", field: PcmdPartialField.TvfCutoffVelocitySens, x: 3, y: 6, label: "Cut Vel" },
    { kind: "knob", field: PcmdPartialField.TvfResonanceVelocitySens, x: 5, y: 6, label: "Res Vel" },
  ],
};

/** How the key plays: assignment, mute group, envelope mode, output, pitch, level and what it receives. */
export const PCMD_MISC_LAYOUT: SectionLayout<PcmdPartialField> = {
  title: "MISC",
  titleEdge: "top",
  w: 12,
  h: 13,
  fields: PCMD_PARTIAL_FIELDS,
  // The switches and lists keep a free Cell from the left, right and bottom borders.
  items: [
    { kind: "switch", field: PcmdPartialField.AssignType, x: 1, y: 0, label: "Assign" },
    { kind: "switch", field: PcmdPartialField.EnvMode, x: 3, y: 0, label: "Env Mode" },
    { kind: "switch", field: PcmdPartialField.OneShotMode, x: 5, y: 0, label: "1-Shot" },
    { kind: "switch", field: PcmdPartialField.ReceiveExpression, x: 7, y: 0, label: "RxExpr" },
    { kind: "switch", field: PcmdPartialField.ReceiveHold1, x: 9, y: 0, label: "RxHold" },
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
export const PCMD_OUTPUT_LAYOUT: SectionLayout<PcmdPartialField> = {
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

/** The family accenting a WMT row's WAVE border. */
export const PCMD_WMT_FAMILY = ParameterFamily.Pitch;

/** Which family accents each per-key Section's border, by placement id. */
export const PCMD_FAMILIES: Readonly<Record<string, ParameterFamily>> = {
  pitchEnv: ParameterFamily.Pitch,
  filter: ParameterFamily.Filter,
  filterEnv: ParameterFamily.Filter,
  ampEnv: ParameterFamily.Amp,
  misc: ParameterFamily.Common,
  output: ParameterFamily.Amp,
};

/** A Section of a WMT row: its switch strip, or its WAVE. */
export type PcmdWmtRowSection = "switch" | "wave";

/** A WMT row's Sections, left to right. */
export const PCMD_WMT_ROW: readonly PcmdWmtRowSection[] = ["switch", "wave"];

/** Placement ids. */
export const PCMD_SECTION = {
  mfx: "mfx",
  /** Section `section` of the row of WMT `layer` (0-based). */
  wmt: (layer: number, section: PcmdWmtRowSection) => `wmt-${layer}.${section}`,
} as const;

/** Every WMT, 0-based, from core; read on first use, not at import. */
const layers = memo(() => Array.from({ length: deviceSpec().pcm_drum_wmt_count }, (_, i) => i));

const wmtIds = rowSectionLookup(() => layers().length, PCMD_WMT_ROW, PCMD_SECTION.wmt);

/** The WMT (0-based) and row Section a placement id names, or `undefined` for another Section. */
export function wmtRowSectionOf(id: string): { layer: number; section: PcmdWmtRowSection } | undefined {
  const at = wmtIds(id);
  return at && { layer: at.row, section: at.section };
}

/** WMT `layer`'s row: its switch strip, then its WAVE. */
const wmtRow = (layer: number): SectionRow =>
  PCMD_WMT_ROW.map((s) => ({
    id: PCMD_SECTION.wmt(layer, s),
    section: s === "switch" ? PCMD_WMT_SWITCH_LAYOUT : PCMD_WMT_LAYOUT,
  }));

/** The per-key row, left to right: every per-key Section but the WMTs. */
export const PCMD_KEY_ROW: SectionRow = [
  { id: "pitchEnv", section: PCMD_PITCH_ENV_LAYOUT },
  { id: "filter", section: PCMD_FILTER_LAYOUT },
  { id: "filterEnv", section: PCMD_FILTER_ENV_LAYOUT },
  { id: "ampEnv", section: PCMD_AMP_ENV_LAYOUT },
  { id: "misc", section: PCMD_MISC_LAYOUT },
  { id: "output", section: PCMD_OUTPUT_LAYOUT },
];

/** The Lane between the key's output and the MFX, in Cells. */
const LANE_W = 2;

function buildScreen(): ScreenLayout {
  const wmt = rowCells(wmtRow(0));
  const key = rowCells(PCMD_KEY_ROW);
  const mfx = sectionCells(MFX_PANEL_FRAME);
  const keyY = layers().length * wmt.h;
  const mfxX = key.w + LANE_W;
  const w = Math.max(wmt.w, mfxX + mfx.w);
  const h = keyY + Math.max(key.h, mfx.h);
  const sections = [
    ...layers().flatMap((layer) => placeRow(wmtRow(layer), layer * wmt.h)),
    ...placeRow(PCMD_KEY_ROW, keyY),
    { id: PCMD_SECTION.mfx, section: MFX_PANEL_FRAME, x: mfxX, y: keyY },
  ];
  return {
    name: "PCM Drum",
    w,
    h,
    sections,
    lanes: freeLanes(w, h, sections),
    // The kit's keys all feed the MFX.
    silkscreen: [
      { kind: "arrow", from: { section: "output", side: "right" }, to: { section: PCMD_SECTION.mfx, side: "left" } },
    ],
  };
}

/**
 * The PCM Drum editor's Grid: one row per WMT, its switch strip then its
 * WAVE, over the per-key row (pitch, filter, amp, how the key plays, its
 * level and pan) and the MFX every key feeds across a Lane for its arrow.
 * Built on first use: it reads the WMT count from core.
 */
export const pcmdScreen: () => ScreenLayout = memo(buildScreen);

/** Every PCM Drum Section layout, for the validator. */
export const PCMD_SECTIONS: readonly SectionLayout<FieldKey>[] = [
  PCMD_KIT_LAYOUT,
  PCMD_WMT_SWITCH_LAYOUT,
  PCMD_WMT_LAYOUT,
  PCMD_PITCH_ENV_LAYOUT,
  PCMD_FILTER_ENV_LAYOUT,
  PCMD_AMP_ENV_LAYOUT,
  PCMD_FILTER_LAYOUT,
  PCMD_MISC_LAYOUT,
  PCMD_OUTPUT_LAYOUT,
];
