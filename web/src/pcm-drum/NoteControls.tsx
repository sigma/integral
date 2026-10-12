import { useState } from "react";
import {
  SynthKnob,
  SynthSwitch,
  SynthSelect,
  SectionPanel,
  BreakpointEnvelope,
  OutputStrip,
} from "../synth-ui";
import { PcmdPartialField, PcmdWmtField, pcmd_partial_spec } from "../../pkg/integral_wasm.js";
import type {
  PartialParamHandler,
  PartialSetter,
  PcmDrumPartial,
  PcmDrumWmt,
  WmtParamHandler,
} from "./types";
import {
  noteName,
  panFmt,
  signedFmt,
  WAVE_GAIN_OPTIONS,
  FILTER_TYPE_OPTIONS,
  VELOCITY_CURVE_OPTIONS,
  ASSIGN_TYPE_OPTIONS,
  ENV_MODE_OPTIONS,
  OUTPUT_ASSIGN_OPTIONS,
  WMT_VELOCITY_CTRL_OPTIONS,
  ALT_PAN_OPTIONS,
} from "./types";
import { knobRange } from "../fieldSpec";
import { deviceSpec } from "../deviceSpec";
import { WavePicker, type ExpansionMarking } from "../WavePicker";
import { waveSourceOf } from "../waveCatalog";
import css from "../PcmDrumEditor.module.css";

// ---------------------------------------------------------------------------
// Per-key note controls
// ---------------------------------------------------------------------------

