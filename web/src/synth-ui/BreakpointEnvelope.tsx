import type { ComponentProps } from "react";
import { SynthFader } from "./SynthFader";
import css from "./BreakpointEnvelope.module.css";

type Stage = Omit<ComponentProps<typeof SynthFader>, "compact">;

interface Props {
  /** Stage times T1–T4. Ti moves from level L(i-1) to Li. */
  times: [Stage, Stage, Stage, Stage];
  /** Levels L0–L4; `null` marks a level fixed at zero (e.g. TVA start/end). */
  levels: [Stage | null, Stage | null, Stage | null, Stage | null, Stage | null];
  compact?: boolean;
}

function normalize(s: Stage | null): number {
  return s ? (s.value - s.min) / (s.max - s.min || 1) : 0;
}

// Layout: 10 half-fader columns. Level i is centered on column line 2i+1,
// time i on 2i, so each time sits between the two levels it connects.
const CURVE_H = 28;

/**
 * Roland-style 4-time / 5-level envelope:
 * L0 → T1 → L1 → T2 → L2 → T3 → L3 (held) → T4 → L4.
 * Levels sit on top, times below between them, linked by a line in stage order.
 */
export function BreakpointEnvelope({ times, levels, compact }: Props) {
  // Shape preview in a 10×1 viewBox: each time takes up to a quarter of the width.
  const ys = levels.map((l) => 1 - normalize(l));
  let x = 0;
  let curve = `M 0 ${ys[0]}`;
  times.forEach((t, i) => {
    x += normalize(t) * 2.5;
    curve += ` L ${x} ${ys[i + 1]}`;
  });

  // Sequence line: L0 down to T1, up to L1, down to T2, ...
  let link = "M 1 0";
  for (let i = 1; i <= 4; i++) {
    const t = 2 * i;
    link += ` C ${t - 1} 0.5 ${t} 0.5 ${t} 1 C ${t} 0.5 ${t + 1} 0.5 ${t + 1} 0`;
  }

  return (
    <div className={css.envelope}>
      <svg className={css.curve} style={{ height: CURVE_H }} viewBox="0 -0.1 10 1.2" preserveAspectRatio="none">
        <path d={curve} fill="none" stroke="#fc8" strokeWidth="2" vectorEffect="non-scaling-stroke" />
      </svg>
      {levels.map((l, i) => (
        <div key={`l${i}`} className={css.level} style={{ gridColumn: `${2 * i + 1} / span 2` }}>
          {l ? (
            <SynthFader {...l} compact={compact} />
          ) : (
            <span className={css.fixedLevel} title="Fixed at zero">L{i}<br />0</span>
          )}
        </div>
      ))}
      <svg className={css.link} viewBox="0 0 10 1" preserveAspectRatio="none">
        <path d={link} fill="none" stroke="#555" strokeWidth="1.5" vectorEffect="non-scaling-stroke" />
      </svg>
      {times.map((t, i) => (
        <div key={`t${i}`} className={css.time} style={{ gridColumn: `${2 * i + 2} / span 2` }}>
          <SynthFader {...t} compact={compact} />
        </div>
      ))}
    </div>
  );
}
