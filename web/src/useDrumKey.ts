import { useCallback, useEffect, useRef, useState } from "react";
import type { MutableRefObject } from "react";
import type { IntegraService } from "./integra";
import { useKeyPrefetch } from "./useKeyPrefetch";

/** The key a drum editor opens on: C2, the kick drum. */
const FIRST_SELECTED = 36;

/** How long a selected key's preview note sounds, ms. */
const PREVIEW_MS = 300;

/** A drum kit editor's key selection: the Scope its key Sections take. */
export interface DrumKey<N> {
  /** The selected key. */
  key: number;
  /** Selects a key and previews it on the part's receive channel. */
  select: (key: number) => void;
  /** The selected key's data, or the previous key's until it arrives. */
  note: N | null;
  /** Whether `note` is the selected key's. */
  current: boolean;
  /** Changes the selected key's data (and its cached copy); a no-op until it arrives. */
  update: (change: (note: N) => N) => void;
  /** Every key's data fetched so far. */
  cache: MutableRefObject<Map<number, N>>;
  /** Bumps whenever `cache` gains a key. */
  cacheVersion: number;
}

/**
 * The selected key of a drum kit in `partIndex`, its data fetched with
 * `fetchKey` (stable, from `useCallback`) and cached, and every key in
 * `first..=last` prefetched once `enabled`. Changing the key only changes
 * the Scope: the editor's Sections stay mounted.
 */
export function useDrumKey<N>(
  service: IntegraService,
  partIndex: number,
  { first, last }: { first: number; last: number },
  enabled: boolean,
  fetchKey: (key: number) => Promise<N | null>,
): DrumKey<N> {
  const [key, setKey] = useState(FIRST_SELECTED);
  const [note, setNote] = useState<{ key: number; data: N } | null>(null);
  const cache = useRef<Map<number, N>>(new Map());

  // Another part (or service) is another kit.
  useEffect(() => {
    cache.current.clear();
    setNote(null);
  }, [partIndex, service]);

  const cacheVersion = useKeyPrefetch(enabled, first, last, cache, fetchKey);

  useEffect(() => {
    if (key < first || key > last) return;
    const cached = cache.current.get(key);
    if (cached) {
      setNote({ key, data: cached });
      return;
    }
    let cancelled = false;
    fetchKey(key)
      .then((data) => {
        if (cancelled || !data) return;
        cache.current.set(key, data);
        setNote({ key, data });
      })
      .catch(() => {
        // Timeout: the Sections stay inactive until a key answers.
      });
    return () => {
      cancelled = true;
    };
  }, [key, first, last, fetchKey]);

  const select = useCallback(
    (k: number) => {
      setKey(k);
      const rs = service.device.readState() as { parts?: { receiveChannel?: number }[] };
      const ch = rs?.parts?.[partIndex]?.receiveChannel ?? partIndex;
      service.sendNoteOn(ch, k, 100);
      setTimeout(() => service.sendNoteOff(ch, k), PREVIEW_MS);
    },
    [partIndex, service],
  );

  const update = useCallback(
    (change: (n: N) => N) => {
      setNote((prev) => {
        if (!prev || prev.key !== key) return prev;
        const data = change(prev.data);
        cache.current.set(key, data);
        return { key, data };
      });
    },
    [key],
  );

  return { key, select, note: note?.data ?? null, current: note?.key === key, update, cache, cacheVersion };
}
