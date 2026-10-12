import { useCallback, useMemo } from "react";
import type { Dispatch, SetStateAction } from "react";
import { MfxHeaderField } from "../pkg/integral_wasm.js";
import type { ToneType } from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import type { MfxCtrl, MfxState } from "./MfxPanel";

/** The `MfxPanel` callbacks that edit a tone's MFX block. */
export interface MfxHandlers {
  onTypeChange: (type_: number) => void;
  /** `index` is the control slot for the per-control fields, 0 otherwise. */
  onHeaderParam: (field: MfxHeaderField, index: number, value: number) => void;
  onNibParam: (paramIndex: number, value: number) => void;
}

/** `mfx` with instance `index` of header `field` set to `value`. */
function withHeaderField(
  mfx: MfxState,
  field: MfxHeaderField,
  index: number,
  value: number,
): MfxState {
  const withControl = (key: keyof MfxCtrl): MfxState => {
    const controls = [...mfx.controls];
    controls[index] = { ...controls[index]!, [key]: value };
    return { ...mfx, controls };
  };
  switch (field) {
    case MfxHeaderField.Type: return { ...mfx, mfxType: value, params: [] };
    case MfxHeaderField.ChorusSend: return { ...mfx, chorusSend: value };
    case MfxHeaderField.ReverbSend: return { ...mfx, reverbSend: value };
    case MfxHeaderField.ControlSource: return withControl("source");
    case MfxHeaderField.ControlSens: return withControl("sens");
    case MfxHeaderField.ControlAssign: return withControl("assign");
  }
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
  const onHeaderParam = useCallback(
    (field: MfxHeaderField, index: number, value: number) => {
      service.device.setMfxHeaderParam(partIndex, toneType, field, index, value);
      setMfx((prev) => prev ? withHeaderField(prev, field, index, value) : prev);
    },
    [partIndex, toneType, service, setMfx],
  );

  // The device keeps the old slot values on a type change, so the core also
  // sends the new type's defaults.
  const onTypeChange = useCallback(
    (type_: number) => {
      const params = [...service.device.setMfxType(partIndex, toneType, type_)];
      setMfx((prev) => prev ? { ...prev, mfxType: type_, params } : prev);
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
