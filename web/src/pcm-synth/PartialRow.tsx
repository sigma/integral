import {
  SynthKnob,
  SynthFader,
  SynthSwitch,
  SectionPanel,
  PartialPowerButton,
  BreakpointEnvelope,
  FaderGroup,
  OutputStrip,
} from "../synth-ui";
import type { ReactNode } from "react";
import { SignalArrows, type Edge } from "../synth-ui/SignalArrows";
import { PcmsPartialField, PcmsPmtField, pcms_partial_spec, pcms_pmt_spec } from "../../pkg/integral_wasm.js";
import { routingOf, type Block, type Side } from "./structureRouting";
import { knobRange, switchOptions } from "../fieldSpec";
import { VelocityCurveSelect } from "./VelocityCurveSelect";
import { LfoScope } from "../synth-ui/LfoScope";
import { FilterScope } from "../synth-ui/FilterScope";
import css from "../PcmSynthEditor.module.css";
import type { PartialParamHandler, PartialSetter, PcmSynthPartial } from "./types";
import { WavePicker, type ExpansionMarking } from "../WavePicker";
import { waveSourceOf } from "../waveCatalog";

/** Whether an LFO Waveform value is STEP, the one that plays the Step LFO. */
function isStepWaveform(waveform: number): boolean {
  return pcms_partial_spec(PcmsPartialField.Lfo1Waveform).formatValue(waveform) === "STEP";
}

/** A box label, with its partial number when the box sits in a merged pair. */
function numbered(label: string, n?: number, sep = " "): string {
  return n ? `${label}${sep}${n}` : label;
}

/** A setting's tooltip, noting when it follows partial `follows` in a merged Structure. */
function followsTitle(title: string, follows?: number): string {
  return follows ? `${title}. Follows partial ${follows} in this Structure.` : title;
}

/** One partial of a pair, with what its row needs. */
export interface PairPartial {
  partial: PcmSynthPartial | null;
  isOn: boolean;
  onParam: PartialParamHandler;
  /** Sets the Partial's PMT Partial Switch. */
  onSwitch: (value: number) => void;
}

/**
 * A pair of partials (1 & 2, or 3 & 4) laid out along their Structure's
 * signal path: each row keeps its modulation boxes (LFOs, STEP LFO), WAVE,
 * PITCH and CONTROL, then the blocks before the join; blocks after the join,
 * and a merged pair's single OUTPUT (the lower partial's), span both rows.
 */
/** Outer grid columns: power + LFOs, lane heads, then each pair's path sub-grid. */
const LFO_COLUMN = 1;
const HEAD_COLUMN = 2;
const PATH_COLUMN = 3;

/** One pair of partials (1 & 2, or 3 & 4) and its Structure. */
export interface Pair {
  /** Index (0-based) of the pair's upper partial: 0 or 2. */
  first: number;
  /** The pair's Structure Type (0-9). */
  structure: number;
  upper: PairPartial;
  lower: PairPartial;
  /** The pair's Booster depth (0-3), and its setter: shown on the path where the Structure boosts. */
  booster: number;
  onBooster: (value: number) => void;
  /** The PMT field for this pair's Booster (Booster 1&2 or 3&4), for its range and labels. */
  boosterField: PcmsPmtField;
}

/**
 * The partials, a pair at a time, on one grid: the LFO and WAVE columns line
 * up across all partials, and each pair's path follows its Structure.
 */
export function PartialGrid({ pairs, marking }: { pairs: Pair[]; marking?: ExpansionMarking }) {
  const laid = pairs.map((pair, i) => pairLayout(pair, 2 * i, marking));
  return (
    <div className={css.partialGrid}>
      {laid.flatMap((l) => l.cells)}
      <SignalArrows edges={laid.flatMap((l) => l.edges)} />
    </div>
  );
}

/**
 * A pair's blocks along its Structure's signal path, on grid rows
 * `rowBase + 1` (upper partial) and `rowBase + 2` (lower): the detached LFO
 * blocks, each lane's head (WAVE, PITCH, CONTROL), its blocks before the
 * join, then a merged block centred on both rows, with arrows between blocks.
 */
