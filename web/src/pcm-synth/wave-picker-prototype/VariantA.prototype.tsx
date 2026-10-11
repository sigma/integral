// PROTOTYPE ONLY (throwaway). Variant A: combobox with type-ahead over every
// Wave Group, section headers, keyboard nav, and a "Stereo" checkbox.
import { useEffect, useMemo, useRef, useState } from "react";
import css from "./prototype.module.css";
import { Floating } from "./Floating.prototype";
import { RawState } from "./RawState.prototype";
import type { WaveValue } from "./prototypeState";
import {
  GROUPS,
  findGroup,
  groupLabel,
  groupTypeFor,
  matches,
  pad4,
  rPartnerOf,
  waveName,
  type WaveGroup,
} from "./waveData.prototype";

interface Option {
  group: WaveGroup;
  n: number;
  name: string;
}

function Combobox({
  text,
  groups,
  current,
  onPick,
}: {
  text: string;
  groups: WaveGroup[];
  current: { groupId: number; n: number };
  onPick: (group: WaveGroup, n: number) => void;
}) {
  const anchor = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const activeRef = useRef<HTMLDivElement>(null);

  const options = useMemo(() => {
    const out: Option[] = [];
    for (const g of groups) {
      g.waves.forEach((name, i) => {
        if (matches(name, query) || pad4(i + 1).includes(query.trim())) out.push({ group: g, n: i + 1, name });
      });
    }
    return out;
  }, [groups, query]);

  useEffect(() => {
    if (!open) return;
    const i = options.findIndex((o) => o.group.id === current.groupId && o.n === current.n);
    setActive(Math.max(0, i));
  }, [open]);
  useEffect(() => {
    setActive(0);
  }, [query]);
  // Block body: scrollIntoView may return a Promise, which React would call as cleanup.
  useEffect(() => {
    activeRef.current?.scrollIntoView({ block: "nearest" });
  }, [active, open]);

  const pick = (o: Option | undefined) => {
    if (!o) return;
    onPick(o.group, o.n);
    setOpen(false);
    setQuery("");
  };

  return (
    <>
      <button ref={anchor} className={css.collapsed} onClick={() => setOpen((v) => !v)}>
        {text}
      </button>
      {open && (
        <Floating anchor={anchor} onClose={() => setOpen(false)} width={300}>
          <input
            autoFocus
            className={css.search}
            placeholder="Type to filter (name or number)…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") {
                e.preventDefault();
                setActive((a) => Math.min(options.length - 1, a + 1));
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setActive((a) => Math.max(0, a - 1));
              } else if (e.key === "Enter") {
                pick(options[active]);
              }
            }}
          />
          <div className={css.dim}>{options.length} matches</div>
          <div className={css.list} style={{ maxHeight: 360 }}>
            {options.map((o, i) => {
              const header = i === 0 || options[i - 1]?.group !== o.group;
              const isCur = o.group.id === current.groupId && o.n === current.n;
              return (
                <div key={`${o.group.id}-${o.n}`}>
                  {header && <div className={css.groupHeader}>{o.group.name}</div>}
                  <div
                    ref={i === active ? activeRef : undefined}
                    className={`${css.item} ${i === active ? css.itemActive : ""} ${isCur ? css.itemCurrent : ""}`}
                    onMouseEnter={() => setActive(i)}
                    onClick={() => pick(o)}
                  >
                    <span className={css.num}>{pad4(o.n)}</span>
                    {o.name}
                  </div>
                </div>
              );
            })}
          </div>
        </Floating>
      )}
    </>
  );
}

export function VariantA({ value, onChange }: { value: WaveValue; onChange: (v: WaveValue) => void }) {
  const group = findGroup(value.groupType, value.groupId);
  const [stereo, setStereo] = useState(() => value.r !== 0 && value.r === rPartnerOf(group, value.l));
  const gl = groupLabel(value.groupType, value.groupId);
  const line = (n: number) => `${gl} ${pad4(n)} ${waveName(group, n)}`;

  const pickL = (g: WaveGroup, n: number, st = stereo) => {
    // Stereo: R follows the L/R name. Mono: keep R only within the same group.
    const r = st ? (rPartnerOf(g, n) ?? 0) : g.id === value.groupId ? value.r : 0;
    onChange({ groupType: groupTypeFor(g.id), groupId: g.id, l: n, r });
  };

  return (
    <div className={css.picker}>
      <div className={css.row}>
        <span className={css.dim}>{stereo ? "L+R" : "L"}</span>
        <Combobox text={line(value.l)} groups={GROUPS} current={{ groupId: value.groupId, n: value.l }} onPick={pickL} />
      </div>
      {!stereo && (
        <div className={css.row}>
          <span className={css.dim}>R</span>
          <Combobox
            text={line(value.r)}
            groups={group ? [group] : []}
            current={{ groupId: value.groupId, n: value.r }}
            onPick={(_g, n) => onChange({ ...value, r: n })}
          />
          <button className={css.button} title="R = OFF" onClick={() => onChange({ ...value, r: 0 })}>×</button>
        </div>
      )}
      {stereo && <div className={css.dim}>R: {value.r === 0 ? "OFF (mono wave)" : line(value.r)}</div>}
      <label className={css.row}>
        <input
          type="checkbox"
          checked={stereo}
          onChange={(e) => {
            setStereo(e.target.checked);
            if (group) pickL(group, value.l, e.target.checked);
          }}
        />
        Stereo (auto-pick R from L/R name)
      </label>
      <RawState value={value} />
    </div>
  );
}
