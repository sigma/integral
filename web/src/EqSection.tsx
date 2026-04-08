import { EqKnob } from "./EqKnob";
import type { EqState } from "./types";
import css from "./EqSection.module.css";

// Display value formatters

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

function formatFreq(v: number, table: string[]): string {
  return table[v] ?? `${v}`;
}

function formatQ(v: number): string {
  return Q_VALUES[v] ?? `${v}`;
}

interface Props {
  eq: EqState;
  onToggleSwitch: () => void;
  onParam: (paramOffset: number, value: number) => void;
  showSwitch?: boolean;
}

export function EqSection({ eq, onToggleSwitch, onParam, showSwitch = true }: Props) {
  return (
    <div className={css.section}>
      {showSwitch && (
        <div className={css.switchRow}>
          <button
            className={`${css.switchButton} ${eq.enabled ? css.switchOn : css.switchOff}`}
            onClick={onToggleSwitch}
          >
            {eq.enabled ? "EQ ON" : "EQ OFF"}
          </button>
        </div>
      )}

      {/* Low band */}
      <div className={css.band}>
        <span className={css.bandLabel}>LOW</span>
        <EqKnob
          value={eq.lowFreq}
          min={0}
          max={1}
          defaultValue={1}
          onChange={(v) => onParam(1, v)}
          formatValue={(v) => formatFreq(v, LOW_FREQ_VALUES)}
          color="#668"
        />
        <EqKnob
          value={eq.lowGain}
          min={0}
          max={30}
          defaultValue={15}
          onChange={(v) => onParam(2, v)}
          formatValue={formatGain}
          color="#686"
        />
      </div>

      {/* Mid band */}
      <div className={css.band}>
        <span className={css.bandLabel}>MID</span>
        <EqKnob
          value={eq.midFreq}
          min={0}
          max={16}
          defaultValue={9}
          onChange={(v) => onParam(3, v)}
          formatValue={(v) => formatFreq(v, MID_FREQ_VALUES)}
          color="#668"
        />
        <EqKnob
          value={eq.midGain}
          min={0}
          max={30}
          defaultValue={15}
          onChange={(v) => onParam(4, v)}
          formatValue={formatGain}
          color="#686"
        />
        <EqKnob
          value={eq.midQ}
          min={0}
          max={4}
          defaultValue={1}
          onChange={(v) => onParam(5, v)}
          formatValue={formatQ}
          color="#866"
        />
      </div>

      {/* High band */}
      <div className={css.band}>
        <span className={css.bandLabel}>HIGH</span>
        <EqKnob
          value={eq.highFreq}
          min={0}
          max={2}
          defaultValue={1}
          onChange={(v) => onParam(6, v)}
          formatValue={(v) => formatFreq(v, HIGH_FREQ_VALUES)}
          color="#668"
        />
        <EqKnob
          value={eq.highGain}
          min={0}
          max={30}
          defaultValue={15}
          onChange={(v) => onParam(7, v)}
          formatValue={formatGain}
          color="#686"
        />
      </div>
    </div>
  );
}
