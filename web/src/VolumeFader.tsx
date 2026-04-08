import css from "./VolumeFader.module.css";

interface Props {
  value: number;
  onChange: (value: number) => void;
}

export function VolumeFader({ value, onChange }: Props) {
  return (
    <div className={css.container}>
      <input
        className={css.fader}
        type="range"
        min={0}
        max={127}
        value={value}
        onInput={(e) => onChange(Number(e.currentTarget.value))}
      />
      <span className={css.value}>{value}</span>
    </div>
  );
}
