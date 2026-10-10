/**
 * PROTOTYPE — throwaway. Answers #45: how do the transport, the Keyboard and
 * the 16 × 64 grid fit on screen?
 *
 * Three variants inside the real app shell, switched via `?variant=A|B|C`:
 *   A — Bottom dock: all 16 Tracks always visible under every page.
 *   B — Sequencer tab: full-page grid, plus a mini transport on other pages.
 *   C — Focus drawer: a piano-roll of the selected Part's Track, with a
 *       16-Track overview map.
 *
 * The Pattern lives in memory only. Playback is a naive main-thread
 * setInterval loop: good enough to judge the layout, not the timing.
 */

import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";
import type { IntegraService } from "../integra";
import type { UseMixerResult } from "../useMixer";

const TRACKS = 16;
const STEPS = 64;
const NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
const noteName = (p: number) => `${NAMES[p % 12]}${Math.floor(p / 12) - 1}`;

interface Note {
  step: number;
  pitch: number;
  vel: number;
  len: number;
}
interface Track {
  length: number;
  notes: Note[];
}

function seed(): Track[] {
  const t: Track[] = Array.from({ length: TRACKS }, () => ({ length: 16, notes: [] }));
  for (let s = 0; s < 16; s += 4) t[0]!.notes.push({ step: s, pitch: 36, vel: 110, len: 1 });
  for (let s = 2; s < 16; s += 2) t[1]!.notes.push({ step: s, pitch: 42, vel: 70 + s * 2, len: 1 });
  t[2]!.length = 32;
  [36, 36, 39, 41, 36, 43, 41, 39].forEach((p, i) =>
    t[2]!.notes.push({ step: i * 4, pitch: p + 12, vel: 100, len: i % 3 === 0 ? 3 : 2 }),
  );
  t[3]!.length = 64;
  [0, 16, 32, 48].forEach((s, i) =>
    [60, 63, 67].forEach((p) => t[3]!.notes.push({ step: s, pitch: p + (i % 2) * 5, vel: 80, len: 14 })),
  );
  return t;
}

export function useProtoSeq(service: IntegraService | null, mixer: UseMixerResult) {
  const [tracks, setTracks] = useState(seed);
  const [cursor, setCursor] = useState(0);
  const [armed, setArmed] = useState(false);
  const [playing, setPlaying] = useState(false);
  const [tick, setTick] = useState(0);
  const [tempo, setTempo] = useState(120);
  const [octave, setOctave] = useState(4);
  const [defVel, setDefVel] = useState(100);
  const tracksRef = useRef(tracks);
  tracksRef.current = tracks;
  const sel = mixer.state.selectedPart;

  useEffect(() => {
    if (!playing || !service) return;
    const stepMs = 60000 / tempo / 4;
    let t = 0;
    const id = setInterval(() => {
      tracksRef.current.forEach((tr, i) => {
        const ch = mixer.state.parts[i]?.receiveChannel ?? i;
        for (const n of tr.notes) {
          if (n.step !== t % tr.length) continue;
          service.sendNoteOn(ch, n.pitch, n.vel);
          setTimeout(() => service.sendNoteOff(ch, n.pitch), n.len * stepMs - 5);
        }
      });
      setTick(t++);
    }, stepMs);
    return () => clearInterval(id);
  }, [playing, tempo, service, mixer.state.parts]);

  const audition = useCallback(
    (pitch: number) => {
      const ch = mixer.state.parts[sel]?.receiveChannel ?? sel;
      service?.sendNoteOn(ch, pitch, defVel);
      setTimeout(() => service?.sendNoteOff(ch, pitch), 250);
    },
    [service, mixer.state.parts, sel, defVel],
  );

  const writeChord = useCallback(
    (pitches: number[]) => {
      setTracks((ts) =>
        ts.map((tr, i) =>
          i !== sel
            ? tr
            : {
                ...tr,
                notes: [
                  ...tr.notes.filter((n) => n.step !== cursor),
                  ...pitches.map((p) => ({ step: cursor, pitch: p, vel: defVel, len: 1 })),
                ],
              },
        ),
      );
      setCursor((c) => (c + 1) % tracksRef.current[sel]!.length);
    },
    [sel, cursor, defVel],
  );

  const setVel = useCallback((track: number, step: number, vel: number) => {
    setTracks((ts) =>
      ts.map((tr, i) =>
        i !== track ? tr : { ...tr, notes: tr.notes.map((n) => (n.step === step ? { ...n, vel } : n)) },
      ),
    );
  }, []);

  const setLength = useCallback((track: number, length: number) => {
    setTracks((ts) => ts.map((tr, i) => (i !== track ? tr : { ...tr, length })));
  }, []);

  return {
    tracks, cursor, setCursor, armed, setArmed, playing, setPlaying, tick, tempo, setTempo,
    octave, setOctave, defVel, setDefVel, sel, selectPart: mixer.selectPart, audition,
    writeChord, setVel, setLength, mixer,
  };
}
export type ProtoSeq = ReturnType<typeof useProtoSeq>;

