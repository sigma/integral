import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Pack,
  planImport,
  type CatalogName,
  type ImportPlan,
  type ImportStatus,
  type ImportWarning,
  type PackEntryInfo,
  type Placement,
} from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import {
  downloadFile,
  libraryType,
  libraryTypes,
  packFileBase,
  PACK_EXTENSION,
  slotLabel,
  type LibraryType,
} from "./librarian";
import { toneSlotIndex } from "./toneBanks";
import type { ToneCatalog } from "./toneCatalog";
import type { UseMixerResult } from "./useMixer";

/** The User slot (0-indexed) a placement names, if any. */
export function placedSlot(placement: Placement): number | undefined {
  return typeof placement === "object" ? placement.user : undefined;
}

/** `set` with `item` added, or removed if it was in. */
export function toggled<T>(set: ReadonlySet<T>, item: T): ReadonlySet<T> {
  const next = new Set(set);
  if (!next.delete(item)) next.add(item);
  return next;
}

/** One entry of the opened Pack, with where it goes. */
export interface ImportEntry {
  /** Index into the Pack's entries. */
  index: number;
  info: PackEntryInfo;
  type: LibraryType;
  /** Where it sat: its slot in a backup, else its position in the file. */
  origin: string;
  placement: Placement;
  /** Its Conversion report, one message per item. */
  report: string[];
  /** How many of its report items are lossy: 0 when clean. */
  lossy: number;
  /** The model a Foreign patch was made on, when its bytes tell. */
  source: string | null;
}

/** A non-free User slot the import will overwrite. */
export interface Overwrite {
  entry: number;
  type: LibraryType;
  slot: number;
  /** The name the slot holds now, if known. */
  current: string;
  incoming: string;
}

/** An import run: the undo export, then storing, then the report. */
export interface ImportRun {
  phase: "undo" | "importing" | "done";
  /** A single entry loaded into Temporary, nothing stored. */
  temporary: boolean;
  /** The Part borrowed to store tones (or loaded into), 0-indexed. */
  part: number;
  /** Each entry's status, indexed like the Pack's entries. */
  statuses: ImportStatus[];
  /** The undo Pack downloaded before importing, if any. */
  undoFile?: string;
  error?: string;
}

export interface UseLibraryImportResult {
  /** The opened file and what it is ("sound export", "SysEx dump"...), if any. */
  file: { name: string; label: string } | null;
  opening: boolean;
  openError: string | null;
  entries: ImportEntry[];
  /**
   * What the file held that is not in the Pack, then warnings about the
   * entries being imported, naming each.
   */
  warnings: string[];
  /** Entries with no destination yet: they block the import. */
  unplaced: number;
  overwrites: Overwrite[];
  /** The lone entry goes to Temporary for audition. */
  temporary: boolean;
  /** The entry a slot click places. */
  focused: number | null;
  focus: (entry: number | null) => void;
  /** Entries whose Conversion report is shown. */
  reportShown: ReadonlySet<number>;
  /** Show or hide `entry`'s Conversion report. */
  toggleReport: (entry: number) => void;
  open: (file: File) => void;
  close: () => void;
  assign: (entry: number, slot: number) => void;
  /** Include or leave out these entries. */
  setIncluded: (entries: number[], included: boolean) => void;
  run: ImportRun | null;
  /** An import is running. */
  busy: boolean;
  /** Imported (or importing): the plan can't change until reopened. */
  locked: boolean;
  /** Import, first downloading the slots it overwrites if `undo`. */
  start: (undo: boolean) => void;
  cancel: () => void;
}

interface Opened {
  name: string;
  pack: Pack;
  plan: ImportPlan;
  entries: PackEntryInfo[];
  label: string;
  /** Conversion reports, indexed like `entries`. */
  reports: string[][];
  /** Lossy items per Conversion report, indexed like `entries`. */
  lossy: Uint32Array;
  /** Source models, indexed like `entries`. */
  sources: (string | null)[];
  fileReport: string[];
}

interface PlanView {
  placements: Placement[];
  unplaced: number[];
  overwrites: number[];
  warnings: ImportWarning[];
}

function readPlan(plan: ImportPlan): PlanView {
  return {
    placements: plan.placements(),
    unplaced: Array.from(plan.unplaced()),
    overwrites: Array.from(plan.overwrites()),
    warnings: plan.warnings(),
  };
}

