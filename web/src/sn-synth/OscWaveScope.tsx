import css from "../SnSynthEditor.module.css";

const SINE = Array.from({ length: 65 }, (_, i) => {
  const x = i / 64;
  return `${i ? "L" : "M"} ${x} ${0.5 - 0.5 * Math.sin(x * 4 * Math.PI)}`;
}).join(" ");

// Fixed pseudo-random samples (LCG), so the noise trace doesn't flicker between renders.
const NOISE = (() => {
  let seed = 7;
  return Array.from({ length: 49 }, (_, i) => {
    seed = (seed * 1103515245 + 12345) % 2147483648;
    return `${i ? "L" : "M"} ${i / 48} ${seed / 2147483648}`;
  }).join(" ");
})();

/** Rising ramps of the given period, offset by `phase`, clipped to 0..1. */
function saw(period: number, phase: number): string {
  let d = `M 0 ${1 - phase / period}`;
  for (let x = period - phase; x < 1; x += period) d += ` L ${x} 0 M ${x} 1`;
  const tail = (1 + phase) % period || period; // ends at a reset: draw up to the peak
  return `${d} L 1 ${1 - tail / period}`;
}

/** Two cycles of each OSC wave, keyed by its OSC_WAVE_NAMES label. */
const SHAPES: Record<string, string> = {
  SAW: saw(0.5, 0),
  SQR: "M 0 0 H 0.25 V 1 H 0.5 V 0 H 0.75 V 1 H 1",
  // shortcut: fixed 25% duty, ignores the Pulse Width param; draw it live once its 0-127 to duty mapping is confirmed.
  "PW-SQR": "M 0 0 H 0.125 V 1 H 0.5 V 0 H 0.625 V 1 H 1",
  TRI: "M 0 1 L 0.25 0 L 0.5 1 L 0.75 0 L 1 1",
  SINE: SINE,
  NOISE: NOISE,
  "SP-SAW": [saw(0.5, 0), saw(0.46, 0.1), saw(0.54, 0.3)].join(" "),
};

/** Scope-style picture of the OSC wave; renders nothing for waves without a fixed shape (PCM). */
export function OscWaveScope({ wave }: { wave: string }) {
  const path = SHAPES[wave];
  if (!path) return null;
  return (
    <div className={css.oscScopeSlot}>
      <svg className={css.oscScope} viewBox="-0.05 -0.15 1.1 1.3" preserveAspectRatio="none">
        <path d={path} fill="none" stroke="#fc8" strokeWidth="2" vectorEffect="non-scaling-stroke" />
      </svg>
    </div>
  );
}
