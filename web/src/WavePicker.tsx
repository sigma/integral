/**
 * Picks the Wave of a PCM Synth partial, a PCM Drum WMT or an SN Synth OSC.
 *
 * In stereo mode: a Wave Group dropdown and two independent rows, `L name`
 * and `R name`, with a "pair" shortcut on L, a clear (×) on R and a warning
 * when L is OFF but R isn't. Otherwise a single name in the given group.
 * Clicking a name opens a popover with search, family chips and an OFF
 * entry. Names, families and Stereo pairs come from core.
 */

import { useMemo, useRef, useState, type ReactNode } from "react";
import type { Expansion } from "./expansionCatalog";
import { LoadExpansionButton, NotLoadedMarker } from "./NotLoadedMarker";
import { Floating, SearchPopover, pad4, type PickOption, type SearchPopoverProps } from "./SearchPopover";
import { pcmWaveSources, type WaveSource } from "./waveCatalog";
import pop from "./SearchPopover.module.css";
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

/** Listed above every search: Wave Number 0. */
const OFF: readonly PickOption[] = [{ value: 0, label: "OFF" }];

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
        number={l}
        onPick={(n) => onChange("l", n)}
      >
        {stereo && (
          <button
            type="button"
            className={`${css.button} ${css.iconButton} ${r !== 0 && r === partner ? css.buttonOn : ""}`}
            aria-label="Pair"
            disabled={partner === null}
            title={
              partner === null
                ? "L is not the L half of a Stereo pair"
                : `Set R to ${source?.name(partner) ?? partner}`
            }
            onClick={() => partner !== null && onChange("r", partner)}
          >
            <svg className={css.icon} viewBox="0 0 24 24" fill="none" stroke="currentColor"
              strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
              <path d="M10 13a5 5 0 0 0 7.5.5l3-3a5 5 0 0 0-7-7l-1.7 1.7" />
              <path d="M14 11a5 5 0 0 0-7.5-.5l-3 3a5 5 0 0 0 7 7l1.7-1.7" />
            </svg>
          </button>
        )}
      </WaveRow>
      {stereo && (
        <WaveRow
          side="R"
          source={source}
            number={r}
          onPick={(n) => onChange("r", n)}
        >
          <button
            type="button"
            className={`${css.button} ${css.iconButton}`}
            aria-label="Clear R"
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

/** One Wave's number and name; clicking it opens the popover. */
function WaveRow({
  side,
  source,
  number,
  onPick,
  children,
}: {
  side: "L" | "R" | null;
  source: WaveSource | null;
  number: number;
  onPick: (number: number) => void;
  children?: ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const anchor = useRef<HTMLButtonElement>(null);
  const name = source?.name(number) ?? "?";
  return (
    <div className={css.row}>
      {side && <span className={css.side}>{side}</span>}
      <button
        type="button"
        ref={anchor}
        className={css.collapsed}
        title={name}
        disabled={!source}
        onClick={() => setOpen((v) => !v)}
      >
        {`${pad4(number)} ${name}`}
      </button>
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
  ...rest
}: { source: WaveSource } & Pick<SearchPopoverProps, "current" | "anchor" | "onPick" | "onClose">) {
  const options = useMemo(
    () => source.waves.map((w) => ({ value: w.number, label: w.name, family: w.family })),
    [source],
  );
  return (
    <SearchPopover
      {...rest}
      options={options}
      pinned={OFF}
      families={source.families}
      placeholder={`Search ${source.id}…`}
      noun="Waves"
      numbered
    />
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
          <div className={pop.list} role="listbox">
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
                    className={`${pop.item} ${s === current ? pop.itemCurrent : ""}`}
                    onClick={() => {
                      onPick(s);
                      setOpen(false);
                    }}
                  >
                    {s.id} <span className={pop.dim}>{s.waves.length}</span>
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
