/**
 * PROTOTYPE, throwaway (#280): where per-Part Studio Set settings live.
 *
 * Three variants under the real Mixer, switchable with `?variant=A|B|C` or the floating bar
 * (← / → keys):
 * - A: a detail drawer for the selected Part, opened from its strip.
 * - B: a tabbed panel under the Mixer, manual-style (KBD / PITCH / OFFSET / MIDI / SCALE).
 * - C: a cross-Part grid, one row per Part, with a keyboard-zone overview for splits and layers.
 * - D: A's drawer, opened from a mock top bar regrouped by scope (#584).
 *
 * Values are stubs held in memory only: nothing is sent to the device. Init buttons only log.
 */
import { useEffect, useState, type CSSProperties } from "react";
import { createPortal } from "react-dom";
import type { ReactNode } from "react";
import type { MixerState } from "../types";
import type { PageTab } from "../pluginPages";

type Kind =
  | { t: "num"; min: number; max: number; fmt?: (v: number) => string }
  | { t: "list"; opts: string[] };
interface Field {
  key: string;
  label: string;
  kind: Kind;
  def: number;
}
const num = (min: number, max: number, fmt?: (v: number) => string): Kind => ({
  t: "num",
  min,
  max,
  fmt,
});
const list = (...opts: string[]): Kind => ({ t: "list", opts });
const signed = (c: number) => (v: number) =>
  v - c > 0 ? `+${v - c}` : `${v - c}`;
const NOTE = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
const note = (v: number) => `${NOTE[v % 12]}${Math.floor(v / 12) - 1}`;
const OFFON = list("OFF", "ON");

/** The per-Part groups, as the manual's Part tabs name them. */
const GROUPS: { id: string; title: string; fields: Field[] }[] = [
  {
    id: "kbd",
    title: "KBD",
    fields: [
      { key: "keyLo", label: "Key Lower", kind: num(0, 127, note), def: 0 },
      { key: "keyHi", label: "Key Upper", kind: num(0, 127, note), def: 127 },
      { key: "keyFadeLo", label: "Key Fade Lo", kind: num(0, 127), def: 0 },
      { key: "keyFadeHi", label: "Key Fade Hi", kind: num(0, 127), def: 0 },
      { key: "velLo", label: "Velo Lower", kind: num(1, 127), def: 1 },
      { key: "velHi", label: "Velo Upper", kind: num(1, 127), def: 127 },
      {
        key: "velSens",
        label: "Velo Sens",
        kind: num(1, 127, signed(64)),
        def: 64,
      },
      {
        key: "mono",
        label: "Mono/Poly",
        kind: list("MONO", "POLY", "TONE"),
        def: 2,
      },
      {
        key: "legato",
        label: "Legato",
        kind: list("OFF", "ON", "TONE"),
        def: 2,
      },
      {
        key: "veloCurve",
        label: "Velo Curve",
        kind: list("OFF", "1", "2", "3", "4"),
        def: 0,
      },
    ],
  },
  {
    id: "pitch",
    title: "PITCH",
    fields: [
      { key: "oct", label: "Octave", kind: num(61, 67, signed(64)), def: 64 },
      {
        key: "coarse",
        label: "Coarse",
        kind: num(16, 112, signed(64)),
        def: 64,
      },
      { key: "fine", label: "Fine", kind: num(14, 114, signed(64)), def: 64 },
      {
        key: "bend",
        label: "Bend Range",
        kind: num(0, 25, (v) => (v === 25 ? "TONE" : `${v}`)),
        def: 25,
      },
      {
        key: "porta",
        label: "Portamento",
        kind: list("OFF", "ON", "TONE"),
        def: 2,
      },
      {
        key: "portaT",
        label: "Porta Time",
        kind: num(0, 128, (v) => (v === 128 ? "TONE" : `${v}`)),
        def: 128,
      },
    ],
  },
  {
    id: "offset",
    title: "OFFSET",
    fields: [
      "Cutoff",
      "Resonance",
      "Attack",
      "Decay",
      "Release",
      "Vib Rate",
      "Vib Depth",
      "Vib Delay",
    ].map((l) => ({
      key: l,
      label: l,
      kind: num(0, 127, signed(64)),
      def: 64,
    })),
  },
  {
    id: "midi",
    title: "MIDI",
    fields: [
      "Rx Switch",
      "Rx PC",
      "Rx BS",
      "Rx Bend",
      "Rx PAft",
      "Rx CAft",
      "Rx Mod",
      "Rx Vol",
      "Rx Pan",
      "Rx Exp",
      "Rx Hold",
    ].map((l) => ({ key: l, label: l, kind: OFFON, def: 1 })),
  },
  {
    id: "scale",
    title: "SCALE",
    fields: [
      {
        key: "scaleType",
        label: "Scale",
        kind: list(
          "CUSTOM",
          "EQUAL",
          "JUST-MAJ",
          "JUST-MIN",
          "PYTHAGORE",
          "KIRNBERGE",
          "MEANTONE",
          "WERCKMEIS",
          "ARABIC",
        ),
        def: 1,
      },
      { key: "scaleKey", label: "Key", kind: list(...NOTE), def: 0 },
      ...NOTE.map((n) => ({
        key: `tune${n}`,
        label: n,
        kind: num(0, 127, signed(64)),
        def: 64,
      })),
    ],
  },
  {
    id: "voice",
    title: "VOICE",
    fields: [
      {
        key: "reserve",
        label: "Voice Reserve",
        kind: num(0, 64, (v) => (v === 64 ? "FULL" : `${v}`)),
        def: 0,
      },
    ],
  },
];
const ALL = GROUPS.flatMap((g) => g.fields);

