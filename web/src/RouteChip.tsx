import css from "./ChannelStrip.module.css";

// PROTOTYPE (#34): compact output-routing select shared by part, Comp+EQ and FX strips.
interface Props {
  value: number;
  names: string[];
  onChange: (value: number) => void;
  /** Output overridden (Motional Surround): shown dimmed as SURR. */
  overridden?: boolean;
  title: string;
}

export function RouteChip({ value, names, onChange, overridden, title }: Props) {
  return (
    <select
      className={css.compEqButton}
      value={value}
      disabled={overridden}
      onChange={(e) => onChange(Number(e.target.value))}
      title={overridden ? "Motional Surround overrides output routing" : title}
    >
      {overridden ? (
        <option value={value}>→ SURR</option>
      ) : (
        names.map((name, v) => <option key={v} value={v}>→ {name}</option>)
      )}
    </select>
  );
}