// ---------------------------------------------------------------------------
// Shared pieces (controls, not layout)
// ---------------------------------------------------------------------------

const C = {
  bg: "#0a0a18", panel: "#12122a", line: "#2a2a4a", text: "#c0c0e0", dim: "#666",
  accent: "#4a6cf7", note: "#7d8cff", rec: "#e0445a", play: "#3ac47d",
};
const btn = (on = false, color = C.accent): CSSProperties => ({
  background: on ? color : "transparent", color: on ? "#fff" : C.text,
  border: `1px solid ${on ? color : C.line}`, borderRadius: 4, padding: "3px 10px",
  fontSize: 12, cursor: "pointer",
});

function Transport({ s, compact = false }: { s: ProtoSeq; compact?: boolean }) {
  return (
    <div style={{ display: "flex", gap: 8, alignItems: "center", fontSize: 12, color: C.text }}>
      <button style={btn(s.playing, C.play)} onClick={() => s.setPlaying(!s.playing)}>
        {s.playing ? "■ Stop" : "▶ Play"}
      </button>
      <button style={btn(s.armed, C.rec)} onClick={() => s.setArmed(!s.armed)}>● Rec</button>
      <label>
        BPM{" "}
        <input type="number" value={s.tempo} onChange={(e) => s.setTempo(+e.target.value || 120)}
          style={{ width: 48, background: C.bg, color: C.text, border: `1px solid ${C.line}` }} />
      </label>
      {!compact && (
        <>
          <span>Oct
            <button style={btn()} onClick={() => s.setOctave((o) => Math.max(0, o - 1))}>-</button>
            {s.octave}
            <button style={btn()} onClick={() => s.setOctave((o) => Math.min(8, o + 1))}>+</button>
          </span>
          <label>
            Vel <input type="range" min={1} max={127} value={s.defVel}
              onChange={(e) => s.setDefVel(+e.target.value)} style={{ width: 70 }} /> {s.defVel}
          </label>
          <label>
            Len <input type="number" min={1} max={64} value={s.tracks[s.sel]!.length}
              onChange={(e) => s.setLength(s.sel, Math.max(1, Math.min(64, +e.target.value || 16)))}
              style={{ width: 40, background: C.bg, color: C.text, border: `1px solid ${C.line}` }} />
          </label>
          <span style={{ color: C.dim }}>Cursor {s.cursor + 1} · Part {s.sel + 1}</span>
        </>
      )}
    </div>
  );
}

/** On-screen piano. Click = audition (+ write when armed). Shift-click = add to pending chord. */
function Piano({ s, octaves = 2, height = 56 }: { s: ProtoSeq; octaves?: number; height?: number }) {
  const [pending, setPending] = useState<number[]>([]);
  const base = s.octave * 12 + 12;
  const keys = Array.from({ length: octaves * 12 + 1 }, (_, i) => base + i);
  const whites = keys.filter((k) => !NAMES[k % 12]!.includes("#"));
  const click = (p: number, shift: boolean) => {
    s.audition(p);
    if (!s.armed) return;
    if (shift) return setPending((c) => [...c, p]);
    s.writeChord([...pending, p]);
    setPending([]);
  };
  const w = 100 / whites.length;
  return (
    <div style={{ position: "relative", height, userSelect: "none" }}>
      {whites.map((k, i) => (
        <div key={k} onClick={(e) => click(k, e.shiftKey)} style={{
          position: "absolute", left: `${i * w}%`, width: `${w}%`, height: "100%",
          background: pending.includes(k) ? C.note : "#ddd", border: "1px solid #333", boxSizing: "border-box",
          fontSize: 9, color: "#555", display: "flex", alignItems: "flex-end", justifyContent: "center",
        }}>{k % 12 === 0 ? noteName(k) : ""}</div>
      ))}
      {keys.filter((k) => NAMES[k % 12]!.includes("#")).map((k) => {
        const i = whites.filter((x) => x < k).length;
        return (
          <div key={k} onClick={(e) => click(k, e.shiftKey)} style={{
            position: "absolute", left: `${i * w - w * 0.3}%`, width: `${w * 0.6}%`, height: "60%",
            background: pending.includes(k) ? C.note : "#222", zIndex: 1,
          }} />
        );
      })}
    </div>
  );
}

