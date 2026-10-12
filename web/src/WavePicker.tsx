/**
 * Picks the Wave of a PCM Synth partial, a PCM Drum WMT or an SN Synth OSC.
 *
 * In stereo mode: a Wave Group dropdown and two independent rows,
 * `L ‹ name ›` and `R ‹ name ›`, with a "pair" shortcut on L, a clear (×)
 * on R and a warning when L is OFF but R isn't. Otherwise a single, larger
 * name button in the given group, without the group label or steppers.
 * Clicking a name opens a popover with search, family chips and an OFF
 * entry. Names, families and Stereo pairs come from core.
 */

import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
  type RefObject,
} from "react";
import { createPortal } from "react-dom";
import type { Expansion } from "./expansionCatalog";
import { LoadExpansionButton, NotLoadedMarker } from "./NotLoadedMarker";
import { pcmWaveSources, type WaveSource } from "./waveCatalog";
import css from "./WavePicker.module.css";

/** Loaded state of the Expansions behind SRX Wave Groups. */
export interface ExpansionMarking {
  /** Whether `expansion` is known not to be loaded (false while unknown). */
  isUnloaded: (expansion: Expansion | null) => boolean;
  /** Open the Expansions page with `expansion` staged. */
  onLoad: (expansion: Expansion) => void;
}

/** Which of a Wave pair a row picks. */
export type WaveSide = "l" | "r";

interface Props {
  /** The Waves to pick from; null when the device reports a reserved group. */
  source: WaveSource | null;
  /** Shown when `source` is null (e.g. "Type 2 ID 0"). */
  unknownGroupLabel?: string;
  /** Two rows (L, R) and a Wave Group dropdown, or a single row. */
  stereo: boolean;
  /** Wave Number L (the only one when not stereo); 0 is OFF. */
  l: number;
  /** Wave Number R; 0 is OFF. Ignored when not stereo. */
  r?: number;
  /** A row picked Wave Number `number`. */
  onChange: (side: WaveSide, number: number) => void;
  /** The dropdown picked another group (stereo only). */
  onGroupChange?: (source: WaveSource) => void;
  /** Loaded state of SRX Expansions; no markers when absent. */
  marking?: ExpansionMarking;
}

/** Families shown as chips: the largest ones, with at least this many Waves. */
const MIN_CHIP_SIZE = 4;
const MAX_CHIPS = 24;
const POPOVER_WIDTH = 280;

function pad4(n: number): string {
  return String(n).padStart(4, "0");
}

export function WavePicker({
  source,
  unknownGroupLabel = "?",
  stereo,
  l,
  r = 0,
  onChange,
  onGroupChange,
  marking,
}: Props) {
  const groupLabel = source?.id ?? unknownGroupLabel;
  const partner = source?.stereoPartner(l) ?? null;
  const unloaded =
    source?.expansion && marking?.isUnloaded(source.expansion)
      ? source.expansion
      : null;

  return (
    <div className={css.picker}>
      {stereo && (
        <div className={css.row}>
          <GroupDropdown
            current={source}
            label={groupLabel}
            onPick={(s) => onGroupChange?.(s)}
            marking={marking}
          />
          {unloaded && marking && (
            <>
              <NotLoadedMarker expansion={unloaded.name} named />
              <LoadExpansionButton
                expansion={unloaded}
                onLoad={(e) => marking.onLoad(e)}
              />
            </>
          )}
        </div>
      )}
      <WaveRow
        side={stereo ? "L" : null}
        source={source}
        groupLabel={groupLabel}
        number={l}
        onPick={(n) => onChange("l", n)}
      >
        {stereo && (
          <button
            type="button"
            className={`${css.button} ${r !== 0 && r === partner ? css.buttonOn : ""}`}
            disabled={partner === null}
            title={
              partner === null
                ? "L is not the L half of a Stereo pair"
                : `Set R to ${source?.name(partner) ?? partner}`
            }
            onClick={() => partner !== null && onChange("r", partner)}
          >
            pair
          </button>
        )}
      </WaveRow>
      {stereo && (
        <WaveRow
          side="R"
          source={source}
          groupLabel={groupLabel}
          number={r}
          onPick={(n) => onChange("r", n)}
        >
          <button
            type="button"
            className={css.button}
            title="R = OFF (mono)"
            disabled={r === 0}
            onClick={() => onChange("r", 0)}
          >
            ×
          </button>
        </WaveRow>
      )}
      {stereo && l === 0 && r !== 0 && (
        <div className={css.warning} role="alert">
          L is OFF: R alone produces no sound
        </div>
      )}
    </div>
  );
}

/**
 * One `‹ group name ›` stepper for a stereo side, or the bare name when
 * `side` is null; clicking the name opens the popover.
 */
