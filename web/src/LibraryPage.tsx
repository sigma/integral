import { useState, type DragEvent } from "react";
import {
  isFree,
  libraryType,
  libraryTypes,
  NO_SELECTION,
  selectSlot,
} from "./librarian";
import { PackExportPane } from "./PackExportPane";
import type { IntegraService } from "./integra";
import { PackImportPane } from "./PackImportPane";
import { ReinitDialog } from "./ReinitDialog";
import { SlotRow } from "./SlotRow";
import type { ToneCatalog } from "./toneCatalog";
import type { UseLibraryExportResult } from "./useLibraryExport";
import { placedSlot, type ImportEntry, type UseLibraryImportResult } from "./useLibraryImport";
import type { UseMixerResult } from "./useMixer";
import { useUserSlots } from "./useUserSlots";
import css from "./LibraryPage.module.css";

interface Props {
  service: IntegraService | null;
  mixer: UseMixerResult;
  catalog: ToneCatalog;
  libraryExport: UseLibraryExportResult;
  libraryImport: UseLibraryImportResult;
}

type Mode = "export" | "import";

/**
 * The Library: the device's User memory on the left; on the right, the
 * Pack being exported or the Pack file being imported.
 */
export function LibraryPage({ service, mixer, catalog, libraryExport, libraryImport: imp }: Props) {
  const types = libraryTypes();
  const [kind, setKind] = useState(() => types[0]?.kind ?? "");
  const type = libraryType(kind);
  const [mode, setMode] = useState<Mode>(() => (imp.file ? "import" : "export"));
  const [selection, setSelection] = useState(NO_SELECTION);
  const [reiniting, setReiniting] = useState(false);
  const { slots, loading } = useUserSlots(type, catalog, mixer);
  const selected = slots.filter((s) => selection.slots.has(s.slot));
  const exporting = libraryExport.run?.phase === "reading";
  const importing = mode === "import";

  const showType = (next: string): void => {
    setKind(next);
    setSelection(NO_SELECTION);
  };

  // Import: which entry lands in each slot of the shown type.
  const incoming = new Map<number, string>();
  if (importing) {
    for (const entry of imp.entries) {
      const slot = placedSlot(entry.placement);
      if (entry.type.kind === kind && slot !== undefined) incoming.set(slot, entry.info.name);
    }
  }
  const focused = imp.focused === null ? undefined : imp.entries[imp.focused];
  const focusedSlot = focused && placedSlot(focused.placement);
  const placing =
    importing && !imp.locked && focused?.type.kind === kind && focused.placement !== "excluded"
      ? focused
      : undefined;

  const focusEntry = (entry: ImportEntry): void => {
    imp.focus(entry.index);
    if (entry.type.kind !== kind) showType(entry.type.kind);
  };

  const dropFile = (e: DragEvent): void => {
    const file = e.dataTransfer.files[0];
    if (!file) return;
    e.preventDefault();
    if (imp.busy) return;
    imp.open(file);
    setMode("import");
  };

  return (
    <div
      className={css.page}
      onDragOver={(e) => {
        if (e.dataTransfer.types.includes("Files")) e.preventDefault();
      }}
      onDrop={dropFile}
    >
      <section className={css.pane}>
        <header className={css.head}>
          <span className={css.title}>INTEGRA-7 · User memory</span>
        </header>
        <div className={css.tabs}>
          {types.map((t) => (
            <button
              key={t.kind}
              className={`${css.tab} ${t.kind === kind ? css.tabActive : ""}`}
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
          {slots.map((slot) => {
            const free = isFree(type, slot.name);
            const into = incoming.get(slot.slot);
            const target = into === undefined ? "" : free ? css.intoFree : css.intoTaken;
            return (
              <SlotRow
                key={slot.key}
                slot={slot}
                active={
                  selection.slots.has(slot.slot) ||
                  (importing && focused?.type.kind === kind && focusedSlot === slot.slot)
                }
                className={`${free ? css.free : ""} ${target}`}
                onClick={(e) => {
                  if (placing) {
                    imp.assign(placing.index, slot.slot);
                    return;
                  }
                  setSelection((sel) =>
                    selectSlot(sel, slot.slot, {
                      range: e.shiftKey,
                      toggle: e.metaKey || e.ctrlKey,
                    }),
                  );
                }}
              >
                {into !== undefined && <span className={css.into}>← {into}</span>}
              </SlotRow>
            );
          })}
        </div>
        <footer className={css.foot}>
          <span className={css.hint}>
            {placing
              ? `Click a slot to place “${placing.info.name}”.`
              : importing && imp.file && selected.length === 0
                ? "Highlighted slots receive an entry. Select an entry on the right, then click a slot to place it."
                : `${selected.length} selected · click, ⇧-click range, ⌘-click toggle`}
          </span>
          <button
            className={css.button}
            disabled={!service || selected.length === 0 || imp.busy || exporting}
            title="Clear the selected slots back to INIT"
            onClick={() => setReiniting(true)}
          >
            Re-init…
          </button>
          {!importing && (
            <button
              className={css.primary}
              disabled={selected.length === 0 || exporting}
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
          )}
        </footer>
        {reiniting && service && (
          <ReinitDialog
            service={service}
            mixer={mixer}
            catalog={catalog}
            type={type}
            slots={selected}
            onClose={() => setReiniting(false)}
          />
        )}
      </section>
      <section className={css.pane}>
        <div className={css.tabs}>
          <button
            className={`${css.tab} ${mode === "export" ? css.tabActive : ""}`}
            onClick={() => setMode("export")}
          >
            Export
          </button>
          <button
            className={`${css.tab} ${importing ? css.tabActive : ""}`}
            onClick={() => setMode("import")}
          >
            Import
          </button>
        </div>
        {importing ? (
          <PackImportPane
            libraryImport={imp}
            slots={slots}
            slotsType={type}
            part={mixer.state.selectedPart}
            onFocus={focusEntry}
            busy={exporting}
          />
        ) : (
          <PackExportPane libraryExport={libraryExport} mixer={mixer} busy={imp.busy} />
        )}
      </section>
    </div>
  );
}
