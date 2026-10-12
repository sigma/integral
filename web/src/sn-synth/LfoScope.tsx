import { ScopeScreen, sampled } from "./ScopeScreen";

/** Deterministic 0..1 value per step, so random shapes don't flicker between renders. */
function rand(k: number): number {
  const s = Math.sin(k * 12.9898 + 78.233) * 43758.5453;
  return s - Math.floor(s);
}

const frac = (t: number) => t - Math.floor(t);

/** One cycle of each LFO shape at phase t, keyed by its LFO_SHAPE_NAMES label. */
const SHAPES: Record<string, (t: number) => number> = {
  TRI: (t) => Math.abs(2 * frac(t) - 1),
  SIN: (t) => 0.5 - 0.5 * Math.sin(2 * Math.PI * t),
  SAW: (t) => 1 - frac(t),
  SQR: (t) => (frac(t) < 0.5 ? 0 : 1),
  "S&H": (t) => rand(Math.floor(t)),
  RND: (t) => {
    const k = Math.floor(t);
    const mix = (1 - Math.cos(Math.PI * frac(t))) / 2;
    return rand(k) + (rand(k + 1) - rand(k)) * mix;
  },
};

/** Scope-style picture of an LFO shape; a higher `rate` (0-127) fits more cycles. */
export function LfoScope({ shape, rate }: { shape: string; rate: number }) {
  const y = SHAPES[shape];
  if (!y) return null;
  // shortcut: 1 to 8 cycles across the screen, a visual cue only; not the device's rate in Hz.
  const cycles = 1 + (7 * Math.min(127, Math.max(0, rate))) / 127;
  return (
    <ScopeScreen>
      <path d={sampled(400, (x) => y(x * cycles))} fill="none" stroke="#a6f" strokeWidth="2"
        vectorEffect="non-scaling-stroke" />
    </ScopeScreen>
  );
}
