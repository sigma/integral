import { DisplayKind, DisplayProp, type ToneType, type WasmDisplaySlot } from "../pkg/integral_wasm.js";
import { slotField, slotOf } from "./layouts";
import { WavePicker, type ExpansionMarking } from "./WavePicker";
import { waveSourceOf } from "./waveCatalog";

/** Props of {@link WaveDisplay}. */
export interface WaveDisplayProps<F> {
  /** The editor whose wave picker binding names the fields. */
  toneType: ToneType;
  /** The field of the editor's block a slot reads (one of the slot's getters). */
  fieldOf: (slot: WasmDisplaySlot) => F | undefined;
  /** A field's value, in the block the Scope names. */
  value: (field: F) => number;
  /** Writes a field, in the block the Scope names. */
  onChange: (field: F, value: number) => void;
  /** Loaded state of the Expansions behind SRX Wave Groups. */
  marking?: ExpansionMarking;
}

/**
 * The wave picker Display: a Wave Group and L/R Wave Numbers, picked by
 * name, its fields from core's binding for `toneType` (a PCM Synth
 * Partial's, a PCM Drum WMT layer's).
 */
export function WaveDisplay<F>({ toneType, fieldOf, value, onChange, marking }: WaveDisplayProps<F>) {
  const field = (prop: DisplayProp): F => {
    const slot = slotOf(DisplayKind.WavePicker, toneType, prop);
    return slotField(slot, fieldOf(slot));
  };
  const type = field(DisplayProp.WaveGroupType);
  const id = field(DisplayProp.WaveGroupId);
  const l = field(DisplayProp.WaveNumberL);
  const r = field(DisplayProp.WaveNumberR);
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
