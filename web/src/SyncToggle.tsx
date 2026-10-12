import css from "./SyncToggle.module.css";

/** Two-way msec/Hz-vs-note switch; `labels` come from the core. */
export function SyncToggle({
  labels,
  noteMode,
  onChange,
}: {
  labels: string[];
  noteMode: boolean;
  onChange: (noteMode: boolean) => void;
}) {
  return (
    <span className={css.sync}>
      {labels.map((l, i) => (
        <button
          key={l}
          className={(i === 1) === noteMode ? css.syncOn : undefined}
          aria-pressed={(i === 1) === noteMode}
          onClick={() => onChange(i === 1)}
        >
          {l}
        </button>
      ))}
    </span>
  );
}
