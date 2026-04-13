import { useState } from "react";
import {
  SynthKnob,
  SynthSwitch,
  SynthSelect,
  SectionPanel,
  ADSREnvelope,
  OutputStrip,
} from "../synth-ui";
import type { PcmDrumPartial } from "./types";
import {
  noteName,
  panFmt,
  signedFmt,
  linearToSysex,
  wmtFieldOffset,
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
import css from "../PcmDrumEditor.module.css";

// ---------------------------------------------------------------------------
// Per-key note controls
// ---------------------------------------------------------------------------

export function NoteControls({
  keyNumber,
  note,
  onParam,
  onNibParam,
}: {
  keyNumber: number;
  note: PcmDrumPartial;
  onParam: (offset: number, value: number) => void;
  onNibParam: (offset: number, value: number) => void;
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
          onNibParam={onNibParam}
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
  onNibParam,
}: {
  note: PcmDrumPartial;
  wmtTab: number;
  onWmtTab: (tab: number) => void;
  onParam: (offset: number, value: number) => void;
  onNibParam: (offset: number, value: number) => void;
}) {
  const wmt = note.wmt[wmtTab]!;
  const layerOffset = (fieldPos: number) => wmtFieldOffset(wmtTab, fieldPos);

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
        {[0, 1, 2, 3].map((i) => {
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
          onChange={(v) => onParam(layerOffset(0), v)}
          title="Wave layer switch" />
        <SynthSelect label="Group" value={wmt.waveGroupType}
          options={WAVE_GROUP_OPTIONS}
          onChange={(v) => onParam(layerOffset(1), v)} />
        <SynthSelect label="Gain" value={wmt.waveGain}
          options={WAVE_GAIN_OPTIONS}
          onChange={(v) => onParam(layerOffset(14), v)} />
      </div>

      <div className={css.wmtLayer}>
        <SynthKnob label="Wave L" value={wmt.waveNumberL} min={0} max={16384} defaultValue={0}
          onChange={(v) => onNibParam(layerOffset(6), v)}
          formatValue={(v) => String(v)} color="#fc8"
          title="Wave Number L / Mono" />
        <SynthKnob label="Wave R" value={wmt.waveNumberR} min={0} max={16384} defaultValue={0}
          onChange={(v) => onNibParam(layerOffset(10), v)}
          formatValue={(v) => String(v)} color="#fc8"
          title="Wave Number R" />
      </div>

      <div className={css.wmtLayer}>
        <SynthSwitch label="FXM" value={wmt.waveFxmSwitch}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={(v) => onParam(layerOffset(15), v)}
          title="FXM Switch" />
        <SynthKnob label="Color" value={wmt.waveFxmColor} min={0} max={3} defaultValue={0}
          onChange={(v) => onParam(layerOffset(16), v)}
          formatValue={(v) => String(v + 1)} color="#ea6"
          title="FXM Color (1-4)" />
        <SynthKnob label="Depth" value={wmt.waveFxmDepth} min={0} max={16} defaultValue={0}
          onChange={(v) => onParam(layerOffset(17), v)}
          formatValue={(v) => String(v)} color="#ea6"
          title="FXM Depth" />
      </div>

      <div className={css.wmtLayer}>
        <SynthKnob label="Coarse" value={wmt.coarseTune} min={16} max={112} defaultValue={64}
          onChange={(v) => onParam(layerOffset(19), v)}
          formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Coarse Tune (-48 to +48)" />
        <SynthKnob label="Fine" value={wmt.fineTune} min={14} max={114} defaultValue={64}
          onChange={(v) => onParam(layerOffset(20), v)}
          formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Fine Tune (-50 to +50)" />
        <SynthKnob label="Pan" value={wmt.pan} min={0} max={127} defaultValue={64}
          onChange={(v) => onParam(layerOffset(21), v)}
          formatValue={panFmt} color="#8cf"
          title="Wave Pan" />
        <SynthKnob label="Level" value={wmt.level} min={0} max={127} defaultValue={127}
          onChange={(v) => onParam(layerOffset(24), v)}
          formatValue={(v) => String(v)} color="#8cf"
          title="Wave Level" />
      </div>

      <div className={css.wmtLayer}>
        <SynthSwitch label="Rnd Pan" value={wmt.randomPanSwitch}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={(v) => onParam(layerOffset(22), v)}
          title="Random Pan Switch" />
        <SynthSelect label="Alt Pan" value={wmt.alternatePanSwitch}
          options={ALT_PAN_OPTIONS}
          onChange={(v) => onParam(layerOffset(23), v)} />
        <SynthSwitch label="T.Sync" value={wmt.waveTempoSync}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={(v) => onParam(layerOffset(18), v)}
          title="Tempo Sync" />
      </div>

      <div className={css.wmtLayer}>
        <SynthKnob label="Vel Lo" value={wmt.velocityRangeLower} min={1} max={127} defaultValue={1}
          onChange={(v) => onParam(layerOffset(25), v)}
          formatValue={(v) => String(v)} color="#a6f"
          title="Velocity Range Lower" />
        <SynthKnob label="Vel Hi" value={wmt.velocityRangeUpper} min={1} max={127} defaultValue={127}
          onChange={(v) => onParam(layerOffset(26), v)}
          formatValue={(v) => String(v)} color="#a6f"
          title="Velocity Range Upper" />
        <SynthKnob label="Fade Lo" value={wmt.velocityFadeLower} min={0} max={127} defaultValue={0}
          onChange={(v) => onParam(layerOffset(27), v)}
          formatValue={(v) => String(v)} color="#a6f"
          title="Velocity Fade Width Lower" />
        <SynthKnob label="Fade Hi" value={wmt.velocityFadeUpper} min={0} max={127} defaultValue={0}
          onChange={(v) => onParam(layerOffset(28), v)}
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
  const base = linearToSysex(149); // 0x0115
  const off = (rel: number) => base + rel;

  return (
    <SectionPanel label="PITCH ENV" accentColor="#fc8">
      <div className={css.knobRow}>
        <SynthKnob label="Depth" value={note.pitchEnvDepth} min={52} max={76} defaultValue={64}
          onChange={(v) => onParam(off(0), v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Pitch envelope depth (-12 to +12)" />
        <SynthKnob label="Vel Sns" value={note.pitchEnvVelocitySens} min={1} max={127} defaultValue={64}
          onChange={(v) => onParam(off(1), v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Pitch envelope velocity sensitivity" />
      </div>
      <ADSREnvelope
        compact
        attack={{
          label: "Atk", value: note.pitchEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(4), v),
        }}
        decay={{
          label: "Dec", value: note.pitchEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(5), v),
        }}
        sustain={{
          label: "Sus", value: note.pitchEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(6), v),
        }}
        release={{
          label: "Rel", value: note.pitchEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(7), v),
        }}
        levels={[
          { label: "Start", value: note.pitchEnvLevel[0] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => onParam(off(8), v), formatValue: (v) => signedFmt(v, 64) },
          { label: "Atk", value: note.pitchEnvLevel[1] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => onParam(off(9), v), formatValue: (v) => signedFmt(v, 64) },
          { label: "Dec", value: note.pitchEnvLevel[2] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => onParam(off(10), v), formatValue: (v) => signedFmt(v, 64) },
          { label: "Sus", value: note.pitchEnvLevel[3] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => onParam(off(11), v), formatValue: (v) => signedFmt(v, 64) },
          { label: "End", value: note.pitchEnvLevel[4] ?? 64, min: 1, max: 127, defaultValue: 64,
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
  const base = linearToSysex(162); // 0x0122
  const off = (rel: number) => base + rel;

  return (
    <SectionPanel label="FILTER" accentColor="#68c">
      <div className={css.panelRow}>
        <SynthSwitch label="Type" value={note.tvfFilterType} vertical
          options={FILTER_TYPE_OPTIONS}
          onChange={(v) => onParam(off(0), v)} title="Filter type" />
        <SynthKnob label="Cutoff" value={note.tvfCutoffFrequency} min={0} max={127} defaultValue={127}
          onChange={(v) => onParam(off(1), v)} formatValue={(v) => String(v)} color="#68c"
          title="Filter cutoff frequency" />
        <SynthKnob label="Reso" value={note.tvfResonance} min={0} max={127} defaultValue={0}
          onChange={(v) => onParam(off(4), v)} formatValue={(v) => String(v)} color="#68c"
          title="Filter resonance" />
      </div>
      <div className={css.knobRow}>
        <SynthKnob label="Vel Sns" value={note.tvfCutoffVelocitySens} min={1} max={127} defaultValue={64}
          onChange={(v) => onParam(off(3), v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Cutoff velocity sensitivity" />
        <SynthSelect label="Vel Crv" value={note.tvfCutoffVelocityCurve}
          options={VELOCITY_CURVE_OPTIONS}
          onChange={(v) => onParam(off(2), v)} />
        <SynthKnob label="Res Vel" value={note.tvfResonanceVelocitySens} min={1} max={127} defaultValue={64}
          onChange={(v) => onParam(off(5), v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Resonance velocity sensitivity" />
      </div>
      <div className={css.knobRow}>
        <SynthKnob label="Depth" value={note.tvfEnvDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => onParam(off(6), v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Filter envelope depth" />
        <SynthKnob label="Env Vel" value={note.tvfEnvVelocitySens} min={1} max={127} defaultValue={64}
          onChange={(v) => onParam(off(8), v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Envelope velocity sensitivity" />
        <SynthSelect label="Env Crv" value={note.tvfEnvVelocityCurve}
          options={VELOCITY_CURVE_OPTIONS}
          onChange={(v) => onParam(off(7), v)} />
      </div>
      <ADSREnvelope
        compact
        attack={{
          label: "Atk", value: note.tvfEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(11), v),
        }}
        decay={{
          label: "Dec", value: note.tvfEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(12), v),
        }}
        sustain={{
          label: "Sus", value: note.tvfEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(13), v),
        }}
        release={{
          label: "Rel", value: note.tvfEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(14), v),
        }}
        levels={[
          { label: "Start", value: note.tvfEnvLevel[0] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onParam(off(15), v) },
          { label: "Atk", value: note.tvfEnvLevel[1] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onParam(off(16), v) },
          { label: "Dec", value: note.tvfEnvLevel[2] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onParam(off(17), v) },
          { label: "Sus", value: note.tvfEnvLevel[3] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onParam(off(18), v) },
          { label: "End", value: note.tvfEnvLevel[4] ?? 0, min: 0, max: 127, defaultValue: 0,
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
  const base = linearToSysex(182); // 0x0136
  const off = (rel: number) => base + rel;

  return (
    <SectionPanel label="AMP" accentColor="#6c8">
      <div className={css.knobRow}>
        <SynthSelect label="Vel Crv" value={note.tvaLevelVelocityCurve}
          options={VELOCITY_CURVE_OPTIONS}
          onChange={(v) => onParam(off(0), v)} />
        <SynthKnob label="Vel Sns" value={note.tvaLevelVelocitySens} min={1} max={127} defaultValue={64}
          onChange={(v) => onParam(off(1), v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
          title="Amplitude velocity sensitivity" />
      </div>
      <ADSREnvelope
        compact
        attack={{
          label: "Atk", value: note.tvaEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(4), v),
        }}
        decay={{
          label: "Dec", value: note.tvaEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(5), v),
        }}
        sustain={{
          label: "Sus", value: note.tvaEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(6), v),
        }}
        release={{
          label: "Rel", value: note.tvaEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(7), v),
        }}
        levels={[
          { label: "", value: 127, min: 0, max: 127, defaultValue: 127, onChange: () => {}, hidden: true },
          { label: "Atk", value: note.tvaEnvLevel[0] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: (v) => onParam(off(8), v) },
          { label: "Dec", value: note.tvaEnvLevel[1] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: (v) => onParam(off(9), v) },
          { label: "Sus", value: note.tvaEnvLevel[2] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onParam(off(10), v) },
          { label: "", value: 0, min: 0, max: 127, defaultValue: 0, onChange: () => {}, hidden: true },
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
          onChange={(v) => onParam(linearToSysex(193), v)}
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
        <SynthKnob label="Fine" value={note.fineTune} min={14} max={114} defaultValue={64}
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
