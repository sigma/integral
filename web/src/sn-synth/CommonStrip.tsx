import {
  SynthKnob,
  SynthSwitch,
  SynthSelect,
  SectionPanel,
} from "../synth-ui";
import { ParameterFamily, SnsCommonField, sns_common_spec } from "../../pkg/integral_wasm.js";
import css from "../SnSynthEditor.module.css";
import type { CommonParamHandler, SnSynthCommon } from "./types";
import { knobProps, switchOptions } from "../fieldSpec";

export function CommonStrip({
  common,
  onChange,
}: {
  common: SnSynthCommon;
  onChange: CommonParamHandler;
}) {
  return (
    <SectionPanel label="COMMON" family={ParameterFamily.Common}>
      <div className={css.commonStrip}>
        {/* Unison group */}
        <div className={css.commonGroup}>
          <span className={css.groupLabel}>UNISON</span>
          <div className={css.groupRow}>
            <SynthSwitch label="SW" value={common.unisonSwitch} options={switchOptions(sns_common_spec(SnsCommonField.UnisonSwitch))}
              onChange={(v) => onChange(SnsCommonField.UnisonSwitch, v)} />
            <SynthKnob label="Size" value={common.unisonSize} {...knobProps(sns_common_spec(SnsCommonField.UnisonSize))}
              onChange={(v) => onChange(SnsCommonField.UnisonSize, v)} />
          </div>
        </div>

        <div className={css.commonDivider} />

        {/* Mono */}
        <div className={css.commonGroup}>
          <SynthSwitch label="MONO" value={common.monoSwitch} options={switchOptions(sns_common_spec(SnsCommonField.MonoSwitch))}
            onChange={(v) => onChange(SnsCommonField.MonoSwitch, v)} />
        </div>

        <div className={css.commonDivider} />

        {/* Portamento group */}
        <div className={css.commonGroup}>
          <span className={css.groupLabel}>PORTAMENTO</span>
          <div className={css.groupRow}>
            <SynthSwitch label="SW" value={common.portamentoSwitch} options={switchOptions(sns_common_spec(SnsCommonField.PortamentoSwitch))}
              onChange={(v) => onChange(SnsCommonField.PortamentoSwitch, v)} />
            <SynthKnob label="Time" value={common.portamentoTime} {...knobProps(sns_common_spec(SnsCommonField.PortamentoTime))}
              onChange={(v) => onChange(SnsCommonField.PortamentoTime, v)} />
            <SynthSelect label="Mode" value={common.portamentoMode}
              options={switchOptions(sns_common_spec(SnsCommonField.PortamentoMode))}
              onChange={(v) => onChange(SnsCommonField.PortamentoMode, v)} />
          </div>
        </div>

        <div className={css.commonDivider} />

        {/* Bend Range */}
        <div className={css.commonGroup}>
          <span className={css.groupLabel}>BEND RANGE</span>
          <div className={css.groupRow}>
            <SynthKnob label="Down" value={common.pitchBendRangeDown} {...knobProps(sns_common_spec(SnsCommonField.PitchBendRangeDown))}
              onChange={(v) => onChange(SnsCommonField.PitchBendRangeDown, v)} />
            <SynthKnob label="Up" value={common.pitchBendRangeUp} {...knobProps(sns_common_spec(SnsCommonField.PitchBendRangeUp))}
              onChange={(v) => onChange(SnsCommonField.PitchBendRangeUp, v)} />
          </div>
        </div>

        <div className={css.commonDivider} />

        {/* Standalone params */}
        <SynthKnob label="Wave Shape" value={common.waveShape} {...knobProps(sns_common_spec(SnsCommonField.WaveShape))}
          onChange={(v) => onChange(SnsCommonField.WaveShape, v)} />
        <SynthKnob label="Tone Level" value={common.toneLevel} {...knobProps(sns_common_spec(SnsCommonField.ToneLevel))}
          onChange={(v) => onChange(SnsCommonField.ToneLevel, v)} />
        <SynthKnob label="Analog Feel" value={common.analogFeel} {...knobProps(sns_common_spec(SnsCommonField.AnalogFeel))}
          onChange={(v) => onChange(SnsCommonField.AnalogFeel, v)} />

        <div className={css.commonDivider} />

        <SynthSwitch label="LEGATO" value={common.legatoSwitch} options={switchOptions(sns_common_spec(SnsCommonField.LegatoSwitch))}
          onChange={(v) => onChange(SnsCommonField.LegatoSwitch, v)} />

        <SynthKnob label="Oct Shift" value={common.octaveShift} {...knobProps(sns_common_spec(SnsCommonField.OctaveShift))}
          onChange={(v) => onChange(SnsCommonField.OctaveShift, v)} />
      </div>
    </SectionPanel>
  );
}
