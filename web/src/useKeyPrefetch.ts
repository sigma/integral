import { useEffect, useState } from "react";
import type { MutableRefObject } from "react";

/**
 * Fetch every key in `firstKey..=lastKey` into `cache` in the background,
 * one request at a time, skipping keys already cached. Returns a version
 * counter that bumps on each new entry so callers can recompute derived data.
 *
 * `fetchKey` must be stable (useCallback); a new identity restarts the run.
 */
export function useKeyPrefetch<T>(
  enabled: boolean,
  firstKey: number,
  lastKey: number,
  cache: MutableRefObject<Map<number, T>>,
  fetchKey: (key: number) => Promise<T | null>,
): number {
  const [version, setVersion] = useState(0);

  useEffect(() => {
    if (!enabled) return;
    let cancelled = false;

    async function prefetch() {
      for (let k = firstKey; k <= lastKey && !cancelled; k++) {
        if (cache.current.has(k)) continue;
        try {
          const n = await fetchKey(k);
          if (cancelled) return;
          if (n && !cache.current.has(k)) {
            cache.current.set(k, n);
            setVersion((v) => v + 1);
          }
        } catch {
          // Timeout -- key stays unknown until selected
        }
      }
    }

    prefetch();
    return () => { cancelled = true; };
  }, [enabled, firstKey, lastKey, cache, fetchKey]);

  return version;
}
