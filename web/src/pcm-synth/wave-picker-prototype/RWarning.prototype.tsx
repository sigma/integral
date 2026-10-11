// PROTOTYPE ONLY (throwaway): Parameter Guide: R alone (L = OFF) is silent.
import css from "./prototype.module.css";
import type { WaveValue } from "./prototypeState";

export function RWarning({ value }: { value: WaveValue }) {
  if (value.l !== 0 || value.r === 0) return null;
  return <div className={css.warn}>⚠ L is OFF: R alone produces no sound</div>;
}
