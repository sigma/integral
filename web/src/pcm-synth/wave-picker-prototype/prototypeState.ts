// PROTOTYPE ONLY (throwaway). Local Wave state for the picker variants; it
// never writes to the device, it only logs what it would send.
import { useState } from "react";
import type { PcmSynthPartial } from "../types";

export interface WaveValue {
  groupType: number;
  groupId: number;
  l: number;
  r: number;
}

export function usePrototypeWave(
  partial: PcmSynthPartial,
  partialIdx: number,
): [WaveValue, (next: WaveValue) => void] {
  const [value, setValue] = useState<WaveValue>({
    groupType: partial.waveGroupType,
    groupId: partial.waveGroupId,
    l: partial.waveNumberL,
    r: partial.waveNumberR,
  });
  const set = (next: WaveValue) => {
    console.log(
      `[wave-picker-prototype] P${partialIdx + 1} would send: ` +
        `0x27 Group Type=${next.groupType}, 0x28 Group ID=${next.groupId} (nib), ` +
        `0x2C Wave L=${next.l} (nib), 0x30 Wave R=${next.r} (nib)`,
    );
    setValue(next);
  };
  return [value, set];
}
