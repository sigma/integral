import type { ComponentProps } from "react";
import { SynthFader } from "./SynthFader";
import css from "./BreakpointEnvelope.module.css";

type Stage = Omit<ComponentProps<typeof SynthFader>, "compact">;

/** A level the device fixes: `at` is its normalized height (0–1), `text` its marker. */
interface FixedLevel {
  at: number;
  text: string;
}

interface Props {
  /** Stage times T1–Tn. Ti moves from level L(i-1) to Li. */
  times: Stage[];
  /** Levels L0–Ln (one more than times). L(n-1) is held while the key is down. */
  levels: (Stage | FixedLevel)[];
  /** Draw L(n-1) as a sustain plateau in the preview. Default: true. */
  hold?: boolean;
  compact?: boolean;
}

function isFixed(l: Stage | FixedLevel): l is FixedLevel {
  return "at" in l;
}

function normalize(s: Stage | FixedLevel): number {
  return isFixed(s) ? s.at : (s.value - s.min) / (s.max - s.min || 1);
}

const CURVE_H = 28;
const HOLD = 0.6;

/**
 * Breakpoint envelope: L0 → T1 → L1 → … → L(n-1) (held if `hold`) → Tn → Ln.
 * Levels sit on top, times below between them, linked by a line in stage
 * order wherever both ends are faders.
 */
export function BreakpointEnvelope({ times, levels, hold = true, compact }: Props) {
  // Grid of half-fader columns: level i centered on column line 2i+1,
  // time i on 2i, so each time sits between the two levels it connects.
  const cols = 2 * levels.length;
  const ys = levels.map((l) => 1 - normalize(l));
  const span = (cols - 2 - (hold ? HOLD : 0)) / times.length;

  let x = 1;
  let curve = `M ${x} ${ys[0]}`;
  times.forEach((t, i) => {
    if (hold && i === times.length - 1) {
      x += HOLD;
      curve += ` L ${x} ${ys[i]}`;
    }
    x += normalize(t) * span;
    curve += ` L ${x} ${ys[i + 1]}`;
  });

  let link = "";
  times.forEach((_, i) => {
    const t = 2 * i + 2;
    if (!isFixed(levels[i]!)) link += ` M ${t - 1} 0 C ${t - 1} 0.5 ${t} 0.5 ${t} 1`;
    if (!isFixed(levels[i + 1]!)) link += ` M ${t} 1 C ${t} 0.5 ${t + 1} 0.5 ${t + 1} 0`;
  });

  return (
    <div className={css.envelope} style={{ gridTemplateColumns: `repeat(${cols}, 1fr)` }}>
      <svg className={css.curve} style={{ height: CURVE_H }} viewBox={`0 -0.1 ${cols} 1.2`} preserveAspectRatio="none">
        <path d={curve} fill="none" stroke="#fc8" strokeWidth="2" vectorEffect="non-scaling-stroke" />
      </svg>
      {levels.map((l, i) => (
        <div key={`l${i}`} className={css.level} style={{ gridColumn: `${2 * i + 1} / span 2` }}>
          {isFixed(l) ? (
            <span className={css.fixedLevel} title="Fixed by the device">{l.text}</span>
          ) : (
            <SynthFader {...l} compact={compact} />
          )}
        </div>
      ))}
      {link && (
        <svg className={css.link} viewBox={`0 0 ${cols} 1`} preserveAspectRatio="none">
          <path d={link} fill="none" stroke="#555" strokeWidth="1.5" vectorEffect="non-scaling-stroke" />
        </svg>
      )}
      {times.map((t, i) => (
        <div key={`t${i}`} className={css.time} style={{ gridColumn: `${2 * i + 2} / span 2` }}>
          <SynthFader {...t} compact={compact} />
        </div>
      ))}
    </div>
  );
}
