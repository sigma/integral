import { useEffect } from "react";
import { expansionSlotOff } from "../pkg/integral_wasm.js";
import {
  expansionByLoadValue,
  expansionCatalog,
  expansionSlotNames,
  type Expansion,
} from "./expansionCatalog";
import { NotLoadedMarker } from "./NotLoadedMarker";
import type { UseExpansionsResult } from "./useExpansions";
import type {
  ExpansionLoadNotice,
  UseExpansionStagingResult,
} from "./useExpansionStaging";
import css from "./ExpansionsPage.module.css";

interface Props {
  expansions: UseExpansionsResult;
  staging: UseExpansionStagingResult;
}

/** Catalog Expansions grouped by family, in catalog order. */
function catalogGroups(): { label: string; expansions: Expansion[] }[] {
  const groups: { label: string; expansions: Expansion[] }[] = [];
  for (const expansion of expansionCatalog()) {
    const last = groups[groups.length - 1];
    if (last?.label === expansion.kindLabel) {
      last.expansions.push(expansion);
    } else {
      groups.push({ label: expansion.kindLabel, expansions: [expansion] });
    }
  }
  return groups;
}

/** "OFF" or the name of the Expansion a slot value selects. */
function slotLabel(value: number): string {
  return expansionByLoadValue(value)?.name ?? "OFF";
}

/** The Expansion slots A–D and every Expansion, to stage and apply a load. */
export function ExpansionsPage({ expansions, staging }: Props) {
  const { slots } = expansions;
  const { selectedSlot, staged, changedSlots } = staging;
  const slotNames = expansionSlotNames();
  const shown = staged ?? slots;

  // Opening the page re-reads which Expansions are loaded.
  const { refresh } = expansions;
  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <div className={css.page}>
      <div className={css.slotStrip}>
        {slotNames.map((name, i) => {
          const value = shown?.[i];
          const changed = changedSlots.includes(i);
          return (
            <button
              key={name}
              className={`${css.slot} ${selectedSlot === i ? css.slotSelected : ""} ${changed ? css.slotStaged : ""}`}
              onClick={() => staging.selectSlot(i)}
              disabled={slots === null}
            >
              <span className={css.slotName}>Slot {name}</span>
              <span className={css.slotValue}>
                {value === undefined ? "?" : slotLabel(value)}
              </span>
              <span className={css.slotTitle}>
                {value !== undefined && expansionByLoadValue(value)?.title}
                {changed && slots && (
                  <em className={css.was}> (was {slotLabel(slots[i]!)})</em>
                )}
              </span>
            </button>
          );
        })}
      </div>

      {slots === null && (
        <div className={css.hint}>
          Expansion slots unknown: no answer from the INTEGRA-7. Nothing can be
          staged until they are read.
        </div>
      )}
      <LoadNotice notice={staging.notice} />

      {changedSlots.length > 0 && (
        <div className={css.applyBar}>
          <span className={css.applyNote}>
            All sound stops while loading (~5 s per Expansion).
          </span>
          {staging.confirming && (
            <span className={css.warn}>
              Parts {staging.confirming.join(", ")} will go silent.
            </span>
          )}
          <button className={css.primary} onClick={staging.apply}>
            {staging.confirming ? "Load anyway" : "Apply"}
          </button>
          <button className={css.secondary} onClick={staging.discard}>
            Discard
          </button>
        </div>
      )}

      <div className={css.catalog}>
        <div className={css.hint}>
          Pick an Expansion for slot {slotNames[selectedSlot]}:
        </div>
        <CatalogRow
          name="OFF"
          active={shown?.[selectedSlot] === expansionSlotOff()}
          disabled={slots === null}
          onPick={() => staging.stage(expansionSlotOff())}
        />
        {catalogGroups().map((group) => (
          <div key={group.label}>
            <div className={css.groupLabel}>{group.label}</div>
            {group.expansions.map((expansion) => (
              <CatalogRow
                key={expansion.name}
                name={expansion.name}
                expansion={expansion}
                slots={slots}
                parts={expansions.partsUsing(expansion)}
                active={shown?.[selectedSlot] === expansion.loadValue}
                disabled={slots === null}
                onPick={() => staging.stage(expansion.loadValue)}
              />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

/** One pickable row of the catalog: an Expansion, or OFF. */
function CatalogRow({
  name,
  expansion,
  slots,
  parts = [],
  active,
  disabled,
  onPick,
}: {
  name: string;
  expansion?: Expansion;
  slots?: readonly number[] | null;
  parts?: number[];
  active: boolean;
  disabled: boolean;
  onPick: () => void;
}) {
  const loadedAt = expansion && slots ? slots.indexOf(expansion.loadValue) : -1;
  return (
    <button
      className={`${css.row} ${active ? css.rowActive : ""}`}
      onClick={onPick}
      disabled={disabled}
    >
      <span className={css.rowName}>{name}</span>
      <span className={css.rowTitle}>{expansion?.title}</span>
      {expansion && expansion.slotCost > 1 && (
        <span className={css.tag}>{expansion.slotCost} slots</span>
      )}
      {parts.length > 0 && (
        <span className={css.tag}>Parts {parts.join(", ")}</span>
      )}
      {expansion && slots && (
        loadedAt >= 0 ? (
          <span className={css.loaded}>
            loaded · {loadedSlots(loadedAt, expansion.slotCost)}
          </span>
        ) : (
          <NotLoadedMarker expansion={expansion.name} />
        )
      )}
    </button>
  );
}

/** "B", or "A–D" for an Expansion taking several slots from `first`. */
function loadedSlots(first: number, cost: number): string {
  const names = expansionSlotNames();
  return cost > 1 ? `${names[first]}–${names[first + cost - 1]}` : names[first]!;
}

/** Tells how the last load ended when it didn't load what was staged. */
function LoadNotice({ notice }: { notice: ExpansionLoadNotice | null }) {
  if (!notice) return null;
  const text =
    notice.kind === "failed"
      ? "The load didn't complete: the slots above are what the INTEGRA-7 reads back."
      : notice.kind === "timedOut"
        ? "No answer from the INTEGRA-7 during the load: the slots above are what it reads back now."
        : `The load couldn't start: ${notice.message}`;
  return <div className={css.notice}>{text}</div>;
}