function pairLayout(
  { first, structure, upper, lower, booster, onBooster, boosterField }: Pair,
  rowBase: number,
  marking?: ExpansionMarking,
): { cells: ReactNode[]; edges: Edge[] } {
  const routing = routingOf(structure);
  const sides = { upper, lower };
  const number = { upper: first + 1, lower: first + 2 };
  const setters = {
    upper: setterFor(upper.onParam),
    lower: setterFor(lower.onParam),
  };
  const merged = !!routing.join;
  /** A box's partial number, shown only once the pair is merged. */
  const nOf = (side: Side) => (merged ? number[side] : undefined);
  const followsFor = (side: Side) => (merged && side === "upper" ? number.lower : undefined);

  /** One box of a block, dimmed with its own partial's power. */
  const box = (key: string, side: Side, content: ReactNode) => (
    <div key={key} className={`${css.blockBox} ${sides[side].isOn ? "" : css.partialDimmed}`}>{content}</div>
  );
  const pathBox = (b: Block, key: string): ReactNode => {
    if (b.kind === "booster") {
      // The booster belongs to the pair; it dims only once both partials are off.
      return (
        <div key={key} className={`${css.blockBox} ${upper.isOn || lower.isOn ? "" : css.partialDimmed}`}>
          <BoosterSection value={booster} onChange={onBooster} field={boosterField} pair={number} />
        </div>
      );
    }
    const side = b.side;
    const p = sides[side].partial;
    if (!p) return null;
    if (b.kind === "filter") return box(key, side, <TvfSection partial={p} set={setters[side]} n={nOf(side)} />);
    if (b.kind === "amp") {
      return box(key, side, <TvaSection partial={p} set={setters[side]} n={nOf(side)} follows={followsFor(side)} />);
    }
    return null; // WAVE heads its lane
  };
  const outputBox = (side: Side, n?: number): ReactNode => {
    const p = sides[side].partial;
    return p && box(`out-${side}`, side, <OutputSection partial={p} set={setters[side]} n={n} />);
  };

  const cells: ReactNode[] = [];
  /** Blocks after the WAVE column: the pair's own path, on its own sub-grid. */
  const path: ReactNode[] = [];
  const edges: Edge[] = [];
  /**
   * Place a block at `column`: in its partial's row, or centred across both
   * rows once the lanes have merged. `node` names it for the arrows.
   */
  const place = (node: string, row: Side | "both", column: number, content: ReactNode, span = 1) => {
    // Shared columns (power, LFOs, heads) sit on the outer grid's rows; the
    // path columns on the pair's sub-grid, whose two rows are the pair's.
    const inPath = column >= PATH_COLUMN;
    const rowOf = (r: number) => (inPath ? r : rowBase + r);
    (inPath ? path : cells).push(
      <div key={node} data-node={node} className={`${css.block} ${row === "both" ? css.mergedCell : ""}`}
        style={{
          gridColumn: `${inPath ? column - PATH_COLUMN + 1 : column} / span ${span}`,
          gridRow: row === "both" ? `${rowOf(1)} / span 2` : rowOf(row === "upper" ? 1 : 2),
        }}>
        {content}
      </div>,
    );
  };

  const lastBlock: Record<Side, string> = { upper: "", lower: "" };
  const preColumn = routing.upper.length > 1 || routing.lower.length > 1;
  for (const side of ["upper", "lower"] as const) {
    const p = sides[side].partial;
    const idx = side === "upper" ? first : first + 1;
    const power = (
      <PartialPowerButton key="power" label={`Partial ${number[side]}`} isOn={sides[side].isOn}
        onToggle={() => sides[side].onSwitch(sides[side].isOn ? 0 : 1)} />
    );
    if (!p) {
      place(`${first}-lfo-${side}`, side, LFO_COLUMN, [power]);
      continue;
    }
    const set = setters[side];
    // The power label, then the modulation sources: detached from the signal path, no arrows.
    place(`${first}-lfo-${side}`, side, LFO_COLUMN, [
      power,
      box("lfo1", side, <Lfo1Section partial={p} set={set} n={nOf(side)} follows={followsFor(side)} />),
      box("lfo2", side, <Lfo2Section partial={p} set={set} n={nOf(side)} follows={followsFor(side)} />),
      box("step", side, <StepLfoSection partial={p} set={set} n={nOf(side)}
        dimmed={!isStepWaveform(p.lfo1Waveform) && !isStepWaveform(p.lfo2Waveform)} />),
    ]);
    const head = [
      box("wave", side, <WaveSection partial={p} set={set} n={nOf(side)} marking={marking} />),
      box("pitch", side, <PitchSection partial={p} set={set} n={nOf(side)} />),
      box("control", side, <ControlSection partial={p} set={set} n={nOf(side)} />),
    ];
    if (!merged) {
      // An independent lane is one straight chain: a single block from WAVE to
      // OUTPUT, starting in the shared WAVE column and running on over the path's.
      place(`out-${idx}`, side, HEAD_COLUMN, [
        ...head,
        ...routing[side].slice(1).map((b, i) => pathBox(b, `pre${i}`)),
        outputBox(side),
      ], 2);
      continue;
    }
    // The lane's head: the wave and what shapes it, aligned in both rows.
    const headNode = `${first}-head-${side}`;
    place(headNode, side, HEAD_COLUMN, head);
    lastBlock[side] = headNode;
    // The lane's own blocks before the join.
    const pre = routing[side].slice(1).map((b, i) => pathBox(b, `pre${i}`));
    if (pre.length) {
      const node = `${first}-pre-${side}`;
      place(node, side, PATH_COLUMN, pre);
      edges.push({ from: headNode, to: node });
      lastBlock[side] = node;
    }
  }

  const join = routing.join;
  /**
   * A lane's edge into the join. A lane with nothing before the join leaves
   * from its head, so it goes around the other lane's blocks in that column.
   */
  const intoJoin = (side: Side, to: string): Edge => {
    const other = side === "upper" ? "lower" : "upper";
    const skips = lastBlock[side].includes("-head-") && lastBlock[other].includes("-pre-");
    return { from: lastBlock[side], to, around: skips ? lastBlock[other] : undefined };
  };
  if (join) {
    let column = PATH_COLUMN + (preColumn ? 1 : 0);
    const node = `out-${first + 1}`;
    if (join.kind === "ring") {
      // The ring modulator: a box of its own, both lanes in, its product out.
      const ring = `${first}-ring`;
      place(ring, "both", column++, (
        <div className={`${css.blockBox} ${upper.isOn || lower.isOn ? "" : css.partialDimmed}`}>
          <RingSection pair={number} />
        </div>
      ));
      edges.push(intoJoin("upper", ring), intoJoin("lower", ring));
      if (join.lowerDry) {
        // Partial 2 (4) also goes in dry, summed with the ring's product.
        edges.push({ from: ring, to: node, glyph: "+" }, { from: lastBlock.lower, to: node, glyph: "+", around: ring });
      } else {
        edges.push({ from: ring, to: node });
      }
    } else {
      // Summed: type 4's booster then opens the merged block.
      edges.push({ ...intoJoin("upper", node), glyph: "+" }, { ...intoJoin("lower", node), glyph: "+" });
    }
    // After the join: one block, centred on both lanes, ending in the pair's single output.
    place(node, "both", column, [
      ...(join.kind === "booster" ? [pathBox({ kind: "booster" }, "boost")] : []),
      ...join.after.map((b, i) => pathBox(b, `after${i}`)),
      outputBox("lower", number.lower),
    ]);
  }

  if (path.length) {
    cells.push(
      <div key={`${first}-path`} className={css.pairPath} style={{ gridColumn: PATH_COLUMN, gridRow: `${rowBase + 1} / span 2` }}>
        {path}
      </div>,
    );
  }
  return { cells, edges };
}

function setterFor(onParam: PartialParamHandler): PartialSetter {
  return (field, key, index = 0) => (v) => onParam(field, key, index, v);
}

// ---------------------------------------------------------------------------
// Wave Section
// ---------------------------------------------------------------------------

function WaveSection({
  partial,
  n,
  set,
  marking,
}: {
  partial: PcmSynthPartial;
  /** Partial number shown on the box, in a merged pair. */
  n?: number;
  set: PartialSetter;
  marking?: ExpansionMarking;
}) {
  return (
    <SectionPanel label={numbered("WAVE", n, " ")} accentColor="#fc8">
      <PartialWave partial={partial} set={set} marking={marking} />
      <div className={css.waveControls}>
        <div className={css.switchColumn}>
          <SynthSwitch label="Tempo" value={partial.waveTempoSync} options={switchOptions(pcms_partial_spec(PcmsPartialField.WaveTempoSync))}
            onChange={set(PcmsPartialField.WaveTempoSync, "waveTempoSync")} title="Sync wave playback to song tempo" />
          <SynthSwitch label="FXM" value={partial.waveFxmSwitch} options={switchOptions(pcms_partial_spec(PcmsPartialField.WaveFxmSwitch))}
            onChange={set(PcmsPartialField.WaveFxmSwitch, "waveFxmSwitch")} title="Frequency Cross Modulation" />
        </div>
        <SynthKnob label="Gain" value={partial.waveGain} {...knobRange(pcms_partial_spec(PcmsPartialField.WaveGain))}
          onChange={set(PcmsPartialField.WaveGain, "waveGain")} color="#fc8" title="Wave output gain" />
        <div className={css.knobGrid}>
          <SynthKnob label="FXM Col" value={partial.waveFxmColor} {...knobRange(pcms_partial_spec(PcmsPartialField.WaveFxmColor))}
            onChange={set(PcmsPartialField.WaveFxmColor, "waveFxmColor")} color="#fc8"
            title="FXM Color — tonal character" />
          <SynthKnob label="FXM Dpt" value={partial.waveFxmDepth} {...knobRange(pcms_partial_spec(PcmsPartialField.WaveFxmDepth))}
            onChange={set(PcmsPartialField.WaveFxmDepth, "waveFxmDepth")} color="#fc8"
            title="FXM Depth — intensity" />
        </div>
      </div>
    </SectionPanel>
  );
}

