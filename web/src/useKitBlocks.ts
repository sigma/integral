/**
 * The kit block loader the drum kit editors share: it reads each block
 * through the service, keeps what loaded, and surfaces what didn't, with a
 * single retry entry point.
 */

import { useCallback, useEffect, useState } from "react";
import type { IntegraService } from "./integra";

/** Where one kit block's read stands. */
export type KitBlockStatus = "idle" | "loading" | "loaded" | "error";

/** One block of a kit to read: its `requestData` address and size, and how
 *  the editor applies the answer to get the block's value (null/undefined
 *  for a block the editor shows nothing of, like the PCM kit's Common2). */
export interface KitBlockRead {
  address: () => number[];
  size: () => number[];
  apply: (data: Uint8Array) => unknown;
}

/** One block's read: where it stands and what it last read. */
export interface KitBlock {
  status: KitBlockStatus;
  value: unknown;
}

/** Reads `reads` through `service`, one block at a time. A rejected read
 *  marks its block `error` and the rest still load. The load re-runs,
 *  re-reading every block, when `deps` change (another part, another scope)
 *  or `retry` is called. */
export function useKitBlocks(service: IntegraService, reads: KitBlockRead[], deps: unknown[]) {
  const [blocks, setBlocks] = useState<KitBlock[]>(() => reads.map(() => ({ status: "idle", value: null })));
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setBlocks(reads.map(() => ({ status: "loading" as const, value: null })));
    (async () => {
      for (let i = 0; i < reads.length; i++) {
        try {
          const data = await service.requestData(reads[i]!.address(), reads[i]!.size());
          if (cancelled) return;
          const value = reads[i]!.apply(data);
          setBlocks((prev) => prev.map((b, j) => (j === i ? { status: "loaded", value: value ?? null } : b)));
        } catch {
          if (!cancelled) setBlocks((prev) => prev.map((b, j) => (j === i ? { status: "error", value: null } : b)));
        }
      }
    })();
    return () => {
      cancelled = true;
    };
    // `reads` and `deps` go together: a dep change is a new read set.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [service, attempt, ...deps]);

  const setValue = useCallback(
    (i: number, next: (prev: unknown) => unknown) =>
      setBlocks((prev) => prev.map((b, j) => (j === i ? { ...b, value: next(b.value) } : b))),
    [],
  );

  return {
    blocks,
    /** True while any block's read is in flight. */
    loading: blocks.some((b) => b.status === "loading"),
    /** True if any block's last read failed. */
    failed: blocks.some((b) => b.status === "error"),
    retry: () => setAttempt((a) => a + 1),
    setValue,
  };
}
