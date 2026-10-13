import type { ComponentProps } from "react";
import { type ParameterFamily, velocity_curve } from "../../pkg/integral_wasm.js";
import { familyVar } from "./family";
import { SynthSelect } from "./SynthSelect";
import { sampled } from "./ScopeScreen";
import css from "./VelocityCurveSelect.module.css";

/**
 * A V-Curve select, with a scope drawing the selected curve (velocity across, effect up)
 * in the curve parameter's family colour. The select itself stays neutral.
 */
export function VelocityCurveSelect({
  family,
  className,
  ...select
}: ComponentProps<typeof SynthSelect> & { family: ParameterFamily; className?: string }) {
  return (
    <div className={`${css.curveSelect} ${className ?? ""}`}>
      <SynthSelect {...select} />
      <svg className={css.curveScope} viewBox="-0.05 -0.15 1.1 1.3" preserveAspectRatio="none">
        <path d={sampled(32, (v) => 1 - velocity_curve(select.value, v))} fill="none" stroke={familyVar(family)}
          strokeWidth="2" vectorEffect="non-scaling-stroke" />
      </svg>
    </div>
  );
}
