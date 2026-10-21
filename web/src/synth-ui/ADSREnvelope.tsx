import type { ComponentProps } from "react";
import { SynthFader } from "./SynthFader";
import css from "./ADSREnvelope.module.css";

type Stage = Omit<ComponentProps<typeof SynthFader>, "compact">;

interface Props {
  attack: Stage;
  /** Decay time; leave it out (with sustain) for an AR envelope. */
  decay?: Stage;
  /** Sustain level and release time; give both for an ADSR, neither for an AD. */
  sustain?: Stage;
  release?: Stage;
  compact?: boolean;
}

function normalize(s: Stage): number {
  return (s.value - s.min) / (s.max - s.min || 1);
}

const CURVE_H = 40;

/** Stage values scaled to 0–1, as `envelopePath` reads them. */
export interface EnvelopeShape {
  attack: number;
  /** Leave it out (with sustain) for an AR envelope. */
  decay?: number;
  /** Give it with release for an ADSR. */
  sustain?: number;
  release?: number;
}

/**
 * The SVG path of an A/D, A/R or A/D/S/R envelope, in a unit box (x right, y
 * down): the shape `ADSREnvelope` draws above its faders, and the mixer's
 * compressor envelope Display draws alone.
 */
export function envelopePath({ attack, decay, sustain, release }: EnvelopeShape): string {
  if (decay === undefined && release !== undefined) {
    // A/R: rise, hold while the key is down, fall.
    const aX = attack * 0.35;
    const hold = aX + 0.25;
    return `M 0 1 L ${aX} 0 L ${hold} 0 L ${hold + release * 0.35} 1`;
  }
  if (decay !== undefined && sustain !== undefined && release !== undefined) {
    const aX = attack * 0.25;
    const dX = aX + decay * 0.25;
    const sY = 1 - sustain;
    const sEnd = dX + 0.2;
    return `M 0 1 L ${aX} 0 L ${dX} ${sY} L ${sEnd} ${sY} L ${sEnd + release * 0.25} 1`;
  }
  const aX = attack * 0.5;
  return `M 0 1 L ${aX} 0 L ${aX + (decay ?? 0) * 0.45} 1`;
}

/** A/D, A/R or A/D/S/R fader row with the envelope shape drawn above it. */
export function ADSREnvelope({ attack, decay, sustain, release, compact }: Props) {
  const path = envelopePath({
    attack: normalize(attack),
    decay: decay && normalize(decay),
    sustain: sustain && normalize(sustain),
    release: release && normalize(release),
  });

  return (
    <div className={css.envelope}>
      <svg className={css.curve} style={{ height: CURVE_H }} viewBox="-0.05 -0.15 1.1 1.3" preserveAspectRatio="none">
        <path d={path} fill="none" stroke="#fc8" strokeWidth="2"
          vectorEffect="non-scaling-stroke" />
      </svg>
      <div className={css.faders}>
        <SynthFader {...attack} compact={compact} />
        {decay && <SynthFader {...decay} compact={compact} />}
        {sustain && <SynthFader {...sustain} compact={compact} />}
        {release && <SynthFader {...release} compact={compact} />}
      </div>
    </div>
  );
}
