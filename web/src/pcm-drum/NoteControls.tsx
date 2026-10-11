import { useState } from "react";
import {
  SynthKnob,
  SynthSwitch,
  SynthSelect,
  SectionPanel,
  BreakpointEnvelope,
  OutputStrip,
} from "../synth-ui";
import { PcmdWmtField } from "../../pkg/integral_wasm.js";
import type { PcmDrumPartial, PcmDrumWmt, WmtParamHandler } from "./types";
import {
  noteName,
  panFmt,
  signedFmt,
  WAVE_GAIN_OPTIONS,
  WAVE_GROUP_OPTIONS,
  FILTER_TYPE_OPTIONS,
  VELOCITY_CURVE_OPTIONS,
  ASSIGN_TYPE_OPTIONS,
  ENV_MODE_OPTIONS,
  OUTPUT_ASSIGN_OPTIONS,
  WMT_VELOCITY_CTRL_OPTIONS,
  ALT_PAN_OPTIONS,
} from "./types";
import { deviceSpec } from "../deviceSpec";
import css from "../PcmDrumEditor.module.css";

// ---------------------------------------------------------------------------
// Per-key note controls
// ---------------------------------------------------------------------------

export function NoteControls({
  keyNumber,
  note,
  onParam,
  onWmtParam,
}: {
  keyNumber: number;
  note: PcmDrumPartial;
  onParam: (offset: number, value: number) => void;
  onWmtParam: WmtParamHandler;
}) {
  const [wmtTab, setWmtTab] = useState(0);

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
          onChange={(v) => onParam(0x1B, v)} />
      </div>

      <div className={css.noteControls}>
        {/* WAVE (WMT) */}
        <WmtSection
          note={note}
          wmtTab={wmtTab}
          onWmtTab={setWmtTab}
          onParam={onParam}
          onWmtParam={onWmtParam}
        />

        {/* PITCH ENV */}
        <PitchEnvSection note={note} onParam={onParam} />

        {/* FILTER (TVF) */}
        <TvfSection note={note} onParam={onParam} />

        {/* AMP (TVA) */}
        <TvaSection note={note} onParam={onParam} />

        {/* MISC */}
        <MiscSection note={note} onParam={onParam} />

        {/* LEVEL / PAN */}
        <SectionPanel label="LEVEL / PAN" accentColor="#8cf">
          <OutputStrip
            pan={note.pan} onPanChange={(v) => onParam(0x12, v)} panFormat={panFmt}
            fx1={note.chorusSend} fx2={note.reverbSend}
            level={note.outputLevel}
            onFx1Change={(v) => onParam(0x19, v)}
            onFx2Change={(v) => onParam(0x1A, v)}
            onLevelChange={(v) => onParam(0x16, v)}
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
  onParam,
  onWmtParam,
}: {
  note: PcmDrumPartial;
  wmtTab: number;
  onWmtTab: (tab: number) => void;
  onParam: (offset: number, value: number) => void;
  onWmtParam: WmtParamHandler;
}) {
  const wmt = note.wmt[wmtTab]!;
  const set = (field: PcmdWmtField, key: keyof PcmDrumWmt) =>
    (v: number) => onWmtParam(wmtTab, field, key, v);

  return (
    <SectionPanel label="WAVE (WMT)" accentColor="#fc8">
      {/* Velocity Control */}
      <div className={css.knobRow}>
        <SynthSelect label="Vel Ctrl" value={note.wmtVelocityControl}
          options={WMT_VELOCITY_CTRL_OPTIONS}
          onChange={(v) => onParam(0x20, v)} />
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
          onChange={set(PcmdWmtField.Switch, "wmtSwitch")}
          title="Wave layer switch" />
        <SynthSelect label="Group" value={wmt.waveGroupType}
          options={WAVE_GROUP_OPTIONS}
          onChange={set(PcmdWmtField.WaveGroupType, "waveGroupType")} />
        <SynthSelect label="Gain" value={wmt.waveGain}
          options={WAVE_GAIN_OPTIONS}
          onChange={set(PcmdWmtField.WaveGain, "waveGain")} />
      </div>

      <div className={css.wmtLayer}>
        <SynthKnob label="Wave L" value={wmt.waveNumberL} min={0} max={16384} defaultValue={0}
          onChange={set(PcmdWmtField.WaveNumberL, "waveNumberL")}
          formatValue={(v) => String(v)} color="#fc8"
          title="Wave Number L / Mono" />
        <SynthKnob label="Wave R" value={wmt.waveNumberR} min={0} max={16384} defaultValue={0}
          onChange={set(PcmdWmtField.WaveNumberR, "waveNumberR")}
          formatValue={(v) => String(v)} color="#fc8"
          title="Wave Number R" />
      </div>

      <div className={css.wmtLayer}>
        <SynthSwitch label="FXM" value={wmt.waveFxmSwitch}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={set(PcmdWmtField.WaveFxmSwitch, "waveFxmSwitch")}
          title="FXM Switch" />
        <SynthKnob label="Color" value={wmt.waveFxmColor} min={0} max={3} defaultValue={0}
          onChange={set(PcmdWmtField.WaveFxmColor, "waveFxmColor")}
          formatValue={(v) => String(v + 1)} color="#ea6"
          title="FXM Color (1-4)" />
        <SynthKnob label="Depth" value={wmt.waveFxmDepth} min={0} max={16} defaultValue={0}
          onChange={set(PcmdWmtField.WaveFxmDepth, "waveFxmDepth")}
          formatValue={(v) => String(v)} color="#ea6"
          title="FXM Depth" />
      </div>

      <div className={css.wmtLayer}>
        <SynthKnob label="Coarse" value={wmt.coarseTune} min={16} max={112} defaultValue={64} center={64}
          onChange={set(PcmdWmtField.CoarseTune, "coarseTune")}
          formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Coarse Tune (-48 to +48)" />
        <SynthKnob label="Fine" value={wmt.fineTune} min={14} max={114} defaultValue={64} center={64}
          onChange={set(PcmdWmtField.FineTune, "fineTune")}
          formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Fine Tune (-50 to +50)" />
        <SynthKnob label="Pan" value={wmt.pan} min={0} max={127} defaultValue={64} center={64}
          onChange={set(PcmdWmtField.Pan, "pan")}
          formatValue={panFmt} color="#8cf"
          title="Wave Pan" />
        <SynthKnob label="Level" value={wmt.level} min={0} max={127} defaultValue={127}
          onChange={set(PcmdWmtField.Level, "level")}
          formatValue={(v) => String(v)} color="#8cf"
          title="Wave Level" />
      </div>

      <div className={css.wmtLayer}>
        <SynthSwitch label="Rnd Pan" value={wmt.randomPanSwitch}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={set(PcmdWmtField.RandomPanSwitch, "randomPanSwitch")}
          title="Random Pan Switch" />
        <SynthSelect label="Alt Pan" value={wmt.alternatePanSwitch}
          options={ALT_PAN_OPTIONS}
          onChange={set(PcmdWmtField.AlternatePanSwitch, "alternatePanSwitch")} />
        <SynthSwitch label="T.Sync" value={wmt.waveTempoSync}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={set(PcmdWmtField.WaveTempoSync, "waveTempoSync")}
          title="Tempo Sync" />
      </div>

      <div className={css.wmtLayer}>
        <SynthKnob label="Vel Lo" value={wmt.velocityRangeLower} min={1} max={127} defaultValue={1}
          onChange={set(PcmdWmtField.VelocityRangeLower, "velocityRangeLower")}
          formatValue={(v) => String(v)} color="#a6f"
          title="Velocity Range Lower" />
        <SynthKnob label="Vel Hi" value={wmt.velocityRangeUpper} min={1} max={127} defaultValue={127}
          onChange={set(PcmdWmtField.VelocityRangeUpper, "velocityRangeUpper")}
          formatValue={(v) => String(v)} color="#a6f"
          title="Velocity Range Upper" />
        <SynthKnob label="Fade Lo" value={wmt.velocityFadeLower} min={0} max={127} defaultValue={0}
          onChange={set(PcmdWmtField.VelocityFadeLower, "velocityFadeLower")}
          formatValue={(v) => String(v)} color="#a6f"
          title="Velocity Fade Width Lower" />
        <SynthKnob label="Fade Hi" value={wmt.velocityFadeUpper} min={0} max={127} defaultValue={0}
          onChange={set(PcmdWmtField.VelocityFadeUpper, "velocityFadeUpper")}
          formatValue={(v) => String(v)} color="#a6f"
          title="Velocity Fade Width Upper" />
      </div>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Pitch Envelope Section
// ---------------------------------------------------------------------------

function PitchEnvSection({
  note,
  onParam,
}: {
  note: PcmDrumPartial;
  onParam: (offset: number, value: number) => void;
}) {
  const base = 0x0115; // Pitch Env Depth
  const off = (rel: number) => base + rel;

  return (
    <SectionPanel label="PITCH ENV" accentColor="#fc8">
      <div className={css.knobRow}>
        <SynthKnob label="Depth" value={note.pitchEnvDepth} min={52} max={76} defaultValue={64} center={64}
          onChange={(v) => onParam(off(0), v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Pitch envelope depth (-12 to +12)" />
        <SynthKnob label="Vel Sns" value={note.pitchEnvVelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={(v) => onParam(off(1), v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Pitch envelope velocity sensitivity" />
        <SynthKnob label="T1 Vel" value={note.pitchEnvT1VelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={(v) => onParam(off(2), v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Pitch envelope Time 1 velocity sensitivity" />
        <SynthKnob label="T4 Vel" value={note.pitchEnvT4VelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={(v) => onParam(off(3), v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Pitch envelope Time 4 velocity sensitivity" />
      </div>
      <BreakpointEnvelope
        compact
        times={[{
          label: "T1", value: note.pitchEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(4), v),
        }, {
          label: "T2", value: note.pitchEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(5), v),
        }, {
          label: "T3", value: note.pitchEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(6), v),
        }, {
          label: "T4", value: note.pitchEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(7), v),
        }]}
        levels={[
          { label: "L0", value: note.pitchEnvLevel[0] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => onParam(off(8), v), formatValue: (v) => signedFmt(v, 64) },
          { label: "L1", value: note.pitchEnvLevel[1] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => onParam(off(9), v), formatValue: (v) => signedFmt(v, 64) },
          { label: "L2", value: note.pitchEnvLevel[2] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => onParam(off(10), v), formatValue: (v) => signedFmt(v, 64) },
          { label: "L3", value: note.pitchEnvLevel[3] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => onParam(off(11), v), formatValue: (v) => signedFmt(v, 64) },
          { label: "L4", value: note.pitchEnvLevel[4] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => onParam(off(12), v), formatValue: (v) => signedFmt(v, 64) },
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
  onParam,
}: {
  note: PcmDrumPartial;
  onParam: (offset: number, value: number) => void;
}) {
  const base = 0x0122; // TVF Filter Type
  const off = (rel: number) => base + rel;

  return (
    <SectionPanel label="FILTER" accentColor="#68c">
      <div className={css.filterGrid}>
        <SynthKnob label="Depth" value={note.tvfEnvDepth} min={1} max={127} defaultValue={64} center={64}
          onChange={(v) => onParam(off(6), v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Filter envelope depth" />
        <SynthKnob label="Vel Sns" value={note.tvfEnvVelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={(v) => onParam(off(8), v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Envelope velocity sensitivity" />
        <SynthKnob label="T1 Vel" value={note.tvfEnvT1VelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={(v) => onParam(off(9), v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Filter envelope Time 1 velocity sensitivity" />
        <SynthKnob label="T4 Vel" value={note.tvfEnvT4VelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={(v) => onParam(off(10), v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Filter envelope Time 4 velocity sensitivity" />
        <SynthSwitch label="Type" value={note.tvfFilterType} vertical
          options={FILTER_TYPE_OPTIONS}
          onChange={(v) => onParam(off(0), v)} title="Filter type" />
        <div className={css.stack}>
          <SynthSelect label="Vel Crv" value={note.tvfCutoffVelocityCurve}
            options={VELOCITY_CURVE_OPTIONS}
            onChange={(v) => onParam(off(2), v)} />
          <SynthSelect label="Env Crv" value={note.tvfEnvVelocityCurve}
            options={VELOCITY_CURVE_OPTIONS}
            onChange={(v) => onParam(off(7), v)} />
        </div>
        <div className={css.stack}>
          <SynthKnob label="Cutoff" value={note.tvfCutoffFrequency} min={0} max={127} defaultValue={127}
            onChange={(v) => onParam(off(1), v)} formatValue={(v) => String(v)} color="#68c"
            title="Filter cutoff frequency" />
          <SynthKnob label="Cut Vel" value={note.tvfCutoffVelocitySens} min={1} max={127} defaultValue={64} center={64}
            onChange={(v) => onParam(off(3), v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
            title="Cutoff velocity sensitivity" />
        </div>
        <div className={css.stack}>
          <SynthKnob label="Reso" value={note.tvfResonance} min={0} max={127} defaultValue={0}
            onChange={(v) => onParam(off(4), v)} formatValue={(v) => String(v)} color="#68c"
            title="Filter resonance" />
          <SynthKnob label="Res Vel" value={note.tvfResonanceVelocitySens} min={1} max={127} defaultValue={64} center={64}
            onChange={(v) => onParam(off(5), v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
            title="Resonance velocity sensitivity" />
        </div>
      </div>
      <BreakpointEnvelope
        compact
        times={[{
          label: "T1", value: note.tvfEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(11), v),
        }, {
          label: "T2", value: note.tvfEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(12), v),
        }, {
          label: "T3", value: note.tvfEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(13), v),
        }, {
          label: "T4", value: note.tvfEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(14), v),
        }]}
        levels={[
          { label: "L0", value: note.tvfEnvLevel[0] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onParam(off(15), v) },
          { label: "L1", value: note.tvfEnvLevel[1] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onParam(off(16), v) },
          { label: "L2", value: note.tvfEnvLevel[2] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onParam(off(17), v) },
          { label: "L3", value: note.tvfEnvLevel[3] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onParam(off(18), v) },
          { label: "L4", value: note.tvfEnvLevel[4] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onParam(off(19), v) },
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
  onParam,
}: {
  note: PcmDrumPartial;
  onParam: (offset: number, value: number) => void;
}) {
  const base = 0x0136; // TVA Level Velocity Curve
  const off = (rel: number) => base + rel;

  return (
    <SectionPanel label="AMP" accentColor="#6c8">
      <div className={css.knobRow}>
        <SynthSelect label="Vel Crv" value={note.tvaLevelVelocityCurve}
          options={VELOCITY_CURVE_OPTIONS}
          onChange={(v) => onParam(off(0), v)} />
        <SynthKnob label="Vel Sns" value={note.tvaLevelVelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={(v) => onParam(off(1), v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
          title="Amplitude velocity sensitivity" />
        <SynthKnob label="T1 Vel" value={note.tvaEnvT1VelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={(v) => onParam(off(2), v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
          title="Amp envelope Time 1 velocity sensitivity" />
        <SynthKnob label="T4 Vel" value={note.tvaEnvT4VelocitySens} min={1} max={127} defaultValue={64} center={64}
          onChange={(v) => onParam(off(3), v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
          title="Amp envelope Time 4 velocity sensitivity" />
      </div>
      <BreakpointEnvelope
        compact
        times={[{
          label: "T1", value: note.tvaEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(4), v),
        }, {
          label: "T2", value: note.tvaEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(5), v),
        }, {
          label: "T3", value: note.tvaEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(6), v),
        }, {
          label: "T4", value: note.tvaEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(7), v),
        }]}
        levels={[
          { at: 0, text: "0" },
          { label: "L1", value: note.tvaEnvLevel[0] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: (v) => onParam(off(8), v) },
          { label: "L2", value: note.tvaEnvLevel[1] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: (v) => onParam(off(9), v) },
          { label: "L3", value: note.tvaEnvLevel[2] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onParam(off(10), v) },
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
  onParam,
}: {
  note: PcmDrumPartial;
  onParam: (offset: number, value: number) => void;
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
          onChange={(v) => onParam(0x0C, v)} />
        <SynthSelect label="Mute Grp" value={note.muteGroup}
          options={muteGroupOptions}
          onChange={(v) => onParam(0x0D, v)} />
        <SynthSelect label="Env Mode" value={note.envMode}
          options={ENV_MODE_OPTIONS}
          onChange={(v) => onParam(0x15, v)} />
        <SynthSelect label="Output" value={note.outputAssign}
          options={OUTPUT_ASSIGN_OPTIONS}
          onChange={(v) => onParam(0x1B, v)} />
        <SynthSwitch label="1-Shot" value={note.oneShotMode}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={(v) => onParam(0x0141, v)}
          title="One Shot Mode" />
      </div>
      <div className={css.knobRow}>
        <SynthKnob label="PB Rng" value={note.pitchBendRange} min={0} max={48} defaultValue={0}
          onChange={(v) => onParam(0x1C, v)} formatValue={(v) => String(v)} color="#a6f"
          title="Pitch Bend Range" />
        <SynthKnob label="Level" value={note.level} min={0} max={127} defaultValue={127}
          onChange={(v) => onParam(0x0E, v)} formatValue={(v) => String(v)} color="#a6f"
          title="Partial Level" />
      </div>
      <div className={css.knobRow}>
        <SynthKnob label="Coarse" value={note.coarseTune} min={0} max={127} defaultValue={64}
          onChange={(v) => onParam(0x0F, v)} formatValue={(v) => noteName(v)} color="#fc8"
          title="Partial Coarse Tune (C-1 to G9)" />
        <SynthKnob label="Fine" value={note.fineTune} min={14} max={114} defaultValue={64} center={64}
          onChange={(v) => onParam(0x10, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Partial Fine Tune (-50 to +50)" />
        <SynthKnob label="Rnd Pit" value={note.randomPitchDepth} min={0} max={30} defaultValue={0}
          onChange={(v) => onParam(0x11, v)} formatValue={(v) => String(v)} color="#fc8"
          title="Random Pitch Depth" />
      </div>
      <div className={css.miscRow}>
        <SynthSwitch label="RxExpr" value={note.receiveExpression}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={(v) => onParam(0x1D, v)}
          title="Receive Expression" />
        <SynthSwitch label="RxHold" value={note.receiveHold1}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={(v) => onParam(0x1E, v)}
          title="Receive Hold-1" />
      </div>
    </SectionPanel>
  );
}
