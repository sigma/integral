import {
  SynthKnob,
  SynthFader,
  SynthSwitch,
  SynthSelect,
  SectionPanel,
  PartialPowerButton,
  BreakpointEnvelope,
  FaderGroup,
  OutputStrip,
} from "../synth-ui";
import { PcmsPartialField } from "../../pkg/integral_wasm.js";
import css from "../PcmSynthEditor.module.css";
import type { PartialParamHandler, PartialSetter, PcmSynthPartial } from "./types";
import { WavePicker, type ExpansionMarking } from "../WavePicker";
import { waveSourceOf } from "../waveCatalog";
import {
  ON_OFF_OPTIONS,
  WAVE_GAIN_NAMES,
  FILTER_TYPE_NAMES,
  LFO_WAVEFORM_NAMES,
  LFO_WAVEFORM_STEP,
  LFO_OFFSET_NAMES,
  LFO_FADE_MODE_NAMES,
  VELOCITY_CURVE_NAMES,
  BIAS_DIR_NAMES,
  ENV_MODE_NAMES,
  DELAY_MODE_NAMES,
  signedFmt,
  panFmt,
  noteName,
} from "./types";

export function PartialRow({
  idx,
  partial,
  isOn,
  onParam,
  onSwitch,
  marking,
}: {
  idx: number;
  partial: PcmSynthPartial | null;
  isOn: boolean;
  onParam: PartialParamHandler;
  /** Sets the Partial's PMT Partial Switch. */
  onSwitch: (value: number) => void;
  /** Loaded state of the Expansions behind SRX Wave Groups. */
  marking?: ExpansionMarking;
}) {
  const dimClass = isOn ? "" : css.partialDimmed;
  const set: PartialSetter = (field, key, index = 0) =>
    (v) => onParam(field, key, index, v);

  return (
    <div className={`${css.partialRow} ${dimClass}`}>
      <PartialPowerButton label={`Partial ${idx + 1}`} isOn={isOn} onToggle={() => onSwitch(isOn ? 0 : 1)} />
      {partial ? (
        <>
          <WaveSection partial={partial} set={set} marking={marking} />
          <PitchSection partial={partial} set={set} />
          <TvfSection partial={partial} set={set} />
          <TvaSection partial={partial} set={set} />
          <Lfo1Section partial={partial} set={set} />
          <Lfo2Section partial={partial} set={set} />
          <StepLfoSection partial={partial} set={set}
            dimmed={partial.lfo1Waveform !== LFO_WAVEFORM_STEP && partial.lfo2Waveform !== LFO_WAVEFORM_STEP} />
          <OutputSection partial={partial} set={set} />
        </>
      ) : (
        <div className={css.loading}>Loading...</div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Wave Section
// ---------------------------------------------------------------------------

function WaveSection({
  partial,
  set,
  marking,
}: {
  partial: PcmSynthPartial;
  set: PartialSetter;
  marking?: ExpansionMarking;
}) {
  return (
    <SectionPanel label="WAVE" accentColor="#fc8">
      <PartialWave partial={partial} set={set} marking={marking} />
      <SynthSwitch label="Gain" value={partial.waveGain}
        options={WAVE_GAIN_NAMES.map((l, i) => ({ value: i, label: l }))}
        onChange={set(PcmsPartialField.WaveGain, "waveGain")} title="Wave output gain" />
      <SynthKnob label="Key Trk" value={partial.wavePitchKeyfollow} min={44} max={84} defaultValue={64} center={64}
        onChange={set(PcmsPartialField.WavePitchKeyfollow, "wavePitchKeyfollow")} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
        title="Pitch keyboard tracking" />
      <SynthSwitch label="Tempo" value={partial.waveTempoSync} options={ON_OFF_OPTIONS}
        onChange={set(PcmsPartialField.WaveTempoSync, "waveTempoSync")} title="Sync wave playback to song tempo" />
      <SynthSwitch label="FXM" value={partial.waveFxmSwitch} options={ON_OFF_OPTIONS}
        onChange={set(PcmsPartialField.WaveFxmSwitch, "waveFxmSwitch")} title="Frequency Cross Modulation" />
      <SynthKnob label="FXM Col" value={partial.waveFxmColor} min={0} max={3} defaultValue={0}
        onChange={set(PcmsPartialField.WaveFxmColor, "waveFxmColor")} formatValue={(v) => String(v + 1)} color="#fc8"
        title="FXM Color — tonal character" />
      <SynthKnob label="FXM Dpt" value={partial.waveFxmDepth} min={0} max={16} defaultValue={0}
        onChange={set(PcmsPartialField.WaveFxmDepth, "waveFxmDepth")} formatValue={(v) => String(v)} color="#fc8"
        title="FXM Depth — intensity" />
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
  set,
}: {
  partial: PcmSynthPartial;
  set: PartialSetter;
}) {
  return (
    <SectionPanel label="PITCH ENV" accentColor="#fc8">
      <div className={css.panelRow}>
        <SynthKnob label="Coarse" value={partial.coarseTune} min={16} max={112} defaultValue={64} center={64}
          onChange={set(PcmsPartialField.CoarseTune, "coarseTune")} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Coarse tuning (semitones)" />
        <SynthKnob label="Fine" value={partial.fineTune} min={14} max={114} defaultValue={64} center={64}
          onChange={set(PcmsPartialField.FineTune, "fineTune")} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Fine tuning (cents)" />
        <SynthKnob label="Random" value={partial.randomPitchDepth} min={0} max={127} defaultValue={0}
          onChange={set(PcmsPartialField.RandomPitchDepth, "randomPitchDepth")} formatValue={(v) => String(v)} color="#fc8"
          title="Random pitch variation on each note" />
      </div>
      <div className={css.panelRow}>
        <SynthKnob label="Depth" value={partial.pitchEnvDepth} min={1} max={127} defaultValue={64} center={64}
          onChange={set(PcmsPartialField.PitchEnvDepth, "pitchEnvDepth")} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Pitch envelope depth — how much the envelope affects pitch" />
        <SynthKnob label="Vel Sns" value={partial.pitchEnvVelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={set(PcmsPartialField.PitchEnvVelocitySens, "pitchEnvVelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Velocity sensitivity — how key velocity affects envelope depth" />
        <SynthKnob label="Key Trk" value={partial.pitchEnvTimeKeyfollow} min={54} max={74} defaultValue={64} center={64}
          onChange={set(PcmsPartialField.PitchEnvTimeKeyfollow, "pitchEnvTimeKeyfollow")} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Envelope time key tracking — higher keys = faster envelope" />
        <SynthKnob label="T1 Vel" value={partial.pitchEnvT1VelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={set(PcmsPartialField.PitchEnvT1VelocitySens, "pitchEnvT1VelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Pitch envelope Time 1 velocity sensitivity — harder keys change the first stage time" />
        <SynthKnob label="T4 Vel" value={partial.pitchEnvT4VelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={set(PcmsPartialField.PitchEnvT4VelocitySens, "pitchEnvT4VelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Pitch envelope Time 4 velocity sensitivity — release velocity changes the release time" />
      </div>
      <BreakpointEnvelope
        compact
        times={[{
          label: "T1", value: partial.pitchEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmsPartialField.PitchEnvTime, "pitchEnvTime", 0),
        }, {
          label: "T2", value: partial.pitchEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmsPartialField.PitchEnvTime, "pitchEnvTime", 1),
        }, {
          label: "T3", value: partial.pitchEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmsPartialField.PitchEnvTime, "pitchEnvTime", 2),
        }, {
          label: "T4", value: partial.pitchEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmsPartialField.PitchEnvTime, "pitchEnvTime", 3),
        }]}
        levels={[
          { label: "L0", value: partial.pitchEnvLevel[0] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: set(PcmsPartialField.PitchEnvLevel, "pitchEnvLevel", 0), formatValue: (v) => signedFmt(v, 64) },
          { label: "L1", value: partial.pitchEnvLevel[1] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: set(PcmsPartialField.PitchEnvLevel, "pitchEnvLevel", 1), formatValue: (v) => signedFmt(v, 64) },
          { label: "L2", value: partial.pitchEnvLevel[2] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: set(PcmsPartialField.PitchEnvLevel, "pitchEnvLevel", 2), formatValue: (v) => signedFmt(v, 64) },
          { label: "L3", value: partial.pitchEnvLevel[3] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: set(PcmsPartialField.PitchEnvLevel, "pitchEnvLevel", 3), formatValue: (v) => signedFmt(v, 64) },
          { label: "L4", value: partial.pitchEnvLevel[4] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: set(PcmsPartialField.PitchEnvLevel, "pitchEnvLevel", 4), formatValue: (v) => signedFmt(v, 64) },
        ]}
      />
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// TVF (Filter) Section
// ---------------------------------------------------------------------------

function TvfSection({
  partial,
  set,
}: {
  partial: PcmSynthPartial;
  set: PartialSetter;
}) {
  return (
    <SectionPanel label="FILTER" accentColor="#68c">
      <div className={css.filterGrid}>
        <SynthKnob label="Depth" value={partial.tvfEnvDepth} min={1} max={127} defaultValue={64} center={64}
          onChange={set(PcmsPartialField.TvfEnvDepth, "tvfEnvDepth")} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Filter envelope depth — how much the envelope sweeps the cutoff" />
        <SynthKnob label="Vel Sns" value={partial.tvfEnvVelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={set(PcmsPartialField.TvfEnvVelocitySens, "tvfEnvVelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Envelope velocity sensitivity — harder keys = deeper sweep" />
        <SynthKnob label="Key Trk" value={partial.tvfEnvTimeKeyfollow} min={54} max={74} defaultValue={64} center={64}
          onChange={set(PcmsPartialField.TvfEnvTimeKeyfollow, "tvfEnvTimeKeyfollow")} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Envelope time key tracking — higher keys = faster envelope" />
        <SynthKnob label="T1 Vel" value={partial.tvfEnvT1VelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={set(PcmsPartialField.TvfEnvT1VelocitySens, "tvfEnvT1VelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Filter envelope Time 1 velocity sensitivity — harder keys change the first stage time" />
        <SynthKnob label="T4 Vel" value={partial.tvfEnvT4VelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={set(PcmsPartialField.TvfEnvT4VelocitySens, "tvfEnvT4VelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Filter envelope Time 4 velocity sensitivity — release velocity changes the release time" />
        <SynthSwitch label="Type" value={partial.tvfFilterType} vertical
          options={FILTER_TYPE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={set(PcmsPartialField.TvfFilterType, "tvfFilterType")} title="Filter type" />
        <div className={css.stack}>
          <SynthSelect label="Vel Crv" value={partial.tvfCutoffVelocityCurve}
            options={VELOCITY_CURVE_NAMES.map((l, i) => ({ value: i, label: l }))}
            onChange={set(PcmsPartialField.TvfCutoffVelocityCurve, "tvfCutoffVelocityCurve")} title="Cutoff velocity response curve shape" />
          <SynthSelect label="Env Crv" value={partial.tvfEnvVelocityCurve}
            options={VELOCITY_CURVE_NAMES.map((l, i) => ({ value: i, label: l }))}
            onChange={set(PcmsPartialField.TvfEnvVelocityCurve, "tvfEnvVelocityCurve")} title="Envelope velocity response curve shape" />
        </div>
        <SynthKnob label="Key Trk" value={partial.tvfCutoffKeyfollow} min={44} max={84} defaultValue={64} center={64}
          onChange={set(PcmsPartialField.TvfCutoffKeyfollow, "tvfCutoffKeyfollow")} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Cutoff key tracking — higher keys = higher cutoff" />
        <div className={css.stack}>
          <SynthKnob label="Cutoff" value={partial.tvfCutoffFrequency} min={0} max={127} defaultValue={127}
            onChange={set(PcmsPartialField.TvfCutoffFrequency, "tvfCutoffFrequency")} formatValue={(v) => String(v)} color="#68c"
            title="Filter cutoff frequency" />
          <SynthKnob label="Cut Vel" value={partial.tvfCutoffVelocitySens} min={1} max={127} defaultValue={64} center={64}
            onChange={set(PcmsPartialField.TvfCutoffVelocitySens, "tvfCutoffVelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#68c"
            title="Cutoff velocity sensitivity — harder keys = higher cutoff" />
        </div>
        <div className={css.stack}>
          <SynthKnob label="Reso" value={partial.tvfResonance} min={0} max={127} defaultValue={0}
            onChange={set(PcmsPartialField.TvfResonance, "tvfResonance")} formatValue={(v) => String(v)} color="#68c"
            title="Filter resonance — emphasis at the cutoff frequency" />
          <SynthKnob label="Res Vel" value={partial.tvfResonanceVelocitySens} min={1} max={127} defaultValue={64} center={64}
            onChange={set(PcmsPartialField.TvfResonanceVelocitySens, "tvfResonanceVelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#68c"
            title="Resonance velocity sensitivity — harder keys = more resonance" />
        </div>
      </div>
      <BreakpointEnvelope
        compact
        times={[{
          label: "T1", value: partial.tvfEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmsPartialField.TvfEnvTime, "tvfEnvTime", 0),
        }, {
          label: "T2", value: partial.tvfEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmsPartialField.TvfEnvTime, "tvfEnvTime", 1),
        }, {
          label: "T3", value: partial.tvfEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmsPartialField.TvfEnvTime, "tvfEnvTime", 2),
        }, {
          label: "T4", value: partial.tvfEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmsPartialField.TvfEnvTime, "tvfEnvTime", 3),
        }]}
        levels={[
          { label: "L0", value: partial.tvfEnvLevel[0] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: set(PcmsPartialField.TvfEnvLevel, "tvfEnvLevel", 0) },
          { label: "L1", value: partial.tvfEnvLevel[1] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: set(PcmsPartialField.TvfEnvLevel, "tvfEnvLevel", 1) },
          { label: "L2", value: partial.tvfEnvLevel[2] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: set(PcmsPartialField.TvfEnvLevel, "tvfEnvLevel", 2) },
          { label: "L3", value: partial.tvfEnvLevel[3] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: set(PcmsPartialField.TvfEnvLevel, "tvfEnvLevel", 3) },
          { label: "L4", value: partial.tvfEnvLevel[4] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: set(PcmsPartialField.TvfEnvLevel, "tvfEnvLevel", 4) },
        ]}
      />
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// TVA (Amp) Section
// ---------------------------------------------------------------------------

function TvaSection({
  partial,
  set,
}: {
  partial: PcmSynthPartial;
  set: PartialSetter;
}) {
  return (
    <SectionPanel label="AMP" accentColor="#6c8">
      <div className={css.panelRow}>
        <div className={`${css.knobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Level" value={partial.level} min={0} max={127} defaultValue={127}
            onChange={set(PcmsPartialField.Level, "level")} formatValue={(v) => String(v)} color="#6c8"
            title="Partial volume level" />
          <SynthKnob label="Pan" value={partial.pan} min={0} max={127} defaultValue={64} center={64}
            onChange={set(PcmsPartialField.Pan, "pan")} formatValue={panFmt} color="#6c8"
            title="Partial stereo panning" />
          <SynthKnob label="Vel Sns" value={partial.tvaLevelVelocitySens} min={1} max={127} defaultValue={64} center={64}
            onChange={set(PcmsPartialField.TvaLevelVelocitySens, "tvaLevelVelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
            title="Volume velocity sensitivity — harder keys = louder" />
          <SynthKnob label="Key Trk" value={partial.tvaEnvTimeKeyfollow} min={54} max={74} defaultValue={64} center={64}
            onChange={set(PcmsPartialField.TvaEnvTimeKeyfollow, "tvaEnvTimeKeyfollow")} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
            title="Envelope time key tracking — higher keys = faster envelope" />
          <SynthKnob label="T1 Vel" value={partial.tvaEnvT1VelocitySens} min={1} max={127} defaultValue={64} center={64}
            onChange={set(PcmsPartialField.TvaEnvT1VelocitySens, "tvaEnvT1VelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
            title="Amp envelope Time 1 velocity sensitivity — harder keys change the first stage time" />
          <SynthKnob label="T4 Vel" value={partial.tvaEnvT4VelocitySens} min={1} max={127} defaultValue={64} center={64}
            onChange={set(PcmsPartialField.TvaEnvT4VelocitySens, "tvaEnvT4VelocitySens")} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
            title="Amp envelope Time 4 velocity sensitivity — release velocity changes the release time" />
        </div>
      </div>
      <div className={css.panelRow}>
        <SynthSelect label="Vel Curve" value={partial.tvaLevelVelocityCurve}
          options={VELOCITY_CURVE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={set(PcmsPartialField.TvaLevelVelocityCurve, "tvaLevelVelocityCurve")} title="Velocity response curve shape" />
        <SynthKnob label="Bias Lvl" value={partial.tvaBiasLevel} min={0} max={127} defaultValue={0}
          onChange={set(PcmsPartialField.TvaBiasLevel, "tvaBiasLevel")} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
          title="Level bias amount — attenuates volume away from the bias key" />
        <SynthKnob label="Bias Key" value={partial.tvaBiasPosition} min={0} max={127} defaultValue={64}
          onChange={set(PcmsPartialField.TvaBiasPosition, "tvaBiasPosition")} formatValue={noteName} color="#6c8"
          title="Bias center key — volume is loudest here, attenuated away" />
        <SynthSwitch label="Bias Dir" value={partial.tvaBiasDirection} vertical
          options={BIAS_DIR_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={set(PcmsPartialField.TvaBiasDirection, "tvaBiasDirection")} led={false}
          title="Bias direction — which side of the center key is attenuated" />
      </div>
      <BreakpointEnvelope
        compact
        times={[{
          label: "T1", value: partial.tvaEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmsPartialField.TvaEnvTime, "tvaEnvTime", 0),
        }, {
          label: "T2", value: partial.tvaEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmsPartialField.TvaEnvTime, "tvaEnvTime", 1),
        }, {
          label: "T3", value: partial.tvaEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmsPartialField.TvaEnvTime, "tvaEnvTime", 2),
        }, {
          label: "T4", value: partial.tvaEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: set(PcmsPartialField.TvaEnvTime, "tvaEnvTime", 3),
        }]}
        levels={[
          { at: 0, text: "0" },
          { label: "L1", value: partial.tvaEnvLevel[0] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: set(PcmsPartialField.TvaEnvLevel, "tvaEnvLevel", 0) },
          { label: "L2", value: partial.tvaEnvLevel[1] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: set(PcmsPartialField.TvaEnvLevel, "tvaEnvLevel", 1) },
          { label: "L3", value: partial.tvaEnvLevel[2] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: set(PcmsPartialField.TvaEnvLevel, "tvaEnvLevel", 2) },
          { at: 0, text: "0" },
        ]}
      />
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// LFO1 Section
// ---------------------------------------------------------------------------

function Lfo1Section({
  partial,
  set,
}: {
  partial: PcmSynthPartial;
  set: PartialSetter;
}) {
  return (
    <SectionPanel label="LFO1" accentColor="#a6f">
      <div className={css.panelRow}>
        <SynthSwitch label="Wave" value={partial.lfo1Waveform} vertical
          options={LFO_WAVEFORM_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={set(PcmsPartialField.Lfo1Waveform, "lfo1Waveform")} title="LFO waveform shape" />
        <div className={`${css.knobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Rate" value={partial.lfo1Rate} min={0} max={149} defaultValue={0}
            onChange={set(PcmsPartialField.Lfo1Rate, "lfo1Rate")} formatValue={(v) => String(v)} color="#a6f"
            title="LFO speed" />
          <SynthKnob label="Detune" value={partial.lfo1RateDetune} min={0} max={127} defaultValue={0}
            onChange={set(PcmsPartialField.Lfo1RateDetune, "lfo1RateDetune")} formatValue={(v) => String(v)} color="#a6f"
            title="Rate randomization — adds organic variation" />
          <SynthKnob label="Delay" value={partial.lfo1DelayTime} min={0} max={127} defaultValue={0}
            onChange={set(PcmsPartialField.Lfo1DelayTime, "lfo1DelayTime")} formatValue={(v) => String(v)} color="#a6f"
            title="Time before LFO starts after note-on" />
          <SynthKnob label="Fade In" value={partial.lfo1FadeTime} min={0} max={127} defaultValue={0}
            onChange={set(PcmsPartialField.Lfo1FadeTime, "lfo1FadeTime")} formatValue={(v) => String(v)} color="#a6f"
            title="Time for LFO to reach full depth after delay" />
        </div>
      </div>
      <div className={css.panelRow}>
        <SynthSwitch label="Offset" value={partial.lfo1Offset} vertical
          options={LFO_OFFSET_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={set(PcmsPartialField.Lfo1Offset, "lfo1Offset")} led={false}
          title="DC offset of the LFO waveform" />
        <SynthSwitch label="Dly KF" value={partial.lfo1DelayTimeKeyfollow}
          options={ON_OFF_OPTIONS}
          onChange={set(PcmsPartialField.Lfo1DelayTimeKeyfollow, "lfo1DelayTimeKeyfollow")}
          title="Delay time key tracking — higher keys = shorter delay" />
        <SynthSwitch label="Fade" value={partial.lfo1FadeMode} vertical
          options={LFO_FADE_MODE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={set(PcmsPartialField.Lfo1FadeMode, "lfo1FadeMode")} led={false}
          title="Fade behavior: fade in on note-on, fade out, or on key-off" />
        <SynthSwitch label="Key Trig" value={partial.lfo1KeyTrigger} options={ON_OFF_OPTIONS}
          onChange={set(PcmsPartialField.Lfo1KeyTrigger, "lfo1KeyTrigger")}
          title="Reset LFO phase on each new note" />
      </div>
      <FaderGroup>
        <SynthFader label="Pitch" value={partial.lfo1PitchDepth} min={1} max={127} defaultValue={64}
          onChange={set(PcmsPartialField.Lfo1PitchDepth, "lfo1PitchDepth")} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on pitch (vibrato)" />
        <SynthFader label="Filter" value={partial.lfo1TvfDepth} min={1} max={127} defaultValue={64}
          onChange={set(PcmsPartialField.Lfo1TvfDepth, "lfo1TvfDepth")} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on filter cutoff (wah)" />
        <SynthFader label="Amp" value={partial.lfo1TvaDepth} min={1} max={127} defaultValue={64}
          onChange={set(PcmsPartialField.Lfo1TvaDepth, "lfo1TvaDepth")} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on amplitude (tremolo)" />
        <SynthFader label="Pan" value={partial.lfo1PanDepth} min={1} max={127} defaultValue={64}
          onChange={set(PcmsPartialField.Lfo1PanDepth, "lfo1PanDepth")} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on panning (auto-pan)" />
      </FaderGroup>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// LFO2 Section
// ---------------------------------------------------------------------------

function Lfo2Section({
  partial,
  set,
}: {
  partial: PcmSynthPartial;
  set: PartialSetter;
}) {
  return (
    <SectionPanel label="LFO2" accentColor="#a6f">
      <div className={css.panelRow}>
        <SynthSwitch label="Wave" value={partial.lfo2Waveform} vertical
          options={LFO_WAVEFORM_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={set(PcmsPartialField.Lfo2Waveform, "lfo2Waveform")} title="LFO waveform shape" />
        <div className={`${css.knobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Rate" value={partial.lfo2Rate} min={0} max={149} defaultValue={0}
            onChange={set(PcmsPartialField.Lfo2Rate, "lfo2Rate")} formatValue={(v) => String(v)} color="#a6f"
            title="LFO speed" />
          <SynthKnob label="Detune" value={partial.lfo2RateDetune} min={0} max={127} defaultValue={0}
            onChange={set(PcmsPartialField.Lfo2RateDetune, "lfo2RateDetune")} formatValue={(v) => String(v)} color="#a6f"
            title="Rate randomization — adds organic variation" />
          <SynthKnob label="Delay" value={partial.lfo2DelayTime} min={0} max={127} defaultValue={0}
            onChange={set(PcmsPartialField.Lfo2DelayTime, "lfo2DelayTime")} formatValue={(v) => String(v)} color="#a6f"
            title="Time before LFO starts after note-on" />
          <SynthKnob label="Fade In" value={partial.lfo2FadeTime} min={0} max={127} defaultValue={0}
            onChange={set(PcmsPartialField.Lfo2FadeTime, "lfo2FadeTime")} formatValue={(v) => String(v)} color="#a6f"
            title="Time for LFO to reach full depth after delay" />
        </div>
      </div>
      <div className={css.panelRow}>
        <SynthSwitch label="Offset" value={partial.lfo2Offset} vertical
          options={LFO_OFFSET_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={set(PcmsPartialField.Lfo2Offset, "lfo2Offset")} led={false}
          title="DC offset of the LFO waveform" />
        <SynthSwitch label="Dly KF" value={partial.lfo2DelayTimeKeyfollow}
          options={ON_OFF_OPTIONS}
          onChange={set(PcmsPartialField.Lfo2DelayTimeKeyfollow, "lfo2DelayTimeKeyfollow")}
          title="Delay time key tracking — higher keys = shorter delay" />
        <SynthSwitch label="Fade" value={partial.lfo2FadeMode} vertical
          options={LFO_FADE_MODE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={set(PcmsPartialField.Lfo2FadeMode, "lfo2FadeMode")} led={false}
          title="Fade behavior: fade in on note-on, fade out, or on key-off" />
        <SynthSwitch label="Key Trig" value={partial.lfo2KeyTrigger} options={ON_OFF_OPTIONS}
          onChange={set(PcmsPartialField.Lfo2KeyTrigger, "lfo2KeyTrigger")}
          title="Reset LFO phase on each new note" />
      </div>
      <FaderGroup>
        <SynthFader label="Pitch" value={partial.lfo2PitchDepth} min={1} max={127} defaultValue={64}
          onChange={set(PcmsPartialField.Lfo2PitchDepth, "lfo2PitchDepth")} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on pitch (vibrato)" />
        <SynthFader label="Filter" value={partial.lfo2TvfDepth} min={1} max={127} defaultValue={64}
          onChange={set(PcmsPartialField.Lfo2TvfDepth, "lfo2TvfDepth")} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on filter cutoff (wah)" />
        <SynthFader label="Amp" value={partial.lfo2TvaDepth} min={1} max={127} defaultValue={64}
          onChange={set(PcmsPartialField.Lfo2TvaDepth, "lfo2TvaDepth")} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on amplitude (tremolo)" />
        <SynthFader label="Pan" value={partial.lfo2PanDepth} min={1} max={127} defaultValue={64}
          onChange={set(PcmsPartialField.Lfo2PanDepth, "lfo2PanDepth")} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on panning (auto-pan)" />
      </FaderGroup>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Step LFO Section
// ---------------------------------------------------------------------------

function StepLfoSection({
  partial,
  set,
  dimmed,
}: {
  partial: PcmSynthPartial;
  set: PartialSetter;
  dimmed?: boolean;
}) {
  const steps = partial.lfoStepValues ?? [];
  const smooth = partial.lfoStepType === 1;
  const dimStyle = dimmed ? { opacity: 0.35 } : undefined;

  // Build SVG path for the step visualizer
  const vizW = 160;
  const vizH = 40;
  const pad = 2;
  const stepW = (vizW - pad * 2) / 16;
  const yRange = vizH - pad * 2;
  const yFor = (v: number) => pad + (1 - (((v ?? 64) - 1) / 126)) * yRange;

  let vizPath = "";
  if (smooth) {
    // Smooth: line through center of each step
    vizPath = steps.map((v, i) => {
      const x = pad + (i + 0.5) * stepW;
      const y = yFor(v);
      return i === 0 ? `M ${x} ${y}` : `L ${x} ${y}`;
    }).join(" ");
  } else {
    // Staircase: horizontal bars per step
    vizPath = steps.map((v, i) => {
      const x1 = pad + i * stepW;
      const x2 = x1 + stepW;
      const y = yFor(v);
      return `${i === 0 ? "M" : "L"} ${x1} ${y} L ${x2} ${y}`;
    }).join(" ");
  }

  return (
    <div style={dimStyle}>
    <SectionPanel label="STEP LFO" accentColor="#a6f">
      <div className={css.panelRow}>
        <SynthSwitch label="Shape" value={partial.lfoStepType}
          options={[{ value: 0, label: "STEP" }, { value: 1, label: "SMOOTH" }]}
          onChange={set(PcmsPartialField.LfoStepType, "lfoStepType")}
          title="Step shape: STEP = hard jumps, SMOOTH = interpolated transitions. Set LFO1 or LFO2 waveform to STEP to use this pattern." />
      </div>
      {/* Step pattern visualizer */}
      <svg width={vizW} height={vizH} className={css.stepViz}>
        {/* Center line (zero) */}
        <line x1={pad} y1={vizH / 2} x2={vizW - pad} y2={vizH / 2}
          stroke="#333" strokeWidth="1" strokeDasharray="2 2" />
        {/* Pattern curve */}
        <path d={vizPath} fill="none" stroke="#a6f" strokeWidth="1.5" />
      </svg>
      <div className={css.stepGrid}>
        {Array.from({ length: 16 }, (_, i) => (
          <SynthFader
            key={i}
            label={`${i + 1}`}
            value={steps[i] ?? 64}
            min={1}
            max={127}
            defaultValue={64}
            onChange={set(PcmsPartialField.LfoStep, "lfoStepValues", i)}
            formatValue={(v) => signedFmt(v, 64)}
            compact
          />
        ))}
      </div>
    </SectionPanel>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Output Section
// ---------------------------------------------------------------------------

function OutputSection({
  partial,
  set,
}: {
  partial: PcmSynthPartial;
  set: PartialSetter;
}) {
  return (
    <SectionPanel label="OUTPUT" accentColor="#6c8">
      <div className={css.panelRow}>
        <SynthSwitch label="Env Mode" value={partial.envMode} vertical
          options={ENV_MODE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={set(PcmsPartialField.EnvMode, "envMode")} led={false}
          title="Envelope mode: NO-SUS plays once, SUSTAIN holds at sustain level" />
        <SynthSwitch label="Delay" value={partial.delayMode} vertical
          options={DELAY_MODE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={set(PcmsPartialField.DelayMode, "delayMode")} led={false}
          title="Partial delay mode — when sound starts relative to note-on" />
        <SynthKnob label="Delay" value={partial.delayTime} min={0} max={149} defaultValue={0}
          onChange={set(PcmsPartialField.DelayTime, "delayTime")} formatValue={(v) => String(v)} color="#6c8"
          title="Delay time before this partial sounds" />
      </div>
      <div className={css.panelRow}>
        <SynthSwitch label="Bender" value={partial.receiveBender} options={ON_OFF_OPTIONS}
          onChange={set(PcmsPartialField.ReceiveBender, "receiveBender")}
          title="Respond to pitch bend" />
        <SynthSwitch label="Express" value={partial.receiveExpression} options={ON_OFF_OPTIONS}
          onChange={set(PcmsPartialField.ReceiveExpression, "receiveExpression")}
          title="Respond to expression pedal (CC11)" />
        <SynthSwitch label="Hold" value={partial.receiveHold1} options={ON_OFF_OPTIONS}
          onChange={set(PcmsPartialField.ReceiveHold1, "receiveHold1")}
          title="Respond to sustain pedal (CC64)" />
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