/** A partial's Wave Group and L/R Wave Numbers, picked by name. */
export function PartialWave({
  partial,
  set,
  marking,
}: {
  partial: Pick<
    PcmSynthPartial,
    "waveGroupType" | "waveGroupId" | "waveNumberL" | "waveNumberR"
  >;
  set: PartialSetter;
  marking?: ExpansionMarking;
}) {
  return (
    <WavePicker
      source={waveSourceOf(partial.waveGroupType, partial.waveGroupId)}
      unknownGroupLabel={`Type ${partial.waveGroupType} ID ${partial.waveGroupId}`}
      stereo
      l={partial.waveNumberL}
      r={partial.waveNumberR}
      onChange={(side, n) =>
        side === "l"
          ? set(PcmsPartialField.WaveNumberL, "waveNumberL")(n)
          : set(PcmsPartialField.WaveNumberR, "waveNumberR")(n)
      }
      onGroupChange={(source) => {
        if (!source.params) return;
        const [type, id] = source.params;
        set(PcmsPartialField.WaveGroupType, "waveGroupType")(type);
        set(PcmsPartialField.WaveGroupId, "waveGroupId")(id);
      }}
      marking={marking}
    />
  );
}

// ---------------------------------------------------------------------------
// Pitch + Envelope Section
// ---------------------------------------------------------------------------