function WaveRow({
  side,
  source,
  groupLabel,
  number,
  onPick,
  children,
}: {
  side: "L" | "R" | null;
  source: WaveSource | null;
  groupLabel: string;
  number: number;
  onPick: (number: number) => void;
  children?: ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const anchor = useRef<HTMLButtonElement>(null);
  const count = source?.waves.length ?? 0;
  const name = source?.name(number) ?? "?";
  return (
    <div className={css.row}>
      {side && <span className={css.side}>{side}</span>}
      {side && (
        <button
          type="button"
          className={css.button}
          aria-label="Previous Wave"
          disabled={number <= 0}
          onClick={() => onPick(number - 1)}
        >
          ‹
        </button>
      )}
      <button
        type="button"
        ref={anchor}
        className={`${css.collapsed} ${side ? "" : css.collapsedSingle}`}
        disabled={!source}
        onClick={() => setOpen((v) => !v)}
      >
        {side ? `${groupLabel} ${pad4(number)} ${name}` : `${pad4(number)} ${name}`}
      </button>
      {side && (
        <button
          type="button"
          className={css.button}
          aria-label="Next Wave"
          disabled={number >= count}
          onClick={() => onPick(number + 1)}
        >
          ›
        </button>
      )}
      {children}
      {open && source && (
        <WavePopover
          source={source}
          current={number}
          anchor={anchor}
          onPick={(n) => {
            onPick(n);
            setOpen(false);
          }}
          onClose={() => setOpen(false)}
        />
      )}
    </div>
  );
}

/** Search box, family chips and the (filtered) Waves of one group. */
function WavePopover({
  source,
  current,
  anchor,
  onPick,
  onClose,
}: {
  source: WaveSource;
  current: number;
  anchor: RefObject<HTMLElement | null>;
  onPick: (number: number) => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [family, setFamily] = useState<string | null>(null);
  const chips = useMemo(() => {
    const counts = new Map<string, number>();
    for (const w of source.waves) {
      counts.set(w.family, (counts.get(w.family) ?? 0) + 1);
    }
    return source.families
      .map((name) => ({ name, count: counts.get(name) ?? 0 }))
      .filter((c) => c.count >= MIN_CHIP_SIZE)
      .sort((a, b) => b.count - a.count)
      .slice(0, MAX_CHIPS);
  }, [source]);
  const needle = query.trim().toLowerCase();
  const shown = source.waves.filter(
    (w) =>
      (family === null || w.family === family) &&
      w.name.toLowerCase().includes(needle),
  );

  return (
    <Floating anchor={anchor} onClose={onClose}>
      <input
        autoFocus
        className={css.search}
        placeholder={`Search ${source.id}…`}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />
      <div className={css.chips}>
        {chips.map((c) => (
          <button
            type="button"
            key={c.name}
            className={`${css.chip} ${family === c.name ? css.chipOn : ""}`}
            aria-pressed={family === c.name}
            onClick={() => setFamily(family === c.name ? null : c.name)}
          >
            {c.name} <span className={css.dim}>{c.count}</span>
          </button>
        ))}
      </div>
      <div className={css.dim}>{shown.length} Waves</div>
      <div className={css.list} role="listbox">
        <WaveItem number={0} name="OFF" current={current} onPick={onPick} />
        {shown.map((w) => (
          <WaveItem
            key={w.number}
            number={w.number}
            name={w.name}
            current={current}
            onPick={onPick}
          />
        ))}
      </div>
    </Floating>
  );
}

function WaveItem({
  number,
  name,
  current,
  onPick,
}: {
  number: number;
  name: string;
  current: number;
  onPick: (number: number) => void;
}) {
  return (
    <button
      type="button"
      role="option"
      aria-selected={number === current}
      className={`${css.item} ${number === current ? css.itemCurrent : ""}`}
      onClick={() => onPick(number)}
    >
      <span className={css.dim}>{pad4(number)}</span>
      {name}
    </button>
  );
}

/** The Wave Group dropdown: every PCM group, marking unloaded ones. */
function GroupDropdown({
  current,
  label,
  onPick,
  marking,
}: {
  current: WaveSource | null;
  label: string;
  onPick: (source: WaveSource) => void;
  marking?: ExpansionMarking;
}) {
  const [open, setOpen] = useState(false);
  const anchor = useRef<HTMLButtonElement>(null);
  return (
    <>
      <button
        type="button"
        ref={anchor}
        className={css.select}
        aria-haspopup="listbox"
        aria-expanded={open}
        title="Wave Group"
        onClick={() => setOpen((v) => !v)}
      >
        {label} ▾
      </button>
      {open && (
        <Floating anchor={anchor} onClose={() => setOpen(false)}>
          <div className={css.list} role="listbox">
            {pcmWaveSources().map((s) => {
              const unloaded =
                s.expansion && marking?.isUnloaded(s.expansion)
                  ? s.expansion
                  : null;
              return (
                <div key={s.id} className={css.groupItem}>
                  <button
                    type="button"
                    role="option"
                    aria-selected={s === current}
                    className={`${css.item} ${s === current ? css.itemCurrent : ""}`}
                    onClick={() => {
                      onPick(s);
                      setOpen(false);
                    }}
                  >
                    {s.id} <span className={css.dim}>{s.waves.length}</span>
                  </button>
                  {unloaded && marking && (
                    <>
                      <NotLoadedMarker expansion={unloaded.name} />
                      <LoadExpansionButton
                        expansion={unloaded}
                        onLoad={(e) => marking.onLoad(e)}
                      />
                    </>
                  )}
                </div>
              );
            })}
          </div>
        </Floating>
      )}
    </>
  );
}

/**
 * A popover anchored under `anchor`, in a portal so a panel's overflow
 * doesn't clip it. Closes on an outside pointer press or Escape.
 */
function Floating({
  anchor,
  onClose,
  children,
}: {
  anchor: RefObject<HTMLElement | null>;
  onClose: () => void;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const onDown = (e: PointerEvent) => {
      const target = e.target;
      if (!(target instanceof Node)) return;
      if (ref.current?.contains(target) || anchor.current?.contains(target)) {
        return;
      }
      onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("pointerdown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("pointerdown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [anchor, onClose]);

  const rect = anchor.current?.getBoundingClientRect();
  const top = (rect?.bottom ?? 0) + 2;
  const left = Math.max(
    0,
    Math.min(rect?.left ?? 0, window.innerWidth - POPOVER_WIDTH - 8),
  );
  return createPortal(
    <div
      ref={ref}
      className={css.floating}
      style={{
        top,
        left,
        width: POPOVER_WIDTH,
        maxHeight: window.innerHeight - top - 16,
      }}
    >
      {children}
    </div>,
    document.body,
  );
}
