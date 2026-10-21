import css from "./BreakpointEnvelope.module.css";

/** Width of the held stage, in columns. */
const HOLD = 0.6;

/**
 * The curve L0 → T1 → L1 → … → L(n-1) (held) → Tn → Ln, from normalized
 * (0–1) times and levels, in a box `2 × levels.length` wide and 1 high:
 * level i sits on column line 2i+1, so each time spans the columns between
 * the two levels it joins.
 */
export function breakpointPath(times: readonly number[], levels: readonly number[]): string {
  const cols = 2 * levels.length;
  const ys = levels.map((l) => 1 - l);
  const span = (cols - 2 - HOLD) / times.length;

  let x = 1;
  let curve = `M ${x} ${ys[0]}`;
  times.forEach((t, i) => {
    if (i === times.length - 1) {
      x += HOLD;
      curve += ` L ${x} ${ys[i]}`;
    }
    x += t * span;
    curve += ` L ${x} ${ys[i + 1]}`;
  });
  return curve;
}

/** The curve of a breakpoint envelope alone, filling its box: a Display on the Grid. */
export function BreakpointCurve({ times, levels }: { times: readonly number[]; levels: readonly number[] }) {
  return (
    <svg className={css.fill} viewBox={`0 -0.1 ${2 * levels.length} 1.2`} preserveAspectRatio="none">
      <path d={breakpointPath(times, levels)} fill="none" stroke="#fc8" strokeWidth="2" vectorEffect="non-scaling-stroke" />
    </svg>
  );
}
