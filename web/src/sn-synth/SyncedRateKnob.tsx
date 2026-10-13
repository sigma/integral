import { sns_partial_spec, sns_partial_sync, type SnsPartialField } from "../../pkg/integral_wasm.js";
import { knobProps } from "../fieldSpec";
import { SynthKnob, SyncedControl } from "../synth-ui";
import type { PartialParamHandler } from "./types";

interface Props {
  /** A rate field whose note lives in a separate field (`sns_partial_sync`). */
  field: SnsPartialField;
  /** Current values of the rate, its tempo-sync switch and its note field. */
  rate: number;
  tempoSync: number;
  note: number;
  onChange: PartialParamHandler;
  label: string;
}

/** A rate knob with a Hz-vs-note toggle; the knob edits the rate or the note, whichever the toggle selects. */
export function SyncedRateKnob({ field, rate, tempoSync, note, onChange, label }: Props) {
  const sync = sns_partial_sync(field);
  if (!sync) {
    return <SynthKnob label={label} value={rate} {...knobProps(sns_partial_spec(field))} onChange={(v) => onChange(field, v)} />;
  }
  const noteMode = tempoSync !== 0;
  const active = noteMode ? sync.noteField : field;
  return (
    <SyncedControl
      labels={sync.labels}
      noteMode={noteMode}
      onNoteModeChange={(on) => onChange(sync.switchField, on ? 1 : 0)}
    >
      <SynthKnob label={label} value={noteMode ? note : rate} {...knobProps(sns_partial_spec(active))}
        onChange={(v) => onChange(active, v)} />
    </SyncedControl>
  );
}
