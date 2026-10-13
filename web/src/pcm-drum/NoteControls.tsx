import { useState } from "react";
import {
  SynthKnob,
  SynthSwitch,
  SynthSelect,
  SectionPanel,
  BreakpointEnvelope,
  OutputStrip,
} from "../synth-ui";
import {
  ParameterFamily,
  PcmdPartialField,
  PcmdWmtField,
  pcmd_partial_spec,
  pcmd_wmt_spec,
} from "../../pkg/integral_wasm.js";
import type {
  PartialParamHandler,
  PartialSetter,
  PcmDrumPartial,
  PcmDrumWmt,
  WmtParamHandler,
} from "./types";
import { knobProps, knobRange, switchOptions } from "../fieldSpec";
import { VelocityCurveSelect } from "../synth-ui/VelocityCurveSelect";
import { FilterScope } from "../synth-ui/FilterScope";
import { deviceSpec } from "../deviceSpec";
import { WavePicker, type ExpansionMarking } from "../WavePicker";
import { waveSourceOf } from "../waveCatalog";
import css from "../PcmDrumEditor.module.css";

// ---------------------------------------------------------------------------
// Per-key note controls
// ---------------------------------------------------------------------------

/** Track height of a key's Level fader, which the kit's Level fader matches. */
export const LEVEL_TRACK_HEIGHT = 200;

export function NoteControls({
  note,
  onParam,
  onWmtParam,
  marking,
}: {
  note: PcmDrumPartial;
  onParam: PartialParamHandler;
  onWmtParam: WmtParamHandler;
  /** Loaded state of the Expansions behind SRX Wave Groups. */
  marking?: ExpansionMarking;
}) {
  const [wmtTab, setWmtTab] = useState(0);
  const set: PartialSetter = (field, key, index = 0) =>
    (v) => onParam(field, key, index, v);

  return (
    <>
      <div className={css.noteControls}>
        {/* WAVE (WMT) */}
        <WmtSection
          note={note}
          wmtTab={wmtTab}
          onWmtTab={setWmtTab}
          set={set}
          onWmtParam={onWmtParam}
          marking={marking}
        />

        {/* PITCH ENV */}
        <PitchEnvSection note={note} set={set} />

        {/* FILTER (TVF) */}
        <TvfSection note={note} set={set} />

        {/* AMP (TVA) */}
        <TvaSection note={note} set={set} />

        {/* MISC */}
        <MiscSection note={note} set={set} />

        {/* LEVEL / PAN */}
        <SectionPanel label="LEVEL / PAN" family={ParameterFamily.Amp}>
          <OutputStrip
            pan={{
              value: note.pan, onChange: set(PcmdPartialField.Pan, "pan"),
              format: (v) => pcmd_partial_spec(PcmdPartialField.Pan).formatValue(v),
              family: pcmd_partial_spec(PcmdPartialField.Pan).family,
            }}
            fx1Family={pcmd_partial_spec(PcmdPartialField.ChorusSend).family}
            fx2Family={pcmd_partial_spec(PcmdPartialField.ReverbSend).family}
            fx1={note.chorusSend} fx2={note.reverbSend}
            level={note.outputLevel}
            onFx1Change={set(PcmdPartialField.ChorusSend, "chorusSend")}
            onFx2Change={set(PcmdPartialField.ReverbSend, "reverbSend")}
            onLevelChange={set(PcmdPartialField.OutputLevel, "outputLevel")}
            trackHeight={LEVEL_TRACK_HEIGHT}
          />
        </SectionPanel>
      </div>
    </>
  );
}

// ---------------------------------------------------------------------------
// WMT Section (tabbed, 4 layers)
// ---------------------------------------------------------------------------

