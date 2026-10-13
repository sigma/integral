import type { ParameterFamily } from "../../pkg/integral_wasm.js";
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
  /** Parameter family of the pan knob, from the caller's field spec. */
  panFamily?: ParameterFamily;
  /** Parameter family of the FX1 send knob, from the caller's field spec. */
  fx1Family?: ParameterFamily;
  /** Parameter family of the FX2 send knob, from the caller's field spec. */
  fx2Family?: ParameterFamily;
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
  panFamily,
  fx1Family,
  fx2Family,
}: Props) {
  return (
    <div className={css.strip}>
      {pan !== undefined && onPanChange && (
        <SynthKnob label="Pan" value={pan} min={0} max={127} defaultValue={64} center={64}
          onChange={onPanChange} formatValue={panFormat ?? ((v) => String(v))} color="#8cf" family={panFamily} />
      )}
      <div className={css.sends}>
        <SynthKnob label="FX1" value={fx1} min={0} max={127} defaultValue={0}
          onChange={onFx1Change} formatValue={(v) => String(v)} color="#668" family={fx1Family}
          title="Chorus Send Level" />
        <SynthKnob label="FX2" value={fx2} min={0} max={127} defaultValue={0}
          onChange={onFx2Change} formatValue={(v) => String(v)} color="#686" family={fx2Family}
          title="Reverb Send Level" />
      </div>
      <div className={css.fader}>
        <SynthFader label="Level" value={level} min={0} max={127} defaultValue={127}
          onChange={onLevelChange} formatValue={(v) => String(v)} trackHeight={trackHeight} />
      </div>
    </div>
  );
}
