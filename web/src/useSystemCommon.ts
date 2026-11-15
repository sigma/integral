/**
 * System Common as the System page shows it
 * ([#284](https://github.com/sigma/integral/issues/284)): read once when the
 * page opens and again on Refresh, never polled
 * ([#464](https://github.com/sigma/integral/issues/464)). An edit goes
 * through the core's setter and shows at once.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import {
  parseSystemCommon,
  system_common_address,
  system_common_size,
  system_fields,
  type SystemField,
} from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";

/** Each System field's raw value, as read or since edited. */
export type SystemValues = ReadonlyMap<SystemField, number>;

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

  const refresh = useCallback(() => {
    pass.current += 1;
    const mine = pass.current;
    setRead((prev) => ({ ...prev, loading: true, error: null }));
    readSystemCommon(service).then(
      (values) => {
        if (pass.current === mine)
          setRead({ values, loading: false, error: null });
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
  }, [service]);

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
      setRead((prev) =>
        prev.values
          ? { ...prev, values: new Map(prev.values).set(field, value) }
          : prev,
      );
    },
    [service],
  );

  return { read, refresh, set };
}
