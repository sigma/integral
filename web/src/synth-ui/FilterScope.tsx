import { ScopeScreen } from "./ScopeScreen";

/** Plotted gain range, in dB: 0 dB sits a third of the way down. */
const TOP_DB = 18;
const BOTTOM_DB = -36;

/**
 * Gain (dB) at `r` = frequency / cutoff of each filter type, keyed by its
 * Filter Type label, for a resonance `q`. LPF2 and LPF3 ignore Resonance
 * (INTEGRA-7 Parameter Guide, TVF tab), so they get a flat-topped q.
 */
const RESPONSES: Record<string, (r: number, q: number) => number> = {
  OFF: () => 1,
  LPF: (r, q) => 1 / denominator(r, q),
  LPF2: (r) => 1 / denominator(r, Math.SQRT1_2),
  LPF3: (r) => 1 / denominator(r, Math.SQRT1_2),
  HPF: (r, q) => (r * r) / denominator(r, q),
  BPF: (r, q) => r / q / denominator(r, q),
  PKG: (r, q) => 1 + ((q - Math.SQRT1_2) * (r / q)) / denominator(r, q),
};

/** |1 - r² + j r/q|: the shared denominator of a 2-pole response. */
function denominator(r: number, q: number): number {
  return Math.hypot(1 - r * r, r / q);
}

/**
 * Scope-style frequency response of a filter: frequency across on a log
 * scale, gain up. `cutoff` and `resonance` are 0-127.
 */
export function FilterScope({
  type,
  cutoff,
  resonance,
  className,
}: {
  type: string;
  cutoff: number;
  resonance: number;
  className?: string;
}) {
  const gain = RESPONSES[type];
  if (!gain) return null;
  // shortcut: a generic 2-pole response with cutoff spread over 8 octaves and Q 0.7-12, a picture of the settings; not the device's measured curves or slopes.
  const fc = Math.min(127, Math.max(0, cutoff)) / 127;
  const q = Math.SQRT1_2 + (11.3 * Math.min(127, Math.max(0, resonance))) / 127;
  const db = (x: number) => 20 * Math.log10(Math.max(gain(2 ** (8 * (x - fc)), q), 1e-6));
  const y = (d: number) => (TOP_DB - Math.min(TOP_DB, Math.max(BOTTOM_DB, d))) / (TOP_DB - BOTTOM_DB);
  // Frequencies cut below the floor draw nothing: the trace runs down to the
  // floor and stops, then starts again where the response comes back up.
  const n = 200;
  let path = "";
  let above = false;
  let prev = 0;
  for (let i = 0; i <= n; i++) {
    const x = i / n;
    const d = db(x);
    if (d >= BOTTOM_DB) {
      path += `${above ? "L" : i ? `M ${prev} ${y(BOTTOM_DB)} L` : "M"} ${x} ${y(d)} `;
    } else if (above) {
      path += `L ${x} ${y(BOTTOM_DB)} `;
    }
    above = d >= BOTTOM_DB;
    prev = x;
  }
  return (
    <ScopeScreen className={className}>
      <path d={path.trim()} fill="none" stroke="#68c" strokeWidth="2" vectorEffect="non-scaling-stroke" />
    </ScopeScreen>
  );
}
