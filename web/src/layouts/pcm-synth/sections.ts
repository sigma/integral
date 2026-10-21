/**
 * The PCM Synth editor's Sections: COMMON and PARTIAL MIX on top, then each
 * partial's switch strip and boxes (LFO1 to OUTPUT), and each pair's BOOST
 * and RING. A Partial Scope (`{ partial }`, 0-based) binds a partial's
 * boxes; the MFX is the shared MFX Section (`MFX_PANEL_FRAME`).
 *
 * Every box of a lane row is {@link ROW_H} Cells tall. The envelope boxes
 * (PITCH, FILTER, AMP) stack their own Controls over one envelope block
 * (`envelopeItems`) at the same rows, so their envelopes line up along a
 * lane. Envelope times and levels and the Step LFO's steps are elements of
 * array fields. Option lists, switches and Displays keep a free Cell from the
 * left, right and bottom borders (`validateClearance`).
 */

import {
  DisplayKind,
  ParameterFamily,
  PcmsCommonField as C,
  PcmsPartialField as F,
  PcmsPmtField,
  ToneType,
  pcms_common_spec,
  pcms_partial_count,
  pcms_partial_spec,
  pcms_pmt_spec,
} from "../../../pkg/integral_wasm.js";
import { envelopeItems, ENV_CELLS, type EnvelopeSpec } from "../envelope";
import { enumFields, memo } from "../fields";
import type { ControlLayout, ItemLayout, SectionLayout } from "../types";

/** The Common fields. */
export const PCMS_COMMON_FIELDS = enumFields<C>("PcmsCommonField", C, pcms_common_spec);

/** The PMT fields, for the Sections that bind the partial mix. */
export const PCMS_PMT_FIELDS = enumFields<PcmsPmtField>("PcmsPmtField", PcmsPmtField, pcms_pmt_spec);

/** The Partial fields; an envelope's times and levels and the Step LFO's steps are array fields. */
export const PCMS_PARTIAL_FIELDS = enumFields<F>("PcmsPartialField", F, pcms_partial_spec, {
  count: pcms_partial_count,
});

/** Content rows of a box's own Controls, above its envelope block. */
const OWN_H = 8;

/** Content height of a lane row's boxes: the envelope boxes' own Controls and their envelope block. */
const BOX_H = OWN_H + ENV_CELLS.h;

/** Cells of one lane row, its boxes' titles included. */
export const ROW_H = BOX_H + 1;

/** Height of COMMON and PARTIAL MIX: a group name row, then a knob. */
const TOP_H = 4;

/** Groups after COMMON's tone-wide Controls, each named over its columns. */
const COMMON_GROUPS = [
  { text: "PORTAMENTO", from: 16, to: 20 },
  { text: "BEND", from: 20, to: 24 },
  { text: "OFFSETS", from: 24, to: 34 },
] as const;

/** Tone-wide settings. */
export const PCMS_COMMON_LAYOUT: SectionLayout<C> = {
  title: "COMMON",
  titleEdge: "top",
  w: 34,
  h: TOP_H,
  fields: PCMS_COMMON_FIELDS,
  // Switches sit level with the knobs' dials, clear of the bottom edge.
  items: [
    { kind: "knob", field: C.ToneLevel, x: 0, y: 1, label: "Level" },
    { kind: "knob", field: C.TonePan, x: 2, y: 1, label: "Pan" },
    { kind: "knob", field: C.CoarseTune, x: 4, y: 1, label: "Coarse" },
    { kind: "knob", field: C.FineTune, x: 6, y: 1, label: "Fine" },
    { kind: "knob", field: C.OctaveShift, x: 8, y: 1, label: "Octave" },
    { kind: "knob", field: C.AnalogFeel, x: 10, y: 1, label: "Analog" },
    { kind: "switch", field: C.MonoPoly, x: 12, y: 2, label: "Voice" },
    { kind: "switch", field: C.LegatoSwitch, x: 14, y: 2, label: "Legato" },
    { kind: "switch", field: C.PortamentoSwitch, x: 16, y: 2, label: "SW" },
    { kind: "knob", field: C.PortamentoTime, x: 18, y: 1, label: "Time" },
    { kind: "knob", field: C.PitchBendRangeDown, x: 20, y: 1, label: "Down" },
    { kind: "knob", field: C.PitchBendRangeUp, x: 22, y: 1, label: "Up" },
    { kind: "knob", field: C.CutoffOffset, x: 24, y: 1, label: "Cutoff" },
    { kind: "knob", field: C.ResonanceOffset, x: 26, y: 1, label: "Reso" },
    { kind: "knob", field: C.AttackTimeOffset, x: 28, y: 1, label: "Attack" },
    { kind: "knob", field: C.ReleaseTimeOffset, x: 30, y: 1, label: "Release" },
    { kind: "knob", field: C.VelocitySensOffset, x: 32, y: 1, label: "Velocity" },
  ],
  silkscreen: [
    // The voice switches set apart from the knobs, then a rule before each group.
    ...[12, ...COMMON_GROUPS.map(({ from }) => from)].map((x) => ({
      kind: "rule" as const,
      points: [[x, 0], [x, TOP_H]] as const,
    })),
    ...COMMON_GROUPS.map(({ text, from, to }) => ({
      kind: "label" as const,
      text,
      x: (from + to) / 2,
      y: 0.5,
      style: "title" as const,
    })),
  ],
};