function PitchSection({
  partial,
  n,
  set,
}: {
  partial: PcmSynthPartial;
  /** Partial number shown on the box, in a merged pair. */
  n?: number;
  set: PartialSetter;
}) {
  return (
    <SectionPanel label={numbered("PITCH", n, " ")} accentColor="#fc8">
      <div className={css.panelRow}>
        <SynthKnob label="Coarse" value={partial.coarseTune} {...knobRange(pcms_partial_spec(PcmsPartialField.CoarseTune))}
          onChange={set(PcmsPartialField.CoarseTune, "coarseTune")} color="#fc8"
          title="Coarse tuning (semitones)" />
        <SynthKnob label="Fine" value={partial.fineTune} {...knobRange(pcms_partial_spec(PcmsPartialField.FineTune))}
          onChange={set(PcmsPartialField.FineTune, "fineTune")} color="#fc8"
          title="Fine tuning (cents)" />
        <SynthKnob label="Random" value={partial.randomPitchDepth}
          {...knobRange(pcms_partial_spec(PcmsPartialField.RandomPitchDepth))}
          onChange={set(PcmsPartialField.RandomPitchDepth, "randomPitchDepth")} color="#fc8"
          title="Random pitch variation on each note (cents)" />
        <SynthKnob label="Key Trk" value={partial.wavePitchKeyfollow} {...knobRange(pcms_partial_spec(PcmsPartialField.WavePitchKeyfollow))}
          onChange={set(PcmsPartialField.WavePitchKeyfollow, "wavePitchKeyfollow")} color="#fc8"
          title="Pitch keyboard tracking" />
      </div>
      <div className={css.envBlock}>
        <div className={css.envGrid}>
          <SynthKnob label="Depth" value={partial.pitchEnvDepth}
            {...knobRange(pcms_partial_spec(PcmsPartialField.PitchEnvDepth))}
            onChange={set(PcmsPartialField.PitchEnvDepth, "pitchEnvDepth")} color="#fc8"
            title="Pitch envelope depth (semitones) — how much the envelope affects pitch" />
          <SynthKnob label="Vel Sns" value={partial.pitchEnvVelocitySens} {...knobRange(pcms_partial_spec(PcmsPartialField.PitchEnvVelocitySens))}
            onChange={set(PcmsPartialField.PitchEnvVelocitySens, "pitchEnvVelocitySens")} color="#fc8"
            title="Velocity sensitivity — how key velocity affects envelope depth" />
          <SynthKnob label="Key Trk" value={partial.pitchEnvTimeKeyfollow} {...knobRange(pcms_partial_spec(PcmsPartialField.PitchEnvTimeKeyfollow))}
            onChange={set(PcmsPartialField.PitchEnvTimeKeyfollow, "pitchEnvTimeKeyfollow")} color="#fc8"
            title="Envelope time key tracking — higher keys = faster envelope" />
          <SynthKnob label="T1 Vel" value={partial.pitchEnvT1VelocitySens} {...knobRange(pcms_partial_spec(PcmsPartialField.PitchEnvT1VelocitySens))}
            onChange={set(PcmsPartialField.PitchEnvT1VelocitySens, "pitchEnvT1VelocitySens")} color="#fc8"
            title="Pitch envelope Time 1 velocity sensitivity — harder keys change the first stage time" />
          <SynthKnob label="T4 Vel" value={partial.pitchEnvT4VelocitySens} {...knobRange(pcms_partial_spec(PcmsPartialField.PitchEnvT4VelocitySens))}
            onChange={set(PcmsPartialField.PitchEnvT4VelocitySens, "pitchEnvT4VelocitySens")} color="#fc8"
            title="Pitch envelope Time 4 velocity sensitivity — release velocity changes the release time" />
        </div>
        <BreakpointEnvelope
          compact
          times={[{
            label: "T1", value: partial.pitchEnvTime[0] ?? 0,
            onChange: set(PcmsPartialField.PitchEnvTime, "pitchEnvTime", 0), ...knobRange(pcms_partial_spec(PcmsPartialField.PitchEnvTime)),
          }, {
            label: "T2", value: partial.pitchEnvTime[1] ?? 0,
            onChange: set(PcmsPartialField.PitchEnvTime, "pitchEnvTime", 1), ...knobRange(pcms_partial_spec(PcmsPartialField.PitchEnvTime)),
          }, {
            label: "T3", value: partial.pitchEnvTime[2] ?? 0,
            onChange: set(PcmsPartialField.PitchEnvTime, "pitchEnvTime", 2), ...knobRange(pcms_partial_spec(PcmsPartialField.PitchEnvTime)),
          }, {
            label: "T4", value: partial.pitchEnvTime[3] ?? 0,
            onChange: set(PcmsPartialField.PitchEnvTime, "pitchEnvTime", 3), ...knobRange(pcms_partial_spec(PcmsPartialField.PitchEnvTime)),
          }]}
          levels={[
            { label: "L0", value: partial.pitchEnvLevel[0] ?? 64,
              onChange: set(PcmsPartialField.PitchEnvLevel, "pitchEnvLevel", 0), ...knobRange(pcms_partial_spec(PcmsPartialField.PitchEnvLevel)), },
            { label: "L1", value: partial.pitchEnvLevel[1] ?? 64,
              onChange: set(PcmsPartialField.PitchEnvLevel, "pitchEnvLevel", 1), ...knobRange(pcms_partial_spec(PcmsPartialField.PitchEnvLevel)), },
            { label: "L2", value: partial.pitchEnvLevel[2] ?? 64,
              onChange: set(PcmsPartialField.PitchEnvLevel, "pitchEnvLevel", 2), ...knobRange(pcms_partial_spec(PcmsPartialField.PitchEnvLevel)), },
            { label: "L3", value: partial.pitchEnvLevel[3] ?? 64,
              onChange: set(PcmsPartialField.PitchEnvLevel, "pitchEnvLevel", 3), ...knobRange(pcms_partial_spec(PcmsPartialField.PitchEnvLevel)), },
            { label: "L4", value: partial.pitchEnvLevel[4] ?? 64,
              onChange: set(PcmsPartialField.PitchEnvLevel, "pitchEnvLevel", 4), ...knobRange(pcms_partial_spec(PcmsPartialField.PitchEnvLevel)), },
          ]}
        />
      </div>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// TVF (Filter) Section
// ---------------------------------------------------------------------------

function TvfSection({
  partial,
  n,
  set,
}: {
  partial: PcmSynthPartial;
  /** Partial number shown on the box, in a merged pair. */
  n?: number;
  set: PartialSetter;
}) {
  return (
    <SectionPanel label={numbered("FILTER", n, " ")} accentColor="#68c">
      <div className={css.envGrid}>
        <div className={css.filterType}>
          <SynthSwitch label="Type" value={partial.tvfFilterType} vertical
            options={switchOptions(pcms_partial_spec(PcmsPartialField.TvfFilterType))}
            onChange={set(PcmsPartialField.TvfFilterType, "tvfFilterType")} title="Filter type" />
        </div>
        <FilterScope className={css.filterScope}
          type={pcms_partial_spec(PcmsPartialField.TvfFilterType).formatValue(partial.tvfFilterType)}
          cutoff={partial.tvfCutoffFrequency} resonance={partial.tvfResonance} />
        <div className={css.at3}>
          <SynthKnob label="Key Trk" value={partial.tvfCutoffKeyfollow} {...knobRange(pcms_partial_spec(PcmsPartialField.TvfCutoffKeyfollow))}
            onChange={set(PcmsPartialField.TvfCutoffKeyfollow, "tvfCutoffKeyfollow")} color="#68c"
            title="Cutoff key tracking — higher keys = higher cutoff" />
        </div>
        <SynthKnob label="Cutoff" value={partial.tvfCutoffFrequency} {...knobRange(pcms_partial_spec(PcmsPartialField.TvfCutoffFrequency))}
          onChange={set(PcmsPartialField.TvfCutoffFrequency, "tvfCutoffFrequency")} color="#68c"
          title="Filter cutoff frequency" />
        <SynthKnob label="Reso" value={partial.tvfResonance} {...knobRange(pcms_partial_spec(PcmsPartialField.TvfResonance))}
          onChange={set(PcmsPartialField.TvfResonance, "tvfResonance")} color="#68c"
          title="Filter resonance — emphasis at the cutoff frequency" />
        <VelocityCurveSelect className={css.at1} label="Vel Crv" value={partial.tvfCutoffVelocityCurve}
          options={switchOptions(pcms_partial_spec(PcmsPartialField.TvfCutoffVelocityCurve))}
          onChange={set(PcmsPartialField.TvfCutoffVelocityCurve, "tvfCutoffVelocityCurve")} color="#68c" title="Cutoff velocity response curve shape" />
        <VelocityCurveSelect label="Env Crv" value={partial.tvfEnvVelocityCurve}
          options={switchOptions(pcms_partial_spec(PcmsPartialField.TvfEnvVelocityCurve))}
          onChange={set(PcmsPartialField.TvfEnvVelocityCurve, "tvfEnvVelocityCurve")} color="#68c" title="Envelope velocity response curve shape" />
        <div className={css.at4}>
          <SynthKnob label="Cut Vel" value={partial.tvfCutoffVelocitySens} {...knobRange(pcms_partial_spec(PcmsPartialField.TvfCutoffVelocitySens))}
            onChange={set(PcmsPartialField.TvfCutoffVelocitySens, "tvfCutoffVelocitySens")} color="#68c"
            title="Cutoff velocity sensitivity — harder keys = higher cutoff" />
        </div>
        <SynthKnob label="Res Vel" value={partial.tvfResonanceVelocitySens} {...knobRange(pcms_partial_spec(PcmsPartialField.TvfResonanceVelocitySens))}
          onChange={set(PcmsPartialField.TvfResonanceVelocitySens, "tvfResonanceVelocitySens")} color="#68c"
          title="Resonance velocity sensitivity — harder keys = more resonance" />
      </div>
      <div className={css.envBlock}>
        <div className={css.envGrid}>
          <SynthKnob label="Depth" value={partial.tvfEnvDepth} {...knobRange(pcms_partial_spec(PcmsPartialField.TvfEnvDepth))}
            onChange={set(PcmsPartialField.TvfEnvDepth, "tvfEnvDepth")} color="#68c"
            title="Filter envelope depth — how much the envelope sweeps the cutoff" />
          <SynthKnob label="Vel Sns" value={partial.tvfEnvVelocitySens} {...knobRange(pcms_partial_spec(PcmsPartialField.TvfEnvVelocitySens))}
            onChange={set(PcmsPartialField.TvfEnvVelocitySens, "tvfEnvVelocitySens")} color="#68c"
            title="Envelope velocity sensitivity — harder keys = deeper sweep" />
          <SynthKnob label="Key Trk" value={partial.tvfEnvTimeKeyfollow} {...knobRange(pcms_partial_spec(PcmsPartialField.TvfEnvTimeKeyfollow))}
            onChange={set(PcmsPartialField.TvfEnvTimeKeyfollow, "tvfEnvTimeKeyfollow")} color="#68c"
            title="Envelope time key tracking — higher keys = faster envelope" />
          <SynthKnob label="T1 Vel" value={partial.tvfEnvT1VelocitySens} {...knobRange(pcms_partial_spec(PcmsPartialField.TvfEnvT1VelocitySens))}
            onChange={set(PcmsPartialField.TvfEnvT1VelocitySens, "tvfEnvT1VelocitySens")} color="#68c"
            title="Filter envelope Time 1 velocity sensitivity — harder keys change the first stage time" />
          <SynthKnob label="T4 Vel" value={partial.tvfEnvT4VelocitySens} {...knobRange(pcms_partial_spec(PcmsPartialField.TvfEnvT4VelocitySens))}
            onChange={set(PcmsPartialField.TvfEnvT4VelocitySens, "tvfEnvT4VelocitySens")} color="#68c"
            title="Filter envelope Time 4 velocity sensitivity — release velocity changes the release time" />
        </div>
        <BreakpointEnvelope
          compact
          times={[{
            label: "T1", value: partial.tvfEnvTime[0] ?? 0,
            onChange: set(PcmsPartialField.TvfEnvTime, "tvfEnvTime", 0), ...knobRange(pcms_partial_spec(PcmsPartialField.TvfEnvTime)),
          }, {
            label: "T2", value: partial.tvfEnvTime[1] ?? 0,
            onChange: set(PcmsPartialField.TvfEnvTime, "tvfEnvTime", 1), ...knobRange(pcms_partial_spec(PcmsPartialField.TvfEnvTime)),
          }, {
            label: "T3", value: partial.tvfEnvTime[2] ?? 0,
            onChange: set(PcmsPartialField.TvfEnvTime, "tvfEnvTime", 2), ...knobRange(pcms_partial_spec(PcmsPartialField.TvfEnvTime)),
          }, {
            label: "T4", value: partial.tvfEnvTime[3] ?? 0,
            onChange: set(PcmsPartialField.TvfEnvTime, "tvfEnvTime", 3), ...knobRange(pcms_partial_spec(PcmsPartialField.TvfEnvTime)),
          }]}
          levels={[
            { label: "L0", value: partial.tvfEnvLevel[0] ?? 0,
              onChange: set(PcmsPartialField.TvfEnvLevel, "tvfEnvLevel", 0), ...knobRange(pcms_partial_spec(PcmsPartialField.TvfEnvLevel)), },
            { label: "L1", value: partial.tvfEnvLevel[1] ?? 127,
              onChange: set(PcmsPartialField.TvfEnvLevel, "tvfEnvLevel", 1), ...knobRange(pcms_partial_spec(PcmsPartialField.TvfEnvLevel)), },
            { label: "L2", value: partial.tvfEnvLevel[2] ?? 127,
              onChange: set(PcmsPartialField.TvfEnvLevel, "tvfEnvLevel", 2), ...knobRange(pcms_partial_spec(PcmsPartialField.TvfEnvLevel)), },
            { label: "L3", value: partial.tvfEnvLevel[3] ?? 127,
              onChange: set(PcmsPartialField.TvfEnvLevel, "tvfEnvLevel", 3), ...knobRange(pcms_partial_spec(PcmsPartialField.TvfEnvLevel)), },
            { label: "L4", value: partial.tvfEnvLevel[4] ?? 0,
              onChange: set(PcmsPartialField.TvfEnvLevel, "tvfEnvLevel", 4), ...knobRange(pcms_partial_spec(PcmsPartialField.TvfEnvLevel)), },
          ]}
        />
      </div>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// TVA (Amp) Section
// ---------------------------------------------------------------------------

function TvaSection({
  partial,
  n,
  set,
  follows,
}: {
  partial: PcmSynthPartial;
  /** Partial number shown on the box, in a merged pair. */
  n?: number;
  set: PartialSetter;
  /** Partial whose pan settings this one follows (merged Structure). */
  follows?: number;
}) {
  return (
    <SectionPanel label={numbered("AMP", n, " ")} accentColor="#6c8">
      <div className={css.envGrid}>
        <div className={css.at2}>
          <SynthKnob label="Pan" value={partial.pan} {...knobRange(pcms_partial_spec(PcmsPartialField.Pan))}
            onChange={set(PcmsPartialField.Pan, "pan")} color="#6c8"
            title="Partial stereo panning" />
        </div>
        <div className={follows ? css.follows : undefined}>
          <SynthKnob label="Pan KF" value={partial.panKeyfollow} {...knobRange(pcms_partial_spec(PcmsPartialField.PanKeyfollow))}
            onChange={set(PcmsPartialField.PanKeyfollow, "panKeyfollow")} color="#6c8"
            title={followsTitle("Pan keyfollow — notes above C4 pan right (+) or left (−). When Structure Type is 2-10, partial 1 follows partial 2 and partial 3 follows partial 4.", follows)} />
        </div>
        <div className={follows ? css.follows : undefined}>
          <SynthKnob label="Rnd Pan" value={partial.randomPanDepth} {...knobRange(pcms_partial_spec(PcmsPartialField.RandomPanDepth))}
            onChange={set(PcmsPartialField.RandomPanDepth, "randomPanDepth")} color="#6c8"
            title={followsTitle("Random pan depth — each note lands at a random position. When Structure Type is 2-10, partial 1 follows partial 2 and partial 3 follows partial 4.", follows)} />
        </div>
        <div className={follows ? css.follows : undefined}>
          <SynthKnob label="Alt Pan" value={partial.alternatePanDepth} {...knobRange(pcms_partial_spec(PcmsPartialField.AlternatePanDepth))}
            onChange={set(PcmsPartialField.AlternatePanDepth, "alternatePanDepth")} color="#6c8"
            title={followsTitle("Alternate pan depth — successive notes alternate left and right; L/R sets which side comes first. When Structure Type is 2-10, partial 1 follows partial 2 and partial 3 follows partial 4.", follows)} />
        </div>
        <VelocityCurveSelect className={css.at1} label="Vel Crv" value={partial.tvaLevelVelocityCurve}
          options={switchOptions(pcms_partial_spec(PcmsPartialField.TvaLevelVelocityCurve))}
          onChange={set(PcmsPartialField.TvaLevelVelocityCurve, "tvaLevelVelocityCurve")} color="#6c8" title="Velocity response curve shape" />
        <div className={css.at3}>
          <SynthKnob label="Bias Lvl" value={partial.tvaBiasLevel} {...knobRange(pcms_partial_spec(PcmsPartialField.TvaBiasLevel))}
            onChange={set(PcmsPartialField.TvaBiasLevel, "tvaBiasLevel")} color="#6c8"
            title="Bias level — how steeply volume changes with distance from the bias key; the sign sets whether it rises or falls" />
        </div>
        <SynthKnob label="Bias Key" value={partial.tvaBiasPosition} {...knobRange(pcms_partial_spec(PcmsPartialField.TvaBiasPosition))}
          onChange={set(PcmsPartialField.TvaBiasPosition, "tvaBiasPosition")} color="#6c8"
          title="Bias key — the pivot key the volume change is measured from" />
        <SynthSwitch label="Bias Dir" value={partial.tvaBiasDirection} vertical
          options={switchOptions(pcms_partial_spec(PcmsPartialField.TvaBiasDirection))}
          onChange={set(PcmsPartialField.TvaBiasDirection, "tvaBiasDirection")} led={false}
          title="Bias direction — which keys the change applies to: below, above or both sides of the bias key, or a slope across all keys" />
      </div>
      <div className={css.envBlock}>
        <div className={css.envGrid}>
          <SynthKnob label="Level" value={partial.level} {...knobRange(pcms_partial_spec(PcmsPartialField.Level))}
            onChange={set(PcmsPartialField.Level, "level")} color="#6c8"
            title="Partial volume level" />
          <SynthKnob label="Vel Sns" value={partial.tvaLevelVelocitySens} {...knobRange(pcms_partial_spec(PcmsPartialField.TvaLevelVelocitySens))}
            onChange={set(PcmsPartialField.TvaLevelVelocitySens, "tvaLevelVelocitySens")} color="#6c8"
            title="Volume velocity sensitivity — harder keys = louder" />
          <SynthKnob label="Key Trk" value={partial.tvaEnvTimeKeyfollow} {...knobRange(pcms_partial_spec(PcmsPartialField.TvaEnvTimeKeyfollow))}
            onChange={set(PcmsPartialField.TvaEnvTimeKeyfollow, "tvaEnvTimeKeyfollow")} color="#6c8"
            title="Envelope time key tracking — higher keys = faster envelope" />
          <SynthKnob label="T1 Vel" value={partial.tvaEnvT1VelocitySens} {...knobRange(pcms_partial_spec(PcmsPartialField.TvaEnvT1VelocitySens))}
            onChange={set(PcmsPartialField.TvaEnvT1VelocitySens, "tvaEnvT1VelocitySens")} color="#6c8"
            title="Amp envelope Time 1 velocity sensitivity — harder keys change the first stage time" />
          <SynthKnob label="T4 Vel" value={partial.tvaEnvT4VelocitySens} {...knobRange(pcms_partial_spec(PcmsPartialField.TvaEnvT4VelocitySens))}
            onChange={set(PcmsPartialField.TvaEnvT4VelocitySens, "tvaEnvT4VelocitySens")} color="#6c8"
            title="Amp envelope Time 4 velocity sensitivity — release velocity changes the release time" />
        </div>
        <BreakpointEnvelope
          compact
          times={[{
            label: "T1", value: partial.tvaEnvTime[0] ?? 0,
            onChange: set(PcmsPartialField.TvaEnvTime, "tvaEnvTime", 0), ...knobRange(pcms_partial_spec(PcmsPartialField.TvaEnvTime)),
          }, {
            label: "T2", value: partial.tvaEnvTime[1] ?? 0,
            onChange: set(PcmsPartialField.TvaEnvTime, "tvaEnvTime", 1), ...knobRange(pcms_partial_spec(PcmsPartialField.TvaEnvTime)),
          }, {
            label: "T3", value: partial.tvaEnvTime[2] ?? 0,
            onChange: set(PcmsPartialField.TvaEnvTime, "tvaEnvTime", 2), ...knobRange(pcms_partial_spec(PcmsPartialField.TvaEnvTime)),
          }, {
            label: "T4", value: partial.tvaEnvTime[3] ?? 0,
            onChange: set(PcmsPartialField.TvaEnvTime, "tvaEnvTime", 3), ...knobRange(pcms_partial_spec(PcmsPartialField.TvaEnvTime)),
          }]}
          levels={[
            { label: "L0", at: 0, text: "0" },
            { label: "L1", value: partial.tvaEnvLevel[0] ?? 127,
              onChange: set(PcmsPartialField.TvaEnvLevel, "tvaEnvLevel", 0), ...knobRange(pcms_partial_spec(PcmsPartialField.TvaEnvLevel)), },
            { label: "L2", value: partial.tvaEnvLevel[1] ?? 127,
              onChange: set(PcmsPartialField.TvaEnvLevel, "tvaEnvLevel", 1), ...knobRange(pcms_partial_spec(PcmsPartialField.TvaEnvLevel)), },
            { label: "L3", value: partial.tvaEnvLevel[2] ?? 0,
              onChange: set(PcmsPartialField.TvaEnvLevel, "tvaEnvLevel", 2), ...knobRange(pcms_partial_spec(PcmsPartialField.TvaEnvLevel)), },
            { label: "L4", at: 0, text: "0" },
          ]}
        />
      </div>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// LFO1 Section
// ---------------------------------------------------------------------------

function Lfo1Section({
  partial,
  n,
  set,
  follows,
}: {
  partial: PcmSynthPartial;
  /** Partial number shown on the box, in a merged pair. */
  n?: number;
  set: PartialSetter;
  /** Partial whose pan settings this one follows (merged Structure). */
  follows?: number;
}) {
  return (
    <SectionPanel label={numbered("LFO1", n, " · ")} accentColor="#a6f">
      <div className={css.panelRow}>
        <SynthSwitch label="Wave" value={partial.lfo1Waveform} vertical
          options={switchOptions(pcms_partial_spec(PcmsPartialField.Lfo1Waveform))}
          onChange={set(PcmsPartialField.Lfo1Waveform, "lfo1Waveform")} title="LFO waveform shape" />
        <div className={css.scopeColumn}>
          <LfoScope className={css.scopeBox} rate={partial.lfo1Rate}
            shape={pcms_partial_spec(PcmsPartialField.Lfo1Waveform).formatValue(partial.lfo1Waveform)}
            steps={partial.lfoStepValues} smooth={partial.lfoStepType === 1} />
          <div className={css.knobGrid}>
            <SynthKnob label="Rate" value={partial.lfo1Rate} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo1Rate))}
              onChange={set(PcmsPartialField.Lfo1Rate, "lfo1Rate")} color="#a6f"
              title="LFO speed" />
            <SynthKnob label="Detune" value={partial.lfo1RateDetune} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo1RateDetune))}
              onChange={set(PcmsPartialField.Lfo1RateDetune, "lfo1RateDetune")} color="#a6f"
              title="Rate randomization — adds organic variation" />
            <SynthKnob label="Delay" value={partial.lfo1DelayTime} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo1DelayTime))}
              onChange={set(PcmsPartialField.Lfo1DelayTime, "lfo1DelayTime")} color="#a6f"
              title="Time before LFO starts after note-on" />
            <SynthKnob label="Dly KF" value={partial.lfo1DelayTimeKeyfollow} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo1DelayTimeKeyfollow))}
              onChange={set(PcmsPartialField.Lfo1DelayTimeKeyfollow, "lfo1DelayTimeKeyfollow")} color="#a6f"
              title="Delay time key tracking — higher keys = shorter delay" />
            <SynthKnob label="Offset" value={partial.lfo1Offset} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo1Offset))}
              onChange={set(PcmsPartialField.Lfo1Offset, "lfo1Offset")} color="#a6f"
              title="DC offset of the LFO waveform" />
          </div>
        </div>
      </div>
      <div className={css.panelRow}>
        <SynthSwitch label="Fade" value={partial.lfo1FadeMode} vertical
          options={switchOptions(pcms_partial_spec(PcmsPartialField.Lfo1FadeMode))}
          onChange={set(PcmsPartialField.Lfo1FadeMode, "lfo1FadeMode")} led={false}
          title="Fade behavior: fade in on note-on, fade out, or on key-off" />
        <SynthKnob label="Fade T" value={partial.lfo1FadeTime} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo1FadeTime))}
          onChange={set(PcmsPartialField.Lfo1FadeTime, "lfo1FadeTime")} color="#a6f"
          title="Fade time — how long the fade set by Fade takes (in or out)" />
        <SynthSwitch label="Key Trig" value={partial.lfo1KeyTrigger} options={switchOptions(pcms_partial_spec(PcmsPartialField.Lfo1KeyTrigger))}
          onChange={set(PcmsPartialField.Lfo1KeyTrigger, "lfo1KeyTrigger")}
          title="Reset LFO phase on each new note" />
      </div>
      <FaderGroup>
        <SynthFader label={n ? "PCH " + n : "Pitch"} value={partial.lfo1PitchDepth} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo1PitchDepth))}
          onChange={set(PcmsPartialField.Lfo1PitchDepth, "lfo1PitchDepth")} compact
          title="LFO modulation depth on pitch (vibrato)" />
        <SynthFader label={n ? "FLT " + n : "Filter"} value={partial.lfo1TvfDepth} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo1TvfDepth))}
          onChange={set(PcmsPartialField.Lfo1TvfDepth, "lfo1TvfDepth")} compact
          title="LFO modulation depth on filter cutoff (wah)" />
        <SynthFader label={n ? "AMP " + n : "Amp"} value={partial.lfo1TvaDepth} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo1TvaDepth))}
          onChange={set(PcmsPartialField.Lfo1TvaDepth, "lfo1TvaDepth")} compact
          title="LFO modulation depth on amplitude (tremolo)" />
        <div className={follows ? css.follows : undefined}>
          <SynthFader label={n ? "PAN " + n : "Pan"} value={partial.lfo1PanDepth} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo1PanDepth))}
            onChange={set(PcmsPartialField.Lfo1PanDepth, "lfo1PanDepth")} compact
            title={followsTitle("LFO modulation depth on panning (auto-pan)", follows)} />
        </div>
      </FaderGroup>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// LFO2 Section
