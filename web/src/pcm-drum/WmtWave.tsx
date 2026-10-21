import { DisplayKind, DisplayProp, ToneType, type PcmdWmtField } from "../../pkg/integral_wasm.js";
import { slotField, slotOf } from "../layouts";
import { WavePicker, type ExpansionMarking } from "../WavePicker";
import { waveSourceOf } from "../waveCatalog";

/** The WMT field core binds the PCM Drum wave picker's `prop` to. */
function wmtField(prop: DisplayProp): PcmdWmtField {
  const slot = slotOf(DisplayKind.WavePicker, ToneType.PcmDrum, prop);
  return slotField(slot, slot.pcmdWmt);
}

/**
 * A WMT layer's Wave Group and L/R Wave Numbers, picked by name: the PCM
 * Drum wave picker Display, its fields from core's binding. `value` and
 * `onChange` read and write the layer the Scope names.
 */
export function WmtWave({
  value,
  onChange,
  marking,
}: {
  value: (field: PcmdWmtField) => number;
  onChange: (field: PcmdWmtField, value: number) => void;
  /** Loaded state of the Expansions behind SRX Wave Groups. */
  marking?: ExpansionMarking;
}) {
  const type = wmtField(DisplayProp.WaveGroupType);
  const id = wmtField(DisplayProp.WaveGroupId);
  const l = wmtField(DisplayProp.WaveNumberL);
  const r = wmtField(DisplayProp.WaveNumberR);
  return (
    <WavePicker
      source={waveSourceOf(value(type), value(id))}
      unknownGroupLabel={`Type ${value(type)} ID ${value(id)}`}
      stereo
      l={value(l)}
      r={value(r)}
      onChange={(side, n) => onChange(side === "l" ? l : r, n)}
      onGroupChange={(source) => {
        if (!source.params) return;
        const [groupType, groupId] = source.params;
        onChange(type, groupType);
        onChange(id, groupId);
      }}
      marking={marking}
    />
  );
}