const partLabel = (s: ProtoSeq, i: number) => {
  const p = s.mixer.state.parts[i];
  return `${i + 1} ${p?.toneName || ""}`.trim();
};

// ---------------------------------------------------------------------------
// Variant A — bottom dock, every Track visible, under every page
// ---------------------------------------------------------------------------

function DenseGrid({ s, rowH }: { s: ProtoSeq; rowH: number }) {
  return (
    <div style={{ overflow: "auto" }}>
      {s.tracks.map((tr, ti) => (
        <div key={ti} style={{ display: "flex", height: rowH, alignItems: "stretch" }}>
          <div onClick={() => s.selectPart(ti)} style={{
            width: 110, flexShrink: 0, fontSize: 10, lineHeight: `${rowH}px`, overflow: "hidden",
            whiteSpace: "nowrap", cursor: "pointer", paddingLeft: 4,
            color: ti === s.sel ? "#fff" : C.dim, background: ti === s.sel ? "#1e2350" : "transparent",
          }}>{partLabel(s, ti)}</div>
          {Array.from({ length: STEPS }, (_, st) => {
            const inLen = st < tr.length;
            const start = tr.notes.find((n) => n.step === st);
            const held = tr.notes.some((n) => n.step < st && st < n.step + n.len);
            const play = s.playing && st === s.tick % tr.length;
            const cur = ti === s.sel && st === s.cursor;
            return (
              <div key={st} onClick={() => { s.selectPart(ti); s.setCursor(st); }} style={{
                flex: 1, minWidth: 8, boxSizing: "border-box",
                borderLeft: `1px solid ${st % 16 === 0 ? "#555" : st % 4 === 0 ? "#333" : "#1c1c30"}`,
                background: !inLen ? "#050510" : start ? C.note : held ? "#3d4690" : play ? "#2a3a2a" : "transparent",
                opacity: start ? 0.4 + start.vel / 211 : 1,
                outline: cur ? `2px solid ${s.armed ? C.rec : "#fff"}` : undefined, outlineOffset: -2,
              }} title={start ? `${noteName(start.pitch)} v${start.vel} ×${start.len}` : undefined} />
            );
          })}
        </div>
      ))}
    </div>
  );
}