type Values = Record<string, number>[];
/** Stub values: a split (Parts 1–2) and a layer (Parts 3–4) so zones have something to show. */
function initialValues(): Values {
  const v = Array.from({ length: 16 }, () =>
    Object.fromEntries(ALL.map((f) => [f.key, f.def])),
  );
  v[0]!.keyHi = 59;
  v[1]!.keyLo = 60;
  v[2]!.velHi = 90;
  v[3]!.velLo = 91;
  v[4]!.keyLo = 36;
  v[4]!.keyHi = 72;
  v[4]!.oct = 63;
  return v;
}

const show = (f: Field, v: number) =>
  f.kind.t === "list" ? f.kind.opts[v] : f.kind.fmt ? f.kind.fmt(v) : `${v}`;

/** One editable control: a compact slider or select with its value. */
function Control({
  f,
  v,
  onChange,
  dense,
}: {
  f: Field;
  v: number;
  onChange: (v: number) => void;
  dense?: boolean;
}) {
  const input =
    f.kind.t === "list" ? (
      <select
        value={v}
        onChange={(e) => onChange(+e.target.value)}
        style={s.select}
      >
        {f.kind.opts.map((o, i) => (
          <option key={o} value={i}>
            {o}
          </option>
        ))}
      </select>
    ) : (
      <input
        type="range"
        min={f.kind.min}
        max={f.kind.max}
        value={v}
        onChange={(e) => onChange(+e.target.value)}
        style={dense ? s.rangeDense : s.range}
      />
    );
  return (
    <label style={dense ? s.ctlDense : s.ctl} title={f.label}>
      {!dense && <span style={s.lbl}>{f.label}</span>}
      {input}
      <span style={s.val}>{show(f, v)}</span>
    </label>
  );
}

const initButtons = (part: number) => (
  <span style={{ display: "inline-flex", gap: 6 }}>
    <button
      style={s.btn}
      onClick={() => console.log(`PROTOTYPE: Part Init, Part ${part + 1}`)}
    >
      Init Part…
    </button>
  </span>
);
const SoundCtrlInit = () => (
  <button
    style={s.btn}
    onClick={() => console.log("PROTOTYPE: Sound Ctrl Init (all 16 Parts)")}
    title="Resets the offsets on all 16 Parts"
  >
    Sound Ctrl Init (all Parts)…
  </button>
);