function WmtSection({
  note,
  wmtTab,
  onWmtTab,
  set,
  onWmtParam,
  marking,
}: {
  note: PcmDrumPartial;
  wmtTab: number;
  onWmtTab: (tab: number) => void;
  set: PartialSetter;
  onWmtParam: WmtParamHandler;
  marking?: ExpansionMarking;
}) {
  const wmt = note.wmt[wmtTab]!;
  const setWmt = (field: PcmdWmtField, key: keyof PcmDrumWmt) =>
    (v: number) => onWmtParam(wmtTab, field, key, v);

  return (
    <SectionPanel label="WAVE (WMT)" family={ParameterFamily.Pitch}>
      {/* The key's partial name, as the kit names it */}
      {note.partialName && <div className={css.partialName}>{note.partialName}</div>}
      {/* Velocity Control */}
      <div className={css.knobRow}>
        <SynthSelect label="Vel Ctrl" value={note.wmtVelocityControl}
          options={switchOptions(pcmd_partial_spec(PcmdPartialField.WmtVelocityControl))}
          onChange={set(PcmdPartialField.WmtVelocityControl, "wmtVelocityControl")} />
      </div>

      {/* Layer tabs */}
      <div className={css.wmtTabs}>
        {Array.from({ length: deviceSpec().pcm_drum_wmt_count }, (_, i) => {
          const w = note.wmt[i]!;
          const isActive = i === wmtTab;
          const isOn = w.wmtSwitch === 1;
          const tabClass = isActive
            ? css.wmtTabActive
            : isOn ? css.wmtTab : css.wmtTabOff;
          return (
            <div key={i} className={tabClass} onClick={() => onWmtTab(i)}>
              WMT{i + 1}
            </div>
          );
        })}
      </div>

      {/* Layer controls */}
      <div className={css.wmtLayer}>
        <SynthSwitch label="Switch" value={wmt.wmtSwitch}
          options={switchOptions(pcmd_wmt_spec(PcmdWmtField.Switch))}
          onChange={setWmt(PcmdWmtField.Switch, "wmtSwitch")}
          title="Wave layer switch" />
        <SynthSelect label="Gain" value={wmt.waveGain}
          options={switchOptions(pcmd_wmt_spec(PcmdWmtField.WaveGain))}
          onChange={setWmt(PcmdWmtField.WaveGain, "waveGain")} />
      </div>

      <WmtWave wmt={wmt} index={wmtTab} onWmtParam={onWmtParam} marking={marking} />

      <div className={css.wmtLayer}>
        <SynthSwitch label="FXM" value={wmt.waveFxmSwitch}
          options={switchOptions(pcmd_wmt_spec(PcmdWmtField.WaveFxmSwitch))}
          onChange={setWmt(PcmdWmtField.WaveFxmSwitch, "waveFxmSwitch")}
          title="FXM Switch" />
        <SynthKnob label="Color" value={wmt.waveFxmColor} {...knobProps(pcmd_wmt_spec(PcmdWmtField.WaveFxmColor))}
          onChange={setWmt(PcmdWmtField.WaveFxmColor, "waveFxmColor")}
          title="FXM Color" />
        <SynthKnob label="Depth" value={wmt.waveFxmDepth} {...knobProps(pcmd_wmt_spec(PcmdWmtField.WaveFxmDepth))}
          onChange={setWmt(PcmdWmtField.WaveFxmDepth, "waveFxmDepth")}
          title="FXM Depth" />
      </div>

      <div className={css.wmtLayer}>
        <SynthKnob label="Coarse" value={wmt.coarseTune} {...knobProps(pcmd_wmt_spec(PcmdWmtField.CoarseTune))}
          onChange={setWmt(PcmdWmtField.CoarseTune, "coarseTune")}
          title="Coarse Tune" />
        <SynthKnob label="Fine" value={wmt.fineTune} {...knobProps(pcmd_wmt_spec(PcmdWmtField.FineTune))}
          onChange={setWmt(PcmdWmtField.FineTune, "fineTune")}
          title="Fine Tune" />
        <SynthKnob label="Pan" value={wmt.pan} {...knobProps(pcmd_wmt_spec(PcmdWmtField.Pan))}
          onChange={setWmt(PcmdWmtField.Pan, "pan")}
          title="Wave Pan" />
        <SynthKnob label="Level" value={wmt.level} {...knobProps(pcmd_wmt_spec(PcmdWmtField.Level))}
          onChange={setWmt(PcmdWmtField.Level, "level")}
          title="Wave Level" />
      </div>

      <div className={css.wmtLayer}>
        <SynthSwitch label="Rnd Pan" value={wmt.randomPanSwitch}
          options={switchOptions(pcmd_wmt_spec(PcmdWmtField.RandomPanSwitch))}
          onChange={setWmt(PcmdWmtField.RandomPanSwitch, "randomPanSwitch")}
          title="Random Pan Switch" />
        <SynthSelect label="Alt Pan" value={wmt.alternatePanSwitch}
          options={switchOptions(pcmd_wmt_spec(PcmdWmtField.AlternatePanSwitch))}
          onChange={setWmt(PcmdWmtField.AlternatePanSwitch, "alternatePanSwitch")} />
        <SynthSwitch label="T.Sync" value={wmt.waveTempoSync}
          options={switchOptions(pcmd_wmt_spec(PcmdWmtField.WaveTempoSync))}
          onChange={setWmt(PcmdWmtField.WaveTempoSync, "waveTempoSync")}
          title="Tempo Sync" />
      </div>

      <div className={css.wmtLayer}>
        <SynthKnob label="Vel Lo" value={wmt.velocityRangeLower} {...knobProps(pcmd_wmt_spec(PcmdWmtField.VelocityRangeLower))}
          onChange={setWmt(PcmdWmtField.VelocityRangeLower, "velocityRangeLower")}
          title="Velocity Range Lower" />
        <SynthKnob label="Vel Hi" value={wmt.velocityRangeUpper} {...knobProps(pcmd_wmt_spec(PcmdWmtField.VelocityRangeUpper))}
          onChange={setWmt(PcmdWmtField.VelocityRangeUpper, "velocityRangeUpper")}
          title="Velocity Range Upper" />
        <SynthKnob label="Fade Lo" value={wmt.velocityFadeLower} {...knobProps(pcmd_wmt_spec(PcmdWmtField.VelocityFadeLower))}
          onChange={setWmt(PcmdWmtField.VelocityFadeLower, "velocityFadeLower")}
          title="Velocity Fade Width Lower" />
        <SynthKnob label="Fade Hi" value={wmt.velocityFadeUpper} {...knobProps(pcmd_wmt_spec(PcmdWmtField.VelocityFadeUpper))}
          onChange={setWmt(PcmdWmtField.VelocityFadeUpper, "velocityFadeUpper")}
          title="Velocity Fade Width Upper" />
      </div>
    </SectionPanel>
  );
}

