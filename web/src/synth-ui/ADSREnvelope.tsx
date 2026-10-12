import type { ComponentProps } from "react";
import { SynthFader } from "./SynthFader";
import css from "./ADSREnvelope.module.css";

type Stage = Omit<ComponentProps<typeof SynthFader>, "compact">;

interface Props {
  attack: Stage;
  decay: Stage;
  /** Sustain level and release time; give both for an ADSR, neither for an AD. */
  sustain?: Stage;
  release?: Stage;
  compact?: boolean;
}

function normalize(s: Stage): number {
  return (s.value - s.min) / (s.max - s.min || 1);
}

const CURVE_H = 40;

/** A/D or A/D/S/R fader row with the envelope shape drawn above it. */
export function ADSREnvelope({ attack, decay, sustain, release, compact }: Props) {
  let path: string;
  if (sustain && release) {
    const aX = normalize(attack) * 0.25;
    const dX = aX + normalize(decay) * 0.25;
    const sY = 1 - normalize(sustain);
    const sEnd = dX + 0.2;
    path = `M 0 1 L ${aX} 0 L ${dX} ${sY} L ${sEnd} ${sY} L ${sEnd + normalize(release) * 0.25} 1`;
  } else {
    const aX = normalize(attack) * 0.5;
    path = `M 0 1 L ${aX} 0 L ${aX + normalize(decay) * 0.45} 1`;
  }

  return (
    <div className={css.envelope}>
      <svg className={css.curve} style={{ height: CURVE_H }} viewBox="-0.05 -0.15 1.1 1.3" preserveAspectRatio="none">
        <path d={path} fill="none" stroke="#fc8" strokeWidth="2"
          vectorEffect="non-scaling-stroke" />
      </svg>
      <div className={css.faders}>
        <SynthFader {...attack} compact={compact} />
        <SynthFader {...decay} compact={compact} />
        {sustain && <SynthFader {...sustain} compact={compact} />}
        {release && <SynthFader {...release} compact={compact} />}
      </div>
    </div>
  );
}
