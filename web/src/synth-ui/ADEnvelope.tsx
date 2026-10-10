import type { ComponentProps } from "react";
import { SynthFader } from "./SynthFader";
import css from "./ADEnvelope.module.css";

type Stage = Omit<ComponentProps<typeof SynthFader>, "compact">;

interface Props {
  attack: Stage;
  decay: Stage;
  compact?: boolean;
}

function normalize(s: Stage): number {
  return (s.value - s.min) / (s.max - s.min || 1);
}

const CURVE_H = 28;

/** Attack/decay fader pair with the envelope shape drawn above it. */
export function ADEnvelope({ attack, decay, compact }: Props) {
  const aX = normalize(attack) * 0.5;
  const dX = aX + normalize(decay) * 0.45;

  return (
    <div className={css.envelope}>
      <svg className={css.curve} style={{ height: CURVE_H }} viewBox="-0.05 -0.15 1.1 1.3" preserveAspectRatio="none">
        <path d={`M 0 1 L ${aX} 0 L ${dX} 1`} fill="none" stroke="#fc8" strokeWidth="2"
          vectorEffect="non-scaling-stroke" />
      </svg>
      <div className={css.faders}>
        <SynthFader {...attack} compact={compact} />
        <SynthFader {...decay} compact={compact} />
      </div>
    </div>
  );
}