/** Each pair's group in PARTIAL MIX: its Structure, and the signal path it draws. */
const MIX_GROUPS = [
  { text: "PARTIAL 1&2", field: PcmsPmtField.StructureType12, display: DisplayKind.Structure12, from: 0 },
  { text: "PARTIAL 3&4", field: PcmsPmtField.StructureType34, display: DisplayKind.Structure34, from: 12 },
] as const;

/** Width of one pair's group: its Structure picker and diagram, a free Cell each side. */
const MIX_GROUP_W = 12;

/** The partial mix: each pair's Structure, picked by number and drawn as its signal path, and the velocity control. */
export const PCMS_PMT_LAYOUT: SectionLayout<PcmsPmtField> = {
  title: "PARTIAL MIX",
  titleEdge: "top",
  w: 2 * MIX_GROUP_W + 2,
  h: TOP_H,
  fields: PCMS_PMT_FIELDS,
  items: [
    ...MIX_GROUPS.flatMap(({ field, display, from }): ItemLayout<PcmsPmtField>[] => [
      { kind: "picker", field, x: from + 1, y: 1, label: "Structure" },
      { kind: "display", display, x: from + 7, y: 1, w: 4, h: 2 },
    ]),
    { kind: "knob", field: PcmsPmtField.PmtVelocityControl, x: 2 * MIX_GROUP_W, y: 1, label: "Vel Ctrl" },
  ],
  silkscreen: [
    ...MIX_GROUPS.map(({ from }) => ({ kind: "rule" as const, points: [[from + MIX_GROUP_W, 0], [from + MIX_GROUP_W, TOP_H]] as const })),
    ...MIX_GROUPS.map(({ text, from }) => ({
      kind: "label" as const,
      text,
      x: from + MIX_GROUP_W / 2,
      y: 0.5,
      style: "title" as const,
    })),
  ],
};

/** A partial's box: top-titled, `w`×{@link BOX_H}, its items given (lazily, for those read from core). */
function box(title: string, w: number, items: () => readonly ItemLayout<F>[]): SectionLayout<F> {
  return {
    title,
    titleEdge: "top",
    w,
    h: BOX_H,
    fields: PCMS_PARTIAL_FIELDS,
    get items() {
      return items();
    },
  };
}

/** Width of the envelope boxes: an envelope block and a free column each side. */
const ENV_BOX_W = ENV_CELLS.w + 2;

/** An envelope box's own Controls, then its envelope block from core's Display binding, under them. */
function envelopeBox(
  title: string,
  own: readonly ItemLayout<F>[],
  env: Omit<EnvelopeSpec<F>, "toneType" | "fieldOf">,
): SectionLayout<F> {
  const items = memo(() => [
    ...own,
    ...envelopeItems({ ...env, toneType: ToneType.PcmSynth, fieldOf: (s) => s.pcmsPartial }, 1, OWN_H),
  ]);
  return box(title, ENV_BOX_W, items);
}

/** PITCH: tuning, and the pitch envelope. */
const PITCH = envelopeBox(
  "PITCH",
  [
    { kind: "knob", field: F.CoarseTune, x: 1, y: 2, label: "Coarse" },
    { kind: "knob", field: F.FineTune, x: 3, y: 2, label: "Fine" },
    { kind: "knob", field: F.RandomPitchDepth, x: 5, y: 2, label: "Random" },
    { kind: "knob", field: F.WavePitchKeyfollow, x: 7, y: 2, label: "Key Trk" },
  ],
  {
    display: DisplayKind.PitchEnvelope,
    knobs: [
      { field: F.PitchEnvDepth, label: "Depth" },
      { field: F.PitchEnvVelocitySens, label: "Vel Sns" },
      { field: F.PitchEnvTimeKeyfollow, label: "Time KF" },
      { field: F.PitchEnvT1VelocitySens, label: "T1 Vel" },
      { field: F.PitchEnvT4VelocitySens, label: "T4 Vel" },
    ],
  },
);

