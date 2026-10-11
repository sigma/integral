// PROTOTYPE ONLY (throwaway). Mounts the selected Wave picker variant in
// WaveSection. State is local; nothing is sent to the device.
import type { PcmSynthPartial } from "../types";
import { usePrototypeWave } from "./prototypeState";
import type { VariantId } from "./variantStore.prototype";
import { VariantA } from "./VariantA.prototype";
import { VariantB } from "./VariantB.prototype";
import { VariantC } from "./VariantC.prototype";

export function WavePickerPrototype({
  variant,
  partial,
  partialIdx,
}: {
  variant: Exclude<VariantId, "0">;
  partial: PcmSynthPartial;
  partialIdx: number;
}) {
  const [value, onChange] = usePrototypeWave(partial, partialIdx);
  if (variant === "A") return <VariantA value={value} onChange={onChange} />;
  if (variant === "B") return <VariantB value={value} onChange={onChange} />;
  return <VariantC value={value} onChange={onChange} />;
}
