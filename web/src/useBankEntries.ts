import { useEffect, useState } from "react";
import type { ToneBank } from "./toneBanks";
import type { ToneCatalog, ToneEntry } from "./toneCatalog";

/**
 * Entries of a tone bank from the catalog, fetching it if needed and
 * following later updates (e.g. pages landing, or a refresh after a save).
 * Pass `null` to track nothing.
 */
export function useBankEntries(
  catalog: ToneCatalog,
  bank: ToneBank | null,
): { entries: ToneEntry[]; loading: boolean } {
  const [entries, setEntries] = useState<ToneEntry[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!bank) {
      setEntries([]);
      setLoading(false);
      return;
    }
    setEntries(catalog.get(bank));
    setLoading(!catalog.isComplete(bank));
    const unsub = catalog.subscribe(bank, (next) => {
      setEntries(next);
      setLoading(!catalog.isComplete(bank));
    });
    // No-op if already complete or in progress.
    catalog.fetch(bank);
    return unsub;
  }, [catalog, bank]);

  return { entries, loading };
}
