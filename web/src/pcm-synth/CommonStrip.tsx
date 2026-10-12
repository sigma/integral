import {
  SynthKnob,
  SynthSwitch,
} from "../synth-ui";
import { PcmsCommonField, pcms_common_spec } from "../../pkg/integral_wasm.js";
import { knobRange, switchOptions } from "../fieldSpec";
import css from "../PcmSynthEditor.module.css";
import type { CommonParamHandler, PcmSynthCommon } from "./types";

/** The tone-wide settings, laid out inside the editor's top strip. */
export function CommonStrip({
  common,
  onChange,
}: {
  common: PcmSynthCommon;
  onChange: CommonParamHandler;
}) {
  const set =
    (field: PcmsCommonField, key: Parameters<CommonParamHandler>[1]) =>
    (v: number) =>
      onChange(field, key, v);
  return (
    <>
      {/* Mono/Poly */}
      <div className={css.commonGroup}>
        <SynthSwitch label="MONO" value={common.monoPoly}
          options={switchOptions(pcms_common_spec(PcmsCommonField.MonoPoly))}
          onChange={set(PcmsCommonField.MonoPoly, "monoPoly")} vertical title="Voice mode: monophonic or polyphonic" />
      </div>

      <div className={css.commonDivider} />

      {/* Portamento */}
      <div className={css.commonGroup}>
        <span className={css.groupLabel}>PORTAMENTO</span>
        <div className={css.groupRow}>
          <SynthSwitch label="SW" value={common.portamentoSwitch} options={switchOptions(pcms_common_spec(PcmsCommonField.PortamentoSwitch))}
            onChange={set(PcmsCommonField.PortamentoSwitch, "portamentoSwitch")} title="Portamento on/off — smooth pitch glide between notes" />
          <SynthKnob label="Time" value={common.portamentoTime} {...knobRange(pcms_common_spec(PcmsCommonField.PortamentoTime))}
            onChange={set(PcmsCommonField.PortamentoTime, "portamentoTime")} color="#8cf"
            title="Portamento time — speed of pitch glide" />
        </div>
      </div>

      <div className={css.commonDivider} />

      {/* Bend Range */}
      <div className={css.commonGroup}>
        <span className={css.groupLabel}>BEND RANGE</span>
        <div className={css.groupRow}>
          <SynthKnob label="Down" value={common.pitchBendRangeDown} {...knobRange(pcms_common_spec(PcmsCommonField.PitchBendRangeDown))}
            onChange={set(PcmsCommonField.PitchBendRangeDown, "pitchBendRangeDown")} color="#8cf"
            title="Pitch bend range down (semitones)" />
          <SynthKnob label="Up" value={common.pitchBendRangeUp} {...knobRange(pcms_common_spec(PcmsCommonField.PitchBendRangeUp))}
            onChange={set(PcmsCommonField.PitchBendRangeUp, "pitchBendRangeUp")} color="#8cf"
            title="Pitch bend range up (semitones)" />
        </div>
      </div>

      <div className={css.commonDivider} />

      {/* Standalone params */}
      <SynthKnob label="Level" value={common.toneLevel} {...knobRange(pcms_common_spec(PcmsCommonField.ToneLevel))}
        onChange={set(PcmsCommonField.ToneLevel, "toneLevel")} color="#8cf"
        title="Overall tone volume" />
      <SynthKnob label="Pan" value={common.tonePan} {...knobRange(pcms_common_spec(PcmsCommonField.TonePan))}
        onChange={set(PcmsCommonField.TonePan, "tonePan")} color="#8cf"
        title="Stereo panning position" />
      <SynthKnob label="Coarse" value={common.coarseTune} {...knobRange(pcms_common_spec(PcmsCommonField.CoarseTune))}
        onChange={set(PcmsCommonField.CoarseTune, "coarseTune")} color="#8cf"
        title="Coarse tuning (semitones)" />
      <SynthKnob label="Fine" value={common.fineTune} {...knobRange(pcms_common_spec(PcmsCommonField.FineTune))}
        onChange={set(PcmsCommonField.FineTune, "fineTune")} color="#8cf"
        title="Fine tuning (cents)" />

      <div className={css.commonDivider} />

      <SynthKnob label="Octave" value={common.octaveShift} {...knobRange(pcms_common_spec(PcmsCommonField.OctaveShift))}
        onChange={set(PcmsCommonField.OctaveShift, "octaveShift")} color="#8cf"
        title="Octave shift (-3 to +3)" />
      <SynthKnob label="Analog" value={common.analogFeel} {...knobRange(pcms_common_spec(PcmsCommonField.AnalogFeel))}
        onChange={set(PcmsCommonField.AnalogFeel, "analogFeel")} color="#8cf"
        title="Analog Feel — adds subtle pitch/timing randomness like vintage synths" />

      <div className={css.commonDivider} />

      <SynthSwitch label="LEGATO" value={common.legatoSwitch} options={switchOptions(pcms_common_spec(PcmsCommonField.LegatoSwitch))}
        onChange={set(PcmsCommonField.LegatoSwitch, "legatoSwitch")} title="Legato — overlapping notes share the same voice" />

      <div className={css.commonDivider} />

      {/* Offsets */}
      <div className={css.commonGroup}>
        <span className={css.groupLabel}>OFFSETS</span>
        <div className={css.groupRow}>
          <SynthKnob label="Cutoff" value={common.cutoffOffset} {...knobRange(pcms_common_spec(PcmsCommonField.CutoffOffset))}
            onChange={set(PcmsCommonField.CutoffOffset, "cutoffOffset")} color="#68c"
            title="Filter cutoff offset — shifts cutoff across all partials" />
          <SynthKnob label="Reso" value={common.resonanceOffset} {...knobRange(pcms_common_spec(PcmsCommonField.ResonanceOffset))}
            onChange={set(PcmsCommonField.ResonanceOffset, "resonanceOffset")} color="#68c"
            title="Resonance offset — shifts resonance across all partials" />
          <SynthKnob label="Attack" value={common.attackTimeOffset} {...knobRange(pcms_common_spec(PcmsCommonField.AttackTimeOffset))}
            onChange={set(PcmsCommonField.AttackTimeOffset, "attackTimeOffset")} color="#6c8"
            title="Attack time offset — shifts envelope attack across all partials" />
          <SynthKnob label="Release" value={common.releaseTimeOffset} {...knobRange(pcms_common_spec(PcmsCommonField.ReleaseTimeOffset))}
            onChange={set(PcmsCommonField.ReleaseTimeOffset, "releaseTimeOffset")} color="#6c8"
            title="Release time offset — shifts envelope release across all partials" />
          <SynthKnob label="Velocity" value={common.velocitySensOffset} {...knobRange(pcms_common_spec(PcmsCommonField.VelocitySensOffset))}
            onChange={set(PcmsCommonField.VelocitySensOffset, "velocitySensOffset")} color="#c8a"
            title="Velocity sensitivity offset — shifts velocity response across all partials" />
        </div>
      </div>
    </>
  );
}
