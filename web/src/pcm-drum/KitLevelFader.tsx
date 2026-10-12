import { SynthFader } from "../synth-ui";

/** The kit-wide Kit Level, as a bare fader beside the key grid. */
export function KitLevelFader({
  kitLevel,
  onChange,
}: {
  kitLevel: number;
  onChange: (value: number) => void;
}) {
  return (
    <SynthFader label="Kit Lv" value={kitLevel} min={0} max={127} defaultValue={127}
      onChange={onChange} formatValue={(v) => String(v)} trackHeight={160}
      title="Kit Level" />
  );
}
