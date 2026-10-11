// PROTOTYPE ONLY (throwaway). Variant B: chip + name, "browse" opens a
// two-pane modal (Wave Groups | searchable list with merged Stereo pairs).
import { useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import css from "./prototype.module.css";
import { RawState } from "./RawState.prototype";
import { RWarning } from "./RWarning.prototype";
import type { WaveValue } from "./prototypeState";
import {
  GROUPS,
  findGroup,
  groupLabel,
  groupTypeFor,
  matches,
  mergedEntries,
  pad4,
  pairLabel,
  rPartnerOf,
  waveName,
} from "./waveData.prototype";

function Browser({
  value,
  onChange,
  onClose,
}: {
  value: WaveValue;
  onChange: (v: WaveValue) => void;
  onClose: () => void;
}) {
  const [groupId, setGroupId] = useState(() => findGroup(value.groupType, value.groupId)?.id ?? 0);
  const [query, setQuery] = useState("");
  const [target, setTarget] = useState<"L" | "R">("L");
  const group = GROUPS.find((g) => g.id === groupId);
  const curGroup = findGroup(value.groupType, value.groupId);
  // One Wave Group per partial: picking in another group clears the other side.
  const assign = (n: number) => {
    const same = groupId === value.groupId;
    const base = { groupType: groupTypeFor(groupId), groupId };
    if (target === "L") onChange({ ...base, l: n, r: same ? value.r : 0 });
    else onChange({ ...base, l: same ? value.l : 0, r: n });
  };
  const entries = useMemo(() => (group ? mergedEntries(group) : []), [group]);
  const shown = entries.filter((e) => matches(e.label, query));
  const curRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    curRef.current?.scrollIntoView({ block: "center" });
  }, [groupId]);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return createPortal(
    <div className={css.backdrop} onPointerDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className={css.modal}>
        <div className={css.modalHeader} style={{ flexWrap: "wrap" }}>
          <strong>Wave browser</strong>
          <span className={css.dim}>Assign to:</span>
          {(["L", "R"] as const).map((t) => (
            <button
              key={t}
              className={`${css.button} ${target === t ? css.buttonOn : ""}`}
              onClick={() => setTarget(t)}
            >
              {t}
            </button>
          ))}
          <span>
            L: <span className={css.itemCurrent}>{waveName(curGroup, value.l)}</span>
            {"  "}R: <span className={css.itemCurrent}>{waveName(curGroup, value.r)}</span>
          </span>
          <RWarning value={value} />
          <input
            autoFocus
            className={css.search}
            style={{ flex: 1 }}
            placeholder={`Search ${group?.name ?? ""}…`}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <span className={css.dim}>{shown.length} rows</span>
          <button className={css.button} onClick={onClose}>Done</button>
        </div>
        <div className={css.groupPane}>
          {GROUPS.map((g) => {
            const pairs = mergedEntries(g).filter((e) => e.r !== 0).length;
            return (
              <div
                key={g.id}
                className={`${css.groupItem} ${g.id === groupId ? css.itemActive : ""}`}
                onClick={() => setGroupId(g.id)}
              >
                <span>{g.name}</span>
                <span className={css.dim} title={`${pairs} Stereo pairs`}>{g.waves.length}</span>
              </div>
            );
          })}
          <div className={css.groupItem} style={{ cursor: "default" }}>
            <span className={css.dim}>(other SRX: no names in prototype)</span>
          </div>
        </div>
        <div className={css.list}>
          <div
            className={css.item}
            onClick={() => assign(0)}
          >
            <span className={css.num}>0000</span>OFF (clear {target})
          </div>
          {shown.map((e) => {
            const inGroup = group?.id === value.groupId;
            const isCur = inGroup && (e.l === value.l || e.l === value.r || (e.r !== 0 && e.r === value.r));
            return (
              <div
                key={e.l}
                ref={isCur ? curRef : undefined}
                className={`${css.item} ${isCur ? css.itemCurrent : ""}`}
                title={e.r ? "Stereo pair: sets L and R" : `Sets ${target}`}
                onClick={() =>
                  e.r ? onChange({ groupType: groupTypeFor(groupId), groupId, l: e.l, r: e.r }) : assign(e.l)
                }
                onDoubleClick={onClose}
              >
                <span className={css.num}>{e.r ? `${pad4(e.l)}/${pad4(e.r)}` : `${pad4(e.l)}     `}</span>
                {e.label}
                {e.r !== 0 && <span className={css.badge}>STEREO</span>}
              </div>
            );
          })}
        </div>
      </div>
    </div>,
    document.body,
  );
}

export function VariantB({ value, onChange }: { value: WaveValue; onChange: (v: WaveValue) => void }) {
  const [open, setOpen] = useState(false);
  const group = findGroup(value.groupType, value.groupId);
  const isPair = value.r !== 0 && value.r === rPartnerOf(group, value.l);
  const name = isPair ? pairLabel(waveName(group, value.l)) : waveName(group, value.l);
  return (
    <div className={css.picker}>
      <div className={css.row}>
        <span className={css.chip}>{groupLabel(value.groupType, value.groupId)}</span>
        {isPair && <span className={css.badge}>ST</span>}
      </div>
      <div className={css.row}>
        <button className={css.collapsed} onClick={() => setOpen(true)} title="Open Wave browser">
          {name}
        </button>
        <button className={css.button} onClick={() => setOpen(true)}>browse…</button>
      </div>
      {!isPair && <div className={css.dim}>R: {waveName(group, value.r)}</div>}
      <RWarning value={value} />
      <RawState value={value} />
      {open && <Browser value={value} onChange={onChange} onClose={() => setOpen(false)} />}
    </div>
  );
}
