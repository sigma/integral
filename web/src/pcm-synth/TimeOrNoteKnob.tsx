import { pcms_partial_spec, pcms_partial_sync, type PcmsPartialField } from "../../pkg/integral_wasm.js";
import { knobRange } from "../fieldSpec";
import { SynthKnob, SyncedControl } from "../synth-ui";

interface Props {
  /** A field whose high values are tempo-synced notes (`pcms_partial_sync`). */
  field: PcmsPartialField;
  value: number;
  onChange: (value: number) => void;
  label: string;
  title?: string;
  color?: string;
}

/**
 * A rate or time knob with a free-vs-note toggle. The field holds both: the
 * knob covers the free values or the note values, whichever the toggle selects.
 */
export function TimeOrNoteKnob({ field, value, onChange, ...look }: Props) {
  const spec = pcms_partial_spec(field);
  const sync = pcms_partial_sync(field);
  if (!sync) return <SynthKnob {...look} value={value} {...knobRange(spec)} onChange={onChange} />;
  const noteMode = value >= sync.noteFrom;
  const range = noteMode
    ? { min: sync.noteFrom, max: spec.max, defaultValue: sync.noteDefault }
    : { min: spec.min, max: sync.noteFrom - 1, defaultValue: spec.defaultValue };
  return (
    <SyncedControl
      labels={sync.labels}
      noteMode={noteMode}
      onNoteModeChange={(on) => {
        // One field holds both values, so switching mode starts from that mode's default.
        if (on !== noteMode) onChange(on ? sync.noteDefault : spec.defaultValue);
      }}
    >
      <SynthKnob {...look} value={value} {...range} formatValue={(v) => spec.formatValue(v)} onChange={onChange} />
    </SyncedControl>
  );
}
