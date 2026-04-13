import {
  SynthKnob,
  SynthSwitch,
} from "../synth-ui";
import css from "../PcmSynthEditor.module.css";
import type { PcmSynthCommon } from "./types";
import { ON_OFF_OPTIONS, signedFmt, panFmt } from "./types";

export function CommonStrip({
  common,
  onChange,
}: {
  common: PcmSynthCommon;
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <div className={css.commonStrip}>
      {/* Mono/Poly */}
      <div className={css.commonGroup}>
        <SynthSwitch label="MONO" value={common.monoPoly}
          options={[{ value: 0, label: "MONO" }, { value: 1, label: "POLY" }, { value: 2, label: "MONO2" }]}
          onChange={(v) => onChange(0x16, v)} vertical title="Voice mode: monophonic, polyphonic, or mono with legato" />
      </div>

      <div className={css.commonDivider} />

      {/* Portamento */}
      <div className={css.commonGroup}>
        <span className={css.groupLabel}>PORTAMENTO</span>
        <div className={css.groupRow}>
          <SynthSwitch label="SW" value={common.portamentoSwitch} options={ON_OFF_OPTIONS}
            onChange={(v) => onChange(0x19, v)} title="Portamento on/off — smooth pitch glide between notes" />
          <SynthKnob label="Time" value={common.portamentoTime} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x1D, v)} formatValue={(v) => String(v)} color="#8cf"
            title="Portamento time — speed of pitch glide" />
        </div>
      </div>

      <div className={css.commonDivider} />

      {/* Bend Range */}
      <div className={css.commonGroup}>
        <span className={css.groupLabel}>BEND RANGE</span>
        <div className={css.groupRow}>
          <SynthKnob label="Down" value={common.pitchBendRangeDown} min={0} max={24} defaultValue={2}
            onChange={(v) => onChange(0x25, v)} formatValue={(v) => String(v)} color="#8cf"
            title="Pitch bend range down (semitones)" />
          <SynthKnob label="Up" value={common.pitchBendRangeUp} min={0} max={24} defaultValue={2}
            onChange={(v) => onChange(0x24, v)} formatValue={(v) => String(v)} color="#8cf"
            title="Pitch bend range up (semitones)" />
        </div>
      </div>

      <div className={css.commonDivider} />

      {/* Standalone params */}
      <SynthKnob label="Level" value={common.toneLevel} min={0} max={127} defaultValue={127}
        onChange={(v) => onChange(0x0E, v)} formatValue={(v) => String(v)} color="#8cf"
        title="Overall tone volume" />
      <SynthKnob label="Pan" value={common.tonePan} min={0} max={127} defaultValue={64}
        onChange={(v) => onChange(0x0F, v)} formatValue={panFmt} color="#8cf"
        title="Stereo panning position" />
      <SynthKnob label="Coarse" value={common.coarseTune} min={16} max={112} defaultValue={64}
        onChange={(v) => onChange(0x11, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cf"
        title="Coarse tuning (semitones)" />
      <SynthKnob label="Fine" value={common.fineTune} min={14} max={114} defaultValue={64}
        onChange={(v) => onChange(0x12, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cf"
        title="Fine tuning (cents)" />

      <div className={css.commonDivider} />

      <SynthKnob label="Octave" value={common.octaveShift} min={61} max={67} defaultValue={64}
        onChange={(v) => onChange(0x13, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cf"
        title="Octave shift (-3 to +3)" />
      <SynthKnob label="Analog" value={common.analogFeel} min={0} max={127} defaultValue={0}
        onChange={(v) => onChange(0x15, v)} formatValue={(v) => String(v)} color="#8cf"
        title="Analog Feel — adds subtle pitch/timing randomness like vintage synths" />

      <div className={css.commonDivider} />

      <SynthSwitch label="LEGATO" value={common.legatoSwitch} options={ON_OFF_OPTIONS}
        onChange={(v) => onChange(0x17, v)} title="Legato — overlapping notes share the same voice" />

      <div className={css.commonDivider} />

      {/* Offsets */}
      <div className={css.commonGroup}>
        <span className={css.groupLabel}>OFFSETS</span>
        <div className={css.groupRow}>
          <SynthKnob label="Cutoff" value={common.cutoffOffset} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x1E, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
            title="Filter cutoff offset — shifts cutoff across all partials" />
          <SynthKnob label="Reso" value={common.resonanceOffset} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x1F, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
            title="Resonance offset — shifts resonance across all partials" />
          <SynthKnob label="Attack" value={common.attackTimeOffset} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x20, v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
            title="Attack time offset — shifts envelope attack across all partials" />
          <SynthKnob label="Release" value={common.releaseTimeOffset} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x21, v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
            title="Release time offset — shifts envelope release across all partials" />
          <SynthKnob label="Velocity" value={common.velocitySensOffset} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x22, v)} formatValue={(v) => signedFmt(v, 64)} color="#c8a"
            title="Velocity sensitivity offset — shifts velocity response across all partials" />
        </div>
      </div>
    </div>
  );
}
