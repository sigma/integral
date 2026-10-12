import { PcmdCommonField, pcmd_common_spec } from "../../pkg/integral_wasm.js";
import { knobRange } from "../fieldSpec";
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
    <SynthFader label="Kit Lv" value={kitLevel} {...knobRange(pcmd_common_spec(PcmdCommonField.KitLevel))}
      onChange={onChange} trackHeight={LEVEL_TRACK_HEIGHT}
      title="Kit Level" />
  );
}