/** WMT `index`'s Wave Group and L/R Wave Numbers, picked by name. */
export function WmtWave({
  wmt,
  index,
  onWmtParam,
  marking,
}: {
  wmt: Pick<PcmDrumWmt, "waveGroupType" | "waveGroupId" | "waveNumberL" | "waveNumberR">;
  index: number;
  onWmtParam: WmtParamHandler;
  marking?: ExpansionMarking;
}) {
  return (
    <WavePicker
      source={waveSourceOf(wmt.waveGroupType, wmt.waveGroupId)}
      unknownGroupLabel={`Type ${wmt.waveGroupType} ID ${wmt.waveGroupId}`}
      stereo
      l={wmt.waveNumberL}
      r={wmt.waveNumberR}
      onChange={(side, n) =>
        side === "l"
          ? onWmtParam(index, PcmdWmtField.WaveNumberL, "waveNumberL", n)
          : onWmtParam(index, PcmdWmtField.WaveNumberR, "waveNumberR", n)
      }
      onGroupChange={(source) => {
        if (!source.params) return;
        const [type, id] = source.params;
        onWmtParam(index, PcmdWmtField.WaveGroupType, "waveGroupType", type);
        onWmtParam(index, PcmdWmtField.WaveGroupId, "waveGroupId", id);
      }}
      marking={marking}
    />
  );
}

