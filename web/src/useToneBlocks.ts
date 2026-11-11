/**
 * A tone editor's block reads, one failure at a time: every block is read in
 * order, a read or parse failure is kept to its block (the chain moves on),
 * and one `retry` re-reads the blocks that failed. The shape mirrors the
 * drum editors' `useKitBlocks` (#524), so the two can be unified.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import type { IntegraService } from "./integra";

/** A block the chain reads: where and how much to ask for, and what parses
 * the reply. A throw from `apply` fails only this block. */
export interface ToneBlockRead {
  /** The block's name, for the console line naming a failed read. */
  name: string;
  /** The block's RQ1 address (4 bytes). */
  address(): number[];
  /** The block's RQ1 size (4 7-bit bytes). */
  size(): number[];
  /** Parse the reply into the editor's data. */
  apply(data: Uint8Array): unknown;
}

/** Where one block's read is. */
export interface ToneBlock {
  /** `idle` before its turn, `loading` while in flight, `loaded` with its
   * data, or `error` with `message` saying why. */
  status: "idle" | "loading" | "loaded" | "error";
  /** The parsed block, once `loaded`. */
  value?: unknown;
  /** Why the read failed, on `error`. */
  message?: string;
}

/** What {@link useToneBlocks} gives the editor. */
export interface ToneBlocks {
  /** Each block, in the order `reads` gave them. */
  blocks: readonly ToneBlock[];
  /** True until the first pass over every block has finished (loaded or
   * failed); a `retry` does not re-enter it. */
  loading: boolean;
  /** Whether any block ended its last attempt in `error`. */
  failed: boolean;
  /** Re-read the blocks that failed. */
  retry(): void;
  /** Replace a loaded block's value (the editor patches its own copy). */
  setValue(i: number, update: (prev: unknown) => unknown): void;
}

/**
 * Read `reads` in order through `service`, each failure kept to its block.
 * `deps` is the dependency list that restarts the chain (a Part change).
 */
export function useToneBlocks(
  service: IntegraService,
  reads: readonly ToneBlockRead[],
  deps: readonly unknown[],
): ToneBlocks {
  const [blocks, setBlocks] = useState<readonly ToneBlock[]>(() => reads.map(() => ({ status: "idle" })));
  const [loading, setLoading] = useState(true);
  // Bumped by every pass, so a pass the starter abandoned cannot write.
  const pass = useRef(0);
  // The blocks as they are, for `retry` to pick the failed ones from.
  const blocksRef = useRef(blocks);
  blocksRef.current = blocks;

  const readOne = useCallback(
    async (i: number, myPass: number) => {
      const read = reads[i]!;
      setBlocks((prev) => prev.map((b, j) => (j === i ? { status: "loading" } : b)));
      try {
        const value = read.apply(await service.requestData(read.address(), read.size()));
        if (pass.current !== myPass) return;
        setBlocks((prev) => prev.map((b, j) => (j === i ? { status: "loaded", value } : b)));
      } catch (err) {
        if (pass.current !== myPass) return;
        const message = err instanceof Error ? err.message : String(err);
        console.error(`[tone blocks] ${read.name} failed: ${message}`);
        setBlocks((prev) => prev.map((b, j) => (j === i ? { status: "error", message } : b)));
      }
    },
    [reads, service],
  );

  useEffect(() => {
    pass.current += 1;
    const myPass = pass.current;
    setBlocks(reads.map(() => ({ status: "idle" })));
    setLoading(true);
    let stopped = false;
    void (async () => {
      for (let i = 0; i < reads.length; i++) {
        if (stopped || pass.current !== myPass) return;
        await readOne(i, myPass);
      }
      if (!stopped && pass.current === myPass) setLoading(false);
    })();
    return () => {
      stopped = true;
    };
    // The caller names what restarts the chain.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  const retry = useCallback(() => {
    const failed = blocksRef.current.map((b, i) => (b.status === "error" ? i : -1)).filter((i) => i >= 0);
    if (failed.length === 0) return;
    pass.current += 1;
    const myPass = pass.current;
    void (async () => {
      for (const i of failed) {
        if (pass.current !== myPass) return;
        await readOne(i, myPass);
      }
    })();
  }, [readOne]);

  const setValue = useCallback((i: number, update: (prev: unknown) => unknown) => {
    setBlocks((prev) =>
      prev.map((b, j) => (j === i && b.status === "loaded" ? { status: "loaded", value: update(b.value) } : b)),
    );
  }, []);

  return {
    blocks,
    loading,
    failed: blocks.some((b) => b.status === "error"),
    retry,
    setValue,
  };
}
