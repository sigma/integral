import { ScopeScreen, sampled } from "./ScopeScreen";

const SINE = sampled(64, (x) => 0.5 - 0.5 * Math.sin(x * 4 * Math.PI));

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

/**
 * Two cycles of a pulse, high for `pw` (0-127) of each: 0 is a 50% square and
 * higher values thin the pulse (JD-Xi Parameter Guide, "Pulse width").
 * `shift` (0-127) scales the whole PW range; 127 leaves it as is.
 */
function pulse(pw: number, shift: number): string {
  // shortcut: assumes 127 thins the pulse to 5%, and that PW Shift scales the PW range linearly; neither mapping is documented, measure the device if it matters.
  const clamp = (v: number) => Math.min(127, Math.max(0, v)) / 127;
  const high = 0.25 - 0.225 * clamp(pw) * clamp(shift);
  return `M 0 0 H ${high} V 1 H 0.5 V 0 H ${0.5 + high} V 1 H 1`;
}

/**
 * Two cycles of the sum of 7 saws spread around the pitch by `detune` (0-127):
 * one clean saw at 0, a jagged staircase as the saws drift apart.
 */
function superSaw(detune: number): string {
  // shortcut: spread exaggerated (±25% pitch at 127) so two cycles show it; not the device's real detune curve.
  const spread = (0.25 * Math.min(127, Math.max(0, detune))) / 127;
  const freqs = [-3, -2, -1, 0, 1, 2, 3].map((k) => 2 * (1 + (spread * k) / 3));
  return sampled(400, (x) => 1 - freqs.reduce((sum, f) => sum + ((x * f) % 1), 0) / freqs.length);
}

/** Two cycles of each OSC wave, keyed by its OSC_WAVE_NAMES label. */
const SHAPES: Record<string, string> = {
  SAW: saw(0.5, 0),
  SQR: "M 0 0 H 0.25 V 1 H 0.5 V 0 H 0.75 V 1 H 1",
  TRI: "M 0 1 L 0.25 0 L 0.5 1 L 0.75 0 L 1 1",
  SINE: SINE,
  NOISE: NOISE,
};

/**
 * Scope-style picture of the OSC wave; renders nothing for waves without a
 * fixed shape (PCM). PW-SQR follows the Pulse Width, with faint traces at the
 * extremes the PWM depth swings it to, both scaled by PW Shift; SP-SAW follows
 * the Super Saw Detune.
 */
export function OscWaveScope({
  wave,
  pulseWidth,
  pwmDepth,
  pwShift,
  superSawDetune,
}: {
  wave: string;
  pulseWidth: number;
  pwmDepth: number;
  pwShift: number;
  superSawDetune: number;
}) {
  const isPw = wave === "PW-SQR";
  const path = isPw ? pulse(pulseWidth, pwShift) : wave === "SP-SAW" ? superSaw(superSawDetune) : SHAPES[wave];
  if (!path) return null;
  return (
    <ScopeScreen>
      {isPw && pwmDepth > 0 && (
        <path d={`${pulse(pulseWidth - pwmDepth, pwShift)} ${pulse(pulseWidth + pwmDepth, pwShift)}`} fill="none"
          stroke="#fc8" strokeOpacity="0.3" strokeWidth="1" vectorEffect="non-scaling-stroke" />
      )}
      <path d={path} fill="none" stroke="#fc8" strokeWidth="2" vectorEffect="non-scaling-stroke" />
    </ScopeScreen>
  );
}
