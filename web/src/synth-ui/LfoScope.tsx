import { ScopeScreen, sampled } from "./ScopeScreen";

/** Deterministic 0..1 value per step, so random shapes don't flicker between renders. */
function rand(k: number): number {
  const s = Math.sin(k * 12.9898 + 78.233) * 43758.5453;
  return s - Math.floor(s);
}

const frac = (t: number) => t - Math.floor(t);

/** Cosine blend from `a` to `b` as `x` goes 0..1. */
const ease = (a: number, b: number, x: number) => a + (b - a) * (1 - Math.cos(Math.PI * x)) / 2;

/** y (0 = top) of each LFO shape at phase t, cycles at integer t. */
const sin = (t: number) => 0.5 - 0.5 * Math.sin(2 * Math.PI * t);
const tri = (t: number) => Math.abs(2 * frac(t) - 1);
const sawUp = (t: number) => 1 - frac(t);
const sqr = (t: number) => (frac(t) < 0.5 ? 0 : 1);
const sampleHold = (t: number) => rand(Math.floor(t));
const rnd = (t: number) => ease(rand(Math.floor(t)), rand(Math.floor(t) + 1), frac(t));

/**
 * One cycle of each LFO shape, keyed by its waveform label: the SN-S LFO
 * Shape and PCM-S LFO Waveform labels from core. Shapes follow the
 * INTEGRA-7 Parameter Guide's descriptions (it draws none of them).
 */
const SHAPES: Record<string, (t: number) => number> = {
  SIN: sin,
  TRI: tri,
  SAW: sawUp,
  "SAW-UP": sawUp,
  "SAW-DW": (t) => frac(t),
  SQR: sqr,
  "S&H": sampleHold,
  RND: rnd,
  // BEND: the first rise (fall) develops, then the output holds.
  "BEND-UP": (t) => (t < 1 ? 1 - Math.sin((Math.PI / 2) * t) : 0),
  "BEND-DW": (t) => (t < 1 ? Math.sin((Math.PI / 2) * t) : 1),
  TRP: (t) => {
    const p = frac(t);
    return p < 0.15 ? 1 - p / 0.15 : p < 0.5 ? 0 : p < 0.65 ? (p - 0.5) / 0.15 : 1;
  },
  // shortcut: CHAOS and VSIN are drawn as the guide describes them (irregular; a sine whose amplitude varies each cycle), not from the device's output.
  CHS: (t) => {
    const u = 2.7 * t + 0.6 * Math.sin(1.3 * t);
    return ease(rand(Math.floor(u) + 100), rand(Math.floor(u) + 101), frac(u));
  },
  VSIN: (t) => 0.5 - 0.5 * (0.3 + 0.7 * rand(Math.floor(t) + 50)) * Math.sin(2 * Math.PI * t),
};

/** One cycle of a 28-100 step pattern (64 = 0): held steps, or eased between them when `smooth`. */
function stepShape(steps: number[], smooth: boolean): (t: number) => number {
  const y = (v: number) => 1 - (v - 28) / 72;
  return (t) => {
    const x = frac(t) * steps.length;
    const i = Math.floor(x);
    const here = y(steps[i] ?? 64);
    return smooth ? ease(here, y(steps[(i + 1) % steps.length] ?? 64), x - i) : here;
  };
}

/**
 * Scope-style picture of an LFO shape; a higher `rate` (0-127; tempo-synced
 * values above draw as 127) fits more cycles. STEP draws `steps`.
 */
export function LfoScope({
  shape,
  rate,
  steps,
  smooth = false,
  className,
}: {
  shape: string;
  rate: number;
  steps?: number[];
  smooth?: boolean;
  className?: string;
}) {
  const y = shape === "STEP" && steps ? stepShape(steps, smooth) : SHAPES[shape];
  if (!y) return null;
  // shortcut: 1 to 8 cycles across the screen, a visual cue only; not the device's rate in Hz.
  // CHAOS runs at its own pace: the device ignores Rate for it.
  const cycles = shape === "CHS" ? 3 : 1 + (7 * Math.min(127, Math.max(0, rate))) / 127;
  return (
    <ScopeScreen className={className}>
      <path d={sampled(400, (x) => y(x * cycles))} fill="none" stroke="#a6f" strokeWidth="2"
        vectorEffect="non-scaling-stroke" />
    </ScopeScreen>
  );
}
