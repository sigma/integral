/**
 * System Common as the System page and the Studio Set settings drawer show it
 * ([#284](https://github.com/sigma/integral/issues/284)): read once when
 * either opens and again on Refresh, never polled
 * ([#464](https://github.com/sigma/integral/issues/464)). An edit goes
 * through the core's setter and shows at once, in every reader of the same
 * service (the Part settings drawer's SCALE shows the Scale Tune Switch the
 * System page edits beside it), as does a read.
 *
 * The parsed wasm dump is freed as soon as its values are copied out, so the
 * hook holds no wasm memory.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import {
  parseSystemCommon,
  system_common_address,
  system_common_size,
  system_field_spec,
  system_fields,
  system_pair_selector,
  system_pair_studio_set_in_force,
  type SystemField,
  type SystemOrStudioSet,
} from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";

/** Each System field's raw value, as read or since edited. */
export type SystemValues = ReadonlyMap<SystemField, number>;

/** `field`'s value as the core displays it, or `—` before a read answers. */
export function systemDisplay(
  values: SystemValues | null,
  field: SystemField,
): string {
  const value = values?.get(field);
  if (value === undefined) return "—";
  const spec = system_field_spec(field);
  try {
    return spec.formatValue(value);
  } finally {
    spec.free();
  }
}

/**
 * Whether the Studio Set's side of `pair` is in force, by its System
 * selector; `null` before a read answers.
 */
export function studioSetInForce(
  values: SystemValues | null,
  pair: SystemOrStudioSet,
): boolean | null {
  const selector = values?.get(system_pair_selector(pair));
  return selector === undefined
    ? null
    : system_pair_studio_set_in_force(pair, selector);
}

/** Where the read stands. A re-read keeps the values shown until it answers. */
export interface SystemRead {
  /** The values last read, none before the first read answers. */
  values: SystemValues | null;
  /** A read is in flight. */
  loading: boolean;
  /** Why the last read failed, none if it didn't. */
  error: string | null;
}

export interface UseSystemCommonResult {
  read: SystemRead;
  /** Read System Common again. */
  refresh: () => void;
  /** Send `value` to `field` and show it. */
  set: (field: SystemField, value: number) => void;
}

/** What one reader tells the others: a read's values, or one field's edit. */
type Change = { values: SystemValues } | { field: SystemField; value: number };

/** Every reader of System Common, by service: each takes the others' changes. */
const readers = new WeakMap<IntegraService, Set<(change: Change) => void>>();

/** Tell every reader of `service` but `from` about `change`. */
function broadcast(service: IntegraService, from: (change: Change) => void, change: Change) {
  for (const reader of readers.get(service) ?? []) if (reader !== from) reader(change);
}

/** Read System Common through `service`: every field's raw value. */
async function readSystemCommon(
  service: IntegraService,
): Promise<SystemValues> {
  const data = await service.requestData(
    Array.from(system_common_address()),
    Array.from(system_common_size()),
  );
  const common = parseSystemCommon(data);
  try {
    return new Map(system_fields().map((field) => [field, common.get(field)]));
  } finally {
    common.free();
  }
}

/** System Common of the device behind `service`, read on mount and on `refresh`. */
export function useSystemCommon(
  service: IntegraService,
): UseSystemCommonResult {
  const [read, setRead] = useState<SystemRead>({
    values: null,
    loading: true,
    error: null,
  });
  // Bumped by every read, so a read overtaken by a later one cannot write.
  const pass = useRef(0);

  // Another reader's read or edit, taken as this one's own.
  const take = useCallback((change: Change) => {
    if ("values" in change) {
      pass.current += 1;
      setRead({ values: change.values, loading: false, error: null });
    } else {
      setRead((prev) =>
        prev.values
          ? { ...prev, values: new Map(prev.values).set(change.field, change.value) }
          : prev,
      );
    }
  }, []);

  useEffect(() => {
    const set = readers.get(service) ?? new Set();
    readers.set(service, set);
    set.add(take);
    return () => {
      set.delete(take);
    };
  }, [service, take]);

  const refresh = useCallback(() => {
    pass.current += 1;
    const mine = pass.current;
    setRead((prev) => ({ ...prev, loading: true, error: null }));
    readSystemCommon(service).then(
      (values) => {
        if (pass.current !== mine) return;
        setRead({ values, loading: false, error: null });
        broadcast(service, take, { values });
      },
      (err: unknown) => {
        if (pass.current !== mine) return;
        setRead((prev) => ({
          ...prev,
          loading: false,
          error: err instanceof Error ? err.message : String(err),
        }));
      },
    );
  }, [service, take]);

  useEffect(() => {
    refresh();
    // Unmounting abandons the read in flight.
    return () => {
      pass.current += 1;
    };
  }, [refresh]);

  const set = useCallback(
    (field: SystemField, value: number) => {
      // The core refuses a value out of range and sends nothing.
      service.device.setSystemField(field, value);
      take({ field, value });
      broadcast(service, take, { field, value });
    },
    [service, take],
  );

  return { read, refresh, set };
}
