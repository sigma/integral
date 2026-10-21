/**
 * A PCM Synth partial's boxes, one Section each, drawing their own Controls.
 * `PcmSynthScreen` places them on the Grid, hosted on whole-Cell footprints.
 */

import {
  SynthKnob,
  SynthFader,
  SynthSwitch,
  SectionPanel,
  BreakpointEnvelope,
  FaderGroup,
  OutputStrip,
} from "../synth-ui";
import { ParameterFamily, PcmsPartialField, pcms_partial_spec } from "../../pkg/integral_wasm.js";
import { knobProps, knobRange, switchOptions } from "../fieldSpec";
import { VelocityCurveSelect } from "../synth-ui/VelocityCurveSelect";
import { LfoScope } from "../synth-ui/LfoScope";
import { TimeOrNoteKnob } from "./TimeOrNoteKnob";
import { FilterScope } from "../synth-ui/FilterScope";
import css from "../PcmSynthEditor.module.css";
import type { PartialParamHandler, PartialSetter, PcmSynthPartial } from "./types";
import { WavePicker, type ExpansionMarking } from "../WavePicker";
import { waveSourceOf } from "../waveCatalog";

/** A box label, with its partial number when the box sits in a merged pair. */
function numbered(label: string, n?: number, sep = " "): string {
  return n ? `${label}${sep}${n}` : label;
}

/** A setting's tooltip, noting when it follows partial `follows` in a merged Structure. */
function followsTitle(title: string, follows?: number): string {
  return follows ? `${title}. Follows partial ${follows} in this Structure.` : title;
}

/** The setter factory a partial's boxes take, writing through `onParam`. */
export function setterFor(onParam: PartialParamHandler): PartialSetter {
  return (field, key, index = 0) => (v) => onParam(field, key, index, v);
}

// ---------------------------------------------------------------------------
// Wave Section
// ---------------------------------------------------------------------------