// ---------------------------------------------------------------------------

function Lfo2Section({
  partial,
  n,
  set,
  follows,
}: {
  partial: PcmSynthPartial;
  /** Partial number shown on the box, in a merged pair. */
  n?: number;
  set: PartialSetter;
  /** Partial whose pan settings this one follows (merged Structure). */
  follows?: number;
}) {
  return (
    <SectionPanel label={numbered("LFO2", n, " · ")} accentColor="#a6f">
      <div className={css.panelRow}>
        <SynthSwitch label="Wave" value={partial.lfo2Waveform} vertical
          options={switchOptions(pcms_partial_spec(PcmsPartialField.Lfo2Waveform))}
          onChange={set(PcmsPartialField.Lfo2Waveform, "lfo2Waveform")} title="LFO waveform shape" />
        <div className={css.scopeColumn}>
          <LfoScope className={css.scopeBox} rate={partial.lfo2Rate}
            shape={pcms_partial_spec(PcmsPartialField.Lfo2Waveform).formatValue(partial.lfo2Waveform)}
            steps={partial.lfoStepValues} smooth={partial.lfoStepType === 1} />
          <div className={css.knobGrid}>
            <SynthKnob label="Rate" value={partial.lfo2Rate} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo2Rate))}
              onChange={set(PcmsPartialField.Lfo2Rate, "lfo2Rate")} color="#a6f"
              title="LFO speed" />
            <SynthKnob label="Detune" value={partial.lfo2RateDetune} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo2RateDetune))}
              onChange={set(PcmsPartialField.Lfo2RateDetune, "lfo2RateDetune")} color="#a6f"
              title="Rate randomization — adds organic variation" />
            <SynthKnob label="Delay" value={partial.lfo2DelayTime} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo2DelayTime))}
              onChange={set(PcmsPartialField.Lfo2DelayTime, "lfo2DelayTime")} color="#a6f"
              title="Time before LFO starts after note-on" />
            <SynthKnob label="Dly KF" value={partial.lfo2DelayTimeKeyfollow} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo2DelayTimeKeyfollow))}
              onChange={set(PcmsPartialField.Lfo2DelayTimeKeyfollow, "lfo2DelayTimeKeyfollow")} color="#a6f"
              title="Delay time key tracking — higher keys = shorter delay" />
            <SynthKnob label="Offset" value={partial.lfo2Offset} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo2Offset))}
              onChange={set(PcmsPartialField.Lfo2Offset, "lfo2Offset")} color="#a6f"
              title="DC offset of the LFO waveform" />
          </div>
        </div>
      </div>
      <div className={css.panelRow}>
        <SynthSwitch label="Fade" value={partial.lfo2FadeMode} vertical
          options={switchOptions(pcms_partial_spec(PcmsPartialField.Lfo2FadeMode))}
          onChange={set(PcmsPartialField.Lfo2FadeMode, "lfo2FadeMode")} led={false}
          title="Fade behavior: fade in on note-on, fade out, or on key-off" />
        <SynthKnob label="Fade T" value={partial.lfo2FadeTime} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo2FadeTime))}
          onChange={set(PcmsPartialField.Lfo2FadeTime, "lfo2FadeTime")} color="#a6f"
          title="Fade time — how long the fade set by Fade takes (in or out)" />
        <SynthSwitch label="Key Trig" value={partial.lfo2KeyTrigger} options={switchOptions(pcms_partial_spec(PcmsPartialField.Lfo2KeyTrigger))}
          onChange={set(PcmsPartialField.Lfo2KeyTrigger, "lfo2KeyTrigger")}
          title="Reset LFO phase on each new note" />
      </div>
      <FaderGroup>
        <SynthFader label={n ? "PCH " + n : "Pitch"} value={partial.lfo2PitchDepth} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo2PitchDepth))}
          onChange={set(PcmsPartialField.Lfo2PitchDepth, "lfo2PitchDepth")} compact
          title="LFO modulation depth on pitch (vibrato)" />
        <SynthFader label={n ? "FLT " + n : "Filter"} value={partial.lfo2TvfDepth} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo2TvfDepth))}
          onChange={set(PcmsPartialField.Lfo2TvfDepth, "lfo2TvfDepth")} compact
          title="LFO modulation depth on filter cutoff (wah)" />
        <SynthFader label={n ? "AMP " + n : "Amp"} value={partial.lfo2TvaDepth} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo2TvaDepth))}
          onChange={set(PcmsPartialField.Lfo2TvaDepth, "lfo2TvaDepth")} compact
          title="LFO modulation depth on amplitude (tremolo)" />
        <div className={follows ? css.follows : undefined}>
          <SynthFader label={n ? "PAN " + n : "Pan"} value={partial.lfo2PanDepth} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo2PanDepth))}
            onChange={set(PcmsPartialField.Lfo2PanDepth, "lfo2PanDepth")} compact
            title={followsTitle("LFO modulation depth on panning (auto-pan)", follows)} />
        </div>
      </FaderGroup>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Step LFO Section
