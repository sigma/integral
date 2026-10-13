import { ParametricEq } from "./synth-ui";
import type { EqControl } from "./synth-ui";
import type { EqState } from "./types";
import { paramFormatter, paramMeta } from "./paramDefaults";

interface Props {
  eq: EqState;
  onToggleSwitch: () => void;
  onParam: (paramOffset: number, value: number) => void;
  /** Offset added to all param indices. Part EQ has switch at 0 so params start at 1.
   *  Master EQ has no switch so params start at 0. Default: 1 (Part EQ). */
  paramBase?: number;
  style?: React.CSSProperties;
}

/** Mixer 3-band EQ (part, master and Comp+EQ) over `ParametricEq`. */
export function EqSection({ eq, onToggleSwitch, onParam, paramBase = 1, style }: Props) {
  /** A band control for registry param `eq.<name>`, at `offset` from `paramBase`. */
  const knob = (value: number, name: string, offset: number, unit = ""): EqControl => {
    const id = `eq.${name}`;
    const { min, max, defaultValue } = paramMeta(id);
    const fmt = paramFormatter(id);
    return {
      value, min, max, defaultValue,
      onChange: (v) => onParam(paramBase + offset, v), formatValue: (v) => `${fmt(v)}${unit}`,
    };
  };

  return (
    <ParametricEq
      enabled={eq.enabled}
      onToggleSwitch={onToggleSwitch}
      style={style}
      bands={[
        {
          label: "HI", family: paramMeta("eq.high_gain").family,
          gain: knob(eq.highGain, "high_gain", 6, "dB"),
          freq: knob(eq.highFreq, "high_freq", 5),
        },
        {
          label: "MID", family: paramMeta("eq.mid_gain").family,
          gain: knob(eq.midGain, "mid_gain", 3, "dB"),
          freq: knob(eq.midFreq, "mid_freq", 2),
          q: knob(eq.midQ, "mid_q", 4),
        },
        {
          label: "LO", family: paramMeta("eq.low_gain").family,
          gain: knob(eq.lowGain, "low_gain", 1, "dB"),
          freq: knob(eq.lowFreq, "low_freq", 0),
        },
      ]}
    />
  );
}
