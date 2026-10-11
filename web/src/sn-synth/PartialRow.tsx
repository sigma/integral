import {
  SynthKnob,
  SynthFader,
  SynthSwitch,
  SynthSelect,
  SectionPanel,
  ADSREnvelope,
  FaderGroup,
  FaderGroupSep,
} from "../synth-ui";
import { SnsPartialField } from "../../pkg/integral_wasm.js";
import css from "../SnSynthEditor.module.css";
import type { PartialParamHandler, SnSynthPartial } from "./types";
import { WavePicker } from "../WavePicker";
import { snSynthWaveSource } from "../waveCatalog";
import {
  OSC_WAVE_NAMES,
  OSC_VARIATION_NAMES,
  FILTER_MODE_NAMES,
  FILTER_SLOPE_NAMES,
  LFO_SHAPE_NAMES,
  WAVE_GAIN_NAMES,
  TEMPO_SYNC_NOTE_NAMES,
  ON_OFF_OPTIONS,
  signedFmt,
  panFmt,
} from "./types";

// ---------------------------------------------------------------------------
// Partial Row -- renders SW + 5 section panels for one partial
// ---------------------------------------------------------------------------

export function PartialRow({
  idx,
  partial,
  isOn,
  onToggle,
  setP,
}: {
  idx: number;
  partial: SnSynthPartial | null;
  isOn: boolean;
  onToggle: () => void;
  setP: PartialParamHandler;
}) {
  const dimClass = isOn ? "" : css.partialRowDimmed;

  return (
    <div className={css.partialRow}>
      {/* Partial switch */}
      <div className={css.partialSwitch}>
        <button
          className={`${css.partialSwitchBtn} ${isOn ? css.partialSwitchBtnOn : ""}`}
          onClick={onToggle}
          title={isOn ? "Power off" : "Power on"}
        >
          <svg className={css.powerIcon} viewBox="0 0 24 24" fill="none" stroke="currentColor"
            strokeWidth="2.5" strokeLinecap="round">
            <path d="M12 3v9M6.3 6.3a8 8 0 1 0 11.4 0" />
          </svg>
          <span className={css.partialSwLabel}>Voice {idx + 1}</span>
        </button>
      </div>

      {/* OSC */}
      <div className={dimClass}>
        {partial && <OscPanel partial={partial} onChange={setP} />}
      </div>

      {/* Filter */}
      <div className={dimClass}>
        {partial && <FilterPanel partial={partial} onChange={setP} />}
      </div>

      {/* Amp */}
      <div className={dimClass}>
        {partial && <AmpPanel partial={partial} onChange={setP} />}
      </div>

      {/* Mod LFO */}
      <div className={dimClass}>
        {partial && <ModLfoPanel partial={partial} onChange={setP} />}
      </div>

      {/* LFO + Aftertouch */}
      <div className={dimClass}>
        {partial && <LfoPanel partial={partial} onChange={setP} />}
      </div>
    </div>
  );
}

/** The OSC's PCM Wave (SN Synth group), picked by name. */
export function OscPcmWave({
  waveNumber,
  onChange,
}: {
  waveNumber: number;
  onChange: PartialParamHandler;
}) {
  return (
    <WavePicker
      source={snSynthWaveSource()}
      stereo={false}
      l={waveNumber}
      onChange={(_side, n) => onChange(SnsPartialField.WaveNumber, n)}
    />
  );
}

// ---------------------------------------------------------------------------
// OSC Panel
// ---------------------------------------------------------------------------