function VariantA({ s }: { s: ProtoSeq }) {
  const [open, setOpen] = useState(true);
  return (
    <div style={{ flexShrink: 0, background: C.panel, borderTop: `1px solid ${C.line}`, padding: "6px 8px", display: "flex", flexDirection: "column", gap: 6 }}>
      <div style={{ display: "flex", justifyContent: "space-between" }}>
        <Transport s={s} />
        <button style={btn()} onClick={() => setOpen(!open)}>{open ? "▾ Hide" : "▴ Show"}</button>
      </div>
      {open && (
        <>
          <DenseGrid s={s} rowH={11} />
          <Piano s={s} octaves={3} height={48} />
        </>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Variant B — dedicated Sequencer page + mini transport on other pages
// ---------------------------------------------------------------------------

function VelocityLane({ s, height }: { s: ProtoSeq; height: number }) {
  const tr = s.tracks[s.sel]!;
  return (
    <div style={{ display: "flex", height, paddingLeft: 110, borderTop: `1px solid ${C.line}` }}>
      {Array.from({ length: STEPS }, (_, st) => {
        const n = tr.notes.find((x) => x.step === st);
        return (
          <div key={st} style={{ flex: 1, minWidth: 8, display: "flex", alignItems: "flex-end", cursor: n ? "ns-resize" : undefined }}
            onMouseDown={(e) => {
              if (!n) return;
              const r = e.currentTarget.getBoundingClientRect();
              s.setVel(s.sel, st, Math.max(1, Math.round(127 * (1 - (e.clientY - r.top) / r.height))));
            }}>
            {n && <div style={{ width: "70%", margin: "0 auto", height: `${(n.vel / 127) * 100}%`, background: C.note }} />}
          </div>
        );
      })}
    </div>
  );
}

function NamedGrid({ s }: { s: ProtoSeq }) {
  return (
    <div style={{ flex: 1, overflow: "auto", minHeight: 0 }}>
      {s.tracks.map((tr, ti) => (
        <div key={ti} style={{ display: "flex", height: 26, borderBottom: "1px solid #1c1c30" }}>
          <div onClick={() => s.selectPart(ti)} style={{
            width: 110, flexShrink: 0, fontSize: 11, lineHeight: "26px", paddingLeft: 6, cursor: "pointer",
            overflow: "hidden", whiteSpace: "nowrap",
            color: ti === s.sel ? "#fff" : C.text, background: ti === s.sel ? "#1e2350" : "transparent",
          }}>{partLabel(s, ti)}</div>
          <div style={{ flex: 1, position: "relative", display: "flex" }}>
            {Array.from({ length: STEPS }, (_, st) => (
              <div key={st} onClick={() => { s.selectPart(ti); s.setCursor(st); }} style={{
                flex: 1, minWidth: 8, boxSizing: "border-box",
                borderLeft: `1px solid ${st % 16 === 0 ? "#555" : st % 4 === 0 ? "#333" : "#181828"}`,
                background: st >= tr.length ? "#050510" : s.playing && st === s.tick % tr.length ? "#203020" : undefined,
                outline: ti === s.sel && st === s.cursor ? `2px solid ${s.armed ? C.rec : "#fff"}` : undefined, outlineOffset: -2,
              }} />
            ))}
            {[...new Set(tr.notes.map((n) => n.step))].map((st) => {
              const chord = tr.notes.filter((n) => n.step === st);
              const len = Math.min(Math.max(...chord.map((n) => n.len)), tr.length - st);
              return (
                <div key={st} style={{
                  position: "absolute", left: `${(st / STEPS) * 100}%`, width: `${(len / STEPS) * 100}%`,
                  top: 3, bottom: 3, background: C.note, borderRadius: 3, fontSize: 9, color: "#fff",
                  overflow: "hidden", whiteSpace: "nowrap", paddingLeft: 2, pointerEvents: "none",
                  opacity: 0.4 + chord[0]!.vel / 211,
                }}>{chord.map((n) => noteName(n.pitch)).join(" ")}</div>
              );
            })}
          </div>
        </div>
      ))}
    </div>
  );
}

export function VariantBPage({ s }: { s: ProtoSeq }) {
  return (
    <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column", gap: 6, padding: 8, background: C.bg }}>
      <Transport s={s} />
      <NamedGrid s={s} />
      <VelocityLane s={s} height={60} />
      <Piano s={s} octaves={4} height={80} />
    </div>
  );
}

function VariantBMini({ s }: { s: ProtoSeq }) {
  const [kb, setKb] = useState(false);
  return (
    <div style={{ position: "fixed", left: 12, bottom: 12, zIndex: 50, background: C.panel, border: `1px solid ${C.line}`, borderRadius: 8, padding: 8, width: kb ? 520 : undefined, boxShadow: "0 4px 16px #000a" }}>
      <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
        <Transport s={s} compact />
        <span style={{ fontSize: 11, color: C.dim }}>step {s.playing ? (s.tick % 64) + 1 : "-"}</span>
        <button style={btn(kb)} onClick={() => setKb(!kb)}>🎹</button>
      </div>
      {kb && <div style={{ marginTop: 6 }}><Piano s={s} octaves={2} height={50} /></div>}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Variant C — focus drawer: piano-roll of the selected Track + overview map
// ---------------------------------------------------------------------------

function Overview({ s }: { s: ProtoSeq }) {
  return (
    <div style={{ width: 200, flexShrink: 0 }}>
      {s.tracks.map((tr, ti) => (
        <div key={ti} onClick={() => s.selectPart(ti)} style={{ display: "flex", height: 8, marginBottom: 1, cursor: "pointer", outline: ti === s.sel ? `1px solid ${C.accent}` : undefined }}>
          {Array.from({ length: STEPS }, (_, st) => (
            <div key={st} style={{
              flex: 1, background: st >= tr.length ? "#050510"
                : tr.notes.some((n) => n.step <= st && st < n.step + n.len) ? C.note
                : s.playing && st === s.tick % tr.length ? "#3a5a3a" : "#1a1a30",
            }} />
          ))}
        </div>
      ))}
    </div>
  );
}

function PianoRoll({ s }: { s: ProtoSeq }) {
  const tr = s.tracks[s.sel]!;
  const pitches = tr.notes.map((n) => n.pitch);
  const lo = Math.min(s.octave * 12 + 12, ...pitches);
  const hi = Math.max(lo + 23, ...pitches);
  const rows = Array.from({ length: hi - lo + 1 }, (_, i) => hi - i);
  const rh = Math.max(5, Math.min(10, 180 / rows.length));
  return (
    <div style={{ flex: 1, minWidth: 0, overflow: "auto", maxHeight: 200 }}>
      {rows.map((p) => (
        <div key={p} style={{ display: "flex", height: rh, background: NAMES[p % 12]!.includes("#") ? "#0e0e20" : "#14142a" }}>
          <div style={{ width: 34, fontSize: 8, color: C.dim, flexShrink: 0 }}>{p % 12 === 0 ? noteName(p) : ""}</div>
          {Array.from({ length: STEPS }, (_, st) => {
            const n = tr.notes.find((x) => x.pitch === p && x.step <= st && st < x.step + x.len);
            return (
              <div key={st} onClick={() => s.setCursor(st)} style={{
                flex: 1, minWidth: 6, borderLeft: `1px solid ${st % 4 === 0 ? "#2a2a44" : "transparent"}`,
                background: st >= tr.length ? "#050510" : n ? C.note
                  : st === s.cursor ? (s.armed ? "#4a1a22" : "#262640")
                  : s.playing && st === s.tick % tr.length ? "#203020" : undefined,
                opacity: n ? 0.4 + n.vel / 211 : 1,
              }} />
            );
          })}
        </div>
      ))}
    </div>
  );
}

function VariantC({ s }: { s: ProtoSeq }) {
  const [open, setOpen] = useState(true);
  return (
    <div style={{ flexShrink: 0, background: C.panel, borderTop: `1px solid ${C.line}`, padding: "6px 8px", display: "flex", flexDirection: "column", gap: 6 }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <Transport s={s} />
        <span style={{ fontSize: 12, color: "#fff" }}>Track {partLabel(s, s.sel)}</span>
        <button style={btn()} onClick={() => setOpen(!open)}>{open ? "▾" : "▴"}</button>
      </div>
      {open && (
        <>
          <div style={{ display: "flex", gap: 10 }}>
            <Overview s={s} />
            <PianoRoll s={s} />
          </div>
          <div style={{ paddingLeft: 244 }}><VelocityLaneC s={s} /></div>
          <Piano s={s} octaves={3} height={48} />
        </>
      )}
    </div>
  );
}

function VelocityLaneC({ s }: { s: ProtoSeq }) {
  return <div style={{ marginLeft: -110 }}><VelocityLane s={s} height={36} /></div>;
}

// ---------------------------------------------------------------------------
// Mounting + switcher
// ---------------------------------------------------------------------------

export const VARIANTS = { A: "Bottom dock", B: "Sequencer tab", C: "Focus drawer" } as const;
export type Variant = keyof typeof VARIANTS;

export function protoVariant(): Variant | null {
  if (!import.meta.env.DEV) return null;
  const v = new URLSearchParams(location.search).get("variant");
  return v && v in VARIANTS ? (v as Variant) : null;
}

/** Rendered at the bottom of the app shell, under whatever page is active. */
export function SequencerPrototype({ s, variant, onSeqTab }: { s: ProtoSeq; variant: Variant; onSeqTab: boolean }) {
  return (
    <>
      {variant === "A" && <VariantA s={s} />}
      {variant === "B" && !onSeqTab && <VariantBMini s={s} />}
      {variant === "C" && <VariantC s={s} />}
      <Switcher current={variant} />
    </>
  );
}

function Switcher({ current }: { current: Variant }) {
  const keys = Object.keys(VARIANTS) as Variant[];
  const go = (d: number) => {
    const next = keys[(keys.indexOf(current) + d + keys.length) % keys.length]!;
    const u = new URL(location.href);
    u.searchParams.set("variant", next);
    location.replace(u);
  };
  return (
    <div style={{ position: "fixed", bottom: 12, left: "50%", transform: "translateX(-50%)", zIndex: 100, background: "#fff", color: "#000", borderRadius: 999, padding: "4px 12px", display: "flex", gap: 10, alignItems: "center", fontSize: 12, fontWeight: 600, boxShadow: "0 4px 16px #000c" }}>
      <button onClick={() => go(-1)}>←</button>
      PROTOTYPE {current} — {VARIANTS[current]}
      <button onClick={() => go(1)}>→</button>
    </div>
  );
}