function message(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/**
 * Read a Pack file (SVD, .syx or .mid) and plan its import against the User
 * slot names (tones from `catalog`, Studio Sets from `loadStudioSetNames`) and
 * the loaded Expansions. The caller owns (and frees) both.
 */
async function planImportFromFile(
  service: IntegraService,
  catalog: ToneCatalog,
  loadStudioSetNames: () => Promise<Map<number, string>>,
  bytes: Uint8Array,
): Promise<{ pack: Pack; plan: ImportPlan }> {
  const pack = Pack.read(bytes);
  try {
    const kinds = new Set(pack.entries().map((e) => e.kind));
    const names: CatalogName[] = [];
    for (const t of libraryTypes().filter((t) => kinds.has(t.kind))) {
      if (!t.bank) {
        // Studio Sets have one bank: LSB 0.
        for (const [pc, name] of await loadStudioSetNames()) {
          names.push({ msb: t.bankMsb, lsb: 0, pc, name });
        }
        continue;
      }
      for (const { msb, lsb, pc, name } of await catalog.load(t.bank)) {
        names.push({ msb, lsb, pc, name });
      }
    }
    const slots = await service.readExpansionSlots();
    return { pack, plan: planImport(pack, names, slots) };
  } catch (e) {
    pack.free();
    throw e;
  }
}

/**
 * The Pack opened on the Library page, its import plan and its import.
 * Lives above the page, so an import keeps running while another page is
 * shown.
 */
export function useLibraryImport(
  service: IntegraService | null,
  mixer: UseMixerResult,
  catalog: ToneCatalog | null,
): UseLibraryImportResult {
  const [opened, setOpened] = useState<Opened | null>(null);
  const [view, setView] = useState<PlanView | null>(null);
  const [opening, setOpening] = useState(false);
  const [openError, setOpenError] = useState<string | null>(null);
  const [focused, setFocused] = useState<number | null>(null);
  const [reportShown, setReportShown] = useState<ReadonlySet<number>>(new Set());
  const [run, setRun] = useState<ImportRun | null>(null);
  const openedRef = useRef<Opened | null>(null);
  const abortRef = useRef<AbortController | null>(null);
  const busy = run !== null && run.phase !== "done";
  // Once imported, the plan's Free slots are stale: reopen to plan again.
  const locked = run !== null;

  const replace = useCallback((next: Opened | null) => {
    const prev = openedRef.current;
    openedRef.current = next;
    if (prev) {
      prev.pack.free();
      prev.plan.free();
    }
    setOpened(next);
    setView(next && readPlan(next.plan));
    setFocused(null);
    setReportShown(new Set());
    setRun(null);
  }, []);

  // A new connection drops the file (and stops an import of it).
  useEffect(
    () => () => {
      abortRef.current?.abort();
      replace(null);
    },
    [service, replace],
  );

  const { loadStudioSetNames } = mixer;
  const open = useCallback(
    (file: File) => {
      if (!service || !catalog || busy) return;
      setOpening(true);
      setOpenError(null);
      file
        .arrayBuffer()
        .then((buffer) =>
          planImportFromFile(service, catalog, loadStudioSetNames, new Uint8Array(buffer)),
        )
        .then(({ pack, plan }) =>
          replace({
            name: file.name,
            pack,
            plan,
            entries: pack.entries(),
            label: pack.label,
            reports: plan.reports(),
            lossy: plan.lossyCounts(),
            sources: plan.sources(),
            fileReport: plan.fileReport(),
          }),
        )
        .catch((e: unknown) => setOpenError(`${file.name}: ${message(e)}`))
        .finally(() => setOpening(false));
    },
    [service, catalog, loadStudioSetNames, busy, replace],
  );

  const close = useCallback(() => {
    if (!busy) replace(null);
  }, [busy, replace]);

  /** Edit the plan, then show it. */
  const edit = useCallback(
    (change: (plan: ImportPlan) => void) => {
      const current = openedRef.current;
      if (!current || locked) return;
      change(current.plan);
      setView(readPlan(current.plan));
    },
    [locked],
  );

  const assign = useCallback(
    (entry: number, slot: number) => edit((plan) => plan.assign(entry, slot)),
    [edit],
  );

  const setIncluded = useCallback(
    (indices: number[], included: boolean) =>
      edit((plan) => {
        for (const i of indices) {
          if (included) plan.include(i);
          else plan.exclude(i);
        }
      }),
    [edit],
  );

  const entries = useMemo(
    (): ImportEntry[] =>
      opened && view
        ? opened.entries.map((info, index) => {
            const type = libraryType(info.kind);
            return {
              index,
              info,
              type,
              origin:
                info.slot === undefined
                  ? `#${info.index + 1}`
                  : `${type.short} ${slotLabel(type, info.slot)}`,
              placement: view.placements[index] ?? "unplaced",
              report: opened.reports[index] ?? [],
              lossy: opened.lossy[index] ?? 0,
              source: opened.sources[index] ?? null,
            };
          })
        : [],
    [opened, view],
  );

  const { studioSetNames } = mixer.state;
  /** The name User slot `slot` of `type` holds now, if known. */
  const slotName = useCallback(
    (type: LibraryType, slot: number): string => {
      if (!type.bank) return studioSetNames.get(slot) ?? "";
      const bank = type.bank;
      const entry = catalog
        ?.get(bank)
        .find((e) => toneSlotIndex(bank, e.lsb, e.pc) === slot);
      return entry?.name ?? "";
    },
    [catalog, studioSetNames],
  );

  const overwrites = useMemo(
    (): Overwrite[] =>
      (view?.overwrites ?? []).flatMap((i) => {
        const entry = entries[i];
        const slot = entry && placedSlot(entry.placement);
        if (!entry || slot === undefined) return [];
        return [
          {
            entry: i,
            type: entry.type,
            slot,
            current: slotName(entry.type, slot),
            incoming: entry.info.name,
          },
        ];
      }),
    [view, entries, slotName],
  );

  const warnings = useMemo(
    () => [
      ...(opened?.fileReport ?? []),
      ...(view?.warnings ?? []).map(
        (w) => `"${opened?.entries[w.entry]?.name ?? ""}" ${w.message}`,
      ),
    ],
    [view, opened],
  );

  const toggleReport = useCallback(
    (entry: number) => setReportShown((shown) => toggled(shown, entry)),
    [],
  );

  const temporary = entries.some((e) => e.placement === "temporary");
  const { selectedPart } = mixer.state;
  const { reload } = mixer;

  const start = useCallback(
    (undo: boolean) => {
      const current = openedRef.current;
      if (!service || !current || locked) return;
      const controller = new AbortController();
      abortRef.current = controller;
      const { signal } = controller;
      const part = selectedPart;
      const undoSources = undo
        ? overwrites.map((o) => ({
            source: "user" as const,
            kind: o.type.kind,
            slot: o.slot,
          }))
        : [];
      const undoFile =
        undoSources.length > 0
          ? `${packFileBase(`UNDO${current.name}`)}${PACK_EXTENSION}`
          : undefined;
      const pending = current.entries.map((_, i): ImportStatus =>
        view?.placements[i] === "excluded" ? "excluded" : "pending",
      );
      setRun({
        phase: undoFile ? "undo" : "importing",
        temporary,
        part,
        statuses: pending,
        undoFile,
      });

      const exportUndo = async (): Promise<void> => {
        if (!undoFile) return;
        const pack = await service.exportPack(undoSources, signal);
        try {
          downloadFile(undoFile, pack.toSvd());
        } finally {
          pack.free();
        }
      };
      const refresh = (statuses: ImportStatus[]): void => {
        reload();
        const stored = new Set(
          current.entries
            .filter((_, i) => statuses[i] === "stored")
            .map((e) => e.kind),
        );
        for (const kind of stored) {
          const { bank } = libraryType(kind);
          if (bank) catalog?.refresh(bank);
          else void loadStudioSetNames(true);
        }
      };

      void (async () => {
        try {
          await exportUndo();
          if (signal.aborted) throw new Error("import cancelled");
          setRun((r) => r && { ...r, phase: "importing" });
          const result = await service.importPack(
            current.pack,
            current.plan,
            part,
            signal,
            (statuses) => setRun((r) => r && { ...r, statuses }),
          );
          setRun(
            (r) =>
              r && {
                ...r,
                phase: "done",
                statuses: result.statuses,
                error: result.error,
              },
          );
          refresh(result.statuses);
        } catch (e) {
          setRun((r) => r && { ...r, phase: "done", error: message(e) });
        }
      })();
    },
    [
      service,
      locked,
      selectedPart,
      overwrites,
      view,
      temporary,
      reload,
      catalog,
      loadStudioSetNames,
    ],
  );

  const cancel = useCallback(() => abortRef.current?.abort(), []);

  return {
    file: opened && { name: opened.name, label: opened.label },
    opening,
    openError,
    entries,
    warnings,
    unplaced: view?.unplaced.length ?? 0,
    overwrites,
    temporary,
    focused,
    focus: setFocused,
    reportShown,
    toggleReport,
    open,
    close,
    assign,
    setIncluded,
    run,
    busy,
    locked,
    start,
    cancel,
  };
}
