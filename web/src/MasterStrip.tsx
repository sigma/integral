import { VolumeFader } from "./VolumeFader";
import css from "./MasterStrip.module.css";

interface Props {
  value: number;
  onChange: (value: number) => void;
}

export function MasterStrip({ value, onChange }: Props) {
  return (
    <div className={css.strip}>
      <div className={css.label}>Master</div>
      <div className={css.faderArea}>
        <VolumeFader value={value} onChange={onChange} />
      </div>
    </div>
  );
}
