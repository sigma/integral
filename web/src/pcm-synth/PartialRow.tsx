import {
  SynthKnob,
  SynthFader,
  SynthSwitch,
  SynthSelect,
  SectionPanel,
  ADSREnvelope,
  FaderGroup,
  OutputStrip,
} from "../synth-ui";
import css from "../PcmSynthEditor.module.css";
import type { PcmSynthPartial } from "./types";
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
  setP,
  setNibP,
  setNib2P,
  setPmtParam,
}: {
  idx: number;
  partial: PcmSynthPartial | null;
  isOn: boolean;
  setP: (offset: number, value: number) => void;
  setNibP: (offset: number, value: number) => void;
  setNib2P: (offset: number, value: number) => void;
  setPmtParam: (offset: number, value: number) => void;
}) {
  const dimClass = isOn ? "" : css.partialDimmed;
  const pmtSwOffset = 0x05 + idx * 9;

  return (
    <div className={`${css.partialRow} ${dimClass}`}>
      {/* Partial switch + label */}
      <div className={css.partialSwitch}>
        <SynthSwitch
          label={`P${idx + 1}`}
          value={isOn ? 1 : 0}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={(v) => setPmtParam(pmtSwOffset, v)}
        />
      </div>
      {partial ? (
        <>
          <WaveSection partial={partial} partialIdx={idx} setP={setP} setNibP={setNibP} />
          <PitchSection partial={partial} setP={setP} />
          <TvfSection partial={partial} setP={setP} />
          <TvaSection partial={partial} setP={setP} />
          <Lfo1Section partial={partial} setP={setP} setNib2P={setNib2P} />
          <Lfo2Section partial={partial} setP={setP} setNib2P={setNib2P} />
          <StepLfoSection partial={partial} setP={setP}
            dimmed={partial.lfo1Waveform !== LFO_WAVEFORM_STEP && partial.lfo2Waveform !== LFO_WAVEFORM_STEP} />
          <OutputSection partial={partial} setP={setP} setNib2P={setNib2P} />
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
  partialIdx: _partialIdx,
  setP,
  setNibP,
}: {
  partial: PcmSynthPartial;
  partialIdx: number;
  setP: (offset: number, value: number) => void;
  setNibP: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="WAVE" accentColor="#fc8">
      <SynthSwitch label="Source" value={partial.waveGroupType}
        options={[{ value: 0, label: "INT" }, { value: 1, label: "SRX" }]}
        onChange={(v) => setP(0x27, v)} title="Wave source: Internal or SRX expansion" />
      <label className={css.selectLabel} title="Left (mono) waveform number">
        Wave L <input type="number" className={css.waveNumberInput}
          value={partial.waveNumberL} min={0} max={16384}
          onChange={(e) => setNibP(0x2C, Math.max(0, Math.min(16384, Number(e.target.value))))} />
      </label>
      <label className={css.selectLabel} title="Right waveform number (stereo pairs)">
        Wave R <input type="number" className={css.waveNumberInput}
          value={partial.waveNumberR} min={0} max={16384}
          onChange={(e) => setNibP(0x30, Math.max(0, Math.min(16384, Number(e.target.value))))} />
      </label>
      <SynthSwitch label="Gain" value={partial.waveGain}
        options={WAVE_GAIN_NAMES.map((l, i) => ({ value: i, label: l }))}
        onChange={(v) => setP(0x34, v)} title="Wave output gain" />
      <SynthKnob label="Key Trk" value={partial.wavePitchKeyfollow} min={44} max={84} defaultValue={64}
        onChange={(v) => setP(0x39, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
        title="Pitch keyboard tracking" />
      <SynthSwitch label="Tempo" value={partial.waveTempoSync} options={ON_OFF_OPTIONS}
        onChange={(v) => setP(0x38, v)} title="Sync wave playback to song tempo" />
      <SynthSwitch label="FXM" value={partial.waveFxmSwitch} options={ON_OFF_OPTIONS}
        onChange={(v) => setP(0x35, v)} title="Frequency Cross Modulation" />
      <SynthKnob label="FXM Col" value={partial.waveFxmColor} min={0} max={3} defaultValue={0}
        onChange={(v) => setP(0x36, v)} formatValue={(v) => String(v + 1)} color="#fc8"
        title="FXM Color — tonal character" />
      <SynthKnob label="FXM Dpt" value={partial.waveFxmDepth} min={0} max={16} defaultValue={0}
        onChange={(v) => setP(0x37, v)} formatValue={(v) => String(v)} color="#fc8"
        title="FXM Depth — intensity" />
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Pitch + Envelope Section
// ---------------------------------------------------------------------------

function PitchSection({
  partial,
  setP,
}: {
  partial: PcmSynthPartial;
  setP: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="PITCH ENV" accentColor="#fc8">
      <div className={css.panelRow}>
        <SynthKnob label="Coarse" value={partial.coarseTune} min={16} max={112} defaultValue={64}
          onChange={(v) => setP(0x01, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Coarse tuning (semitones)" />
        <SynthKnob label="Fine" value={partial.fineTune} min={14} max={114} defaultValue={64}
          onChange={(v) => setP(0x02, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Fine tuning (cents)" />
        <SynthKnob label="Random" value={partial.randomPitchDepth} min={0} max={127} defaultValue={0}
          onChange={(v) => setP(0x03, v)} formatValue={(v) => String(v)} color="#fc8"
          title="Random pitch variation on each note" />
      </div>
      <div className={css.panelRow}>
        <SynthKnob label="Depth" value={partial.pitchEnvDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x3A, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Pitch envelope depth — how much the envelope affects pitch" />
        <SynthKnob label="Vel Sns" value={partial.pitchEnvVelocitySens} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x3B, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Velocity sensitivity — how key velocity affects envelope depth" />
        <SynthKnob label="Key Trk" value={partial.pitchEnvTimeKeyfollow} min={54} max={74} defaultValue={64}
          onChange={(v) => setP(0x3E, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Envelope time key tracking — higher keys = faster envelope" />
      </div>
      <ADSREnvelope
        compact
        attack={{
          label: "Atk", value: partial.pitchEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x3F, v),
        }}
        decay={{
          label: "Dec", value: partial.pitchEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x40, v),
        }}
        sustain={{
          label: "Sus", value: partial.pitchEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x41, v),
        }}
        release={{
          label: "Rel", value: partial.pitchEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x42, v),
        }}
        levels={[
          { label: "Start", value: partial.pitchEnvLevel[0] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => setP(0x43, v), formatValue: (v) => signedFmt(v, 64) },
          { label: "Atk", value: partial.pitchEnvLevel[1] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => setP(0x44, v), formatValue: (v) => signedFmt(v, 64) },
          { label: "Dec", value: partial.pitchEnvLevel[2] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => setP(0x45, v), formatValue: (v) => signedFmt(v, 64) },
          { label: "Sus", value: partial.pitchEnvLevel[3] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => setP(0x46, v), formatValue: (v) => signedFmt(v, 64) },
          { label: "End", value: partial.pitchEnvLevel[4] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => setP(0x47, v), formatValue: (v) => signedFmt(v, 64) },
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
  setP,
}: {
  partial: PcmSynthPartial;
  setP: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="FILTER" accentColor="#68c">
      <div className={css.panelRow}>
        <SynthSwitch label="Type" value={partial.tvfFilterType} vertical
          options={FILTER_TYPE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x48, v)} title="Filter type" />
        <div className={`${css.knobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Cutoff" value={partial.tvfCutoffFrequency} min={0} max={127} defaultValue={127}
            onChange={(v) => setP(0x49, v)} formatValue={(v) => String(v)} color="#68c"
            title="Filter cutoff frequency" />
          <SynthKnob label="Reso" value={partial.tvfResonance} min={0} max={127} defaultValue={0}
            onChange={(v) => setP(0x4D, v)} formatValue={(v) => String(v)} color="#68c"
            title="Filter resonance — emphasis at the cutoff frequency" />
          <SynthKnob label="Key Trk" value={partial.tvfCutoffKeyfollow} min={44} max={84} defaultValue={64}
            onChange={(v) => setP(0x4A, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
            title="Cutoff key tracking — higher keys = higher cutoff" />
          <SynthKnob label="Vel Sns" value={partial.tvfCutoffVelocitySens} min={1} max={127} defaultValue={64}
            onChange={(v) => setP(0x4C, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
            title="Cutoff velocity sensitivity — harder keys = higher cutoff" />
        </div>
      </div>
      <div className={css.panelRow}>
        <SynthKnob label="Depth" value={partial.tvfEnvDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x4F, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Filter envelope depth — how much the envelope sweeps the cutoff" />
        <SynthKnob label="Vel Sns" value={partial.tvfEnvVelocitySens} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x51, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Envelope velocity sensitivity — harder keys = deeper sweep" />
        <SynthKnob label="Key Trk" value={partial.tvfEnvTimeKeyfollow} min={54} max={74} defaultValue={64}
          onChange={(v) => setP(0x54, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Envelope time key tracking — higher keys = faster envelope" />
        <SynthSelect label="Vel Curve" value={partial.tvfCutoffVelocityCurve}
          options={VELOCITY_CURVE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x4B, v)} title="Velocity response curve shape" />
      </div>
      <ADSREnvelope
        compact
        attack={{
          label: "Atk", value: partial.tvfEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x55, v),
        }}
        decay={{
          label: "Dec", value: partial.tvfEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x56, v),
        }}
        sustain={{
          label: "Sus", value: partial.tvfEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x57, v),
        }}
        release={{
          label: "Rel", value: partial.tvfEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x58, v),
        }}
        levels={[
          { label: "Start", value: partial.tvfEnvLevel[0] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => setP(0x59, v) },
          { label: "Atk", value: partial.tvfEnvLevel[1] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: (v) => setP(0x5A, v) },
          { label: "Dec", value: partial.tvfEnvLevel[2] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: (v) => setP(0x5B, v) },
          { label: "Sus", value: partial.tvfEnvLevel[3] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: (v) => setP(0x5C, v) },
          { label: "End", value: partial.tvfEnvLevel[4] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => setP(0x5D, v) },
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
  setP,
}: {
  partial: PcmSynthPartial;
  setP: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="AMP" accentColor="#6c8">
      <div className={css.panelRow}>
        <div className={`${css.knobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Level" value={partial.level} min={0} max={127} defaultValue={127}
            onChange={(v) => setP(0x00, v)} formatValue={(v) => String(v)} color="#6c8"
            title="Partial volume level" />
          <SynthKnob label="Pan" value={partial.pan} min={0} max={127} defaultValue={64}
            onChange={(v) => setP(0x04, v)} formatValue={panFmt} color="#6c8"
            title="Partial stereo panning" />
          <SynthKnob label="Vel Sns" value={partial.tvaLevelVelocitySens} min={1} max={127} defaultValue={64}
            onChange={(v) => setP(0x62, v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
            title="Volume velocity sensitivity — harder keys = louder" />
          <SynthKnob label="Key Trk" value={partial.tvaEnvTimeKeyfollow} min={54} max={74} defaultValue={64}
            onChange={(v) => setP(0x65, v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
            title="Envelope time key tracking — higher keys = faster envelope" />
        </div>
      </div>
      <div className={css.panelRow}>
        <SynthSelect label="Vel Curve" value={partial.tvaLevelVelocityCurve}
          options={VELOCITY_CURVE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x61, v)} title="Velocity response curve shape" />
        <SynthKnob label="Bias Lvl" value={partial.tvaBiasLevel} min={0} max={127} defaultValue={0}
          onChange={(v) => setP(0x5E, v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
          title="Level bias amount — attenuates volume away from the bias key" />
        <SynthKnob label="Bias Key" value={partial.tvaBiasPosition} min={0} max={127} defaultValue={64}
          onChange={(v) => setP(0x5F, v)} formatValue={noteName} color="#6c8"
          title="Bias center key — volume is loudest here, attenuated away" />
        <SynthSwitch label="Bias Dir" value={partial.tvaBiasDirection} vertical
          options={BIAS_DIR_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x60, v)} led={false}
          title="Bias direction — which side of the center key is attenuated" />
      </div>
      <ADSREnvelope
        compact
        attack={{
          label: "Atk", value: partial.tvaEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x66, v),
        }}
        decay={{
          label: "Dec", value: partial.tvaEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x67, v),
        }}
        sustain={{
          label: "Sus", value: partial.tvaEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x68, v),
        }}
        release={{
          label: "Rel", value: partial.tvaEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x69, v),
        }}
        levels={[
          { label: "", value: 127, min: 0, max: 127, defaultValue: 127, onChange: () => {}, hidden: true },
          { label: "Atk", value: partial.tvaEnvLevel[0] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: (v) => setP(0x6A, v) },
          { label: "Dec", value: partial.tvaEnvLevel[1] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: (v) => setP(0x6B, v) },
          { label: "Sus", value: partial.tvaEnvLevel[2] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => setP(0x6C, v) },
          { label: "", value: 0, min: 0, max: 127, defaultValue: 0, onChange: () => {}, hidden: true },
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
  setP,
  setNib2P,
}: {
  partial: PcmSynthPartial;
  setP: (offset: number, value: number) => void;
  setNib2P: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="LFO1" accentColor="#a6f">
      <div className={css.panelRow}>
        <SynthSwitch label="Wave" value={partial.lfo1Waveform} vertical
          options={LFO_WAVEFORM_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x6D, v)} title="LFO waveform shape" />
        <div className={`${css.knobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Rate" value={partial.lfo1Rate} min={0} max={149} defaultValue={0}
            onChange={(v) => setNib2P(0x6E, v)} formatValue={(v) => String(v)} color="#a6f"
            title="LFO speed" />
          <SynthKnob label="Detune" value={partial.lfo1RateDetune} min={0} max={127} defaultValue={0}
            onChange={(v) => setP(0x71, v)} formatValue={(v) => String(v)} color="#a6f"
            title="Rate randomization — adds organic variation" />
          <SynthKnob label="Delay" value={partial.lfo1DelayTime} min={0} max={127} defaultValue={0}
            onChange={(v) => setP(0x72, v)} formatValue={(v) => String(v)} color="#a6f"
            title="Time before LFO starts after note-on" />
          <SynthKnob label="Fade In" value={partial.lfo1FadeTime} min={0} max={127} defaultValue={0}
            onChange={(v) => setP(0x75, v)} formatValue={(v) => String(v)} color="#a6f"
            title="Time for LFO to reach full depth after delay" />
        </div>
      </div>
      <div className={css.panelRow}>
        <SynthSwitch label="Offset" value={partial.lfo1Offset} vertical
          options={LFO_OFFSET_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x70, v)} led={false}
          title="DC offset of the LFO waveform" />
        <SynthSwitch label="Dly KF" value={partial.lfo1DelayTimeKeyfollow}
          options={ON_OFF_OPTIONS}
          onChange={(v) => setP(0x73, v)}
          title="Delay time key tracking — higher keys = shorter delay" />
        <SynthSwitch label="Fade" value={partial.lfo1FadeMode} vertical
          options={LFO_FADE_MODE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x74, v)} led={false}
          title="Fade behavior: fade in on note-on, fade out, or on key-off" />
        <SynthSwitch label="Key Trig" value={partial.lfo1KeyTrigger} options={ON_OFF_OPTIONS}
          onChange={(v) => setP(0x76, v)}
          title="Reset LFO phase on each new note" />
      </div>
      <FaderGroup>
        <SynthFader label="Pitch" value={partial.lfo1PitchDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x77, v)} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on pitch (vibrato)" />
        <SynthFader label="Filter" value={partial.lfo1TvfDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x78, v)} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on filter cutoff (wah)" />
        <SynthFader label="Amp" value={partial.lfo1TvaDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x79, v)} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on amplitude (tremolo)" />
        <SynthFader label="Pan" value={partial.lfo1PanDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x7A, v)} formatValue={(v) => signedFmt(v, 64)} compact
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
  setP,
  setNib2P,
}: {
  partial: PcmSynthPartial;
  setP: (offset: number, value: number) => void;
  setNib2P: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="LFO2" accentColor="#a6f">
      <div className={css.panelRow}>
        <SynthSwitch label="Wave" value={partial.lfo2Waveform} vertical
          options={LFO_WAVEFORM_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x7B, v)} title="LFO waveform shape" />
        <div className={`${css.knobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Rate" value={partial.lfo2Rate} min={0} max={149} defaultValue={0}
            onChange={(v) => setNib2P(0x7C, v)} formatValue={(v) => String(v)} color="#a6f"
            title="LFO speed" />
          <SynthKnob label="Detune" value={partial.lfo2RateDetune} min={0} max={127} defaultValue={0}
            onChange={(v) => setP(0x7F, v)} formatValue={(v) => String(v)} color="#a6f"
            title="Rate randomization — adds organic variation" />
          <SynthKnob label="Delay" value={partial.lfo2DelayTime} min={0} max={127} defaultValue={0}
            onChange={(v) => setP(0x0100, v)} formatValue={(v) => String(v)} color="#a6f"
            title="Time before LFO starts after note-on" />
          <SynthKnob label="Fade In" value={partial.lfo2FadeTime} min={0} max={127} defaultValue={0}
            onChange={(v) => setP(0x0103, v)} formatValue={(v) => String(v)} color="#a6f"
            title="Time for LFO to reach full depth after delay" />
        </div>
      </div>
      <div className={css.panelRow}>
        <SynthSwitch label="Offset" value={partial.lfo2Offset} vertical
          options={LFO_OFFSET_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x7E, v)} led={false}
          title="DC offset of the LFO waveform" />
        <SynthSwitch label="Dly KF" value={partial.lfo2DelayTimeKeyfollow}
          options={ON_OFF_OPTIONS}
          onChange={(v) => setP(0x0101, v)}
          title="Delay time key tracking — higher keys = shorter delay" />
        <SynthSwitch label="Fade" value={partial.lfo2FadeMode} vertical
          options={LFO_FADE_MODE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x0102, v)} led={false}
          title="Fade behavior: fade in on note-on, fade out, or on key-off" />
        <SynthSwitch label="Key Trig" value={partial.lfo2KeyTrigger} options={ON_OFF_OPTIONS}
          onChange={(v) => setP(0x0104, v)}
          title="Reset LFO phase on each new note" />
      </div>
      <FaderGroup>
        <SynthFader label="Pitch" value={partial.lfo2PitchDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x0105, v)} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on pitch (vibrato)" />
        <SynthFader label="Filter" value={partial.lfo2TvfDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x0106, v)} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on filter cutoff (wah)" />
        <SynthFader label="Amp" value={partial.lfo2TvaDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x0107, v)} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on amplitude (tremolo)" />
        <SynthFader label="Pan" value={partial.lfo2PanDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x0108, v)} formatValue={(v) => signedFmt(v, 64)} compact
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
  setP,
  dimmed,
}: {
  partial: PcmSynthPartial;
  setP: (offset: number, value: number) => void;
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
          onChange={(v) => setP(0x0109, v)}
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
            onChange={(v) => setP(0x010A + i, v)}
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
  setP,
  setNib2P,
}: {
  partial: PcmSynthPartial;
  setP: (offset: number, value: number) => void;
  setNib2P: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="OUTPUT" accentColor="#6c8">
      <div className={css.panelRow}>
        <SynthSwitch label="Env Mode" value={partial.envMode} vertical
          options={ENV_MODE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x08, v)} led={false}
          title="Envelope mode: NO-SUS plays once, SUSTAIN holds at sustain level" />
        <SynthSwitch label="Delay" value={partial.delayMode} vertical
          options={DELAY_MODE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x09, v)} led={false}
          title="Partial delay mode — when sound starts relative to note-on" />
        <SynthKnob label="Delay" value={partial.delayTime} min={0} max={149} defaultValue={0}
          onChange={(v) => setNib2P(0x0A, v)} formatValue={(v) => String(v)} color="#6c8"
          title="Delay time before this partial sounds" />
      </div>
      <div className={css.panelRow}>
        <SynthSwitch label="Bender" value={partial.receiveBender} options={ON_OFF_OPTIONS}
          onChange={(v) => setP(0x0F, v)}
          title="Respond to pitch bend" />
        <SynthSwitch label="Express" value={partial.receiveExpression} options={ON_OFF_OPTIONS}
          onChange={(v) => setP(0x10, v)}
          title="Respond to expression pedal (CC11)" />
        <SynthSwitch label="Hold" value={partial.receiveHold1} options={ON_OFF_OPTIONS}
          onChange={(v) => setP(0x11, v)}
          title="Respond to sustain pedal (CC64)" />
      </div>
      <OutputStrip
        fx1={partial.chorusSend}
        fx2={partial.reverbSend}
        level={partial.outputLevel}
        onFx1Change={(v) => setP(0x0D, v)}
        onFx2Change={(v) => setP(0x0E, v)}
        onLevelChange={(v) => setP(0x0C, v)}
        trackHeight={150}
      />
    </SectionPanel>
  );
}
