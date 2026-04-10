import { SynthKnob } from "./SynthKnob";
import { SynthFader } from "./SynthFader";
import css from "./OutputStrip.module.css";

interface Props {
  /** FX1 (chorus) send level 0–127. */
  fx1: number;
  /** FX2 (reverb) send level 0–127. */
  fx2: number;
  /** Level 0–127. */
  level: number;
  onFx1Change: (v: number) => void;
  onFx2Change: (v: number) => void;
  onLevelChange: (v: number) => void;
  /** Level fader track height in px. Defaults to 150. */
  trackHeight?: number;
  /** Optional pan knob (0–127, center=64). Shown above sends when provided. */
  pan?: number;
  onPanChange?: (v: number) => void;
  panFormat?: (v: number) => string;
}

export function OutputStrip({
  fx1,
  fx2,
  level,
  onFx1Change,
  onFx2Change,
  onLevelChange,
  trackHeight = 150,
  pan,
  onPanChange,
  panFormat,
}: Props) {
  return (
    <div className={css.strip}>
      {pan !== undefined && onPanChange && (
        <SynthKnob label="Pan" value={pan} min={0} max={127} defaultValue={64}
          onChange={onPanChange} formatValue={panFormat ?? ((v) => String(v))} color="#8cf" />
      )}
      <div className={css.sends}>
        <SynthKnob label="FX1" value={fx1} min={0} max={127} defaultValue={0}
          onChange={onFx1Change} formatValue={(v) => String(v)} color="#668"
          title="Chorus Send Level" />
        <SynthKnob label="FX2" value={fx2} min={0} max={127} defaultValue={0}
          onChange={onFx2Change} formatValue={(v) => String(v)} color="#686"
          title="Reverb Send Level" />
      </div>
      <div className={css.fader}>
        <SynthFader label="Level" value={level} min={0} max={127} defaultValue={127}
          onChange={onLevelChange} formatValue={(v) => String(v)} trackHeight={trackHeight} />
      </div>
    </div>
  );
}