function OscPanel({
  partial,
  onChange,
}: {
  partial: SnSynthPartial;
  onChange: PartialParamHandler;
}) {
  const hasVariation = partial.oscWave <= 5; // SAW..NOISE
  const isPcm = partial.oscWave === 7;
  const isSuperSaw = partial.oscWave === 6;

  return (
    <SectionPanel label="OSC" accentColor="#fc8">
      <div className={css.panelRow}>
        <div className={`${css.oscSelectors} ${isPcm ? css.oscSelectorsPcm : ""}`}>
          <SynthSwitch label="Wave" value={partial.oscWave} vertical
            options={OSC_WAVE_NAMES.map((l, i) => ({ value: i, label: l }))}
            onChange={(v) => onChange(SnsPartialField.OscWave, v)} led={false} />
          {hasVariation && (
            <SynthSwitch label="Var" value={partial.oscWaveVariation} vertical
              options={OSC_VARIATION_NAMES.map((l, i) => ({ value: i, label: l }))}
              onChange={(v) => onChange(SnsPartialField.OscWaveVariation, v)} led={false} />
          )}
          {isPcm && <OscPcmWave waveNumber={partial.waveNumber} onChange={onChange} />}
        </div>
        <div className={css.oscKnobGrid}>
          <SynthKnob label="Pitch" value={partial.oscPitch} min={40} max={88} defaultValue={64} center={64}
            onChange={(v) => onChange(SnsPartialField.OscPitch, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8" />
          <SynthKnob label="HPF" value={partial.hpfCutoff} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(SnsPartialField.HpfCutoff, v)} formatValue={(v) => String(v)} color="#fc8" />
          <SynthKnob label="Detune" value={partial.oscDetune} min={14} max={114} defaultValue={64} center={64}
            onChange={(v) => onChange(SnsPartialField.OscDetune, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8" />
          {isSuperSaw && (
            <SynthKnob label="S-Saw" value={partial.superSawDetune} min={0} max={127} defaultValue={0}
              onChange={(v) => onChange(SnsPartialField.SuperSawDetune, v)} formatValue={(v) => String(v)} color="#fc8" />
          )}
        </div>
      </div>
      <FaderGroup>
        <div className={css.faderWithCurveSpace}>
          <SynthFader label="PWM" value={partial.oscPwModDepth} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(SnsPartialField.OscPwModDepth, v)} compact />
        </div>
        <div className={css.faderWithCurveSpace}>
          <SynthFader label="PW" value={partial.oscPulseWidth} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(SnsPartialField.OscPulseWidth, v)} compact />
        </div>
        <FaderGroupSep />
        <ADSREnvelope
          compact
          attack={{ label: "A", value: partial.oscPitchEnvAttack, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onChange(SnsPartialField.OscPitchEnvAttack, v) }}
          decay={{ label: "D", value: partial.oscPitchEnvDecay, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onChange(SnsPartialField.OscPitchEnvDecay, v) }}
        />
        <FaderGroupSep />
        <div className={css.faderWithCurveSpace}>
          <SynthFader label="Dep" value={partial.oscPitchEnvDepth} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(SnsPartialField.OscPitchEnvDepth, v)} formatValue={(v) => signedFmt(v, 64)} compact />
        </div>
      </FaderGroup>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Filter Panel
// ---------------------------------------------------------------------------

function FilterPanel({
  partial,
  onChange,
}: {
  partial: SnSynthPartial;
  onChange: PartialParamHandler;
}) {
  return (
    <SectionPanel label="FILTER" accentColor="#68c">
      <div className={css.panelRow}>
        <SynthSwitch label="Mode" value={partial.filterMode} vertical
          options={FILTER_MODE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => onChange(SnsPartialField.FilterMode, v)} />
        <SynthSwitch label="Slope" value={partial.filterSlope} vertical
          options={FILTER_SLOPE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => onChange(SnsPartialField.FilterSlope, v)} led={false} />
        <div className={`${css.oscKnobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Cutoff" value={partial.filterCutoff} min={0} max={127} defaultValue={127}
            onChange={(v) => onChange(SnsPartialField.FilterCutoff, v)} formatValue={(v) => String(v)} color="#68c" />
          <SynthKnob label="Reso" value={partial.filterResonance} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(SnsPartialField.FilterResonance, v)} formatValue={(v) => String(v)} color="#68c" />
          <SynthKnob label="KeyF" value={partial.filterKeyfollow} min={54} max={74} defaultValue={64} center={64}
            onChange={(v) => onChange(SnsPartialField.FilterKeyfollow, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c" />
          <SynthKnob label="Vel Sns" value={partial.filterEnvVelSens} min={1} max={127} defaultValue={64} center={64}
            onChange={(v) => onChange(SnsPartialField.FilterEnvVelSens, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c" />
        </div>
      </div>
      <FaderGroup>
        <ADSREnvelope
          compact
          attack={{ label: "A", value: partial.filterEnvAttack, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onChange(SnsPartialField.FilterEnvAttack, v) }}
          decay={{ label: "D", value: partial.filterEnvDecay, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onChange(SnsPartialField.FilterEnvDecay, v) }}
          sustain={{ label: "S", value: partial.filterEnvSustain, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onChange(SnsPartialField.FilterEnvSustain, v) }}
          release={{ label: "R", value: partial.filterEnvRelease, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onChange(SnsPartialField.FilterEnvRelease, v) }}
        />
        <FaderGroupSep />
        <div className={css.faderWithCurveSpace}>
          <SynthFader label="Dep" value={partial.filterEnvDepth} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(SnsPartialField.FilterEnvDepth, v)} formatValue={(v) => signedFmt(v, 64)} compact />
        </div>
      </FaderGroup>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Amp Panel
// ---------------------------------------------------------------------------

function AmpPanel({
  partial,
  onChange,
}: {
  partial: SnSynthPartial;
  onChange: PartialParamHandler;
}) {
  return (
    <SectionPanel label="AMP" accentColor="#6c8">
      <div className={css.panelRow}>
        <SynthSwitch label="Gain" value={partial.waveGain} vertical
          options={WAVE_GAIN_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => onChange(SnsPartialField.WaveGain, v)} led={false} />
        <div className={`${css.oscKnobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Level" value={partial.ampLevel} min={0} max={127} defaultValue={127}
            onChange={(v) => onChange(SnsPartialField.AmpLevel, v)} formatValue={(v) => String(v)} color="#6c8" />
          <SynthKnob label="Pan" value={partial.ampPan} min={0} max={127} defaultValue={64} center={64}
            onChange={(v) => onChange(SnsPartialField.AmpPan, v)} formatValue={panFmt} color="#6c8" />
          <SynthKnob label="Vel Sns" value={partial.ampVelSens} min={1} max={127} defaultValue={64} center={64}
            onChange={(v) => onChange(SnsPartialField.AmpVelSens, v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8" />
          <SynthKnob label="KeyF" value={partial.ampLevelKeyfollow} min={54} max={74} defaultValue={64} center={64}
            onChange={(v) => onChange(SnsPartialField.AmpLevelKeyfollow, v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8" />
        </div>
      </div>
      <ADSREnvelope
        compact
        attack={{ label: "A", value: partial.ampEnvAttack, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onChange(SnsPartialField.AmpEnvAttack, v) }}
        decay={{ label: "D", value: partial.ampEnvDecay, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onChange(SnsPartialField.AmpEnvDecay, v) }}
        sustain={{ label: "S", value: partial.ampEnvSustain, min: 0, max: 127, defaultValue: 127,
          onChange: (v) => onChange(SnsPartialField.AmpEnvSustain, v) }}
        release={{ label: "R", value: partial.ampEnvRelease, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onChange(SnsPartialField.AmpEnvRelease, v) }}
      />
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Mod LFO Panel
// ---------------------------------------------------------------------------

function ModLfoPanel({
  partial,
  onChange,
}: {
  partial: SnSynthPartial;
  onChange: PartialParamHandler;
}) {
  return (
    <SectionPanel label="MOD LFO" accentColor="#a6f">
      <div className={css.panelRow}>
        <SynthSwitch label="Shape" value={partial.modLfoShape} vertical
          options={LFO_SHAPE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => onChange(SnsPartialField.ModLfoShape, v)} led={false} />
        <div>
          <SynthSwitch label="T.Sync" value={partial.modLfoTempoSync} options={ON_OFF_OPTIONS}
            onChange={(v) => onChange(SnsPartialField.ModLfoTempoSync, v)} />
          {partial.modLfoTempoSync !== 0 && (
            <SynthSelect label="Note" value={partial.modLfoTempoSyncNote}
              options={TEMPO_SYNC_NOTE_NAMES.map((l, i) => ({ value: i, label: l }))}
              onChange={(v) => onChange(SnsPartialField.ModLfoTempoSyncNote, v)} />
          )}
        </div>
        <div className={`${css.oscKnobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Rate" value={partial.modLfoRate} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(SnsPartialField.ModLfoRate, v)} formatValue={(v) => String(v)} color="#a6f" />
          <SynthKnob label="PW Shft" value={partial.pwShift} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(SnsPartialField.PwShift, v)} formatValue={(v) => String(v)} color="#a6f" />
          <SynthKnob label="Rate Ctrl" value={partial.modLfoRateControl} min={1} max={127} defaultValue={64} center={64}
            onChange={(v) => onChange(SnsPartialField.ModLfoRateControl, v)} formatValue={(v) => signedFmt(v, 64)} color="#a6f" />
        </div>
      </div>
      <FaderGroup>
        <SynthFader label="Pit" value={partial.modLfoPitchDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => onChange(SnsPartialField.ModLfoPitchDepth, v)} formatValue={(v) => signedFmt(v, 64)} compact />
        <SynthFader label="Flt" value={partial.modLfoFilterDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => onChange(SnsPartialField.ModLfoFilterDepth, v)} formatValue={(v) => signedFmt(v, 64)} compact />
        <SynthFader label="Amp" value={partial.modLfoAmpDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => onChange(SnsPartialField.ModLfoAmpDepth, v)} formatValue={(v) => signedFmt(v, 64)} compact />
        <SynthFader label="Pan" value={partial.modLfoPanDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => onChange(SnsPartialField.ModLfoPanDepth, v)} formatValue={(v) => signedFmt(v, 64)} compact />
      </FaderGroup>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// LFO Panel (includes Aftertouch at bottom)
// ---------------------------------------------------------------------------

function LfoPanel({
  partial,
  onChange,
}: {
  partial: SnSynthPartial;
  onChange: PartialParamHandler;
}) {
  return (
    <SectionPanel label="LFO" accentColor="#a6f">
      <div className={css.panelRow}>
        <SynthSwitch label="Shape" value={partial.lfoShape} vertical
          options={LFO_SHAPE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => onChange(SnsPartialField.LfoShape, v)} led={false} />
        <div>
          <SynthSwitch label="T.Sync" value={partial.lfoTempoSync} options={ON_OFF_OPTIONS}
            onChange={(v) => onChange(SnsPartialField.LfoTempoSync, v)} />
          {partial.lfoTempoSync !== 0 && (
            <SynthSelect label="Note" value={partial.lfoTempoSyncNote}
              options={TEMPO_SYNC_NOTE_NAMES.map((l, i) => ({ value: i, label: l }))}
              onChange={(v) => onChange(SnsPartialField.LfoTempoSyncNote, v)} />
          )}
          <SynthSwitch label="KeyTrig" value={partial.lfoKeyTrigger} options={ON_OFF_OPTIONS}
            onChange={(v) => onChange(SnsPartialField.LfoKeyTrigger, v)} />
        </div>
        <div className={`${css.oscKnobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Rate" value={partial.lfoRate} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(SnsPartialField.LfoRate, v)} formatValue={(v) => String(v)} color="#a6f" />
          <SynthKnob label="Fade" value={partial.lfoFadeTime} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(SnsPartialField.LfoFadeTime, v)} formatValue={(v) => String(v)} color="#a6f" />
          <SynthKnob label="Aft Cut" value={partial.aftertouchCutoff} min={1} max={127} defaultValue={64} center={64}
            onChange={(v) => onChange(SnsPartialField.AftertouchCutoff, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cc" />
          <SynthKnob label="Aft Lvl" value={partial.aftertouchLevel} min={1} max={127} defaultValue={64} center={64}
            onChange={(v) => onChange(SnsPartialField.AftertouchLevel, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cc" />
        </div>
      </div>
      <FaderGroup>
          <SynthFader label="Pit" value={partial.lfoPitchDepth} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(SnsPartialField.LfoPitchDepth, v)} formatValue={(v) => signedFmt(v, 64)} compact />
          <SynthFader label="Flt" value={partial.lfoFilterDepth} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(SnsPartialField.LfoFilterDepth, v)} formatValue={(v) => signedFmt(v, 64)} compact />
          <SynthFader label="Amp" value={partial.lfoAmpDepth} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(SnsPartialField.LfoAmpDepth, v)} formatValue={(v) => signedFmt(v, 64)} compact />
          <SynthFader label="Pan" value={partial.lfoPanDepth} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(SnsPartialField.LfoPanDepth, v)} formatValue={(v) => signedFmt(v, 64)} compact />
      </FaderGroup>
    </SectionPanel>
  );
}
