// PROTOTYPE ONLY (throwaway). Variant C: group dropdown + ‹ name › stepper;
// L and R each get a stepper row and a popover (search + derived name-family
// chips); "pair" on L sets R to its Stereo partner.
import { useMemo, useRef, useState } from "react";
import css from "./prototype.module.css";
import { Floating } from "./Floating.prototype";
import { RawState } from "./RawState.prototype";
import { RWarning } from "./RWarning.prototype";
import type { WaveValue } from "./prototypeState";
import {
  BPM_FAMILY,
  GROUPS,
  families,
  familyOf,
  findGroup,
  groupLabel,
  groupTypeFor,
  matches,
  pad4,
  rPartnerOf,
  waveName,
  type WaveGroup,
} from "./waveData.prototype";

const MAX_CHIPS = 24;

function FamilyPopover({
  group,
  current,
  anchor,
  onPick,
  onClose,
}: {
  group: WaveGroup;
  current: number;
  anchor: React.RefObject<HTMLElement | null>;
  onPick: (n: number) => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [family, setFamily] = useState<string | null>(null);
  const chips = useMemo(() => {
    const all = families(group);
    const top = all.slice(0, MAX_CHIPS);
    const bpm = all.find((f) => f.name === BPM_FAMILY);
    if (bpm && !top.includes(bpm)) top.unshift(bpm);
    return top;
  }, [group]);
  const shown = group.waves
    .map((name, i) => ({ name, n: i + 1 }))
    .filter((w) => (family === null || familyOf(w.name) === family) && matches(w.name, query));

  return (
    <Floating anchor={anchor} onClose={onClose} width={280}>
      <input
        autoFocus
        className={css.search}
        placeholder={`Search ${group.name}…`}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />
      <div className={css.chips}>
        {chips.map((c) => (
          <span
            key={c.name}
            className={`${css.familyChip} ${family === c.name ? css.familyChipOn : ""}`}
            onClick={() => setFamily(family === c.name ? null : c.name)}
          >
            {c.name} <span className={css.dim}>{c.count}</span>
          </span>
        ))}
      </div>
      <div className={css.dim}>{shown.length} waves</div>
      <div className={css.list} style={{ maxHeight: 280 }}>
        <div
          className={`${css.item} ${current === 0 ? css.itemCurrent : ""}`}
          onClick={() => {
            onPick(0);
            onClose();
          }}
        >
          <span className={css.num}>0000</span>OFF
        </div>
        {shown.map((w) => (
          <div
            key={w.n}
            className={`${css.item} ${w.n === current ? css.itemCurrent : ""}`}
            onClick={() => {
              onPick(w.n);
              onClose();
            }}
          >
            <span className={css.num}>{pad4(w.n)}</span>
            {w.name}
          </div>
        ))}
      </div>
    </Floating>
  );
}

function StepRow({
  side,
  group,
  n,
  onPick,
  extra,
}: {
  side: "L" | "R";
  group: WaveGroup | undefined;
  n: number;
  onPick: (n: number) => void;
  extra: React.ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const anchor = useRef<HTMLButtonElement>(null);
  const count = group?.waves.length ?? 16384;
  return (
    <div className={css.row}>
      <span className={css.dim}>{side}</span>
      <button className={css.button} disabled={n <= 0} onClick={() => onPick(n - 1)}>‹</button>
      <button ref={anchor} className={css.collapsed} onClick={() => group && setOpen((v) => !v)}>
        {waveName(group, n)}
      </button>
      <button className={css.button} disabled={n >= count} onClick={() => onPick(n + 1)}>›</button>
      {extra}
      {open && group && (
        <FamilyPopover group={group} current={n} anchor={anchor} onPick={onPick} onClose={() => setOpen(false)} />
      )}
    </div>
  );
}

export function VariantC({ value, onChange }: { value: WaveValue; onChange: (v: WaveValue) => void }) {
  const group = findGroup(value.groupType, value.groupId);
  const partner = rPartnerOf(group, value.l);
  const isPair = value.r !== 0 && value.r === partner;

  return (
    <div className={css.picker}>
      <select
        className={css.select}
        value={group ? String(group.id) : "unknown"}
        onChange={(e) => {
          const g = Number(e.target.value);
          onChange({ groupType: groupTypeFor(g), groupId: g, l: 1, r: 0 });
        }}
      >
        {!group && <option value="unknown">{groupLabel(value.groupType, value.groupId)} (no names)</option>}
        {GROUPS.map((g) => (
          <option key={g.id} value={g.id}>{g.name} ({g.waves.length})</option>
        ))}
      </select>
      <StepRow
        side="L"
        group={group}
        n={value.l}
        onPick={(l) => onChange({ ...value, l })}
        extra={
          <button
            className={`${css.button} ${isPair ? css.buttonOn : ""}`}
            disabled={partner === null}
            title={partner === null ? "L is not the L half of a Stereo pair" : `Set R to ${waveName(group, partner)}`}
            onClick={() => partner !== null && onChange({ ...value, r: partner })}
          >
            pair
          </button>
        }
      />
      <StepRow
        side="R"
        group={group}
        n={value.r}
        onPick={(r) => onChange({ ...value, r })}
        extra={
          <button className={css.button} title="R = OFF (mono)" disabled={value.r === 0}
            onClick={() => onChange({ ...value, r: 0 })}>
            ×
          </button>
        }
      />
      <RWarning value={value} />
      <RawState value={value} />
    </div>
  );
}