// ---------------------------------------------------------------------------

function StepLfoSection({
  partial,
  n,
  set,
  dimmed,
}: {
  partial: PcmSynthPartial;
  /** Partial number shown on the box, in a merged pair. */
  n?: number;
  set: PartialSetter;
  dimmed?: boolean;
}) {
  const steps = partial.lfoStepValues ?? [];
  const smooth = partial.lfoStepType === 1;
  // A flex wrapper, so the panel still fills its grid cell's height.
  const wrapStyle = { display: "flex", opacity: dimmed ? 0.35 : undefined };

  return (
    <div style={wrapStyle}>
    <SectionPanel label={numbered("STEP LFO", n, " · ")} accentColor="#a6f">
      <LfoScope className={css.stepViz} shape="STEP" rate={0} steps={steps} smooth={smooth} />
      <div className={css.panelRow}>
        <SynthSwitch label="Shape" value={partial.lfoStepType}
          options={switchOptions(pcms_partial_spec(PcmsPartialField.LfoStepType))}
          onChange={set(PcmsPartialField.LfoStepType, "lfoStepType")}
          title="Step shape: STEP = hard jumps, SMOOTH = interpolated transitions. Set LFO1 or LFO2 waveform to STEP to use this pattern." />
      </div>
      <div className={css.stepGrid}>
        {Array.from({ length: 16 }, (_, i) => (
          <SynthFader
            key={i}
            label={`${i + 1}`}
            value={steps[i] ?? 64} {...knobRange(pcms_partial_spec(PcmsPartialField.LfoStep))}
            onChange={set(PcmsPartialField.LfoStep, "lfoStepValues", i)}
            compact
          />
        ))}
      </div>
    </SectionPanel>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Control Section: how the partial responds to playing, kept per partial
// ---------------------------------------------------------------------------

function ControlSection({
  partial,
  n,
  set,
}: {
  partial: PcmSynthPartial;
  /** Partial number shown on the box, in a merged pair. */
  n?: number;
  set: PartialSetter;
}) {
  return (
    <SectionPanel label={numbered("CTRL", n, " ")} accentColor="#6c8">
      <SynthSwitch label="Env Mode" value={partial.envMode} vertical
        options={switchOptions(pcms_partial_spec(PcmsPartialField.EnvMode))}
        onChange={set(PcmsPartialField.EnvMode, "envMode")} led={false}
        title="Envelope mode: NO-SUS plays once, SUSTAIN holds at sustain level" />
      <div className={css.switchColumn}>
        <SynthSwitch label="Bender" value={partial.receiveBender} options={switchOptions(pcms_partial_spec(PcmsPartialField.ReceiveBender))}
          onChange={set(PcmsPartialField.ReceiveBender, "receiveBender")}
          title="Respond to pitch bend" />
        <SynthSwitch label="Express" value={partial.receiveExpression} options={switchOptions(pcms_partial_spec(PcmsPartialField.ReceiveExpression))}
          onChange={set(PcmsPartialField.ReceiveExpression, "receiveExpression")}
          title="Respond to expression pedal (CC11)" />
        <SynthSwitch label="Hold" value={partial.receiveHold1} options={switchOptions(pcms_partial_spec(PcmsPartialField.ReceiveHold1))}
          onChange={set(PcmsPartialField.ReceiveHold1, "receiveHold1")}
          title="Respond to sustain pedal (CC64)" />
      </div>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Booster Section: the pair's booster, on its signal path (Structure 3 and 4)
// ---------------------------------------------------------------------------

function BoosterSection({
  value,
  onChange,
  field,
  pair,
}: {
  value: number;
  onChange: (value: number) => void;
  field: PcmsPmtField;
  pair: { upper: number; lower: number };
}) {
  return (
    <SectionPanel label="BOOST" accentColor="#e86">
      <SynthKnob label="Boost" value={value} {...knobRange(pcms_pmt_spec(field))} onChange={onChange} color="#e86"
        title={`Booster depth for partials ${pair.upper} & ${pair.lower}: overdrives the signal; higher distorts more`} />
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Ring Section: the pair's ring modulator, on its signal path (Structure 5-10)
// ---------------------------------------------------------------------------

function RingSection({ pair }: { pair: { upper: number; lower: number } }) {
  return (
    <SectionPanel label="RING" accentColor="#e86">
      <div className={css.ringProduct}
        title={`Ring modulator: multiplies partials ${pair.upper} and ${pair.lower}, adding new (often metallic) overtones. No settings; partial ${pair.upper}'s AMP sets its depth.`}>
        {pair.upper} × {pair.lower}
      </div>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Output Section
// ---------------------------------------------------------------------------

function OutputSection({
  partial,
  n,
  set,
}: {
  partial: PcmSynthPartial;
  /** Partial number shown on the box, in a merged pair. */
  n?: number;
  set: PartialSetter;
}) {
  return (
    <SectionPanel label={numbered("OUTPUT", n, " ")} accentColor="#6c8">
      <div className={css.panelRow}>
        <SynthSwitch label="Delay" value={partial.delayMode} vertical
          options={switchOptions(pcms_partial_spec(PcmsPartialField.DelayMode))}
          onChange={set(PcmsPartialField.DelayMode, "delayMode")} led={false}
          title="Partial delay mode — when sound starts relative to note-on" />
        <SynthKnob label="Delay" value={partial.delayTime} {...knobRange(pcms_partial_spec(PcmsPartialField.DelayTime))}
          onChange={set(PcmsPartialField.DelayTime, "delayTime")} color="#6c8"
          title="Delay time before this partial sounds" />
      </div>
      <OutputStrip
        fx1={partial.chorusSend}
        fx2={partial.reverbSend}
        level={partial.outputLevel}
        onFx1Change={set(PcmsPartialField.ChorusSend, "chorusSend")}
        onFx2Change={set(PcmsPartialField.ReverbSend, "reverbSend")}
        onLevelChange={set(PcmsPartialField.OutputLevel, "outputLevel")}
        trackHeight={150}
      />
    </SectionPanel>
  );
}
