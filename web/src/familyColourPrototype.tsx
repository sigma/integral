// PROTOTYPE (#160), throwaway: Parameter family colour variants.
// Dev builds only. URL params: ?variant=A..D &palette=0..2 &eq=same|ramp &cvd=none|deut|prot|trit|grey
// Families are guessed from today's hex literals and param names; the real
// classification comes from core (ADR 0002), not from here.
import { useSyncExternalStore } from "react";
import { createPortal } from "react-dom";

export type Family = "P" | "F" | "A" | "M" | "T" | "C";
const FAMILIES: Family[] = ["P", "F", "A", "M", "T", "C"];
const FAMILY_NAMES: Record<Family, string> = {
  P: "Pitch / Source",
  F: "Filter / Tone",
  A: "Amp / Level",
  M: "Modulation",
  T: "Time / Space",
  C: "Common / Global",
};

const PALETTES: { name: string; colours: Record<Family, string> }[] = [
  { name: "Today's hues", colours: { P: "#fc8", F: "#68c", A: "#6c8", M: "#a6f", T: "#c8a", C: "#8cf" } },
  { name: "Okabe-Ito", colours: { P: "#E69F00", F: "#56B4E9", A: "#009E73", M: "#CC79A7", T: "#F0E442", C: "#999999" } },
  { name: "Hue + lightness", colours: { P: "#ff9e4a", F: "#3d8bff", A: "#5fe08f", M: "#b57cff", T: "#fff07a", C: "#7c7c94" } },
];

const VARIANTS = {
  A: "Arc only",
  B: "Tinted cap",
  C: "Coloured label",
  D: "Arc + pointer + label",
} as const;
export type Variant = keyof typeof VARIANTS;
const VARIANT_KEYS = Object.keys(VARIANTS) as Variant[];
const EQ_MODES = ["same", "ramp"] as const;
const CVD_MODES = ["none", "deut", "prot", "trit", "grey"] as const;

export interface Proto {
  variant: Variant;
  palette: number;
  eq: (typeof EQ_MODES)[number];
  cvd: (typeof CVD_MODES)[number];
}

const EVENT = "family-proto";
let cached: { search: string; proto: Proto } | null = null;
function read(): Proto | null {
  if (!import.meta.env.DEV) return null;
  const search = window.location.search;
  if (cached?.search === search) return cached.proto;
  const q = new URLSearchParams(search);
  const pick = <T extends string>(list: readonly T[], v: string | null): T =>
    list.find((x) => x === v) ?? list[0]!;
  const palette = Number(q.get("palette") ?? 0);
  cached = {
    search,
    proto: {
      variant: pick(VARIANT_KEYS, q.get("variant")),
      palette: palette >= 0 && palette < PALETTES.length ? palette : 0,
      eq: pick(EQ_MODES, q.get("eq")),
      cvd: pick(CVD_MODES, q.get("cvd")),
    },
  };
  return cached.proto;
}
function write(p: Proto) {
  const q = new URLSearchParams(window.location.search);
  q.set("variant", p.variant);
  q.set("palette", String(p.palette));
  q.set("eq", p.eq);
  q.set("cvd", p.cvd);
  window.history.replaceState(null, "", `?${q}`);
  window.dispatchEvent(new Event(EVENT));
}
function subscribe(cb: () => void) {
  window.addEventListener(EVENT, cb);
  window.addEventListener("popstate", cb);
  return () => {
    window.removeEventListener(EVENT, cb);
    window.removeEventListener("popstate", cb);
  };
}

/** The prototype settings, or null outside dev builds. */
export function useProto(): Proto | null {
  return useSyncExternalStore(subscribe, read);
}

export function familyColour(p: Proto, f: Family): string {
  return PALETTES[p.palette]!.colours[f];
}

// shortcut: hex → family guesses from today's literals; the real families come from core.
const BY_HEX: Record<string, Family> = {
  "#fc8": "P", "#68c": "F", "#6c8": "A", "#a6f": "M", "#8cf": "C",
  "#e86": "F", "#6ae": "F", "#ea6": "F", "#f64": "F", "#c96": "A",
  "#c8a": "T", "#a8e": "T", "#4ce0e0": "C",
};
export function familyOfHex(hex: string | undefined, label = ""): Family {
  if (label.toUpperCase().split(" ").includes("PAN")) return "T";
  return (hex && BY_HEX[hex.toLowerCase()]) || "C";
}

// shortcut: keyword guesses by MFX param name, roughly the #159 rules.
const WORDS: [Family, string[]][] = [
  ["T", ["Pan", "Pre Delay", "Time", "Delay", "Spread", "Separation", "Feedback Mode"]],
  ["M", ["Rate", "Depth", "Speed", "Step", "Phase", "Wave", "Sens", "Polarity", "Brake", "Sync", "Duty", "Acceleration", "Shuffle"]],
  ["P", ["Coarse", "Fine", "Sample Rate", "Bit", "LoFi", "Frequency"]],
  ["F", ["Gain", "Freq", "Q", "Filter", "Cutoff", "Resonance", "Tone", "Bass", "Treble", "Middle", "Presence", "Bright", "Drive", "Damp", "Manual", "Peak", "Band", "Vowel", "Feedback", "Type", "Mode", "Color"]],
  ["A", ["Level", "Balance", "Mix", "Threshold", "Attack", "Release", "Hold", "Volume", "Master", "Ratio"]],
];
export function familyOfName(name: string): Family {
  const words = name.split(" ");
  if (name.endsWith(" Sw") || name.endsWith("Switch")) return "C";
  if (words.includes("Level") || words.includes("Balance")) return "A";
  for (const [f, keys] of WORDS) if (keys.some((k) => name.includes(k))) return f;
  return "C";
}