export function NoteControls({
  keyNumber,
  note,
  onParam,
  onWmtParam,
  marking,
}: {
  keyNumber: number;
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
      <div className={css.keyHeader}>
        <span className={css.keyLabel}>
          Key {keyNumber} ({noteName(keyNumber)})
        </span>
        {note.partialName && (
          <span className={css.partialName}>{note.partialName}</span>
        )}
        <SynthSelect label="Out" value={note.outputAssign}
          options={OUTPUT_ASSIGN_OPTIONS}
          onChange={set(PcmdPartialField.OutputAssign, "outputAssign")} />
      </div>

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
        <SectionPanel label="LEVEL / PAN" accentColor="#8cf">
          <OutputStrip
            pan={note.pan} onPanChange={set(PcmdPartialField.Pan, "pan")} panFormat={panFmt}
            fx1={note.chorusSend} fx2={note.reverbSend}
            level={note.outputLevel}
            onFx1Change={set(PcmdPartialField.ChorusSend, "chorusSend")}
            onFx2Change={set(PcmdPartialField.ReverbSend, "reverbSend")}
            onLevelChange={set(PcmdPartialField.OutputLevel, "outputLevel")}
            trackHeight={200}
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
    <SectionPanel label="WAVE (WMT)" accentColor="#fc8">
      {/* Velocity Control */}
      <div className={css.knobRow}>
        <SynthSelect label="Vel Ctrl" value={note.wmtVelocityControl}
          options={WMT_VELOCITY_CTRL_OPTIONS}
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
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={setWmt(PcmdWmtField.Switch, "wmtSwitch")}
          title="Wave layer switch" />
        <SynthSelect label="Gain" value={wmt.waveGain}
          options={WAVE_GAIN_OPTIONS}
          onChange={setWmt(PcmdWmtField.WaveGain, "waveGain")} />
      </div>

      <WmtWave wmt={wmt} index={wmtTab} onWmtParam={onWmtParam} marking={marking} />

      <div className={css.wmtLayer}>
        <SynthSwitch label="FXM" value={wmt.waveFxmSwitch}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={setWmt(PcmdWmtField.WaveFxmSwitch, "waveFxmSwitch")}
          title="FXM Switch" />
        <SynthKnob label="Color" value={wmt.waveFxmColor} min={0} max={3} defaultValue={0}
          onChange={setWmt(PcmdWmtField.WaveFxmColor, "waveFxmColor")}
          formatValue={(v) => String(v + 1)} color="#ea6"
          title="FXM Color (1-4)" />
        <SynthKnob label="Depth" value={wmt.waveFxmDepth} min={0} max={16} defaultValue={0}
          onChange={setWmt(PcmdWmtField.WaveFxmDepth, "waveFxmDepth")}
          formatValue={(v) => String(v)} color="#ea6"
          title="FXM Depth" />
      </div>

      <div className={css.wmtLayer}>
        <SynthKnob label="Coarse" value={wmt.coarseTune} min={16} max={112} defaultValue={64} center={64}
          onChange={setWmt(PcmdWmtField.CoarseTune, "coarseTune")}
          formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Coarse Tune (-48 to +48)" />
        <SynthKnob label="Fine" value={wmt.fineTune} min={14} max={114} defaultValue={64} center={64}
          onChange={setWmt(PcmdWmtField.FineTune, "fineTune")}
          formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Fine Tune (-50 to +50)" />
        <SynthKnob label="Pan" value={wmt.pan} min={0} max={127} defaultValue={64} center={64}
          onChange={setWmt(PcmdWmtField.Pan, "pan")}
          formatValue={panFmt} color="#8cf"
          title="Wave Pan" />
        <SynthKnob label="Level" value={wmt.level} min={0} max={127} defaultValue={127}
          onChange={setWmt(PcmdWmtField.Level, "level")}
          formatValue={(v) => String(v)} color="#8cf"
          title="Wave Level" />
      </div>

      <div className={css.wmtLayer}>
        <SynthSwitch label="Rnd Pan" value={wmt.randomPanSwitch}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={setWmt(PcmdWmtField.RandomPanSwitch, "randomPanSwitch")}
          title="Random Pan Switch" />
        <SynthSelect label="Alt Pan" value={wmt.alternatePanSwitch}
          options={ALT_PAN_OPTIONS}
          onChange={setWmt(PcmdWmtField.AlternatePanSwitch, "alternatePanSwitch")} />
        <SynthSwitch label="T.Sync" value={wmt.waveTempoSync}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={setWmt(PcmdWmtField.WaveTempoSync, "waveTempoSync")}
          title="Tempo Sync" />
      </div>

      <div className={css.wmtLayer}>
        <SynthKnob label="Vel Lo" value={wmt.velocityRangeLower} min={1} max={127} defaultValue={1}
          onChange={setWmt(PcmdWmtField.VelocityRangeLower, "velocityRangeLower")}
          formatValue={(v) => String(v)} color="#a6f"
          title="Velocity Range Lower" />
        <SynthKnob label="Vel Hi" value={wmt.velocityRangeUpper} min={1} max={127} defaultValue={127}
          onChange={setWmt(PcmdWmtField.VelocityRangeUpper, "velocityRangeUpper")}
          formatValue={(v) => String(v)} color="#a6f"
          title="Velocity Range Upper" />
        <SynthKnob label="Fade Lo" value={wmt.velocityFadeLower} min={0} max={127} defaultValue={0}
          onChange={setWmt(PcmdWmtField.VelocityFadeLower, "velocityFadeLower")}
          formatValue={(v) => String(v)} color="#a6f"
          title="Velocity Fade Width Lower" />
        <SynthKnob label="Fade Hi" value={wmt.velocityFadeUpper} min={0} max={127} defaultValue={0}
          onChange={setWmt(PcmdWmtField.VelocityFadeUpper, "velocityFadeUpper")}
          formatValue={(v) => String(v)} color="#a6f"
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
    <SectionPanel label="PITCH ENV" accentColor="#fc8">
      <div className={css.knobRow}>
        <SynthKnob label="Depth" value={note.pitchEnvDepth} center={64}
          {...knobRange(pcmd_partial_spec(PcmdPartialField.PitchEnvDepth))}
          onChange={set(PcmdPartialField.PitchEnvDepth, "pitchEnvDepth")} color="#fc8"
          title="Pitch envelope depth (-12 to +12)" />
        <SynthKnob label="Vel Sns" value={note.pitchEnvVelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={set(PcmdPartialField.PitchEnvVelocitySens, "pitchEnvVelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Pitch envelope velocity sensitivity" />
        <SynthKnob label="T1 Vel" value={note.pitchEnvT1VelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={set(PcmdPartialField.PitchEnvT1VelocitySens, "pitchEnvT1VelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Pitch envelope Time 1 velocity sensitivity" />
        <SynthKnob label="T4 Vel" value={note.pitchEnvT4VelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={set(PcmdPartialField.PitchEnvT4VelocitySens, "pitchEnvT4VelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Pitch envelope Time 4 velocity sensitivity" />
      </div>
      <BreakpointEnvelope
        compact
        times={[{
          label: "T1", value: note.pitchEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmdPartialField.PitchEnvTime, "pitchEnvTime", 0),
        }, {
          label: "T2", value: note.pitchEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmdPartialField.PitchEnvTime, "pitchEnvTime", 1),
        }, {
          label: "T3", value: note.pitchEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmdPartialField.PitchEnvTime, "pitchEnvTime", 2),
        }, {
          label: "T4", value: note.pitchEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmdPartialField.PitchEnvTime, "pitchEnvTime", 3),
        }]}
        levels={[
          { label: "L0", value: note.pitchEnvLevel[0] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: set(PcmdPartialField.PitchEnvLevel, "pitchEnvLevel", 0), formatValue: (v) => signedFmt(v, 64) },
          { label: "L1", value: note.pitchEnvLevel[1] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: set(PcmdPartialField.PitchEnvLevel, "pitchEnvLevel", 1), formatValue: (v) => signedFmt(v, 64) },
          { label: "L2", value: note.pitchEnvLevel[2] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: set(PcmdPartialField.PitchEnvLevel, "pitchEnvLevel", 2), formatValue: (v) => signedFmt(v, 64) },
          { label: "L3", value: note.pitchEnvLevel[3] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: set(PcmdPartialField.PitchEnvLevel, "pitchEnvLevel", 3), formatValue: (v) => signedFmt(v, 64) },
          { label: "L4", value: note.pitchEnvLevel[4] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: set(PcmdPartialField.PitchEnvLevel, "pitchEnvLevel", 4), formatValue: (v) => signedFmt(v, 64) },
        ]}
      />
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
    <SectionPanel label="FILTER" accentColor="#68c">
      <div className={css.filterGrid}>
        <SynthKnob label="Depth" value={note.tvfEnvDepth} min={1} max={127} defaultValue={64} center={64}
          onChange={set(PcmdPartialField.TvfEnvDepth, "tvfEnvDepth")} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Filter envelope depth" />
        <SynthKnob label="Vel Sns" value={note.tvfEnvVelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={set(PcmdPartialField.TvfEnvVelocitySens, "tvfEnvVelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Envelope velocity sensitivity" />
        <SynthKnob label="T1 Vel" value={note.tvfEnvT1VelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={set(PcmdPartialField.TvfEnvT1VelocitySens, "tvfEnvT1VelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Filter envelope Time 1 velocity sensitivity" />
        <SynthKnob label="T4 Vel" value={note.tvfEnvT4VelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={set(PcmdPartialField.TvfEnvT4VelocitySens, "tvfEnvT4VelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Filter envelope Time 4 velocity sensitivity" />
        <SynthSwitch label="Type" value={note.tvfFilterType} vertical
          options={FILTER_TYPE_OPTIONS}
          onChange={set(PcmdPartialField.TvfFilterType, "tvfFilterType")} title="Filter type" />
        <div className={css.stack}>
          <SynthSelect label="Vel Crv" value={note.tvfCutoffVelocityCurve}
            options={VELOCITY_CURVE_OPTIONS}
            onChange={set(PcmdPartialField.TvfCutoffVelocityCurve, "tvfCutoffVelocityCurve")} />
          <SynthSelect label="Env Crv" value={note.tvfEnvVelocityCurve}
            options={VELOCITY_CURVE_OPTIONS}
            onChange={set(PcmdPartialField.TvfEnvVelocityCurve, "tvfEnvVelocityCurve")} />
        </div>
        <div className={css.stack}>
          <SynthKnob label="Cutoff" value={note.tvfCutoffFrequency} min={0} max={127} defaultValue={127}
            onChange={set(PcmdPartialField.TvfCutoffFrequency, "tvfCutoffFrequency")} formatValue={(v) => String(v)} color="#68c"
            title="Filter cutoff frequency" />
          <SynthKnob label="Cut Vel" value={note.tvfCutoffVelocitySens} min={1} max={127} defaultValue={64} center={64}
            onChange={set(PcmdPartialField.TvfCutoffVelocitySens, "tvfCutoffVelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#68c"
            title="Cutoff velocity sensitivity" />
        </div>
        <div className={css.stack}>
          <SynthKnob label="Reso" value={note.tvfResonance} min={0} max={127} defaultValue={0}
            onChange={set(PcmdPartialField.TvfResonance, "tvfResonance")} formatValue={(v) => String(v)} color="#68c"
            title="Filter resonance" />
          <SynthKnob label="Res Vel" value={note.tvfResonanceVelocitySens} min={1} max={127} defaultValue={64} center={64}
            onChange={set(PcmdPartialField.TvfResonanceVelocitySens, "tvfResonanceVelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#68c"
            title="Resonance velocity sensitivity" />
        </div>
      </div>
      <BreakpointEnvelope
        compact
        times={[{
          label: "T1", value: note.tvfEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmdPartialField.TvfEnvTime, "tvfEnvTime", 0),
        }, {
          label: "T2", value: note.tvfEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmdPartialField.TvfEnvTime, "tvfEnvTime", 1),
        }, {
          label: "T3", value: note.tvfEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmdPartialField.TvfEnvTime, "tvfEnvTime", 2),
        }, {
          label: "T4", value: note.tvfEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmdPartialField.TvfEnvTime, "tvfEnvTime", 3),
        }]}
        levels={[
          { label: "L0", value: note.tvfEnvLevel[0] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: set(PcmdPartialField.TvfEnvLevel, "tvfEnvLevel", 0) },
          { label: "L1", value: note.tvfEnvLevel[1] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: set(PcmdPartialField.TvfEnvLevel, "tvfEnvLevel", 1) },
          { label: "L2", value: note.tvfEnvLevel[2] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: set(PcmdPartialField.TvfEnvLevel, "tvfEnvLevel", 2) },
          { label: "L3", value: note.tvfEnvLevel[3] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: set(PcmdPartialField.TvfEnvLevel, "tvfEnvLevel", 3) },
          { label: "L4", value: note.tvfEnvLevel[4] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: set(PcmdPartialField.TvfEnvLevel, "tvfEnvLevel", 4) },
        ]}
      />
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
    <SectionPanel label="AMP" accentColor="#6c8">
      <div className={css.knobRow}>
        <SynthSelect label="Vel Crv" value={note.tvaLevelVelocityCurve}
          options={VELOCITY_CURVE_OPTIONS}
          onChange={set(PcmdPartialField.TvaLevelVelocityCurve, "tvaLevelVelocityCurve")} />
        <SynthKnob label="Vel Sns" value={note.tvaLevelVelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={set(PcmdPartialField.TvaLevelVelocitySens, "tvaLevelVelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
          title="Amplitude velocity sensitivity" />
        <SynthKnob label="T1 Vel" value={note.tvaEnvT1VelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={set(PcmdPartialField.TvaEnvT1VelocitySens, "tvaEnvT1VelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
          title="Amp envelope Time 1 velocity sensitivity" />
        <SynthKnob label="T4 Vel" value={note.tvaEnvT4VelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={set(PcmdPartialField.TvaEnvT4VelocitySens, "tvaEnvT4VelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
          title="Amp envelope Time 4 velocity sensitivity" />
      </div>
      <BreakpointEnvelope
        compact
        times={[{
          label: "T1", value: note.tvaEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmdPartialField.TvaEnvTime, "tvaEnvTime", 0),
        }, {
          label: "T2", value: note.tvaEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmdPartialField.TvaEnvTime, "tvaEnvTime", 1),
        }, {
          label: "T3", value: note.tvaEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmdPartialField.TvaEnvTime, "tvaEnvTime", 2),
        }, {
          label: "T4", value: note.tvaEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmdPartialField.TvaEnvTime, "tvaEnvTime", 3),
        }]}
        levels={[
          { at: 0, text: "0" },
          { label: "L1", value: note.tvaEnvLevel[0] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: set(PcmdPartialField.TvaEnvLevel, "tvaEnvLevel", 0) },
          { label: "L2", value: note.tvaEnvLevel[1] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: set(PcmdPartialField.TvaEnvLevel, "tvaEnvLevel", 1) },
          { label: "L3", value: note.tvaEnvLevel[2] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: set(PcmdPartialField.TvaEnvLevel, "tvaEnvLevel", 2) },
          { at: 0, text: "0" },
        ]}
      />
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
  const muteGroupOptions = [{ value: 0, label: "OFF" }];
  for (let i = 1; i <= 31; i++) {
    muteGroupOptions.push({ value: i, label: String(i) });
  }

  return (
    <SectionPanel label="MISC" accentColor="#a6f">
      <div className={css.noteSelects}>
        <SynthSelect label="Assign" value={note.assignType}
          options={ASSIGN_TYPE_OPTIONS}
          onChange={set(PcmdPartialField.AssignType, "assignType")} />
        <SynthSelect label="Mute Grp" value={note.muteGroup}
          options={muteGroupOptions}
          onChange={set(PcmdPartialField.MuteGroup, "muteGroup")} />
        <SynthSelect label="Env Mode" value={note.envMode}
          options={ENV_MODE_OPTIONS}
          onChange={set(PcmdPartialField.EnvMode, "envMode")} />
        <SynthSelect label="Output" value={note.outputAssign}
          options={OUTPUT_ASSIGN_OPTIONS}
          onChange={set(PcmdPartialField.OutputAssign, "outputAssign")} />
        <SynthSwitch label="1-Shot" value={note.oneShotMode}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={set(PcmdPartialField.OneShotMode, "oneShotMode")}
          title="One Shot Mode" />
      </div>
      <div className={css.knobRow}>
        <SynthKnob label="PB Rng" value={note.pitchBendRange} min={0} max={48} defaultValue={0}
          onChange={set(PcmdPartialField.PitchBendRange, "pitchBendRange")} formatValue={(v) => String(v)} color="#a6f"
          title="Pitch Bend Range" />
        <SynthKnob label="Level" value={note.level} min={0} max={127} defaultValue={127}
          onChange={set(PcmdPartialField.Level, "level")} formatValue={(v) => String(v)} color="#a6f"
          title="Partial Level" />
      </div>
      <div className={css.knobRow}>
        <SynthKnob label="Coarse" value={note.coarseTune} min={0} max={127} defaultValue={64}
          onChange={set(PcmdPartialField.CoarseTune, "coarseTune")} formatValue={(v) => noteName(v)} color="#fc8"
          title="Partial Coarse Tune (C-1 to G9)" />
        <SynthKnob label="Fine" value={note.fineTune} min={14} max={114} defaultValue={64} center={64}
          onChange={set(PcmdPartialField.FineTune, "fineTune")} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Partial Fine Tune (-50 to +50)" />
        <SynthKnob label="Rnd Pit" value={note.randomPitchDepth} 
          {...knobRange(pcmd_partial_spec(PcmdPartialField.RandomPitchDepth))}
          onChange={set(PcmdPartialField.RandomPitchDepth, "randomPitchDepth")} color="#fc8"
          title="Random Pitch Depth (cents)" />
      </div>
      <div className={css.miscRow}>
        <SynthSwitch label="RxExpr" value={note.receiveExpression}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={set(PcmdPartialField.ReceiveExpression, "receiveExpression")}
          title="Receive Expression" />
        <SynthSwitch label="RxHold" value={note.receiveHold1}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={set(PcmdPartialField.ReceiveHold1, "receiveHold1")}
          title="Receive Hold-1" />
      </div>
    </SectionPanel>
  );
}