/** FILTER (TVF): type and response, cutoff and resonance, and the filter envelope. */
const FILTER = envelopeBox(
  "FILTER",
  [
    { kind: "list", field: F.TvfFilterType, x: 1, y: 0, h: OWN_H, label: "Type" },
    { kind: "display", display: DisplayKind.FilterScope, x: 3, y: 0, w: 8, h: 2 },
    { kind: "knob", field: F.TvfCutoffFrequency, x: 3, y: 2, label: "Cutoff" },
    { kind: "knob", field: F.TvfResonance, x: 5, y: 2, label: "Reso" },
    { kind: "knob", field: F.TvfCutoffKeyfollow, x: 7, y: 2, label: "Key Trk" },
    { kind: "knob", field: F.TvfCutoffVelocityCurve, x: 9, y: 2, label: "Vel Crv" },
    { kind: "knob", field: F.TvfCutoffVelocitySens, x: 3, y: 5, label: "Cut Vel" },
    { kind: "knob", field: F.TvfResonanceVelocitySens, x: 5, y: 5, label: "Res Vel" },
    { kind: "knob", field: F.TvfEnvVelocityCurve, x: 7, y: 5, label: "Env Crv" },
  ],
  {
    display: DisplayKind.FilterEnvelope,
    knobs: [
      { field: F.TvfEnvDepth, label: "Depth" },
      { field: F.TvfEnvVelocitySens, label: "Vel Sns" },
      { field: F.TvfEnvTimeKeyfollow, label: "Time KF" },
      { field: F.TvfEnvT1VelocitySens, label: "T1 Vel" },
      { field: F.TvfEnvT4VelocitySens, label: "T4 Vel" },
    ],
  },
);

/** AMP (TVA): pan, level velocity and bias, and the amp envelope. */
const AMP = envelopeBox(
  "AMP",
  [
    { kind: "list", field: F.TvaBiasDirection, x: 1, y: 2, h: 5, label: "Bias Dir" },
    { kind: "knob", field: F.Pan, x: 3, y: 2, label: "Pan" },
    { kind: "knob", field: F.PanKeyfollow, x: 5, y: 2, label: "Pan KF" },
    { kind: "knob", field: F.RandomPanDepth, x: 7, y: 2, label: "Rnd Pan" },
    { kind: "knob", field: F.AlternatePanDepth, x: 9, y: 2, label: "Alt Pan" },
    { kind: "knob", field: F.TvaLevelVelocityCurve, x: 3, y: 5, label: "Vel Crv" },
    { kind: "knob", field: F.TvaBiasLevel, x: 5, y: 5, label: "Bias Lvl" },
    { kind: "knob", field: F.TvaBiasPosition, x: 7, y: 5, label: "Bias Key" },
  ],
  {
    display: DisplayKind.AmpEnvelope,
    knobs: [
      { field: F.Level, label: "Level" },
      { field: F.TvaLevelVelocitySens, label: "Vel Sns" },
      { field: F.TvaEnvTimeKeyfollow, label: "Time KF" },
      { field: F.TvaEnvT1VelocitySens, label: "T1 Vel" },
      { field: F.TvaEnvT4VelocitySens, label: "T4 Vel" },
    ],
  },
);

/** An LFO's fields, LFO1's or LFO2's. */
interface LfoFields {
  title: string;
  scope: DisplayKind;
  waveform: F;
  rate: F;
  detune: F;
  delay: F;
  delayKeyfollow: F;
  offset: F;
  fadeMode: F;
  fadeTime: F;
  keyTrigger: F;
  depths: readonly [F, F, F, F];
}

/** Top row of an LFO's depth faders, and their height. */
const LFO_DEPTH = { y: 10, h: 6 } as const;