const partLabel = (state: MixerState, i: number) =>
  `Part ${i + 1} · ch ${state.parts[i]!.receiveChannel + 1} · ${state.parts[i]!.toneName.trim() || "—"}`;

/** The drawer itself: every group of one Part, stacked. */
function Drawer({
  state,
  values,
  set,
  onClose,
}: VProps & { onClose: () => void }) {
  const p = state.selectedPart;
  return (
    <aside style={s.drawer}>
      <header style={s.drawerHead}>
        <b>{partLabel(state, p)}</b>
        <span style={{ flex: 1 }} />
        {initButtons(p)}
        <button style={s.btn} onClick={onClose}>
          ✕
        </button>
      </header>
      <p style={s.hint}>
        Follows the Part selected in the Mixer: press a strip to switch.
      </p>
      {GROUPS.map((g) => (
        <section key={g.id} style={s.drawerSec}>
          <h4 style={s.h4}>
            {g.title} {g.id === "offset" && <SoundCtrlInit />}
          </h4>
          <div style={s.wrap}>
            {g.fields.map((f) => (
              <Control
                key={f.key}
                f={f}
                v={values[p]![f.key]!}
                onChange={(v) => set(p, f.key, v)}
              />
            ))}
          </div>
        </section>
      ))}
    </aside>
  );
}

/** A: a drawer for the selected Part, opened from a floating pill. */
function VariantA(props: VProps) {
  const [open, setOpen] = useState(true);
  if (!open)
    return (
      <button
        style={{ ...s.btn, position: "fixed", right: 16, top: 120 }}
        onClick={() => setOpen(true)}
      >
        ⚙ Part {props.state.selectedPart + 1} settings
      </button>
    );
  return <Drawer {...props} onClose={() => setOpen(false)} />;
}

/**
 * D: navigation by scope (#584), one row where each scope's selector heads its pages, with
 * drawer A opened from the Part group. It replaces the real tab row while shown; its page
 * buttons switch the real pages, its selectors and SAVEs only log (Part selection is real).
 */
function VariantD(props: VProps) {
  const { state, selectPart, nav } = props;
  const [open, setOpen] = useState(false);
  const part = state.parts[state.selectedPart]!;
  const bar = document.querySelector("nav");
  const log = (what: string) => () => console.log(`PROTOTYPE: ${what}`);
  const page = (id: PageTab, label: string) =>
    nav.pages.includes(id) && (
      <button
        key={id}
        style={nav.tab === id ? s.tabOn : s.tab}
        onClick={() => nav.setTab(id)}
      >
        {label}
      </button>
    );
  return (
    <>
      {bar &&
        createPortal(
          <div style={s.bar} data-proto>
            {/* Hides the real tab row and top bar: the row below replaces them. */}
            <style>
              {"nav > :not([data-proto]) { display: none !important; }"}
            </style>
            <span style={s.scope}>
              <span style={s.scopeLbl}>DEVICE</span>
              <button
                style={s.tab}
                onClick={log("open System page (#284, not built)")}
              >
                System
              </button>
              {page("expansions", "Expansions")}
              {page("library", "Library")}
            </span>
            <span style={s.crumb}>│</span>
            <span style={s.scope}>
              <span style={s.scopeLbl}>STUDIO SET</span>
              <button style={s.pick} onClick={log("open Studio Set selector")}>
                {state.studioSetPC + 1}: {state.studioSetName.trim() || "—"} ▾
              </button>
              <button style={s.btn} onClick={log("save Studio Set")}>
                SAVE
              </button>
              <span style={s.crumb}>·</span>
              {page("mixer", "Mixer")}
              {page("surround", "Surround")}
              {page("sequencer", "Sequencer")}
              {nav.transport}
            </span>
            <span style={s.crumb}>│</span>
            <span style={s.scope}>
              <span style={s.scopeLbl}>PART</span>
              <select
                value={state.selectedPart}
                onChange={(e) => selectPart(+e.target.value)}
                style={s.pick}
              >
                {state.parts.map((_, i) => (
                  <option key={i} value={i}>
                    {i + 1}
                  </option>
                ))}
              </select>
              <span style={s.hint}>ch {part.receiveChannel + 1}</span>
              <button
                style={open ? s.tabOn : s.btn}
                onClick={() => setOpen(!open)}
                title="Part settings"
              >
                ⚙
              </button>
              <button
                style={s.btn}
                onClick={log(`preview Part ${state.selectedPart + 1}`)}
              >
                PREVIEW
              </button>
            </span>
            <span style={s.crumb}>│</span>
            <span style={s.scope}>
              <span style={s.scopeLbl}>TONE</span>
              <button style={s.pick} onClick={log("open tone selector")}>
                {part.toneName.trim() || "—"} ▾
              </button>
              <button style={s.btn} onClick={log("save tone")}>
                SAVE
              </button>
              <span style={s.crumb}>·</span>
              {page("toneEdit", "Edit")}
            </span>
          </div>,
          bar,
        )}
      {open && <Drawer {...props} onClose={() => setOpen(false)} />}
    </>
  );
}

