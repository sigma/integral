import type { ComponentProps } from "react";
import { velocity_curve } from "../../pkg/integral_wasm.js";
import { SynthSelect } from "../synth-ui";
import { sampled } from "../sn-synth/ScopeScreen";
import css from "../PcmSynthEditor.module.css";

/** A V-Curve select, with a scope drawing the selected curve (velocity across, effect up). */
export function VelocityCurveSelect({ color, ...select }: ComponentProps<typeof SynthSelect> & { color: string }) {
  return (
    <div className={css.curveSelect}>
      <SynthSelect {...select} />
      <svg className={css.curveScope} viewBox="-0.05 -0.15 1.1 1.3" preserveAspectRatio="none">
        <path d={sampled(32, (v) => 1 - velocity_curve(select.value, v))} fill="none" stroke={color}
          strokeWidth="2" vectorEffect="non-scaling-stroke" />
      </svg>
    </div>
  );
}
