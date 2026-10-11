import { useState } from "react";
import {
  isFree,
  libraryType,
  libraryTypes,
  NO_SELECTION,
  selectSlot,
} from "./librarian";
import { PackExportPane } from "./PackExportPane";
import { SlotRow } from "./SlotRow";
import type { ToneCatalog } from "./toneCatalog";
import type { UseLibraryExportResult } from "./useLibraryExport";
import type { UseMixerResult } from "./useMixer";
import { useUserSlots } from "./useUserSlots";
import css from "./LibraryPage.module.css";

interface Props {
  mixer: UseMixerResult;
  catalog: ToneCatalog;
  libraryExport: UseLibraryExportResult;
}

/**
 * The Library: the device's User memory on the left, the Pack being
 * exported on the right.
 */
export function LibraryPage({ mixer, catalog, libraryExport }: Props) {
  const types = libraryTypes();
  const [kind, setKind] = useState(() => types[0]?.kind ?? "");
  const type = libraryType(kind);
  const [selection, setSelection] = useState(NO_SELECTION);
  const { slots, loading } = useUserSlots(type, catalog, mixer);
  const selected = slots.filter((s) => selection.slots.has(s.slot));

  const showType = (next: string): void => {
    setKind(next);
    setSelection(NO_SELECTION);
  };

  return (
    <div className={css.page}>
      <section className={css.pane}>
        <header className={css.head}>
          <span className={css.title}>INTEGRA-7 · User memory</span>
        </header>
        <div className={css.typeTabs}>
          {types.map((t) => (
            <button
              key={t.kind}
              className={`${css.typeTab} ${t.kind === kind ? css.typeTabActive : ""}`}
              onClick={() => showType(t.kind)}
              title={t.label}
            >
              {t.short}
            </button>
          ))}
        </div>
        <div className={css.slotList}>
          {slots.length === 0 && (
            <div className={css.empty}>
              {loading ? "Loading slots..." : "No reply from the INTEGRA-7"}
            </div>
          )}
          {slots.map((slot) => (
            <SlotRow
              key={slot.key}
              slot={slot}
              active={selection.slots.has(slot.slot)}
              className={isFree(type, slot.name) ? css.free : undefined}
              onClick={(e) =>
                setSelection((sel) =>
                  selectSlot(sel, slot.slot, {
                    range: e.shiftKey,
                    toggle: e.metaKey || e.ctrlKey,
                  }),
                )
              }
            />
          ))}
        </div>
        <footer className={css.foot}>
          <span className={css.hint}>
            {selected.length} selected · click, ⇧-click range, ⌘-click toggle
          </span>
          <button
            className={css.primary}
            disabled={selected.length === 0 || libraryExport.run?.phase === "reading"}
            onClick={() => {
              libraryExport.add(
                selected.map((s) => ({
                  source: { source: "user", kind, slot: s.slot },
                  name: s.name,
                })),
              );
              setSelection(NO_SELECTION);
            }}
          >
            Add to Pack →
          </button>
        </footer>
      </section>
      <section className={css.pane}>
        <PackExportPane libraryExport={libraryExport} mixer={mixer} busy={false} />
      </section>
    </div>
  );
}
