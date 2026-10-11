import { ParametricEq } from "./synth-ui";
import type { EqControl } from "./synth-ui";
import type { EqState } from "./types";

const LOW_FREQ_VALUES = ["200", "400"];
const MID_FREQ_VALUES = [
  "200", "250", "315", "400", "500", "630", "800",
  "1.0k", "1.2k", "1.6k", "2.0k", "2.5k", "3.1k",
  "4.0k", "5.0k", "6.3k", "8.0k",
];
const HIGH_FREQ_VALUES = ["2.0k", "4.0k", "8.0k"];
const Q_VALUES = ["0.5", "1.0", "2.0", "4.0", "8.0"];

function formatGain(v: number): string {
  const db = v - 15;
  return `${db >= 0 ? "+" : ""}${db}dB`;
}

function formatFrom(table: string[]): (v: number) => string {
  return (v) => table[v] ?? `${v}`;
}

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
  const knob = (
    value: number, max: number, defaultValue: number, offset: number, formatValue: (v: number) => string,
  ): EqControl => ({ value, min: 0, max, defaultValue, onChange: (v) => onParam(paramBase + offset, v), formatValue });

  return (
    <ParametricEq
      enabled={eq.enabled}
      onToggleSwitch={onToggleSwitch}
      style={style}
      bands={[
        {
          label: "HI", color: "#c66",
          gain: knob(eq.highGain, 30, 15, 6, formatGain),
          freq: knob(eq.highFreq, 2, 1, 5, formatFrom(HIGH_FREQ_VALUES)),
        },
        {
          label: "MID", color: "#6c6",
          gain: knob(eq.midGain, 30, 15, 3, formatGain),
          freq: knob(eq.midFreq, 16, 7, 2, formatFrom(MID_FREQ_VALUES)),
          q: knob(eq.midQ, 4, 0, 4, formatFrom(Q_VALUES)),
        },
        {
          label: "LO", color: "#66c",
          gain: knob(eq.lowGain, 30, 15, 1, formatGain),
          freq: knob(eq.lowFreq, 1, 1, 0, formatFrom(LOW_FREQ_VALUES)),
        },
      ]}
    />
  );
}