/** An LFO: its waveform and scope, rate (synced in the knob) and delay, fade, and the depths it sends. */
function lfoLayout(l: LfoFields): SectionLayout<F> {
  const depthLabels = ["Pitch", "Filter", "Amp", "Pan"] as const;
  const items: ItemLayout<F>[] = [
    { kind: "list", field: l.waveform, x: 1, y: 0, h: 14, label: "Wave" },
    { kind: "display", display: l.scope, x: 3, y: 0, w: 8, h: 2 },
    { kind: "knob", field: l.rate, x: 3, y: 2, label: "Rate" },
    { kind: "knob", field: l.detune, x: 5, y: 2, label: "Detune" },
    { kind: "knob", field: l.delay, x: 7, y: 2, label: "Delay" },
    { kind: "knob", field: l.delayKeyfollow, x: 9, y: 2, label: "Dly KF" },
    { kind: "knob", field: l.offset, x: 3, y: 5, label: "Offset" },
    { kind: "knob", field: l.fadeTime, x: 5, y: 5, label: "Fade T" },
    { kind: "list", field: l.fadeMode, x: 7, y: 5, h: 5, label: "Fade" },
    { kind: "switch", field: l.keyTrigger, x: 9, y: 5, label: "Key Trig" },
    ...l.depths.map(
      (field, i): ControlLayout<F> => ({ kind: "fader", field, x: 3 + 2 * i, ...LFO_DEPTH, label: depthLabels[i] }),
    ),
  ];
  return box(l.title, 12, () => items);
}

const LFO1 = lfoLayout({
  title: "LFO1",
  scope: DisplayKind.Lfo1Scope,
  waveform: F.Lfo1Waveform,
  rate: F.Lfo1Rate,
  detune: F.Lfo1RateDetune,
  delay: F.Lfo1DelayTime,
  delayKeyfollow: F.Lfo1DelayTimeKeyfollow,
  offset: F.Lfo1Offset,
  fadeMode: F.Lfo1FadeMode,
  fadeTime: F.Lfo1FadeTime,
  keyTrigger: F.Lfo1KeyTrigger,
  depths: [F.Lfo1PitchDepth, F.Lfo1TvfDepth, F.Lfo1TvaDepth, F.Lfo1PanDepth],
});

const LFO2 = lfoLayout({
  title: "LFO2",
  scope: DisplayKind.Lfo2Scope,
  waveform: F.Lfo2Waveform,
  rate: F.Lfo2Rate,
  detune: F.Lfo2RateDetune,
  delay: F.Lfo2DelayTime,
  delayKeyfollow: F.Lfo2DelayTimeKeyfollow,
  offset: F.Lfo2Offset,
  fadeMode: F.Lfo2FadeMode,
  fadeTime: F.Lfo2FadeTime,
  keyTrigger: F.Lfo2KeyTrigger,
  depths: [F.Lfo2PitchDepth, F.Lfo2TvfDepth, F.Lfo2TvaDepth, F.Lfo2PanDepth],
});

/** Step faders per row of the STEP LFO. */
const STEPS_PER_ROW = 4;

/** STEP LFO: its pattern's scope, its shape, and one fader per step, four to a row. */
const STEP = box("STEP LFO", 2 * STEPS_PER_ROW + 2, () => [
  { kind: "display", display: DisplayKind.StepLfoScope, x: 1, y: 0, w: 2 * STEPS_PER_ROW, h: 2 },
  { kind: "switch", field: F.LfoStepType, x: 1, y: 2, label: "Shape" },
  ...Array.from(
    { length: pcms_partial_count(F.LfoStep) },
    (_, i): ControlLayout<F> => ({
      kind: "fader",
      field: F.LfoStep,
      index: i,
      x: 1 + 2 * (i % STEPS_PER_ROW),
      y: 3 + 4 * Math.floor(i / STEPS_PER_ROW),
      h: 4,
      label: `${i + 1}`,
    }),
  ),
]);

/** WAVE: the wave picker (group and L/R numbers), tempo sync, gain and FXM. */
const WAVE = box("WAVE", 10, () => [
  { kind: "display", display: DisplayKind.WavePicker, x: 1, y: 0, w: 8, h: 5 },
  { kind: "switch", field: F.WaveTempoSync, x: 1, y: 5, label: "Tempo" },
  { kind: "switch", field: F.WaveFxmSwitch, x: 3, y: 5, label: "FXM" },
  { kind: "list", field: F.WaveGain, x: 1, y: 6, h: 5, label: "Gain" },
  { kind: "knob", field: F.WaveFxmColor, x: 3, y: 6, label: "FXM Col" },
  { kind: "knob", field: F.WaveFxmDepth, x: 5, y: 6, label: "FXM Dpt" },
]);

/** CTRL: how the partial responds to playing. */
const CTRL = box("CTRL", 4, () => [
  { kind: "switch", field: F.EnvMode, x: 1, y: 0, label: "Env Mode" },
  { kind: "switch", field: F.ReceiveBender, x: 1, y: 2, label: "Bender" },
  { kind: "switch", field: F.ReceiveExpression, x: 1, y: 4, label: "RxExpr" },
  { kind: "switch", field: F.ReceiveHold1, x: 1, y: 6, label: "RxHold" },
]);

