import { ScopeScreen } from "./ScopeScreen";

/** Plotted gain range, in dB: 0 dB sits a third of the way down. */
const TOP_DB = 18;
const BOTTOM_DB = -36;

/**
 * Gain at `r` = frequency / cutoff of a filter for a resonance `q`, through
 * `stages` 2-pole stages: 1 at -12 dB/oct, 2 at -24 dB/oct.
 */
export type FilterResponse = (r: number, q: number, stages: number) => number;

/** One 2-pole low-pass, high-pass and band-pass stage. */
const lpf = (r: number, q: number) => 1 / denominator(r, q);
const hpf = (r: number, q: number) => (r * r) / denominator(r, q);
const bpf = (r: number, q: number) => r / q / denominator(r, q);
const peaking: FilterResponse = (r, q) => 1 + ((q - Math.SQRT1_2) * (r / q)) / denominator(r, q);

/** `stage` cascaded `stages` times; only the first stage resonates. */
const cascade =
  (stage: (r: number, q: number) => number): FilterResponse =>
  (r, q, stages) =>
    stage(r, q) * stage(r, Math.SQRT1_2) ** (stages - 1);

/**
 * PCM Synth and PCM Drum TVF responses, keyed by its Filter Type label.
 * LPF2 and LPF3 ignore Resonance (INTEGRA-7 Parameter Guide, TVF tab), so
 * they get a flat-topped q.
 */
const TVF_RESPONSES: Record<string, FilterResponse> = {
  OFF: () => 1,
  LPF: lpf,
  LPF2: (r) => lpf(r, Math.SQRT1_2),
  LPF3: (r) => lpf(r, Math.SQRT1_2),
  HPF: hpf,
  BPF: bpf,
  PKG: peaking,
};

/**
 * SN Synth FILTER responses, keyed by its Mode label; LPF, HPF, BPF and
 * LPF2–LPF4 steepen with its Slope.
 */
export const SNS_FILTER_RESPONSES: Record<string, FilterResponse> = {
  BYP: () => 1,
  LPF: cascade(lpf),
  HPF: cascade(hpf),
  BPF: cascade(bpf),
  // shortcut: a peak has no stages to stack; drawn the same at either Slope.
  PKG: peaking,
  // shortcut: LPF2–LPF4 differ from LPF in resonance character only (Parameter Guide); drawn as LPF.
  LPF2: cascade(lpf),
  LPF3: cascade(lpf),
  LPF4: cascade(lpf),
};

/** 2-pole stages per Slope label; no slope is one stage. */
const STAGES: Record<string, number> = { "-12dB": 1, "-24dB": 2 };

/** |1 - r² + j r/q|: the shared denominator of a 2-pole response. */
function denominator(r: number, q: number): number {
  return Math.hypot(1 - r * r, r / q);
}

/**
 * Scope-style frequency response of a filter: frequency across on a log
 * scale, gain up. `cutoff` and `resonance` are 0-127; `type` and `slope`
 * are core's labels, `type` a key of `responses`.
 */
export function FilterScope({
  type,
  cutoff,
  resonance,
  slope,
  responses = TVF_RESPONSES,
  className,
}: {
  type: string;
  cutoff: number;
  resonance: number;
  slope?: string;
  responses?: Record<string, FilterResponse>;
  className?: string;
}) {
  const response = responses[type];
  if (!response) return null;
  const stages = (slope && STAGES[slope]) || 1;
  // shortcut: a generic 2-pole response with cutoff spread over 8 octaves and Q 0.7-12, a picture of the settings; not the device's measured curves or slopes.
  const fc = Math.min(127, Math.max(0, cutoff)) / 127;
  const q = Math.SQRT1_2 + (11.3 * Math.min(127, Math.max(0, resonance))) / 127;
  const db = (x: number) => 20 * Math.log10(Math.max(response(2 ** (8 * (x - fc)), q, stages), 1e-6));
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