/** The WAVE box: the wave picker, tempo sync, FXM and gain. */
export function WaveSection({
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
    <SectionPanel label={numbered("WAVE", n, " ")} family={ParameterFamily.Pitch}>
      <PartialWave partial={partial} set={set} marking={marking} />
      <div className={css.waveControls}>
        <div className={css.switchColumn}>
          <SynthSwitch label="Tempo" value={partial.waveTempoSync} options={switchOptions(pcms_partial_spec(PcmsPartialField.WaveTempoSync))}
            onChange={set(PcmsPartialField.WaveTempoSync, "waveTempoSync")} title="Sync wave playback to song tempo" />
          <SynthSwitch label="FXM" value={partial.waveFxmSwitch} options={switchOptions(pcms_partial_spec(PcmsPartialField.WaveFxmSwitch))}
            onChange={set(PcmsPartialField.WaveFxmSwitch, "waveFxmSwitch")} title="Frequency Cross Modulation" />
        </div>
        <SynthKnob label="Gain" value={partial.waveGain} {...knobProps(pcms_partial_spec(PcmsPartialField.WaveGain))}
          onChange={set(PcmsPartialField.WaveGain, "waveGain")} title="Wave output gain" />
        <div className={css.knobGrid}>
          <SynthKnob label="FXM Col" value={partial.waveFxmColor} {...knobProps(pcms_partial_spec(PcmsPartialField.WaveFxmColor))}
            onChange={set(PcmsPartialField.WaveFxmColor, "waveFxmColor")}
            title="FXM Color — tonal character" />
          <SynthKnob label="FXM Dpt" value={partial.waveFxmDepth} {...knobProps(pcms_partial_spec(PcmsPartialField.WaveFxmDepth))}
            onChange={set(PcmsPartialField.WaveFxmDepth, "waveFxmDepth")}
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

/** The PITCH box: tuning, and the pitch envelope with its knobs. */
export function PitchSection({
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
    <SectionPanel label={numbered("PITCH", n, " ")} family={ParameterFamily.Pitch}>
      <div className={css.panelRow}>
        <SynthKnob label="Coarse" value={partial.coarseTune} {...knobProps(pcms_partial_spec(PcmsPartialField.CoarseTune))}
          onChange={set(PcmsPartialField.CoarseTune, "coarseTune")}
          title="Coarse tuning (semitones)" />
        <SynthKnob label="Fine" value={partial.fineTune} {...knobProps(pcms_partial_spec(PcmsPartialField.FineTune))}
          onChange={set(PcmsPartialField.FineTune, "fineTune")}
          title="Fine tuning (cents)" />
        <SynthKnob label="Random" value={partial.randomPitchDepth}
          {...knobProps(pcms_partial_spec(PcmsPartialField.RandomPitchDepth))}
          onChange={set(PcmsPartialField.RandomPitchDepth, "randomPitchDepth")}
          title="Random pitch variation on each note (cents)" />
        <SynthKnob label="Key Trk" value={partial.wavePitchKeyfollow} {...knobProps(pcms_partial_spec(PcmsPartialField.WavePitchKeyfollow))}
          onChange={set(PcmsPartialField.WavePitchKeyfollow, "wavePitchKeyfollow")}
          title="Pitch keyboard tracking" />
      </div>
      <div className={css.envBlock}>
        <div className={css.envGrid}>
          <SynthKnob label="Depth" value={partial.pitchEnvDepth}
            {...knobProps(pcms_partial_spec(PcmsPartialField.PitchEnvDepth))}
            onChange={set(PcmsPartialField.PitchEnvDepth, "pitchEnvDepth")}
            title="Pitch envelope depth (semitones) — how much the envelope affects pitch" />
          <SynthKnob label="Vel Sns" value={partial.pitchEnvVelocitySens} {...knobProps(pcms_partial_spec(PcmsPartialField.PitchEnvVelocitySens))}
            onChange={set(PcmsPartialField.PitchEnvVelocitySens, "pitchEnvVelocitySens")}
            title="Velocity sensitivity — how key velocity affects envelope depth" />
          <SynthKnob label="Key Trk" value={partial.pitchEnvTimeKeyfollow} {...knobProps(pcms_partial_spec(PcmsPartialField.PitchEnvTimeKeyfollow))}
            onChange={set(PcmsPartialField.PitchEnvTimeKeyfollow, "pitchEnvTimeKeyfollow")}
            title="Envelope time key tracking — higher keys = faster envelope" />
          <SynthKnob label="T1 Vel" value={partial.pitchEnvT1VelocitySens} {...knobProps(pcms_partial_spec(PcmsPartialField.PitchEnvT1VelocitySens))}
            onChange={set(PcmsPartialField.PitchEnvT1VelocitySens, "pitchEnvT1VelocitySens")}
            title="Pitch envelope Time 1 velocity sensitivity — harder keys change the first stage time" />
          <SynthKnob label="T4 Vel" value={partial.pitchEnvT4VelocitySens} {...knobProps(pcms_partial_spec(PcmsPartialField.PitchEnvT4VelocitySens))}
            onChange={set(PcmsPartialField.PitchEnvT4VelocitySens, "pitchEnvT4VelocitySens")}
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

/** The FILTER (TVF) box: type, cutoff and resonance, and the filter envelope. */
export function TvfSection({
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
    <SectionPanel label={numbered("FILTER", n, " ")} family={ParameterFamily.Filter}>
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
          <SynthKnob label="Key Trk" value={partial.tvfCutoffKeyfollow} {...knobProps(pcms_partial_spec(PcmsPartialField.TvfCutoffKeyfollow))}
            onChange={set(PcmsPartialField.TvfCutoffKeyfollow, "tvfCutoffKeyfollow")}
            title="Cutoff key tracking — higher keys = higher cutoff" />
        </div>
        <SynthKnob label="Cutoff" value={partial.tvfCutoffFrequency} {...knobProps(pcms_partial_spec(PcmsPartialField.TvfCutoffFrequency))}
          onChange={set(PcmsPartialField.TvfCutoffFrequency, "tvfCutoffFrequency")}
          title="Filter cutoff frequency" />
        <SynthKnob label="Reso" value={partial.tvfResonance} {...knobProps(pcms_partial_spec(PcmsPartialField.TvfResonance))}
          onChange={set(PcmsPartialField.TvfResonance, "tvfResonance")}
          title="Filter resonance — emphasis at the cutoff frequency" />
        <VelocityCurveSelect className={css.at1} label="Vel Crv" value={partial.tvfCutoffVelocityCurve}
          options={switchOptions(pcms_partial_spec(PcmsPartialField.TvfCutoffVelocityCurve))}
          onChange={set(PcmsPartialField.TvfCutoffVelocityCurve, "tvfCutoffVelocityCurve")} family={pcms_partial_spec(PcmsPartialField.TvfCutoffVelocityCurve).family} title="Cutoff velocity response curve shape" />
        <VelocityCurveSelect label="Env Crv" value={partial.tvfEnvVelocityCurve}
          options={switchOptions(pcms_partial_spec(PcmsPartialField.TvfEnvVelocityCurve))}
          onChange={set(PcmsPartialField.TvfEnvVelocityCurve, "tvfEnvVelocityCurve")} family={pcms_partial_spec(PcmsPartialField.TvfEnvVelocityCurve).family} title="Envelope velocity response curve shape" />
        <div className={css.at4}>
          <SynthKnob label="Cut Vel" value={partial.tvfCutoffVelocitySens} {...knobProps(pcms_partial_spec(PcmsPartialField.TvfCutoffVelocitySens))}
            onChange={set(PcmsPartialField.TvfCutoffVelocitySens, "tvfCutoffVelocitySens")}
            title="Cutoff velocity sensitivity — harder keys = higher cutoff" />
        </div>
        <SynthKnob label="Res Vel" value={partial.tvfResonanceVelocitySens} {...knobProps(pcms_partial_spec(PcmsPartialField.TvfResonanceVelocitySens))}
          onChange={set(PcmsPartialField.TvfResonanceVelocitySens, "tvfResonanceVelocitySens")}
          title="Resonance velocity sensitivity — harder keys = more resonance" />
      </div>
      <div className={css.envBlock}>
        <div className={css.envGrid}>
          <SynthKnob label="Depth" value={partial.tvfEnvDepth} {...knobProps(pcms_partial_spec(PcmsPartialField.TvfEnvDepth))}
            onChange={set(PcmsPartialField.TvfEnvDepth, "tvfEnvDepth")}
            title="Filter envelope depth — how much the envelope sweeps the cutoff" />
          <SynthKnob label="Vel Sns" value={partial.tvfEnvVelocitySens} {...knobProps(pcms_partial_spec(PcmsPartialField.TvfEnvVelocitySens))}
            onChange={set(PcmsPartialField.TvfEnvVelocitySens, "tvfEnvVelocitySens")}
            title="Envelope velocity sensitivity — harder keys = deeper sweep" />
          <SynthKnob label="Key Trk" value={partial.tvfEnvTimeKeyfollow} {...knobProps(pcms_partial_spec(PcmsPartialField.TvfEnvTimeKeyfollow))}
            onChange={set(PcmsPartialField.TvfEnvTimeKeyfollow, "tvfEnvTimeKeyfollow")}
            title="Envelope time key tracking — higher keys = faster envelope" />
          <SynthKnob label="T1 Vel" value={partial.tvfEnvT1VelocitySens} {...knobProps(pcms_partial_spec(PcmsPartialField.TvfEnvT1VelocitySens))}
            onChange={set(PcmsPartialField.TvfEnvT1VelocitySens, "tvfEnvT1VelocitySens")}
            title="Filter envelope Time 1 velocity sensitivity — harder keys change the first stage time" />
          <SynthKnob label="T4 Vel" value={partial.tvfEnvT4VelocitySens} {...knobProps(pcms_partial_spec(PcmsPartialField.TvfEnvT4VelocitySens))}
            onChange={set(PcmsPartialField.TvfEnvT4VelocitySens, "tvfEnvT4VelocitySens")}
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

/** The AMP (TVA) box: level, pan, and the amp envelope. */
export function TvaSection({
  partial,
  n,
  set,
  followed,
}: {
  partial: PcmSynthPartial;
  /** Partial number shown on the box, in a merged pair. */
  n?: number;
  set: PartialSetter;
  /** The partial whose setting an inactive pan `field` follows (merged Structure), if any. */
  followed: (field: PcmsPartialField) => number | undefined;
}) {
  return (
    <SectionPanel label={numbered("AMP", n, " ")} family={ParameterFamily.Amp}>
      <div className={css.envGrid}>
        <div className={css.at2}>
          <SynthKnob label="Pan" value={partial.pan} {...knobProps(pcms_partial_spec(PcmsPartialField.Pan))}
            onChange={set(PcmsPartialField.Pan, "pan")}
            title="Partial stereo panning" />
        </div>
        <div className={followed(PcmsPartialField.PanKeyfollow) ? css.follows : undefined}>
          <SynthKnob label="Pan KF" value={partial.panKeyfollow} {...knobProps(pcms_partial_spec(PcmsPartialField.PanKeyfollow))}
            onChange={set(PcmsPartialField.PanKeyfollow, "panKeyfollow")}
            title={followsTitle("Pan keyfollow — notes above C4 pan right (+) or left (−). When Structure Type is 2-10, partial 1 follows partial 2 and partial 3 follows partial 4.", followed(PcmsPartialField.PanKeyfollow))} />
        </div>
        <div className={followed(PcmsPartialField.RandomPanDepth) ? css.follows : undefined}>
          <SynthKnob label="Rnd Pan" value={partial.randomPanDepth} {...knobProps(pcms_partial_spec(PcmsPartialField.RandomPanDepth))}
            onChange={set(PcmsPartialField.RandomPanDepth, "randomPanDepth")}
            title={followsTitle("Random pan depth — each note lands at a random position. When Structure Type is 2-10, partial 1 follows partial 2 and partial 3 follows partial 4.", followed(PcmsPartialField.RandomPanDepth))} />
        </div>
        <div className={followed(PcmsPartialField.AlternatePanDepth) ? css.follows : undefined}>
          <SynthKnob label="Alt Pan" value={partial.alternatePanDepth} {...knobProps(pcms_partial_spec(PcmsPartialField.AlternatePanDepth))}
            onChange={set(PcmsPartialField.AlternatePanDepth, "alternatePanDepth")}
            title={followsTitle("Alternate pan depth — successive notes alternate left and right; L/R sets which side comes first. When Structure Type is 2-10, partial 1 follows partial 2 and partial 3 follows partial 4.", followed(PcmsPartialField.AlternatePanDepth))} />
        </div>
        <VelocityCurveSelect className={css.at1} label="Vel Crv" value={partial.tvaLevelVelocityCurve}
          options={switchOptions(pcms_partial_spec(PcmsPartialField.TvaLevelVelocityCurve))}
          onChange={set(PcmsPartialField.TvaLevelVelocityCurve, "tvaLevelVelocityCurve")} family={pcms_partial_spec(PcmsPartialField.TvaLevelVelocityCurve).family} title="Velocity response curve shape" />
        <div className={css.at3}>
          <SynthKnob label="Bias Lvl" value={partial.tvaBiasLevel} {...knobProps(pcms_partial_spec(PcmsPartialField.TvaBiasLevel))}
            onChange={set(PcmsPartialField.TvaBiasLevel, "tvaBiasLevel")}
            title="Bias level — how steeply volume changes with distance from the bias key; the sign sets whether it rises or falls" />
        </div>
        <SynthKnob label="Bias Key" value={partial.tvaBiasPosition} {...knobProps(pcms_partial_spec(PcmsPartialField.TvaBiasPosition))}
          onChange={set(PcmsPartialField.TvaBiasPosition, "tvaBiasPosition")}
          title="Bias key — the pivot key the volume change is measured from" />
        <SynthSwitch label="Bias Dir" value={partial.tvaBiasDirection} vertical
          options={switchOptions(pcms_partial_spec(PcmsPartialField.TvaBiasDirection))}
          onChange={set(PcmsPartialField.TvaBiasDirection, "tvaBiasDirection")} led={false}
          title="Bias direction — which keys the change applies to: below, above or both sides of the bias key, or a slope across all keys" />
      </div>
      <div className={css.envBlock}>
        <div className={css.envGrid}>
          <SynthKnob label="Level" value={partial.level} {...knobProps(pcms_partial_spec(PcmsPartialField.Level))}
            onChange={set(PcmsPartialField.Level, "level")}
            title="Partial volume level" />
          <SynthKnob label="Vel Sns" value={partial.tvaLevelVelocitySens} {...knobProps(pcms_partial_spec(PcmsPartialField.TvaLevelVelocitySens))}
            onChange={set(PcmsPartialField.TvaLevelVelocitySens, "tvaLevelVelocitySens")}
            title="Volume velocity sensitivity — harder keys = louder" />
          <SynthKnob label="Key Trk" value={partial.tvaEnvTimeKeyfollow} {...knobProps(pcms_partial_spec(PcmsPartialField.TvaEnvTimeKeyfollow))}
            onChange={set(PcmsPartialField.TvaEnvTimeKeyfollow, "tvaEnvTimeKeyfollow")}
            title="Envelope time key tracking — higher keys = faster envelope" />
          <SynthKnob label="T1 Vel" value={partial.tvaEnvT1VelocitySens} {...knobProps(pcms_partial_spec(PcmsPartialField.TvaEnvT1VelocitySens))}
            onChange={set(PcmsPartialField.TvaEnvT1VelocitySens, "tvaEnvT1VelocitySens")}
            title="Amp envelope Time 1 velocity sensitivity — harder keys change the first stage time" />
          <SynthKnob label="T4 Vel" value={partial.tvaEnvT4VelocitySens} {...knobProps(pcms_partial_spec(PcmsPartialField.TvaEnvT4VelocitySens))}
            onChange={set(PcmsPartialField.TvaEnvT4VelocitySens, "tvaEnvT4VelocitySens")}
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

/** The LFO1 box: its wave, rate and the depths it sends. */
export function Lfo1Section({
  partial,
  n,
  set,
  followed,
}: {
  partial: PcmSynthPartial;
  /** Partial number shown on the box, in a merged pair. */
  n?: number;
  set: PartialSetter;
  /** The partial whose setting an inactive pan `field` follows (merged Structure), if any. */
  followed: (field: PcmsPartialField) => number | undefined;
}) {
  return (
    <SectionPanel label={numbered("LFO1", n, " · ")} family={ParameterFamily.Modulation}>
      <div className={css.panelRow}>
        <SynthSwitch label="Wave" value={partial.lfo1Waveform} vertical
          options={switchOptions(pcms_partial_spec(PcmsPartialField.Lfo1Waveform))}
          onChange={set(PcmsPartialField.Lfo1Waveform, "lfo1Waveform")} title="LFO waveform shape" />
        <div className={css.scopeColumn}>
          <LfoScope className={css.scopeBox} rate={partial.lfo1Rate}
            shape={pcms_partial_spec(PcmsPartialField.Lfo1Waveform).formatValue(partial.lfo1Waveform)}
            steps={partial.lfoStepValues} smooth={partial.lfoStepType === 1} />
          <div className={css.knobGrid}>
            <TimeOrNoteKnob label="Rate" field={PcmsPartialField.Lfo1Rate} value={partial.lfo1Rate}
              onChange={set(PcmsPartialField.Lfo1Rate, "lfo1Rate")}
              title="LFO speed" />
            <SynthKnob label="Detune" value={partial.lfo1RateDetune} {...knobProps(pcms_partial_spec(PcmsPartialField.Lfo1RateDetune))}
              onChange={set(PcmsPartialField.Lfo1RateDetune, "lfo1RateDetune")}
              title="Rate randomization — adds organic variation" />
            <SynthKnob label="Delay" value={partial.lfo1DelayTime} {...knobProps(pcms_partial_spec(PcmsPartialField.Lfo1DelayTime))}
              onChange={set(PcmsPartialField.Lfo1DelayTime, "lfo1DelayTime")}
              title="Time before LFO starts after note-on" />
            <SynthKnob label="Dly KF" value={partial.lfo1DelayTimeKeyfollow} {...knobProps(pcms_partial_spec(PcmsPartialField.Lfo1DelayTimeKeyfollow))}
              onChange={set(PcmsPartialField.Lfo1DelayTimeKeyfollow, "lfo1DelayTimeKeyfollow")}
              title="Delay time key tracking — higher keys = shorter delay" />
            <SynthKnob label="Offset" value={partial.lfo1Offset} {...knobProps(pcms_partial_spec(PcmsPartialField.Lfo1Offset))}
              onChange={set(PcmsPartialField.Lfo1Offset, "lfo1Offset")}
              title="DC offset of the LFO waveform" />
          </div>
        </div>
      </div>
      <div className={css.panelRow}>
        <SynthSwitch label="Fade" value={partial.lfo1FadeMode} vertical
          options={switchOptions(pcms_partial_spec(PcmsPartialField.Lfo1FadeMode))}
          onChange={set(PcmsPartialField.Lfo1FadeMode, "lfo1FadeMode")} led={false}
          title="Fade behavior: fade in on note-on, fade out, or on key-off" />
        <SynthKnob label="Fade T" value={partial.lfo1FadeTime} {...knobProps(pcms_partial_spec(PcmsPartialField.Lfo1FadeTime))}
          onChange={set(PcmsPartialField.Lfo1FadeTime, "lfo1FadeTime")}
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
        <div className={followed(PcmsPartialField.Lfo1PanDepth) ? css.follows : undefined}>
          <SynthFader label={n ? "PAN " + n : "Pan"} value={partial.lfo1PanDepth} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo1PanDepth))}
            onChange={set(PcmsPartialField.Lfo1PanDepth, "lfo1PanDepth")} compact
            title={followsTitle("LFO modulation depth on panning (auto-pan)", followed(PcmsPartialField.Lfo1PanDepth))} />
        </div>
      </FaderGroup>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// LFO2 Section
// ---------------------------------------------------------------------------

/** The LFO2 box: its wave, rate and the depths it sends. */
export function Lfo2Section({
  partial,
  n,
  set,
  followed,
}: {
  partial: PcmSynthPartial;
  /** Partial number shown on the box, in a merged pair. */
  n?: number;
  set: PartialSetter;
  /** The partial whose setting an inactive pan `field` follows (merged Structure), if any. */
  followed: (field: PcmsPartialField) => number | undefined;
}) {
  return (
    <SectionPanel label={numbered("LFO2", n, " · ")} family={ParameterFamily.Modulation}>
      <div className={css.panelRow}>
        <SynthSwitch label="Wave" value={partial.lfo2Waveform} vertical
          options={switchOptions(pcms_partial_spec(PcmsPartialField.Lfo2Waveform))}
          onChange={set(PcmsPartialField.Lfo2Waveform, "lfo2Waveform")} title="LFO waveform shape" />
        <div className={css.scopeColumn}>
          <LfoScope className={css.scopeBox} rate={partial.lfo2Rate}
            shape={pcms_partial_spec(PcmsPartialField.Lfo2Waveform).formatValue(partial.lfo2Waveform)}
            steps={partial.lfoStepValues} smooth={partial.lfoStepType === 1} />
          <div className={css.knobGrid}>
            <TimeOrNoteKnob label="Rate" field={PcmsPartialField.Lfo2Rate} value={partial.lfo2Rate}
              onChange={set(PcmsPartialField.Lfo2Rate, "lfo2Rate")}
              title="LFO speed" />
            <SynthKnob label="Detune" value={partial.lfo2RateDetune} {...knobProps(pcms_partial_spec(PcmsPartialField.Lfo2RateDetune))}
              onChange={set(PcmsPartialField.Lfo2RateDetune, "lfo2RateDetune")}
              title="Rate randomization — adds organic variation" />
            <SynthKnob label="Delay" value={partial.lfo2DelayTime} {...knobProps(pcms_partial_spec(PcmsPartialField.Lfo2DelayTime))}
              onChange={set(PcmsPartialField.Lfo2DelayTime, "lfo2DelayTime")}
              title="Time before LFO starts after note-on" />
            <SynthKnob label="Dly KF" value={partial.lfo2DelayTimeKeyfollow} {...knobProps(pcms_partial_spec(PcmsPartialField.Lfo2DelayTimeKeyfollow))}
              onChange={set(PcmsPartialField.Lfo2DelayTimeKeyfollow, "lfo2DelayTimeKeyfollow")}
              title="Delay time key tracking — higher keys = shorter delay" />
            <SynthKnob label="Offset" value={partial.lfo2Offset} {...knobProps(pcms_partial_spec(PcmsPartialField.Lfo2Offset))}
              onChange={set(PcmsPartialField.Lfo2Offset, "lfo2Offset")}
              title="DC offset of the LFO waveform" />
          </div>
        </div>
      </div>
      <div className={css.panelRow}>
        <SynthSwitch label="Fade" value={partial.lfo2FadeMode} vertical
          options={switchOptions(pcms_partial_spec(PcmsPartialField.Lfo2FadeMode))}
          onChange={set(PcmsPartialField.Lfo2FadeMode, "lfo2FadeMode")} led={false}
          title="Fade behavior: fade in on note-on, fade out, or on key-off" />
        <SynthKnob label="Fade T" value={partial.lfo2FadeTime} {...knobProps(pcms_partial_spec(PcmsPartialField.Lfo2FadeTime))}
          onChange={set(PcmsPartialField.Lfo2FadeTime, "lfo2FadeTime")}
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
        <div className={followed(PcmsPartialField.Lfo2PanDepth) ? css.follows : undefined}>
          <SynthFader label={n ? "PAN " + n : "Pan"} value={partial.lfo2PanDepth} {...knobRange(pcms_partial_spec(PcmsPartialField.Lfo2PanDepth))}
            onChange={set(PcmsPartialField.Lfo2PanDepth, "lfo2PanDepth")} compact
            title={followsTitle("LFO modulation depth on panning (auto-pan)", followed(PcmsPartialField.Lfo2PanDepth))} />
        </div>
      </FaderGroup>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Step LFO Section
// ---------------------------------------------------------------------------

/** The STEP LFO box: its type and 16 steps. */
export function StepLfoSection({
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
    <SectionPanel label={numbered("STEP LFO", n, " · ")} family={ParameterFamily.Modulation}>
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

/** The CTRL box: how the partial responds to playing. */
export function ControlSection({
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
    <SectionPanel label={numbered("CTRL", n, " ")} family={ParameterFamily.Common}>
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
// Output Section
// ---------------------------------------------------------------------------

/** The OUTPUT box: delay, sends and output level. */
export function OutputSection({
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
    <SectionPanel label={numbered("OUTPUT", n, " ")} family={ParameterFamily.Amp}>
      <div className={css.panelRow}>
        <SynthSwitch label="Delay" value={partial.delayMode} vertical
          options={switchOptions(pcms_partial_spec(PcmsPartialField.DelayMode))}
          onChange={set(PcmsPartialField.DelayMode, "delayMode")} led={false}
          title="Partial delay mode — when sound starts relative to note-on" />
        <TimeOrNoteKnob label="Delay" field={PcmsPartialField.DelayTime} value={partial.delayTime}
          onChange={set(PcmsPartialField.DelayTime, "delayTime")}
          title="Delay time before this partial sounds" />
      </div>
      <OutputStrip
        fx1={partial.chorusSend}
        fx2={partial.reverbSend}
        level={partial.outputLevel}
        onFx1Change={set(PcmsPartialField.ChorusSend, "chorusSend")}
        onFx2Change={set(PcmsPartialField.ReverbSend, "reverbSend")}
        onLevelChange={set(PcmsPartialField.OutputLevel, "outputLevel")}
        fx1Spec={knobProps(pcms_partial_spec(PcmsPartialField.ChorusSend))}
        fx2Spec={knobProps(pcms_partial_spec(PcmsPartialField.ReverbSend))}
        levelSpec={knobRange(pcms_partial_spec(PcmsPartialField.OutputLevel))}
        trackHeight={150}
      />
    </SectionPanel>
  );
}
