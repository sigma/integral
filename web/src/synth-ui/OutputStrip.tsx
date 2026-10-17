import type { KnobProps, KnobRangeProps } from "../fieldSpec";
import { SynthKnob } from "./SynthKnob";
import { SynthFader } from "./SynthFader";
import css from "./OutputStrip.module.css";

/** The optional pan knob, driven by its field spec's range, format and family. */
export interface OutputPan {
  value: number;
  onChange: (v: number) => void;
  /** The pan field's spec (`knobProps` over the caller's field spec). */
  spec: KnobProps;
}

interface Props {
  /** FX1 (chorus) send level. */
  fx1: number;
  /** FX2 (reverb) send level. */
  fx2: number;
  /** Level. */
  level: number;
  /** The FX1 send knob's spec, from the caller's field spec. */
  fx1Spec: KnobProps;
  /** The FX2 send knob's spec, from the caller's field spec. */
  fx2Spec: KnobProps;
  /** The level fader's spec, from the caller's field spec. */
  levelSpec: KnobRangeProps;
  onFx1Change: (v: number) => void;
  onFx2Change: (v: number) => void;
  onLevelChange: (v: number) => void;
  /** Level fader track height in px. Defaults to 150. */
  trackHeight?: number;
  /** Optional pan knob, shown above sends when provided. */
  pan?: OutputPan;
}

export function OutputStrip({
  fx1,
  fx2,
  level,
  fx1Spec,
  fx2Spec,
  levelSpec,
  onFx1Change,
  onFx2Change,
  onLevelChange,
  trackHeight = 150,
  pan,
}: Props) {
  return (
    <div className={css.strip}>
      {pan && (
        <SynthKnob label="Pan" value={pan.value} {...pan.spec} onChange={pan.onChange} />
      )}
      <div className={css.sends}>
        <SynthKnob label="FX1" value={fx1} {...fx1Spec}
          onChange={onFx1Change} title="Chorus Send Level" />
        <SynthKnob label="FX2" value={fx2} {...fx2Spec}
          onChange={onFx2Change} title="Reverb Send Level" />
      </div>
      <div className={css.fader}>
        <SynthFader label="Level" value={level} {...levelSpec}
          onChange={onLevelChange} trackHeight={trackHeight} />
      </div>
    </div>
  );
}
