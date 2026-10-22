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
 * What sweeps the cutoff while playing, each -1..1 across its field's
 * range (0 at its center); a missing one is 0.
 */
export interface CutoffModulation {
  /** Filter Env Depth. */
  envDepth?: number;
  /** Filter Env Velocity Sens. */
  envVelSens?: number;
  /** Cutoff Aftertouch Sens. */
  aftertouch?: number;
  /** LFO Filter Depth. */
  lfo?: number;
  /** Modulation LFO Filter Depth. */
  modLfo?: number;
  /** Cutoff Keyfollow. */
  keyfollow?: number;
}

/**
 * Cutoff steps (of 0-127) each source moves the cutoff at full scale.
 * shortcut: Roland doesn't publish how these map to frequency, nor how Env
 * Velocity Sens scales Env Depth; each moves the cutoff linearly, a
 * full-scale setting by half the cutoff range, and at velocity 127 Vel Sens
 * +1 doubles the envelope's depth and -1 cancels it. Keyfollow pivots on
 * the middle of the keyboard, so it swings both ways.
 */
const SWEEP = { env: 64, aftertouch: 64, lfo: 64, keyfollow: 64 } as const;

/**
 * The lowest and highest cutoff (0-127) `m` can move `cutoff` to while
 * playing, scaled by {@link SWEEP}: the envelope's peak at velocity 127,
 * full aftertouch, both LFOs' swing either way, and Keyfollow from the
 * lowest to the highest key.
 */
function cutoffSweep(cutoff: number, m: CutoffModulation): [number, number] {
  const env = (m.envDepth ?? 0) * (1 + (m.envVelSens ?? 0)) * SWEEP.env;
  const aft = (m.aftertouch ?? 0) * SWEEP.aftertouch;
  const swing =
    (Math.abs(m.lfo ?? 0) + Math.abs(m.modLfo ?? 0)) * SWEEP.lfo + Math.abs(m.keyfollow ?? 0) * SWEEP.keyfollow;
  const low = cutoff + Math.min(0, env) + Math.min(0, aft) - swing;
  const high = cutoff + Math.max(0, env) + Math.max(0, aft) + swing;
  return [clampCutoff(low), clampCutoff(high)];
}

/** `v` within the 0-127 cutoff range. */
function clampCutoff(v: number): number {
  return Math.min(127, Math.max(0, v));
}

/** Samples across the plot. */
const SAMPLES = 200;

/**
 * Scope-style frequency response of a filter: frequency across on a log
 * scale, gain up. `cutoff`, `resonance` and `hpf` are 0-127; `type` and
 * `slope` are core's labels, `type` a key of `responses`. A non-zero `hpf`
 * folds in a gentle high-pass ahead of the filter; `modulation` draws a
 * translucent band between the responses at the lowest and highest cutoff
 * it sweeps to.
 */
export function FilterScope({
  type,
  cutoff,
  resonance,
  slope,
  hpf = 0,
  modulation,
  responses = TVF_RESPONSES,
  className,
}: {
  type: string;
  cutoff: number;
  resonance: number;
  slope?: string;
  hpf?: number;
  modulation?: CutoffModulation;
  responses?: Record<string, FilterResponse>;
  className?: string;
}) {
  const response = responses[type];
  if (!response) return null;
  const stages = (slope && STAGES[slope]) || 1;
  // shortcut: a generic 2-pole response with cutoff spread over 8 octaves and Q 0.7-12, a picture of the settings; not the device's measured curves or slopes.
  const q = Math.SQRT1_2 + (11.3 * Math.min(127, Math.max(0, resonance))) / 127;
  // shortcut: the HPF drawn as a 1-pole (6 dB/oct) high-pass on the same axis; 0 is off.
  const fh = clampCutoff(hpf) / 127;
  const highPass = (x: number) => (fh > 0 ? onePoleHighPass(2 ** (8 * (x - fh))) : 1);
  /** Gain (dB) at `x` across the plot with the filter's cutoff at `c` (0-127). */
  const db = (c: number, x: number) =>
    20 * Math.log10(Math.max(response(2 ** (8 * (x - clampCutoff(c) / 127)), q, stages) * highPass(x), 1e-6));
  const y = (d: number) => (TOP_DB - Math.min(TOP_DB, Math.max(BOTTOM_DB, d))) / (TOP_DB - BOTTOM_DB);
  // Frequencies cut below the floor draw nothing: the trace runs down to the
  // floor and stops, then starts again where the response comes back up.
  let path = "";
  let above = false;
  let prev = 0;
  for (let i = 0; i <= SAMPLES; i++) {
    const x = i / SAMPLES;
    const d = db(cutoff, x);
    if (d >= BOTTOM_DB) {
      path += `${above ? "L" : i ? `M ${prev} ${y(BOTTOM_DB)} L` : "M"} ${x} ${y(d)} `;
    } else if (above) {
      path += `L ${x} ${y(BOTTOM_DB)} `;
    }
    above = d >= BOTTOM_DB;
    prev = x;
  }
  const [low, high] = modulation ? cutoffSweep(clampCutoff(cutoff), modulation) : [cutoff, cutoff];
  // The band: along the response at the highest cutoff, back along the lowest.
  const xs = Array.from({ length: SAMPLES + 1 }, (_, i) => i / SAMPLES);
  const band =
    low < high
      ? [...xs.map((x) => [x, db(high, x)] as const), ...[...xs].reverse().map((x) => [x, db(low, x)] as const)]
          .map(([x, d], i) => `${i ? "L" : "M"} ${x} ${y(d)}`)
          .join(" ") + " Z"
      : "";
  return (
    <ScopeScreen className={className}>
      {band && <path data-trace="sweep" d={band} fill="#68c" fillOpacity="0.25" stroke="none" />}
      <path
        data-trace="response"
        d={path.trim()}
        fill="none"
        stroke="#68c"
        strokeWidth="2"
        vectorEffect="non-scaling-stroke"
      />
    </ScopeScreen>
  );
}

/** Gain of a 1-pole high-pass at `r` = frequency / cutoff. */
function onePoleHighPass(r: number): number {
  return r / Math.hypot(1, r);
}
