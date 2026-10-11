import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ExportSource } from "../pkg/integral_wasm.js";
import type { IntegraService, PackExportProgress } from "./integra";
import {
  downloadFile,
  packFileBase,
  PACK_EXTENSION,
  sourceKey,
  sourceLabel,
  sourceType,
  type LibraryType,
} from "./librarian";

/** Something to export, with the name it holds. */
export interface NamedSource {
  source: ExportSource;
  name: string;
}

/** One row of the Pack being built. */
export interface PackItem {
  key: string;
  type: LibraryType;
  /** Where it is read from, e.g. "SN-S 0012". */
  label: string;
  /** Empty until known (a tone a Studio Set pulls in, before reading). */
  name: string;
  /** Pulled in by a Studio Set of the Pack: can't be removed. */
  auto: boolean;
  /** Read status while exporting. */
  status: "pending" | "reading" | "read" | "failed" | null;
}

/** An export run: reading the sources, then the download. */
export interface ExportRun {
  phase: "reading" | "done" | "failed";
  /** Sources read so far, out of `total`. */
  read: number;
  total: number;
  fileName: string;
  error?: string;
}

export interface UseLibraryExportResult {
  items: PackItem[];
  /** 8.3 file name base, without the extension. */
  fileBase: string;
  setFileBase: (input: string) => void;
  add: (sources: NamedSource[]) => void;
  remove: (key: string) => void;
  run: ExportRun | null;
  /** Read the sources, then download the Pack. */
  start: () => void;
  cancel: () => void;
}

const DEFAULT_FILE_BASE = "PACK";

interface RunState {
  phase: ExportRun["phase"];
  fileName: string;
  progress: PackExportProgress;
  /** The exported entries' names, in Pack order, once done. */
  names: string[] | null;
  error?: string;
}

/**
 * The Pack being built on the Library page and its export. Lives above
 * the page, so an export keeps running while another page is shown.
 */
export function useLibraryExport(service: IntegraService | null): UseLibraryExportResult {
  const [sources, setSources] = useState<NamedSource[]>([]);
  const [editedBase, setEditedBase] = useState<string | null>(null);
  const [run, setRun] = useState<RunState | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => () => abortRef.current?.abort(), [service]);

  const fileBase =
    editedBase ?? (packFileBase(sources[0]?.name ?? "") || DEFAULT_FILE_BASE);
  const running = run?.phase === "reading";

  const add = useCallback(
    (more: NamedSource[]) => {
      if (running) return;
      setRun(null);
      setSources((prev) => {
        const keys = new Set(prev.map((s) => sourceKey(s.source)));
        return [...prev, ...more.filter((s) => !keys.has(sourceKey(s.source)))];
      });
    },
    [running],
  );

  const remove = useCallback(
    (key: string) => {
      if (running) return;
      setRun(null);
      setSources((prev) => prev.filter((s) => sourceKey(s.source) !== key));
    },
    [running],
  );

  const start = useCallback(() => {
    if (!service || running || sources.length === 0) return;
    const controller = new AbortController();
    abortRef.current = controller;
    const fileName = `${fileBase}${PACK_EXTENSION}`;
    const empty = { progress: [], autoIncluded: [] };
    setRun({ phase: "reading", fileName, progress: empty, names: null });
    const onProgress = (progress: PackExportProgress): void =>
      setRun((r) => r && { ...r, progress });
    service
      .exportPack(
        sources.map((s) => s.source),
        controller.signal,
        onProgress,
      )
      .then((pack) => {
        try {
          const names = pack.entries().map((e) => e.name);
          downloadFile(fileName, pack.toSvd());
          setRun((r) => r && { ...r, phase: "done", names });
        } finally {
          pack.free();
        }
      })
      .catch((e: unknown) => {
        const error = e instanceof Error ? e.message : String(e);
        setRun((r) => r && { ...r, phase: "failed", error });
      });
  }, [service, sources, fileBase, running]);

  const cancel = useCallback(() => abortRef.current?.abort(), []);

  const items = useMemo((): PackItem[] => {
    const names = new Map(sources.map((s) => [sourceKey(s.source), s.name]));
    const item = (source: ExportSource, auto: boolean): PackItem => ({
      key: sourceKey(source),
      type: sourceType(source),
      label: sourceLabel(source),
      name: names.get(sourceKey(source)) ?? "",
      auto,
      status: null,
    });
    const progress = run?.progress.progress ?? [];
    if (progress.length === 0) return sources.map((s) => item(s.source, false));
    const auto = new Set(run?.progress.autoIncluded.map(sourceKey));
    return progress.map((p, i) => {
      const it = item(p.source, auto.has(sourceKey(p.source)));
      return { ...it, name: run?.names?.[i] ?? it.name, status: p.status };
    });
  }, [sources, run]);

  const exportRun = useMemo((): ExportRun | null => {
    if (!run) return null;
    const { progress } = run.progress;
    return {
      phase: run.phase,
      read: progress.filter((p) => p.status === "read").length,
      total: progress.length,
      fileName: run.fileName,
      error: run.error,
    };
  }, [run]);

  return {
    items,
    fileBase,
    setFileBase: (input) => setEditedBase(packFileBase(input)),
    add,
    remove,
    run: exportRun,
    start,
    cancel,
  };
}
