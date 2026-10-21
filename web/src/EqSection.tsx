import { MixerField } from "../pkg/integral_wasm.js";
import { ParametricEq } from "./synth-ui";
import type { EqControl } from "./synth-ui";
import type { EqState } from "./types";
import { paramFormatter, paramMeta } from "./paramDefaults";

interface Props {
  eq: EqState;
  /** Set an EQ field; the strip's scope (part, master or Comp+EQ unit) resolves its address in core. */
  onField: (field: MixerField, value: number) => void;
  style?: React.CSSProperties;
}

/** Mixer 3-band EQ (part, master and Comp+EQ) over `ParametricEq`. */
export function EqSection({ eq, onField, style }: Props) {
  /** A band control for the EQ field `field`. */
  const knob = (value: number, field: MixerField, unit = ""): EqControl => {
    const { min, max, defaultValue } = paramMeta(field);
    const fmt = paramFormatter(field);
    return {
      value, min, max, defaultValue,
      onChange: (v) => onField(field, v), formatValue: (v) => `${fmt(v)}${unit}`,
    };
  };

  return (
    <ParametricEq
      enabled={eq.enabled}
      onToggleSwitch={() => onField(MixerField.EqSwitch, eq.enabled ? 0 : 1)}
      style={style}
      bands={[
        {
          label: "HI", family: paramMeta(MixerField.EqHighGain).family,
          gain: knob(eq.highGain, MixerField.EqHighGain, "dB"),
          freq: knob(eq.highFreq, MixerField.EqHighFreq),
        },
        {
          label: "MID", family: paramMeta(MixerField.EqMidGain).family,
          gain: knob(eq.midGain, MixerField.EqMidGain, "dB"),
          freq: knob(eq.midFreq, MixerField.EqMidFreq),
          q: knob(eq.midQ, MixerField.EqMidQ),
        },
        {
          label: "LO", family: paramMeta(MixerField.EqLowGain).family,
          gain: knob(eq.lowGain, MixerField.EqLowGain, "dB"),
          freq: knob(eq.lowFreq, MixerField.EqLowFreq),
        },
      ]}
    />
  );
}
