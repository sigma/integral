import css from "./SyncUnit.module.css";

/**
 * A tempo-syncable readout's unit, `ms`/`Hz` free or `♪` on a note, as the
 * button that switches between them; `labels` come from core, `[free, note]`.
 */
export function SyncUnit({
  labels,
  noteMode,
  onChange,
}: {
  labels: readonly string[];
  noteMode: boolean;
  onChange: (noteMode: boolean) => void;
}) {
  const [free, note] = labels;
  return (
    <button
      type="button"
      className={css.unit}
      aria-pressed={noteMode}
      title={noteMode ? `Tempo sync on: click to set freely (${free})` : `Tempo sync off: click to set as a note (${note})`}
      onClick={() => onChange(!noteMode)}
    >
      {noteMode ? note : free}
    </button>
  );
}