/** OUTPUT: delay (its time synced in the knob), effect sends and output level. */
const OUTPUT = box("OUTPUT", 7, () => [
  { kind: "list", field: F.DelayMode, x: 1, y: 0, h: 5, label: "Delay" },
  { kind: "knob", field: F.DelayTime, x: 3, y: 0, label: "Time" },
  { kind: "knob", field: F.ChorusSend, x: 1, y: 5, label: "FX1" },
  { kind: "knob", field: F.ReverbSend, x: 3, y: 5, label: "FX2" },
  { kind: "fader", field: F.OutputLevel, x: 5, y: 0, h: 8, label: "Level" },
]);

/** A partial's box, by its id in a lane. */
export type PcmsBox = "lfo1" | "lfo2" | "step" | "wave" | "pitch" | "ctrl" | "tvf" | "tva" | "out";

/** A partial's box: its layout, as in an independent lane and numbered once the pair is merged, and its border's family. */
export interface PcmsBoxSpec {
  layout: SectionLayout<F>;
  /** The layout with its partial's number in the title, for a merged pair's boxes. */
  numbered: SectionLayout<F>;
  family: ParameterFamily;
}

/** `layout` and its numbered twin, titled `{title}{sep}{partial}`. */
function spec(layout: SectionLayout<F>, family: ParameterFamily, sep = " "): PcmsBoxSpec {
  const { titleEdge, w, h, fields } = layout;
  const numbered: SectionLayout<F> = {
    title: `${layout.title}${sep}{partial}`,
    titleEdge,
    w,
    h,
    fields,
    // Read through, as `layout` reads its items from core on first use.
    get items() {
      return layout.items;
    },
  };
  return { layout, numbered, family };
}

/** Every partial's box, by id. */
export const PCMS_BOXES: Readonly<Record<PcmsBox, PcmsBoxSpec>> = {
  lfo1: spec(LFO1, ParameterFamily.Modulation, " · "),
  lfo2: spec(LFO2, ParameterFamily.Modulation, " · "),
  step: spec(STEP, ParameterFamily.Modulation, " · "),
  wave: spec(WAVE, ParameterFamily.Pitch),
  pitch: spec(PITCH, ParameterFamily.Pitch),
  ctrl: spec(CTRL, ParameterFamily.Common),
  tvf: spec(FILTER, ParameterFamily.Filter),
  tva: spec(AMP, ParameterFamily.Amp),
  out: spec(OUTPUT, ParameterFamily.Amp),
};

/**
 * One partial's on/off switch, a strip at the head of its lane with the title
 * down its side, as in SN Synth; a Partial Scope binds it. The switch sits
 * under the top border's row and clear of the right edge (`validateClearance`).
 */
export const PARTIAL_LAYOUT: SectionLayout<PcmsPmtField> = {
  title: "Partial {partial}",
  titleEdge: "side",
  w: 3,
  h: ROW_H,
  fields: PCMS_PMT_FIELDS,
  // The title already names the switch.
  items: [{ kind: "switch", field: PcmsPmtField.PartialSwitch, x: 0, y: 1, label: false }],
};

/** Content rows of a BOOST, as tall as a lane. */
const BOOST_H = ROW_H - 1;

/** A pair's booster, on its signal path (Structure 3 and 4), as tall as a lane. */
function boostLayout(field: PcmsPmtField): SectionLayout<PcmsPmtField> {
  return {
    title: "BOOST",
    titleEdge: "top",
    w: 2,
    h: BOOST_H,
    fields: PCMS_PMT_FIELDS,
    // Centred on the lane, where the arrows meet it.
    items: [{ kind: "knob", field, x: 0, y: Math.floor((BOOST_H - 3) / 2), label: "Boost" }],
  };
}

/** Partials 1 & 2's booster (Booster 1&2). */
export const BOOST_12_LAYOUT = boostLayout(PcmsPmtField.Booster12);
/** Partials 3 & 4's booster (Booster 3&4). */
export const BOOST_34_LAYOUT = boostLayout(PcmsPmtField.Booster34);

/** A pair's ring modulator (Structure 5-10): no settings, only what it multiplies. */
function ringLayout(product: string): SectionLayout<PcmsPmtField> {
  return {
    title: "RING",
    titleEdge: "top",
    w: 3,
    h: 3,
    fields: PCMS_PMT_FIELDS,
    items: [],
    silkscreen: [{ kind: "label", text: product, x: 1.5, y: 1.5 }],
  };
}

/** Partials 1 & 2's ring modulator. */
export const RING_12_LAYOUT = ringLayout("1×2");
/** Partials 3 & 4's ring modulator. */
export const RING_34_LAYOUT = ringLayout("3×4");