/** EQ band colour: one Filter / Tone hue, or a lightness ramp of it by band index. */
export function eqBandColour(p: Proto, band: number, bands: number): string {
  const base = familyColour(p, "F");
  if (p.eq === "same") return base;
  const mix = bands > 1 ? band / (bands - 1) : 0; // 0 = lightest
  return `color-mix(in oklab, ${base} ${100 - mix * 55}%, ${mix > 0.5 ? "#000" : "#fff"} ${mix * 55}%)`;
}

const CVD_MATRICES: Record<Exclude<Proto["cvd"], "none">, string> = {
  prot: "0.152286 1.052583 -0.204868 0 0 0.114503 0.786281 0.099216 0 0 -0.003882 -0.048116 1.051998 0 0 0 0 0 1 0",
  deut: "0.367322 0.860646 -0.227968 0 0 0.280085 0.672501 0.047413 0 0 -0.011820 0.042940 0.968881 0 0 0 0 0 1 0",
  trit: "1.255528 -0.076749 -0.178779 0 0 -0.078411 0.930809 0.147602 0 0 0.004733 0.691367 0.303900 0 0 0 0 0 1 0",
  grey: "0.2126 0.7152 0.0722 0 0 0.2126 0.7152 0.0722 0 0 0.2126 0.7152 0.0722 0 0 0 0 0 1 0",
};

/** Floating bar plus the colour-blindness filter; renders nothing outside dev builds. */
export function FamilyPrototypeBar() {
  const p = useProto();
  if (!p) return null;
  const cycle = <T,>(list: readonly T[], cur: T, d: number) =>
    list[(list.indexOf(cur) + d + list.length) % list.length]!;
  const set = (patch: Partial<Proto>) => write({ ...p, ...patch });
  const root = document.getElementById("root");
  if (root) root.style.filter = p.cvd === "none" ? "" : `url(#family-proto-${p.cvd})`;

  const onKey = (e: KeyboardEvent) => {
    const t = e.target as HTMLElement | null;
    if (t?.closest("input, textarea, select, [contenteditable], [role=slider]")) return;
    const cur = read()!;
    const go = (patch: Partial<Proto>) => write({ ...cur, ...patch });
    if (e.key === "ArrowLeft") go({ variant: cycle(VARIANT_KEYS, cur.variant, -1) });
    else if (e.key === "ArrowRight") go({ variant: cycle(VARIANT_KEYS, cur.variant, 1) });
    else if (e.key === "p") go({ palette: (cur.palette + 1) % PALETTES.length });
    else if (e.key === "e") go({ eq: cycle(EQ_MODES, cur.eq, 1) });
    else if (e.key === "c") go({ cvd: cycle(CVD_MODES, cur.cvd, 1) });
  };
  window.onkeydown = onKey;

  const btn: React.CSSProperties = {
    background: "#333", color: "#fff", border: "1px solid #666", borderRadius: 4,
    padding: "2px 8px", cursor: "pointer", font: "inherit",
  };
  return createPortal(
    <>
      <svg width="0" height="0" style={{ position: "absolute" }} aria-hidden>
        {Object.entries(CVD_MATRICES).map(([k, m]) => (
          <filter key={k} id={`family-proto-${k}`}>
            <feColorMatrix type="matrix" values={m} />
          </filter>
        ))}
      </svg>
      <div
        style={{
          position: "fixed", bottom: 12, left: "50%", transform: "translateX(-50%)", zIndex: 10000,
          display: "flex", gap: 10, alignItems: "center", padding: "6px 12px", borderRadius: 999,
          background: "#fff", color: "#000", boxShadow: "0 2px 12px #000a", font: "12px system-ui",
        }}
      >
        <b>PROTOTYPE #160</b>
        <button style={btn} onClick={() => set({ variant: cycle(VARIANT_KEYS, p.variant, -1) })}>◀</button>
        <span>
          {p.variant} ({VARIANTS[p.variant]})
        </span>
        <button style={btn} onClick={() => set({ variant: cycle(VARIANT_KEYS, p.variant, 1) })}>▶</button>
        <button style={btn} onClick={() => set({ palette: (p.palette + 1) % PALETTES.length })}>
          palette [p]: {PALETTES[p.palette]!.name}
        </button>
        <button style={btn} onClick={() => set({ eq: cycle(EQ_MODES, p.eq, 1) })}>EQ [e]: {p.eq}</button>
        <button style={btn} onClick={() => set({ cvd: cycle(CVD_MODES, p.cvd, 1) })}>CVD [c]: {p.cvd}</button>
        <span style={{ display: "flex", gap: 6 }}>
          {FAMILIES.map((f) => (
            <span key={f} title={FAMILY_NAMES[f]} style={{ display: "flex", alignItems: "center", gap: 3 }}>
              <span style={{ width: 12, height: 12, borderRadius: 6, background: familyColour(p, f), outline: "1px solid #0004" }} />
              {f}
            </span>
          ))}
        </span>
      </div>
    </>,
    document.body,
  );
}
