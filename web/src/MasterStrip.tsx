import { VolumeFader } from "./VolumeFader";
import css from "./MasterStrip.module.css";

interface Props {
  value: number;
  onChange: (value: number) => void;
}

export function MasterStrip({ value, onChange }: Props) {
  return (
    <div className={css.strip}>
      <span className={css.label}>Master Level</span>
      <VolumeFader value={value} onChange={onChange} />
    </div>
  );
}
