import {
  SynthKnob,
  SynthFader,
  SynthSwitch,
  SynthSelect,
  SectionPanel,
  ADSREnvelope,
  ADEnvelope,
  FaderGroup,
  FaderGroupSep,
} from "../synth-ui";
import css from "../SnSynthEditor.module.css";
import type { SnSynthPartial } from "./types";
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
  setNibP,
}: {
  idx: number;
  partial: SnSynthPartial | null;
  isOn: boolean;
  onToggle: () => void;
  setP: (offset: number, value: number) => void;
  setNibP: (offset: number, value: number) => void;
}) {
  const dimClass = isOn ? "" : css.partialRowDimmed;

  return (
    <div className={css.partialRow}>
      {/* Partial switch */}
      <div className={css.partialSwitch}>
        <button
          className={`${css.partialSwitchBtn} ${isOn ? css.partialSwitchBtnOn : ""}`}
          onClick={onToggle}
        >
          <span className={css.partialNum}>{idx + 1}</span>
          <span className={css.partialSwLabel}>{isOn ? "ON" : "OFF"}</span>
        </button>
      </div>

      {/* OSC */}
      <div className={dimClass}>
        {partial && <OscPanel partial={partial} onChange={setP} onNibChange={setNibP} />}
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

// ---------------------------------------------------------------------------
// OSC Panel
// ---------------------------------------------------------------------------

function OscPanel({
  partial,
  onChange,
  onNibChange,
}: {
  partial: SnSynthPartial;
  onChange: (offset: number, value: number) => void;
  onNibChange: (offset: number, value: number) => void;
}) {
  const hasVariation = partial.oscWave <= 5; // SAW..NOISE
  const isPcm = partial.oscWave === 7;
  const isSuperSaw = partial.oscWave === 6;

  return (
    <SectionPanel label="OSC" accentColor="#fc8">
      <div className={css.panelRow}>
        <div className={css.oscSelectors}>
          <SynthSwitch label="Wave" value={partial.oscWave} vertical
            options={OSC_WAVE_NAMES.map((l, i) => ({ value: i, label: l }))}
            onChange={(v) => onChange(0x00, v)} led={false} />
          {hasVariation && (
            <SynthSwitch label="Var" value={partial.oscWaveVariation} vertical
              options={OSC_VARIATION_NAMES.map((l, i) => ({ value: i, label: l }))}
              onChange={(v) => onChange(0x01, v)} led={false} />
          )}
          {isPcm && (
            <label className={css.selectLabel}>
              PCM #
              <input
                type="number"
                className={css.waveNumberInput}
                value={partial.waveNumber}
                min={0}
                max={16384}
                onChange={(e) => {
                  const v = Math.max(0, Math.min(16384, Number(e.target.value)));
                  onNibChange(0x35, v);
                }}
              />
            </label>
          )}
        </div>
        <div className={css.oscKnobGrid}>
          <SynthKnob label="Pitch" value={partial.oscPitch} min={40} max={88} defaultValue={64}
            onChange={(v) => onChange(0x03, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8" />
          <SynthKnob label="HPF" value={partial.hpfCutoff} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x39, v)} formatValue={(v) => String(v)} color="#fc8" />
          <SynthKnob label="Detune" value={partial.oscDetune} min={14} max={114} defaultValue={64}
            onChange={(v) => onChange(0x04, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8" />
          {isSuperSaw && (
            <SynthKnob label="S-Saw" value={partial.superSawDetune} min={0} max={127} defaultValue={0}
              onChange={(v) => onChange(0x3A, v)} formatValue={(v) => String(v)} color="#fc8" />
          )}
        </div>
      </div>
      <FaderGroup>
        <div className={css.faderWithCurveSpace}>
          <SynthFader label="PWM" value={partial.oscPwModDepth} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x05, v)} compact />
        </div>
        <div className={css.faderWithCurveSpace}>
          <SynthFader label="PW" value={partial.oscPulseWidth} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x06, v)} compact />
        </div>
        <FaderGroupSep />
        <ADEnvelope
          compact
          attack={{
            label: "A", value: partial.oscPitchEnvAttack, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onChange(0x07, v),
          }}
          decay={{
            label: "D", value: partial.oscPitchEnvDecay, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onChange(0x08, v),
          }}
          extra={{
            label: "Dep", value: partial.oscPitchEnvDepth, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => onChange(0x09, v), formatValue: (v) => signedFmt(v, 64),
          }}
        />
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
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="FILTER" accentColor="#68c">
      <div className={css.panelRow}>
        <SynthSwitch label="Mode" value={partial.filterMode} vertical
          options={FILTER_MODE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => onChange(0x0A, v)} />
        <SynthSwitch label="Slope" value={partial.filterSlope} vertical
          options={FILTER_SLOPE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => onChange(0x0B, v)} led={false} />
        <div className={`${css.oscKnobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Cutoff" value={partial.filterCutoff} min={0} max={127} defaultValue={127}
            onChange={(v) => onChange(0x0C, v)} formatValue={(v) => String(v)} color="#68c" />
          <SynthKnob label="Reso" value={partial.filterResonance} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x0F, v)} formatValue={(v) => String(v)} color="#68c" />
          <SynthKnob label="KeyF" value={partial.filterKeyfollow} min={54} max={74} defaultValue={64}
            onChange={(v) => onChange(0x0D, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c" />
          <SynthKnob label="Vel Sns" value={partial.filterEnvVelSens} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x0E, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c" />
        </div>
      </div>
      <ADSREnvelope
        compact
        attack={{
          label: "A", value: partial.filterEnvAttack, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onChange(0x10, v),
        }}
        decay={{
          label: "D", value: partial.filterEnvDecay, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onChange(0x11, v),
        }}
        sustain={{
          label: "S", value: partial.filterEnvSustain, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onChange(0x12, v),
        }}
        release={{
          label: "R", value: partial.filterEnvRelease, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onChange(0x13, v),
        }}
        extra={{
          label: "Dep", value: partial.filterEnvDepth, min: 1, max: 127, defaultValue: 64,
          onChange: (v) => onChange(0x14, v), formatValue: (v) => signedFmt(v, 64),
        }}
      />
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
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="AMP" accentColor="#6c8">
      <div className={css.panelRow}>
        <SynthSwitch label="Gain" value={partial.waveGain} vertical
          options={WAVE_GAIN_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => onChange(0x34, v)} led={false} />
        <div className={`${css.oscKnobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Level" value={partial.ampLevel} min={0} max={127} defaultValue={127}
            onChange={(v) => onChange(0x15, v)} formatValue={(v) => String(v)} color="#6c8" />
          <SynthKnob label="Pan" value={partial.ampPan} min={0} max={127} defaultValue={64}
            onChange={(v) => onChange(0x1B, v)} formatValue={panFmt} color="#6c8" />
          <SynthKnob label="Vel Sns" value={partial.ampVelSens} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x16, v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8" />
          <SynthKnob label="KeyF" value={partial.ampLevelKeyfollow} min={54} max={74} defaultValue={64}
            onChange={(v) => onChange(0x3C, v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8" />
        </div>
      </div>
      <ADSREnvelope
        compact
        attack={{
          label: "A", value: partial.ampEnvAttack, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onChange(0x17, v),
        }}
        decay={{
          label: "D", value: partial.ampEnvDecay, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onChange(0x18, v),
        }}
        sustain={{
          label: "S", value: partial.ampEnvSustain, min: 0, max: 127, defaultValue: 127,
          onChange: (v) => onChange(0x19, v),
        }}
        release={{
          label: "R", value: partial.ampEnvRelease, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onChange(0x1A, v),
        }}
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
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="MOD LFO" accentColor="#a6f">
      <div className={css.panelRow}>
        <SynthSwitch label="Shape" value={partial.modLfoShape} vertical
          options={LFO_SHAPE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => onChange(0x26, v)} led={false} />
        <div>
          <SynthSwitch label="T.Sync" value={partial.modLfoTempoSync} options={ON_OFF_OPTIONS}
            onChange={(v) => onChange(0x28, v)} />
          {partial.modLfoTempoSync !== 0 && (
            <SynthSelect label="Note" value={partial.modLfoTempoSyncNote}
              options={TEMPO_SYNC_NOTE_NAMES.map((l, i) => ({ value: i, label: l }))}
              onChange={(v) => onChange(0x29, v)} />
          )}
        </div>
        <div className={`${css.oscKnobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Rate" value={partial.modLfoRate} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x27, v)} formatValue={(v) => String(v)} color="#a6f" />
          <SynthKnob label="PW Shft" value={partial.pwShift} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x2A, v)} formatValue={(v) => String(v)} color="#a6f" />
          <SynthKnob label="Rate Ctrl" value={partial.modLfoRateControl} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x3B, v)} formatValue={(v) => signedFmt(v, 64)} color="#a6f" />
        </div>
      </div>
      <FaderGroup>
        <SynthFader label="Pit" value={partial.modLfoPitchDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => onChange(0x2C, v)} formatValue={(v) => signedFmt(v, 64)} compact />
        <SynthFader label="Flt" value={partial.modLfoFilterDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => onChange(0x2D, v)} formatValue={(v) => signedFmt(v, 64)} compact />
        <SynthFader label="Amp" value={partial.modLfoAmpDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => onChange(0x2E, v)} formatValue={(v) => signedFmt(v, 64)} compact />
        <SynthFader label="Pan" value={partial.modLfoPanDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => onChange(0x2F, v)} formatValue={(v) => signedFmt(v, 64)} compact />
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
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="LFO" accentColor="#a6f">
      <div className={css.panelRow}>
        <SynthSwitch label="Shape" value={partial.lfoShape} vertical
          options={LFO_SHAPE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => onChange(0x1C, v)} led={false} />
        <div>
          <SynthSwitch label="T.Sync" value={partial.lfoTempoSync} options={ON_OFF_OPTIONS}
            onChange={(v) => onChange(0x1E, v)} />
          {partial.lfoTempoSync !== 0 && (
            <SynthSelect label="Note" value={partial.lfoTempoSyncNote}
              options={TEMPO_SYNC_NOTE_NAMES.map((l, i) => ({ value: i, label: l }))}
              onChange={(v) => onChange(0x1F, v)} />
          )}
          <SynthSwitch label="KeyTrig" value={partial.lfoKeyTrigger} options={ON_OFF_OPTIONS}
            onChange={(v) => onChange(0x21, v)} />
        </div>
        <div className={`${css.oscKnobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Rate" value={partial.lfoRate} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x1D, v)} formatValue={(v) => String(v)} color="#a6f" />
          <SynthKnob label="Fade" value={partial.lfoFadeTime} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x20, v)} formatValue={(v) => String(v)} color="#a6f" />
          <SynthKnob label="Aft Cut" value={partial.aftertouchCutoff} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x30, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cc" />
          <SynthKnob label="Aft Lvl" value={partial.aftertouchLevel} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x31, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cc" />
        </div>
      </div>
      <FaderGroup>
          <SynthFader label="Pit" value={partial.lfoPitchDepth} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x22, v)} formatValue={(v) => signedFmt(v, 64)} compact />
          <SynthFader label="Flt" value={partial.lfoFilterDepth} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x23, v)} formatValue={(v) => signedFmt(v, 64)} compact />
          <SynthFader label="Amp" value={partial.lfoAmpDepth} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x24, v)} formatValue={(v) => signedFmt(v, 64)} compact />
          <SynthFader label="Pan" value={partial.lfoPanDepth} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x25, v)} formatValue={(v) => signedFmt(v, 64)} compact />
      </FaderGroup>
    </SectionPanel>
  );
}
