// PROTOTYPE ONLY (throwaway): surface the raw Wave state under each variant.
import css from "./prototype.module.css";
import type { WaveValue } from "./prototypeState";

export function RawState({ value }: { value: WaveValue }) {
  return (
    <div className={css.raw}>
      type={value.groupType} id={value.groupId} L={value.l} R={value.r}
    </div>
  );
}
