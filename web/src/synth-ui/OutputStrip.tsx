import type { ParameterFamily } from "../../pkg/integral_wasm.js";
import { SynthKnob } from "./SynthKnob";
import { SynthFader } from "./SynthFader";
import css from "./OutputStrip.module.css";

/** The optional pan knob (0–127, center=64), coloured by its field spec's family. */
export interface OutputPan {
  value: number;
  onChange: (v: number) => void;
  format?: (v: number) => string;
  /** Parameter family of the pan knob, from the caller's field spec. */
  family: ParameterFamily;
}

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
  /** Optional pan knob, shown above sends when provided. */
  pan?: OutputPan;
  /** Parameter family of the FX1 send knob, from the caller's field spec. */
  fx1Family: ParameterFamily;
  /** Parameter family of the FX2 send knob, from the caller's field spec. */
  fx2Family: ParameterFamily;
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
  fx1Family,
  fx2Family,
}: Props) {
  return (
    <div className={css.strip}>
      {pan && (
        <SynthKnob label="Pan" value={pan.value} min={0} max={127} defaultValue={64} center={64}
          onChange={pan.onChange} formatValue={pan.format ?? ((v) => String(v))} family={pan.family} />
      )}
      <div className={css.sends}>
        <SynthKnob label="FX1" value={fx1} min={0} max={127} defaultValue={0}
          onChange={onFx1Change} formatValue={(v) => String(v)} family={fx1Family}
          title="Chorus Send Level" />
        <SynthKnob label="FX2" value={fx2} min={0} max={127} defaultValue={0}
          onChange={onFx2Change} formatValue={(v) => String(v)} family={fx2Family}
          title="Reverb Send Level" />
      </div>
      <div className={css.fader}>
        <SynthFader label="Level" value={level} min={0} max={127} defaultValue={127}
          onChange={onLevelChange} formatValue={(v) => String(v)} trackHeight={trackHeight} />
      </div>
    </div>
  );
}