/** B: a tabbed panel under the Mixer for the selected Part, one manual tab at a time. */
function VariantB({ state, values, set, selectPart }: VProps) {
  const [tab, setTab] = useState(GROUPS[0]!.id);
  const p = state.selectedPart;
  const g = GROUPS.find((x) => x.id === tab)!;
  return (
    <section style={s.panel}>
      <div style={s.tabsRow}>
        <select
          value={p}
          onChange={(e) => selectPart(+e.target.value)}
          style={s.select}
        >
          {state.parts.map((_, i) => (
            <option key={i} value={i}>
              {partLabel(state, i)}
            </option>
          ))}
        </select>
        {GROUPS.map((x) => (
          <button
            key={x.id}
            style={x.id === tab ? s.tabOn : s.tab}
            onClick={() => setTab(x.id)}
          >
            {x.title}
          </button>
        ))}
        <span style={{ flex: 1 }} />
        {tab === "offset" ? <SoundCtrlInit /> : initButtons(p)}
      </div>
      <div style={s.wrap}>
        {g.fields.map((f) => (
          <Control
            key={f.key}
            f={f}
            v={values[p]![f.key]!}
            onChange={(v) => set(p, f.key, v)}
          />
        ))}
      </div>
    </section>
  );
}

/** C: every Part at once, one row each: a zone overview, then one group's columns. */
function VariantC({ state, values, set, selectPart }: VProps) {
  const [tab, setTab] = useState(GROUPS[0]!.id);
  const g = GROUPS.find((x) => x.id === tab)!;
  const bar = (
    lo: number,
    hi: number,
    max: number,
    color: string,
  ): CSSProperties => ({
    position: "absolute",
    left: `${(lo / max) * 100}%`,
    width: `${(Math.max(hi - lo, 0) / max) * 100}%`,
    height: "100%",
    background: color,
    borderRadius: 2,
  });
  return (
    <section style={s.panel}>
      <div style={s.tabsRow}>
        <b style={{ marginRight: 8 }}>All Parts</b>
        {GROUPS.map((x) => (
          <button
            key={x.id}
            style={x.id === tab ? s.tabOn : s.tab}
            onClick={() => setTab(x.id)}
          >
            {x.title}
          </button>
        ))}
        <span style={{ flex: 1 }} />
        <SoundCtrlInit />
      </div>
      <table style={s.table}>
        <thead>
          <tr>
            <th style={s.th}>Part</th>
            <th style={{ ...s.th, width: 260 }}>
              Zones: keys (blue) / velocity (orange)
            </th>
            {g.fields.map((f) => (
              <th key={f.key} style={s.th}>
                {f.label}
              </th>
            ))}
            <th style={s.th} />
          </tr>
        </thead>
        <tbody>
          {state.parts.map((part, i) => {
            const v = values[i]!;
            return (
              <tr
                key={i}
                style={i === state.selectedPart ? s.rowOn : undefined}
                onPointerDown={() => selectPart(i)}
              >
                <td style={s.td}>
                  {i + 1} · ch{part.receiveChannel + 1} ·{" "}
                  {part.toneName.trim() || "—"}
                </td>
                <td style={s.td}>
                  <div style={s.zone}>
                    <div style={bar(v.keyLo!, v.keyHi!, 127, "#4f6bd8")} />
                  </div>
                  <div style={{ ...s.zone, height: 4, marginTop: 2 }}>
                    <div style={bar(v.velLo!, v.velHi!, 127, "#d8904f")} />
                  </div>
                </td>
                {g.fields.map((f) => (
                  <td key={f.key} style={s.td}>
                    <Control
                      dense
                      f={f}
                      v={v[f.key]!}
                      onChange={(x) => set(i, f.key, x)}
                    />
                  </td>
                ))}
                <td style={s.td}>{initButtons(i)}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </section>
  );
}

interface VProps {
  state: MixerState;
  values: Values;
  set: (part: number, key: string, v: number) => void;
  selectPart: (part: number) => void;
  nav: Nav;
}

/** The app's page navigation, for D's row. */
interface Nav {
  tab: PageTab;
  pages: readonly PageTab[];
  setTab: (tab: PageTab) => void;
  transport: ReactNode;
}

const VARIANTS = [
  { key: "A", name: "Drawer from the strip", C: VariantA },
  { key: "B", name: "Tabbed panel under the Mixer", C: VariantB },
  { key: "C", name: "Cross-Part grid", C: VariantC },
  { key: "D", name: "Top bar by scope, drawer from Part", C: VariantD },
] as const;

/** The prototype's mount point under the Mixer, with its floating switcher. */
export function PartSettingsPrototype({
  state,
  selectPart,
  ...nav
}: Nav & {
  state: MixerState;
  selectPart: (p: number) => void;
}) {
  const read = () => new URLSearchParams(location.search).get("variant") ?? "A";
  const [variant, setVariant] = useState(read);
  const [values, setValues] = useState(initialValues);
  const idx = Math.max(
    0,
    VARIANTS.findIndex((v) => v.key === variant),
  );
  const go = (d: number) => {
    const next = VARIANTS[(idx + d + VARIANTS.length) % VARIANTS.length]!.key;
    const u = new URL(location.href);
    u.searchParams.set("variant", next);
    history.replaceState(null, "", u);
    setVariant(next);
  };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement;
      if (t.closest("input, textarea, select, [contenteditable]")) return;
      if (e.key === "ArrowLeft") go(-1);
      if (e.key === "ArrowRight") go(1);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
  if (import.meta.env.PROD) return null;
  const V = VARIANTS[idx]!;
  const set = (part: number, key: string, v: number) =>
    setValues((old) =>
      old.map((row, i) => (i === part ? { ...row, [key]: v } : row)),
    );
  return (
    <>
      {/* B and C are Mixer panels; A's and D's drawer and D's row follow every page. */}
      {(nav.tab === "mixer" || V.key === "A" || V.key === "D") && (
        <V.C
          state={state}
          values={values}
          set={set}
          selectPart={selectPart}
          nav={nav}
        />
      )}
      <div style={s.switcher}>
        <button style={s.swBtn} onClick={() => go(-1)}>
          ←
        </button>
        <span>
          PROTOTYPE {V.key} ({V.name})
        </span>
        <button style={s.swBtn} onClick={() => go(1)}>
          →
        </button>
      </div>
    </>
  );
}

const s: Record<string, CSSProperties> = {
  drawer: {
    position: "fixed",
    top: 0,
    right: 0,
    bottom: 0,
    width: 440,
    overflow: "auto",
    background: "#1b1d33",
    borderLeft: "1px solid #5566c8",
    padding: 12,
    zIndex: 50,
    boxShadow: "-6px 0 18px #0008",
  },
  drawerHead: { display: "flex", alignItems: "center", gap: 8 },
  drawerSec: { borderTop: "1px solid #2d3160", paddingTop: 6, marginTop: 6 },
  h4: {
    margin: "4px 0",
    fontSize: 12,
    color: "#9aa6ff",
    display: "flex",
    gap: 8,
    alignItems: "center",
  },
  hint: { fontSize: 11, color: "#888", margin: "4px 0" },
  panel: {
    margin: 12,
    padding: 10,
    background: "#1b1d33",
    border: "1px solid #2d3160",
    borderRadius: 6,
  },
  tabsRow: {
    display: "flex",
    alignItems: "center",
    gap: 4,
    marginBottom: 8,
    flexWrap: "wrap",
  },
  tab: {
    background: "#262a4d",
    color: "#ccc",
    border: "1px solid #3a3f70",
    padding: "3px 10px",
    borderRadius: 4,
    cursor: "pointer",
  },
  tabOn: {
    background: "#5566c8",
    color: "#fff",
    border: "1px solid #5566c8",
    padding: "3px 10px",
    borderRadius: 4,
    cursor: "pointer",
  },
  wrap: { display: "flex", flexWrap: "wrap", gap: "6px 14px" },
  ctl: { display: "flex", flexDirection: "column", width: 120, fontSize: 11 },
  ctlDense: { display: "flex", alignItems: "center", gap: 4, fontSize: 11 },
  lbl: { color: "#aab" },
  val: { color: "#fff", fontVariantNumeric: "tabular-nums", minWidth: 34 },
  range: { width: "100%" },
  rangeDense: { width: 70 },
  select: {
    background: "#262a4d",
    color: "#eee",
    border: "1px solid #3a3f70",
    fontSize: 11,
  },
  btn: {
    background: "#262a4d",
    color: "#ddd",
    border: "1px solid #3a3f70",
    padding: "3px 8px",
    borderRadius: 4,
    cursor: "pointer",
    fontSize: 11,
  },
  table: { borderCollapse: "collapse", fontSize: 11, width: "100%" },
  th: {
    textAlign: "left",
    color: "#9aa6ff",
    padding: "2px 6px",
    borderBottom: "1px solid #2d3160",
    fontWeight: 500,
  },
  td: {
    padding: "2px 6px",
    borderBottom: "1px solid #24274a",
    whiteSpace: "nowrap",
  },
  rowOn: { background: "rgb(85 102 200 / 18%)" },
  zone: {
    position: "relative",
    height: 8,
    background: "#262a4d",
    borderRadius: 2,
  },
  bar: {
    display: "flex",
    alignItems: "center",
    gap: 10,
    margin: "8px 12px",
    padding: "6px 10px",
    background: "#1b1d33",
    border: "1px dashed #ffde59",
    borderRadius: 6,
    fontSize: 12,
    flexWrap: "wrap",
  },
  scope: { display: "inline-flex", alignItems: "center", gap: 6 },
  scopeLbl: {
    color: "#9aa6ff",
    fontSize: 10,
    fontWeight: 600,
    letterSpacing: 1,
  },
  pick: {
    background: "#262a4d",
    color: "#fff",
    border: "1px solid #3a3f70",
    padding: "3px 8px",
    borderRadius: 4,
    cursor: "pointer",
    fontSize: 12,
  },
  crumb: { color: "#5566c8", fontSize: 14 },
  switcher: {
    position: "fixed",
    bottom: 14,
    left: "50%",
    transform: "translateX(-50%)",
    display: "flex",
    alignItems: "center",
    gap: 10,
    background: "#ffde59",
    color: "#111",
    padding: "6px 12px",
    borderRadius: 999,
    boxShadow: "0 4px 14px #0009",
    zIndex: 100,
    fontSize: 13,
    fontWeight: 600,
  },
  swBtn: {
    background: "#111",
    color: "#ffde59",
    border: 0,
    borderRadius: 999,
    width: 26,
    height: 26,
    cursor: "pointer",
  },
};