// ---------------------------------------------------------------------------
// Pitch Envelope Section
// ---------------------------------------------------------------------------

function PitchEnvSection({
  note,
  set,
}: {
  note: PcmDrumPartial;
  set: PartialSetter;
}) {
  return (
    <SectionPanel label="PITCH ENV" family={ParameterFamily.Pitch}>
      <div className={css.envBlock}>
        <div className={css.knobRow}>
          <SynthKnob label="Depth" value={note.pitchEnvDepth}
            {...knobProps(pcmd_partial_spec(PcmdPartialField.PitchEnvDepth))}
            onChange={set(PcmdPartialField.PitchEnvDepth, "pitchEnvDepth")}
            title="Pitch envelope depth" />
          <SynthKnob label="Vel Sns" value={note.pitchEnvVelocitySens} {...knobProps(pcmd_partial_spec(PcmdPartialField.PitchEnvVelocitySens))}
            onChange={set(PcmdPartialField.PitchEnvVelocitySens, "pitchEnvVelocitySens")}
            title="Pitch envelope velocity sensitivity" />
          <SynthKnob label="T1 Vel" value={note.pitchEnvT1VelocitySens} {...knobProps(pcmd_partial_spec(PcmdPartialField.PitchEnvT1VelocitySens))}
            onChange={set(PcmdPartialField.PitchEnvT1VelocitySens, "pitchEnvT1VelocitySens")}
            title="Pitch envelope Time 1 velocity sensitivity" />
          <SynthKnob label="T4 Vel" value={note.pitchEnvT4VelocitySens} {...knobProps(pcmd_partial_spec(PcmdPartialField.PitchEnvT4VelocitySens))}
            onChange={set(PcmdPartialField.PitchEnvT4VelocitySens, "pitchEnvT4VelocitySens")}
            title="Pitch envelope Time 4 velocity sensitivity" />
        </div>
        <BreakpointEnvelope
          compact
          times={[{
            label: "T1", value: note.pitchEnvTime[0] ?? 0,
            onChange: set(PcmdPartialField.PitchEnvTime, "pitchEnvTime", 0), ...knobRange(pcmd_partial_spec(PcmdPartialField.PitchEnvTime)),
          }, {
            label: "T2", value: note.pitchEnvTime[1] ?? 0,
            onChange: set(PcmdPartialField.PitchEnvTime, "pitchEnvTime", 1), ...knobRange(pcmd_partial_spec(PcmdPartialField.PitchEnvTime)),
          }, {
            label: "T3", value: note.pitchEnvTime[2] ?? 0,
            onChange: set(PcmdPartialField.PitchEnvTime, "pitchEnvTime", 2), ...knobRange(pcmd_partial_spec(PcmdPartialField.PitchEnvTime)),
          }, {
            label: "T4", value: note.pitchEnvTime[3] ?? 0,
            onChange: set(PcmdPartialField.PitchEnvTime, "pitchEnvTime", 3), ...knobRange(pcmd_partial_spec(PcmdPartialField.PitchEnvTime)),
          }]}
          levels={[
            { label: "L0", value: note.pitchEnvLevel[0] ?? 64,
              onChange: set(PcmdPartialField.PitchEnvLevel, "pitchEnvLevel", 0), ...knobRange(pcmd_partial_spec(PcmdPartialField.PitchEnvLevel)) },
            { label: "L1", value: note.pitchEnvLevel[1] ?? 64,
              onChange: set(PcmdPartialField.PitchEnvLevel, "pitchEnvLevel", 1), ...knobRange(pcmd_partial_spec(PcmdPartialField.PitchEnvLevel)) },
            { label: "L2", value: note.pitchEnvLevel[2] ?? 64,
              onChange: set(PcmdPartialField.PitchEnvLevel, "pitchEnvLevel", 2), ...knobRange(pcmd_partial_spec(PcmdPartialField.PitchEnvLevel)) },
            { label: "L3", value: note.pitchEnvLevel[3] ?? 64,
              onChange: set(PcmdPartialField.PitchEnvLevel, "pitchEnvLevel", 3), ...knobRange(pcmd_partial_spec(PcmdPartialField.PitchEnvLevel)) },
            { label: "L4", value: note.pitchEnvLevel[4] ?? 64,
              onChange: set(PcmdPartialField.PitchEnvLevel, "pitchEnvLevel", 4), ...knobRange(pcmd_partial_spec(PcmdPartialField.PitchEnvLevel)) },
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
  note,
  set,
}: {
  note: PcmDrumPartial;
  set: PartialSetter;
}) {
  return (
    <SectionPanel label="FILTER" family={ParameterFamily.Filter}>
      <FilterScope className={css.filterScope} cutoff={note.tvfCutoffFrequency} resonance={note.tvfResonance}
        type={pcmd_partial_spec(PcmdPartialField.TvfFilterType).formatValue(note.tvfFilterType)} />
      <div className={css.filterGrid}>
        <SynthSwitch label="Type" value={note.tvfFilterType} vertical
          options={switchOptions(pcmd_partial_spec(PcmdPartialField.TvfFilterType))}
          onChange={set(PcmdPartialField.TvfFilterType, "tvfFilterType")} title="Filter type" />
        <div className={css.stack}>
          <VelocityCurveSelect label="Vel Crv" value={note.tvfCutoffVelocityCurve}
            options={switchOptions(pcmd_partial_spec(PcmdPartialField.TvfCutoffVelocityCurve))}
            onChange={set(PcmdPartialField.TvfCutoffVelocityCurve, "tvfCutoffVelocityCurve")} family={pcmd_partial_spec(PcmdPartialField.TvfCutoffVelocityCurve).family} />
          <VelocityCurveSelect label="Env Crv" value={note.tvfEnvVelocityCurve}
            options={switchOptions(pcmd_partial_spec(PcmdPartialField.TvfEnvVelocityCurve))}
            onChange={set(PcmdPartialField.TvfEnvVelocityCurve, "tvfEnvVelocityCurve")} family={pcmd_partial_spec(PcmdPartialField.TvfEnvVelocityCurve).family} />
        </div>
        <div className={css.stack}>
          <SynthKnob label="Cutoff" value={note.tvfCutoffFrequency} {...knobProps(pcmd_partial_spec(PcmdPartialField.TvfCutoffFrequency))}
            onChange={set(PcmdPartialField.TvfCutoffFrequency, "tvfCutoffFrequency")}
            title="Filter cutoff frequency" />
          <SynthKnob label="Cut Vel" value={note.tvfCutoffVelocitySens} {...knobProps(pcmd_partial_spec(PcmdPartialField.TvfCutoffVelocitySens))}
            onChange={set(PcmdPartialField.TvfCutoffVelocitySens, "tvfCutoffVelocitySens")}
            title="Cutoff velocity sensitivity" />
        </div>
        <div className={css.stack}>
          <SynthKnob label="Reso" value={note.tvfResonance} {...knobProps(pcmd_partial_spec(PcmdPartialField.TvfResonance))}
            onChange={set(PcmdPartialField.TvfResonance, "tvfResonance")}
            title="Filter resonance" />
          <SynthKnob label="Res Vel" value={note.tvfResonanceVelocitySens} {...knobProps(pcmd_partial_spec(PcmdPartialField.TvfResonanceVelocitySens))}
            onChange={set(PcmdPartialField.TvfResonanceVelocitySens, "tvfResonanceVelocitySens")}
            title="Resonance velocity sensitivity" />
        </div>
      </div>
      <div className={css.envBlock}>
        <div className={css.knobRow}>
          <SynthKnob label="Depth" value={note.tvfEnvDepth} {...knobProps(pcmd_partial_spec(PcmdPartialField.TvfEnvDepth))}
            onChange={set(PcmdPartialField.TvfEnvDepth, "tvfEnvDepth")}
            title="Filter envelope depth" />
          <SynthKnob label="Vel Sns" value={note.tvfEnvVelocitySens} {...knobProps(pcmd_partial_spec(PcmdPartialField.TvfEnvVelocitySens))}
            onChange={set(PcmdPartialField.TvfEnvVelocitySens, "tvfEnvVelocitySens")}
            title="Envelope velocity sensitivity" />
          <SynthKnob label="T1 Vel" value={note.tvfEnvT1VelocitySens} {...knobProps(pcmd_partial_spec(PcmdPartialField.TvfEnvT1VelocitySens))}
            onChange={set(PcmdPartialField.TvfEnvT1VelocitySens, "tvfEnvT1VelocitySens")}
            title="Filter envelope Time 1 velocity sensitivity" />
          <SynthKnob label="T4 Vel" value={note.tvfEnvT4VelocitySens} {...knobProps(pcmd_partial_spec(PcmdPartialField.TvfEnvT4VelocitySens))}
            onChange={set(PcmdPartialField.TvfEnvT4VelocitySens, "tvfEnvT4VelocitySens")}
            title="Filter envelope Time 4 velocity sensitivity" />
        </div>
        <BreakpointEnvelope
          compact
          times={[{
            label: "T1", value: note.tvfEnvTime[0] ?? 0,
            onChange: set(PcmdPartialField.TvfEnvTime, "tvfEnvTime", 0), ...knobRange(pcmd_partial_spec(PcmdPartialField.TvfEnvTime)),
          }, {
            label: "T2", value: note.tvfEnvTime[1] ?? 0,
            onChange: set(PcmdPartialField.TvfEnvTime, "tvfEnvTime", 1), ...knobRange(pcmd_partial_spec(PcmdPartialField.TvfEnvTime)),
          }, {
            label: "T3", value: note.tvfEnvTime[2] ?? 0,
            onChange: set(PcmdPartialField.TvfEnvTime, "tvfEnvTime", 2), ...knobRange(pcmd_partial_spec(PcmdPartialField.TvfEnvTime)),
          }, {
            label: "T4", value: note.tvfEnvTime[3] ?? 0,
            onChange: set(PcmdPartialField.TvfEnvTime, "tvfEnvTime", 3), ...knobRange(pcmd_partial_spec(PcmdPartialField.TvfEnvTime)),
          }]}
          levels={[
            { label: "L0", value: note.tvfEnvLevel[0] ?? 0,
              onChange: set(PcmdPartialField.TvfEnvLevel, "tvfEnvLevel", 0), ...knobRange(pcmd_partial_spec(PcmdPartialField.TvfEnvLevel)) },
            { label: "L1", value: note.tvfEnvLevel[1] ?? 0,
              onChange: set(PcmdPartialField.TvfEnvLevel, "tvfEnvLevel", 1), ...knobRange(pcmd_partial_spec(PcmdPartialField.TvfEnvLevel)) },
            { label: "L2", value: note.tvfEnvLevel[2] ?? 0,
              onChange: set(PcmdPartialField.TvfEnvLevel, "tvfEnvLevel", 2), ...knobRange(pcmd_partial_spec(PcmdPartialField.TvfEnvLevel)) },
            { label: "L3", value: note.tvfEnvLevel[3] ?? 0,
              onChange: set(PcmdPartialField.TvfEnvLevel, "tvfEnvLevel", 3), ...knobRange(pcmd_partial_spec(PcmdPartialField.TvfEnvLevel)) },
            { label: "L4", value: note.tvfEnvLevel[4] ?? 0,
              onChange: set(PcmdPartialField.TvfEnvLevel, "tvfEnvLevel", 4), ...knobRange(pcmd_partial_spec(PcmdPartialField.TvfEnvLevel)) },
          ]}
        />
      </div>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// TVA (Amplifier) Section
// ---------------------------------------------------------------------------

function TvaSection({
  note,
  set,
}: {
  note: PcmDrumPartial;
  set: PartialSetter;
}) {
  return (
    <SectionPanel label="AMP" family={ParameterFamily.Amp}>
      <div className={css.envBlock}>
        <div className={css.knobRow}>
          <VelocityCurveSelect label="Vel Crv" value={note.tvaLevelVelocityCurve}
            options={switchOptions(pcmd_partial_spec(PcmdPartialField.TvaLevelVelocityCurve))}
            onChange={set(PcmdPartialField.TvaLevelVelocityCurve, "tvaLevelVelocityCurve")} family={pcmd_partial_spec(PcmdPartialField.TvaLevelVelocityCurve).family} />
          <SynthKnob label="Vel Sns" value={note.tvaLevelVelocitySens} {...knobProps(pcmd_partial_spec(PcmdPartialField.TvaLevelVelocitySens))}
            onChange={set(PcmdPartialField.TvaLevelVelocitySens, "tvaLevelVelocitySens")}
            title="Amplitude velocity sensitivity" />
          <SynthKnob label="T1 Vel" value={note.tvaEnvT1VelocitySens} {...knobProps(pcmd_partial_spec(PcmdPartialField.TvaEnvT1VelocitySens))}
            onChange={set(PcmdPartialField.TvaEnvT1VelocitySens, "tvaEnvT1VelocitySens")}
            title="Amp envelope Time 1 velocity sensitivity" />
          <SynthKnob label="T4 Vel" value={note.tvaEnvT4VelocitySens} {...knobProps(pcmd_partial_spec(PcmdPartialField.TvaEnvT4VelocitySens))}
            onChange={set(PcmdPartialField.TvaEnvT4VelocitySens, "tvaEnvT4VelocitySens")}
            title="Amp envelope Time 4 velocity sensitivity" />
        </div>
        <BreakpointEnvelope
          compact
          times={[{
            label: "T1", value: note.tvaEnvTime[0] ?? 0,
            onChange: set(PcmdPartialField.TvaEnvTime, "tvaEnvTime", 0), ...knobRange(pcmd_partial_spec(PcmdPartialField.TvaEnvTime)),
          }, {
            label: "T2", value: note.tvaEnvTime[1] ?? 0,
            onChange: set(PcmdPartialField.TvaEnvTime, "tvaEnvTime", 1), ...knobRange(pcmd_partial_spec(PcmdPartialField.TvaEnvTime)),
          }, {
            label: "T3", value: note.tvaEnvTime[2] ?? 0,
            onChange: set(PcmdPartialField.TvaEnvTime, "tvaEnvTime", 2), ...knobRange(pcmd_partial_spec(PcmdPartialField.TvaEnvTime)),
          }, {
            label: "T4", value: note.tvaEnvTime[3] ?? 0,
            onChange: set(PcmdPartialField.TvaEnvTime, "tvaEnvTime", 3), ...knobRange(pcmd_partial_spec(PcmdPartialField.TvaEnvTime)),
          }]}
          levels={[
            { label: "L0", at: 0, text: "0" },
            { label: "L1", value: note.tvaEnvLevel[0] ?? 0,
              onChange: set(PcmdPartialField.TvaEnvLevel, "tvaEnvLevel", 0), ...knobRange(pcmd_partial_spec(PcmdPartialField.TvaEnvLevel)) },
            { label: "L2", value: note.tvaEnvLevel[1] ?? 0,
              onChange: set(PcmdPartialField.TvaEnvLevel, "tvaEnvLevel", 1), ...knobRange(pcmd_partial_spec(PcmdPartialField.TvaEnvLevel)) },
            { label: "L3", value: note.tvaEnvLevel[2] ?? 0,
              onChange: set(PcmdPartialField.TvaEnvLevel, "tvaEnvLevel", 2), ...knobRange(pcmd_partial_spec(PcmdPartialField.TvaEnvLevel)) },
            { label: "L4", at: 0, text: "0" },
          ]}
        />
      </div>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Misc Section
// ---------------------------------------------------------------------------

function MiscSection({
  note,
  set,
}: {
  note: PcmDrumPartial;
  set: PartialSetter;
}) {
  return (
    <SectionPanel label="MISC" family={ParameterFamily.Common}>
      <div className={css.noteSelects}>
        <SynthSelect label="Assign" value={note.assignType}
          options={switchOptions(pcmd_partial_spec(PcmdPartialField.AssignType))}
          onChange={set(PcmdPartialField.AssignType, "assignType")} />
        <SynthSelect label="Mute Grp" value={note.muteGroup}
          options={switchOptions(pcmd_partial_spec(PcmdPartialField.MuteGroup))}
          onChange={set(PcmdPartialField.MuteGroup, "muteGroup")} />
        <SynthSelect label="Env Mode" value={note.envMode}
          options={switchOptions(pcmd_partial_spec(PcmdPartialField.EnvMode))}
          onChange={set(PcmdPartialField.EnvMode, "envMode")} />
        <SynthSelect label="Output" value={note.outputAssign}
          options={switchOptions(pcmd_partial_spec(PcmdPartialField.OutputAssign))}
          onChange={set(PcmdPartialField.OutputAssign, "outputAssign")} />
        <SynthSwitch label="1-Shot" value={note.oneShotMode}
          options={switchOptions(pcmd_partial_spec(PcmdPartialField.OneShotMode))}
          onChange={set(PcmdPartialField.OneShotMode, "oneShotMode")}
          title="One Shot Mode" />
      </div>
      <div className={css.knobRow}>
        <SynthKnob label="PB Rng" value={note.pitchBendRange} {...knobProps(pcmd_partial_spec(PcmdPartialField.PitchBendRange))}
          onChange={set(PcmdPartialField.PitchBendRange, "pitchBendRange")}
          title="Pitch Bend Range" />
        <SynthKnob label="Level" value={note.level} {...knobProps(pcmd_partial_spec(PcmdPartialField.Level))}
          onChange={set(PcmdPartialField.Level, "level")}
          title="Partial Level" />
      </div>
      <div className={css.knobRow}>
        <SynthKnob label="Coarse" value={note.coarseTune} {...knobProps(pcmd_partial_spec(PcmdPartialField.CoarseTune))}
          onChange={set(PcmdPartialField.CoarseTune, "coarseTune")}
          title="Partial Coarse Tune" />
        <SynthKnob label="Fine" value={note.fineTune} {...knobProps(pcmd_partial_spec(PcmdPartialField.FineTune))}
          onChange={set(PcmdPartialField.FineTune, "fineTune")}
          title="Partial Fine Tune" />
        <SynthKnob label="Rnd Pit" value={note.randomPitchDepth}
          {...knobProps(pcmd_partial_spec(PcmdPartialField.RandomPitchDepth))}
          onChange={set(PcmdPartialField.RandomPitchDepth, "randomPitchDepth")}
          title="Random Pitch Depth (cents)" />
      </div>
      <div className={css.miscRow}>
        <SynthSwitch label="RxExpr" value={note.receiveExpression}
          options={switchOptions(pcmd_partial_spec(PcmdPartialField.ReceiveExpression))}
          onChange={set(PcmdPartialField.ReceiveExpression, "receiveExpression")}
          title="Receive Expression" />
        <SynthSwitch label="RxHold" value={note.receiveHold1}
          options={switchOptions(pcmd_partial_spec(PcmdPartialField.ReceiveHold1))}
          onChange={set(PcmdPartialField.ReceiveHold1, "receiveHold1")}
          title="Receive Hold-1" />
      </div>
    </SectionPanel>
  );
}
