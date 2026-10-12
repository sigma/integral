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

/**
 * Two cycles of a pulse, high for `pw` (0-127) of each: 0 is a 50% square and
 * higher values thin the pulse (JD-Xi Parameter Guide, "Pulse width").
 */
function pulse(pw: number): string {
  // shortcut: assumes 127 thins the pulse to 5%; measure the device's real minimum if it matters.
  const high = 0.25 - (0.225 * Math.min(127, Math.max(0, pw))) / 127;
  return `M 0 0 H ${high} V 1 H 0.5 V 0 H ${0.5 + high} V 1 H 1`;
}

/** Two cycles of each OSC wave, keyed by its OSC_WAVE_NAMES label. */
const SHAPES: Record<string, string> = {
  SAW: saw(0.5, 0),
  SQR: "M 0 0 H 0.25 V 1 H 0.5 V 0 H 0.75 V 1 H 1",
  TRI: "M 0 1 L 0.25 0 L 0.5 1 L 0.75 0 L 1 1",
  SINE: SINE,
  NOISE: NOISE,
  "SP-SAW": [saw(0.5, 0), saw(0.46, 0.1), saw(0.54, 0.3)].join(" "),
};

/**
 * Scope-style picture of the OSC wave; renders nothing for waves without a
 * fixed shape (PCM). PW-SQR follows the Pulse Width, with faint traces at the
 * extremes the PWM depth swings it to.
 */
export function OscWaveScope({
  wave,
  pulseWidth,
  pwmDepth,
}: {
  wave: string;
  pulseWidth: number;
  pwmDepth: number;
}) {
  const isPw = wave === "PW-SQR";
  const path = isPw ? pulse(pulseWidth) : SHAPES[wave];
  if (!path) return null;
  return (
    <div className={css.oscScopeSlot}>
      <svg className={css.oscScope} viewBox="-0.05 -0.15 1.1 1.3" preserveAspectRatio="none">
        {isPw && pwmDepth > 0 && (
          <path d={`${pulse(pulseWidth - pwmDepth)} ${pulse(pulseWidth + pwmDepth)}`} fill="none"
            stroke="#fc8" strokeOpacity="0.3" strokeWidth="1" vectorEffect="non-scaling-stroke" />
        )}
        <path d={path} fill="none" stroke="#fc8" strokeWidth="2" vectorEffect="non-scaling-stroke" />
      </svg>
    </div>
  );
}
