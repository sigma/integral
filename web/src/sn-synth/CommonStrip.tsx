import {
  SynthKnob,
  SynthSwitch,
  SynthSelect,
} from "../synth-ui";
import css from "../SnSynthEditor.module.css";
import type { SnSynthCommon } from "./types";
import { ON_OFF_OPTIONS, UNISON_SIZE_NAMES, signedFmt } from "./types";

export function CommonStrip({
  common,
  onChange,
}: {
  common: SnSynthCommon;
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <div className={css.commonStrip}>
      {/* Unison group */}
      <div className={css.commonGroup}>
        <span className={css.groupLabel}>UNISON</span>
        <div className={css.groupRow}>
          <SynthSwitch label="SW" value={common.unisonSwitch} options={ON_OFF_OPTIONS}
            onChange={(v) => onChange(0x2E, v)} />
          <SynthKnob label="Size" value={common.unisonSize} min={0} max={3} defaultValue={0}
            onChange={(v) => onChange(0x3C, v)} formatValue={(v) => UNISON_SIZE_NAMES[v] ?? String(v)} color="#8cf" />
        </div>
      </div>

      <div className={css.commonDivider} />

      {/* Mono */}
      <div className={css.commonGroup}>
        <SynthSwitch label="MONO" value={common.monoSwitch} options={ON_OFF_OPTIONS}
          onChange={(v) => onChange(0x14, v)} />
      </div>

      <div className={css.commonDivider} />

      {/* Portamento group */}
      <div className={css.commonGroup}>
        <span className={css.groupLabel}>PORTAMENTO</span>
        <div className={css.groupRow}>
          <SynthSwitch label="SW" value={common.portamentoSwitch} options={ON_OFF_OPTIONS}
            onChange={(v) => onChange(0x12, v)} />
          <SynthKnob label="Time" value={common.portamentoTime} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x13, v)} formatValue={(v) => String(v)} color="#8cf" />
          <SynthSelect label="Mode" value={common.portamentoMode}
            options={[{ value: 0, label: "NORMAL" }, { value: 1, label: "LEGATO" }]}
            onChange={(v) => onChange(0x31, v)} />
        </div>
      </div>

      <div className={css.commonDivider} />

      {/* Bend Range */}
      <div className={css.commonGroup}>
        <span className={css.groupLabel}>BEND RANGE</span>
        <div className={css.groupRow}>
          <SynthKnob label="Down" value={common.pitchBendRangeDown} min={0} max={24} defaultValue={2}
            onChange={(v) => onChange(0x17, v)} formatValue={(v) => String(v)} color="#8cf" />
          <SynthKnob label="Up" value={common.pitchBendRangeUp} min={0} max={24} defaultValue={2}
            onChange={(v) => onChange(0x16, v)} formatValue={(v) => String(v)} color="#8cf" />
        </div>
      </div>

      <div className={css.commonDivider} />

      {/* Standalone params */}
      <SynthKnob label="Wave Shape" value={common.waveShape} min={0} max={127} defaultValue={0}
        onChange={(v) => onChange(0x35, v)} formatValue={(v) => String(v)} color="#8cf" />
      <SynthKnob label="Tone Level" value={common.toneLevel} min={0} max={127} defaultValue={127}
        onChange={(v) => onChange(0x0C, v)} formatValue={(v) => String(v)} color="#8cf" />
      <SynthKnob label="Analog Feel" value={common.analogFeel} min={0} max={127} defaultValue={0}
        onChange={(v) => onChange(0x34, v)} formatValue={(v) => String(v)} color="#8cf" />

      <div className={css.commonDivider} />

      <SynthSwitch label="LEGATO" value={common.legatoSwitch} options={ON_OFF_OPTIONS}
        onChange={(v) => onChange(0x32, v)} />

      <SynthKnob label="Oct Shift" value={common.octaveShift} min={61} max={67} defaultValue={64}
        onChange={(v) => onChange(0x15, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cf" />
    </div>
  );
}
