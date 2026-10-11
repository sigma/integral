import { useEffect, useMemo, useState } from "react";
import type { LibraryType } from "./librarian";
import type { ToneCatalog } from "./toneCatalog";
import type { UseMixerResult } from "./useMixer";
import { useBankEntries } from "./useBankEntries";
import { studioSetUserSlots, toneUserSlots, type UserSlot } from "./userSlots";

/**
 * The User slots of `type` with their names: tone slots from the tone
 * catalog, Studio Set slots from the mixer's Studio Set names. Fetches them
 * if needed and follows later refreshes.
 */
export function useUserSlots(
  type: LibraryType,
  catalog: ToneCatalog,
  mixer: UseMixerResult,
): { slots: UserSlot[]; loading: boolean } {
  const { entries, loading: tonesLoading } = useBankEntries(catalog, type.bank);
  const { loadStudioSetNames } = mixer;
  const setNames = mixer.state.studioSetNames;
  const [setsLoading, setSetsLoading] = useState(false);

  useEffect(() => {
    if (!type.studioSet) return;
    let live = true;
    setSetsLoading(true);
    void loadStudioSetNames().finally(() => {
      if (live) setSetsLoading(false);
    });
    return () => {
      live = false;
    };
  }, [type.studioSet, loadStudioSetNames]);

  const slots = useMemo(
    () =>
      type.bank
        ? toneUserSlots(entries, type.bank)
        : studioSetUserSlots(setNames, type.bankMsb),
    [type, entries, setNames],
  );
  return { slots, loading: type.studioSet ? setsLoading : tonesLoading };
}
