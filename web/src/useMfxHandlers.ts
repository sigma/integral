import { useCallback, useMemo } from "react";
import type { Dispatch, SetStateAction } from "react";
import type { ToneType } from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import type { MfxState } from "./MfxPanel";
import { deviceSpec } from "./deviceSpec";

/** The `MfxPanel` callbacks that edit a tone's MFX block. */
export interface MfxHandlers {
  onTypeChange: (type_: number) => void;
  onHeaderParam: (offset: number, value: number) => void;
  onNibParam: (paramIndex: number, value: number) => void;
}

/**
 * MFX edit callbacks for Part `partIndex`'s tone of type `toneType`: each
 * sends the edit to the device and applies it to the editor's MFX state.
 */
export function useMfxHandlers(
  service: IntegraService,
  partIndex: number,
  toneType: ToneType,
  setMfx: Dispatch<SetStateAction<MfxState | null>>,
): MfxHandlers {
  const onTypeChange = useCallback(
    (type_: number) => {
      service.device.setMfxParam(partIndex, toneType, 0x00, type_);
      setMfx((prev) => prev ? { ...prev, mfxType: type_, params: [] } : prev);
    },
    [partIndex, toneType, service, setMfx],
  );

  const onHeaderParam = useCallback(
    (offset: number, value: number) => {
      service.device.setMfxParam(partIndex, toneType, offset, value);
      setMfx((prev) => {
        if (!prev) return prev;
        const next = { ...prev, controls: [...prev.controls] };
        if (offset === 0x02) next.chorusSend = value;
        else if (offset === 0x03) next.reverbSend = value;
        for (let s = 0; s < deviceSpec().mfx_control_count; s++) {
          if (offset === 0x05 + s * 2) {
            next.controls[s] = { ...next.controls[s]!, source: value };
          } else if (offset === 0x06 + s * 2) {
            next.controls[s] = { ...next.controls[s]!, sens: value };
          } else if (offset === 0x0D + s) {
            next.controls[s] = { ...next.controls[s]!, assign: value };
          }
        }
        return next;
      });
    },
    [partIndex, toneType, service, setMfx],
  );

  const onNibParam = useCallback(
    (paramIndex: number, value: number) => {
      service.device.setMfxNibParam(partIndex, toneType, paramIndex, value);
      setMfx((prev) => {
        if (!prev) return prev;
        const params = [...prev.params];
        params[paramIndex] = value;
        return { ...prev, params };
      });
    },
    [partIndex, toneType, service, setMfx],
  );

  return useMemo(
    () => ({ onTypeChange, onHeaderParam, onNibParam }),
    [onTypeChange, onHeaderParam, onNibParam],
  );
}
