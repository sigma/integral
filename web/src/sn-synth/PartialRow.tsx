import {
  SynthKnob,
  SynthFader,
  SynthSwitch,
  SectionPanel,
  PartialPowerButton,
  ADSREnvelope,
  FaderGroup,
  FaderGroupSep,
} from "../synth-ui";
import {
  SnsPartialField,
  sns_osc_wave_pcm,
  sns_osc_wave_super_saw,
  sns_partial_spec,
} from "../../pkg/integral_wasm.js";
import css from "../SnSynthEditor.module.css";
import type { PartialParamHandler, SnSynthPartial } from "./types";
import { WavePicker } from "../WavePicker";
import { OscWaveScope } from "./OscWaveScope";
import { LfoScope } from "../synth-ui/LfoScope";
import { SyncedRateKnob } from "./SyncedRateKnob";
import { snSynthWaveSource } from "../waveCatalog";
import { knobRange, switchOptions } from "../fieldSpec";

/** The display name of `value` of a Partial field, as core formats it. */
const nameOf = (field: SnsPartialField, value: number) => sns_partial_spec(field).formatValue(value);

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
      <PartialPowerButton label={`Partial ${idx + 1}`} isOn={isOn} onToggle={onToggle} />

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

      {/* LFO + Aftertouch: the row's end, where its arrow into the MFX leaves */}
      <div className={dimClass} data-node={`out-${idx}`}>
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
  const wave = nameOf(SnsPartialField.OscWave, partial.oscWave);
  const isPcm = partial.oscWave === sns_osc_wave_pcm();
  const isSuperSaw = partial.oscWave === sns_osc_wave_super_saw();
  const hasVariation = !isPcm && !isSuperSaw;

  return (
    <SectionPanel label="OSC" accentColor="#fc8">
      <div className={css.panelRow}>
        <div className={css.oscSelectors}>
          <SynthSwitch label="Wave" value={partial.oscWave} vertical
            options={switchOptions(sns_partial_spec(SnsPartialField.OscWave))}
            onChange={(v) => onChange(SnsPartialField.OscWave, v)} led={false} />
          {hasVariation && (
            <SynthSwitch label="Var" value={partial.oscWaveVariation} vertical
              options={switchOptions(sns_partial_spec(SnsPartialField.OscWaveVariation))}
              onChange={(v) => onChange(SnsPartialField.OscWaveVariation, v)} led={false} />
          )}
        </div>
        <div className={`${css.scopeKnobArea} ${css.oscKnobArea}`}>
          {isPcm ? (
            <div className={css.oscPcmSlot}>
              <OscPcmWave waveNumber={partial.waveNumber} onChange={onChange} />
            </div>
          ) : (
            <OscWaveScope wave={wave}
              pulseWidth={partial.oscPulseWidth} pwmDepth={partial.oscPwModDepth}
              pwShift={partial.pwShift} superSawDetune={partial.superSawDetune} />
          )}
          <div className={css.scopeKnobRow}>
            <SynthKnob label="Pitch" value={partial.oscPitch} {...knobRange(sns_partial_spec(SnsPartialField.OscPitch))}
              onChange={(v) => onChange(SnsPartialField.OscPitch, v)} color="#fc8" />
            <SynthKnob label="HPF" value={partial.hpfCutoff} {...knobRange(sns_partial_spec(SnsPartialField.HpfCutoff))}
              onChange={(v) => onChange(SnsPartialField.HpfCutoff, v)} color="#fc8" />
            <SynthKnob label="Detune" value={partial.oscDetune} {...knobRange(sns_partial_spec(SnsPartialField.OscDetune))}
              onChange={(v) => onChange(SnsPartialField.OscDetune, v)} color="#fc8" />
            {/* Hidden, not removed, so it keeps its room and the panel's width */}
            <div className={isSuperSaw ? undefined : css.hiddenKnob}>
              <SynthKnob label="S-Saw" value={partial.superSawDetune} {...knobRange(sns_partial_spec(SnsPartialField.SuperSawDetune))}
                onChange={(v) => onChange(SnsPartialField.SuperSawDetune, v)} color="#fc8" />
            </div>
          </div>
        </div>
      </div>
      <FaderGroup>
        <SynthFader label="PWM" value={partial.oscPwModDepth} {...knobRange(sns_partial_spec(SnsPartialField.OscPwModDepth))}
          onChange={(v) => onChange(SnsPartialField.OscPwModDepth, v)} compact />
        <SynthFader label="PW" value={partial.oscPulseWidth} {...knobRange(sns_partial_spec(SnsPartialField.OscPulseWidth))}
          onChange={(v) => onChange(SnsPartialField.OscPulseWidth, v)} compact />
        <SynthFader label="Shift" value={partial.pwShift} {...knobRange(sns_partial_spec(SnsPartialField.PwShift))}
          onChange={(v) => onChange(SnsPartialField.PwShift, v)} compact />
        <FaderGroupSep />
        <ADSREnvelope
          compact
          attack={{ label: "A", value: partial.oscPitchEnvAttack,
            onChange: (v) => onChange(SnsPartialField.OscPitchEnvAttack, v), ...knobRange(sns_partial_spec(SnsPartialField.OscPitchEnvAttack)) }}
          decay={{ label: "D", value: partial.oscPitchEnvDecay,
            onChange: (v) => onChange(SnsPartialField.OscPitchEnvDecay, v), ...knobRange(sns_partial_spec(SnsPartialField.OscPitchEnvDecay)) }}
        />
        <FaderGroupSep />
        <SynthFader label="Dep" value={partial.oscPitchEnvDepth} {...knobRange(sns_partial_spec(SnsPartialField.OscPitchEnvDepth))}
          onChange={(v) => onChange(SnsPartialField.OscPitchEnvDepth, v)} compact />
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
          options={switchOptions(sns_partial_spec(SnsPartialField.FilterMode))}
          onChange={(v) => onChange(SnsPartialField.FilterMode, v)} />
        <SynthSwitch label="Slope" value={partial.filterSlope} vertical
          options={switchOptions(sns_partial_spec(SnsPartialField.FilterSlope))}
          onChange={(v) => onChange(SnsPartialField.FilterSlope, v)} led={false} />
        <div className={`${css.oscKnobGrid} ${css.knobGrid3} ${css.knobsRight}`}>
          <SynthKnob label="Cutoff" value={partial.filterCutoff} {...knobRange(sns_partial_spec(SnsPartialField.FilterCutoff))}
            onChange={(v) => onChange(SnsPartialField.FilterCutoff, v)} color="#68c" />
          <SynthKnob label="Reso" value={partial.filterResonance} {...knobRange(sns_partial_spec(SnsPartialField.FilterResonance))}
            onChange={(v) => onChange(SnsPartialField.FilterResonance, v)} color="#68c" />
          <SynthKnob label="KeyF" value={partial.filterKeyfollow} {...knobRange(sns_partial_spec(SnsPartialField.FilterKeyfollow))}
            onChange={(v) => onChange(SnsPartialField.FilterKeyfollow, v)} color="#68c" />
          <SynthKnob label="Vel Sns" value={partial.filterEnvVelSens} {...knobRange(sns_partial_spec(SnsPartialField.FilterEnvVelSens))}
            onChange={(v) => onChange(SnsPartialField.FilterEnvVelSens, v)} color="#68c" />
          <SynthKnob label="Aft Cut" value={partial.aftertouchCutoff} {...knobRange(sns_partial_spec(SnsPartialField.AftertouchCutoff))}
            onChange={(v) => onChange(SnsPartialField.AftertouchCutoff, v)} color="#68c" />
        </div>
      </div>
      <FaderGroup>
        <ADSREnvelope
          compact
          attack={{ label: "A", value: partial.filterEnvAttack,
            onChange: (v) => onChange(SnsPartialField.FilterEnvAttack, v), ...knobRange(sns_partial_spec(SnsPartialField.FilterEnvAttack)) }}
          decay={{ label: "D", value: partial.filterEnvDecay,
            onChange: (v) => onChange(SnsPartialField.FilterEnvDecay, v), ...knobRange(sns_partial_spec(SnsPartialField.FilterEnvDecay)) }}
          sustain={{ label: "S", value: partial.filterEnvSustain,
            onChange: (v) => onChange(SnsPartialField.FilterEnvSustain, v), ...knobRange(sns_partial_spec(SnsPartialField.FilterEnvSustain)) }}
          release={{ label: "R", value: partial.filterEnvRelease,
            onChange: (v) => onChange(SnsPartialField.FilterEnvRelease, v), ...knobRange(sns_partial_spec(SnsPartialField.FilterEnvRelease)) }}
        />
        <FaderGroupSep />
        <SynthFader label="Dep" value={partial.filterEnvDepth} {...knobRange(sns_partial_spec(SnsPartialField.FilterEnvDepth))}
          onChange={(v) => onChange(SnsPartialField.FilterEnvDepth, v)} compact />
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
          options={switchOptions(sns_partial_spec(SnsPartialField.WaveGain))}
          onChange={(v) => onChange(SnsPartialField.WaveGain, v)} led={false} />
        <div className={`${css.oscKnobGrid} ${css.knobGrid3} ${css.knobsRight}`}>
          <SynthKnob label="Level" value={partial.ampLevel} {...knobRange(sns_partial_spec(SnsPartialField.AmpLevel))}
            onChange={(v) => onChange(SnsPartialField.AmpLevel, v)} color="#6c8" />
          <SynthKnob label="Pan" value={partial.ampPan} {...knobRange(sns_partial_spec(SnsPartialField.AmpPan))}
            onChange={(v) => onChange(SnsPartialField.AmpPan, v)} color="#6c8" />
          <SynthKnob label="Vel Sns" value={partial.ampVelSens} {...knobRange(sns_partial_spec(SnsPartialField.AmpVelSens))}
            onChange={(v) => onChange(SnsPartialField.AmpVelSens, v)} color="#6c8" />
          <SynthKnob label="Aft Lvl" value={partial.aftertouchLevel} {...knobRange(sns_partial_spec(SnsPartialField.AftertouchLevel))}
            onChange={(v) => onChange(SnsPartialField.AftertouchLevel, v)} color="#6c8" />
          <SynthKnob label="KeyF" value={partial.ampLevelKeyfollow} {...knobRange(sns_partial_spec(SnsPartialField.AmpLevelKeyfollow))}
            onChange={(v) => onChange(SnsPartialField.AmpLevelKeyfollow, v)} color="#6c8" />
        </div>
      </div>
      <ADSREnvelope
        compact
        attack={{ label: "A", value: partial.ampEnvAttack,
          onChange: (v) => onChange(SnsPartialField.AmpEnvAttack, v), ...knobRange(sns_partial_spec(SnsPartialField.AmpEnvAttack)) }}
        decay={{ label: "D", value: partial.ampEnvDecay,
          onChange: (v) => onChange(SnsPartialField.AmpEnvDecay, v), ...knobRange(sns_partial_spec(SnsPartialField.AmpEnvDecay)) }}
        sustain={{ label: "S", value: partial.ampEnvSustain,
          onChange: (v) => onChange(SnsPartialField.AmpEnvSustain, v), ...knobRange(sns_partial_spec(SnsPartialField.AmpEnvSustain)) }}
        release={{ label: "R", value: partial.ampEnvRelease,
          onChange: (v) => onChange(SnsPartialField.AmpEnvRelease, v), ...knobRange(sns_partial_spec(SnsPartialField.AmpEnvRelease)) }}
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
        <div className={css.switchColumn}>
          <SynthSwitch label="Shape" value={partial.modLfoShape} vertical
            options={switchOptions(sns_partial_spec(SnsPartialField.ModLfoShape))}
            onChange={(v) => onChange(SnsPartialField.ModLfoShape, v)} led={false} />
        </div>
        <div className={css.scopeKnobArea}>
          <LfoScope shape={nameOf(SnsPartialField.ModLfoShape, partial.modLfoShape)} rate={partial.modLfoRate} />
          <div className={css.scopeKnobRow}>
            <SyncedRateKnob label="Rate" field={SnsPartialField.ModLfoRate} rate={partial.modLfoRate}
              tempoSync={partial.modLfoTempoSync} note={partial.modLfoTempoSyncNote} onChange={onChange} color="#a6f" />
            <SynthKnob label="Rate Ctrl" value={partial.modLfoRateControl} {...knobRange(sns_partial_spec(SnsPartialField.ModLfoRateControl))}
              onChange={(v) => onChange(SnsPartialField.ModLfoRateControl, v)} color="#a6f" />
          </div>
        </div>
      </div>
      <FaderGroup>
        <SynthFader label="Pit" value={partial.modLfoPitchDepth} {...knobRange(sns_partial_spec(SnsPartialField.ModLfoPitchDepth))}
          onChange={(v) => onChange(SnsPartialField.ModLfoPitchDepth, v)} compact />
        <SynthFader label="Flt" value={partial.modLfoFilterDepth} {...knobRange(sns_partial_spec(SnsPartialField.ModLfoFilterDepth))}
          onChange={(v) => onChange(SnsPartialField.ModLfoFilterDepth, v)} compact />
        <SynthFader label="Amp" value={partial.modLfoAmpDepth} {...knobRange(sns_partial_spec(SnsPartialField.ModLfoAmpDepth))}
          onChange={(v) => onChange(SnsPartialField.ModLfoAmpDepth, v)} compact />
        <SynthFader label="Pan" value={partial.modLfoPanDepth} {...knobRange(sns_partial_spec(SnsPartialField.ModLfoPanDepth))}
          onChange={(v) => onChange(SnsPartialField.ModLfoPanDepth, v)} compact />
      </FaderGroup>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// LFO Panel
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
        <div className={css.switchColumn}>
          <SynthSwitch label="Shape" value={partial.lfoShape} vertical
            options={switchOptions(sns_partial_spec(SnsPartialField.LfoShape))}
            onChange={(v) => onChange(SnsPartialField.LfoShape, v)} led={false} />
          <div>
            <SynthSwitch label="KeyTrig" value={partial.lfoKeyTrigger} options={switchOptions(sns_partial_spec(SnsPartialField.LfoKeyTrigger))}
              onChange={(v) => onChange(SnsPartialField.LfoKeyTrigger, v)} />
          </div>
        </div>
        <div className={css.scopeKnobArea}>
          <LfoScope shape={nameOf(SnsPartialField.LfoShape, partial.lfoShape)} rate={partial.lfoRate} />
          <div className={css.scopeKnobRow}>
            <SyncedRateKnob label="Rate" field={SnsPartialField.LfoRate} rate={partial.lfoRate}
              tempoSync={partial.lfoTempoSync} note={partial.lfoTempoSyncNote} onChange={onChange} color="#a6f" />
            <SynthKnob label="Fade" value={partial.lfoFadeTime} {...knobRange(sns_partial_spec(SnsPartialField.LfoFadeTime))}
              onChange={(v) => onChange(SnsPartialField.LfoFadeTime, v)} color="#a6f" />
          </div>
        </div>
      </div>
      <FaderGroup>
          <SynthFader label="Pit" value={partial.lfoPitchDepth} {...knobRange(sns_partial_spec(SnsPartialField.LfoPitchDepth))}
            onChange={(v) => onChange(SnsPartialField.LfoPitchDepth, v)} compact />
          <SynthFader label="Flt" value={partial.lfoFilterDepth} {...knobRange(sns_partial_spec(SnsPartialField.LfoFilterDepth))}
            onChange={(v) => onChange(SnsPartialField.LfoFilterDepth, v)} compact />
          <SynthFader label="Amp" value={partial.lfoAmpDepth} {...knobRange(sns_partial_spec(SnsPartialField.LfoAmpDepth))}
            onChange={(v) => onChange(SnsPartialField.LfoAmpDepth, v)} compact />
          <SynthFader label="Pan" value={partial.lfoPanDepth} {...knobRange(sns_partial_spec(SnsPartialField.LfoPanDepth))}
            onChange={(v) => onChange(SnsPartialField.LfoPanDepth, v)} compact />
      </FaderGroup>
    </SectionPanel>
  );
}
