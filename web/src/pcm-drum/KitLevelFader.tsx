import { SynthFader } from "../synth-ui";
import { LEVEL_TRACK_HEIGHT } from "./NoteControls";

/** The kit-wide Kit Level: a bare fader left of the key grid, as tall as a key's Level. */
export function KitLevelFader({
  kitLevel,
  onChange,
}: {
  kitLevel: number;
  onChange: (value: number) => void;
}) {
  return (
    <SynthFader label="Kit Lv" value={kitLevel} min={0} max={127} defaultValue={127}
      onChange={onChange} formatValue={(v) => String(v)} trackHeight={LEVEL_TRACK_HEIGHT}
      title="Kit Level" />
  );
}
