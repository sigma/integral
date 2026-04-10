import css from "./SynthSelect.module.css";

interface Props {
  value: number;
  options: { value: number; label: string }[];
  onChange: (value: number) => void;
  label?: string;
}

export function SynthSelect({ value, options, onChange, label }: Props) {
  return (
    <label className={css.wrapper}>
      {label && <span className={css.label}>{label}</span>}
      <select
        className={css.select}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      >
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </label>
  );
}
