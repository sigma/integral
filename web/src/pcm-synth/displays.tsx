/**
 * The PCM Synth editor's Displays, drawn from core's Display bindings
 * (`display_binding`): which fields each reads is core's, and this module
 * only turns the values read into a picture. The wave picker writes the
 * fields it reads.
 */

import {
  DisplayKind,
  DisplayProp,
  ToneType,
  type PcmsPartialField,
  type WasmDisplaySlot,
} from "../../pkg/integral_wasm.js";
import { BreakpointDisplay, displaySlots, slotField, type SlotValue } from "../layouts";
import { FilterScope } from "../synth-ui/FilterScope";
import { LfoScope } from "../synth-ui/LfoScope";
import type { ExpansionMarking } from "../WavePicker";
import { WaveDisplay } from "../WaveDisplay";
import { partialSpec, partialValue, pmtValue } from "./binding";
import { StructureDiagram } from "./StructureDiagram";
import type { PcmSynthPartial, PcmSynthPmt } from "./types";
import css from "../PcmSynthEditor.module.css";

/** The Partial field a slot reads. */
const partialField = (slot: WasmDisplaySlot): PcmsPartialField => slotField(slot, slot.pcmsPartial);

/** Props of {@link PartialDisplay}. */
export interface PartialDisplayProps {
  /** Which Display. */
  kind: DisplayKind;
  /** The partial the Scope names. */
  partial: PcmSynthPartial;
  /** Writes one of its fields (element 0): the wave picker's. */
  onChange: (field: PcmsPartialField, value: number) => void;
  /** Loaded state of the Expansions behind SRX Wave Groups. */
  marking?: ExpansionMarking;
}

/** One Display of a PCM Synth partial's box, filling its Cells. */
export function PartialDisplay({ kind, partial, onChange, marking }: PartialDisplayProps) {
  const slots = displaySlots(kind, ToneType.PcmSynth);
  const read = (slot: WasmDisplaySlot): SlotValue => {
    const field = partialField(slot);
    const { min, max } = partialSpec(field);
    return { value: partialValue(partial, field, slot.index), min, max };
  };
  /** The slots feeding `prop`. */
  const of = (prop: DisplayProp) => slots.filter((s) => s.prop === prop);
  /** The raw value of the first slot feeding `prop`. */
  const raw = (prop: DisplayProp) => {
    const slot = of(prop)[0];
    return slot ? read(slot).value : 0;
  };
  /** Core's name of the value of the first slot feeding `prop`. */
  const label = (prop: DisplayProp) => {
    const slot = of(prop)[0];
    return slot ? partialSpec(partialField(slot)).formatValue(read(slot).value) : "";
  };
  const steps = () => of(DisplayProp.Steps).map((s) => read(s).value);
  const smooth = () => raw(DisplayProp.Smooth) === 1;
  switch (kind) {
    case DisplayKind.PitchEnvelope:
    case DisplayKind.FilterEnvelope:
    case DisplayKind.AmpEnvelope:
      return <BreakpointDisplay kind={kind} toneType={ToneType.PcmSynth} read={read} />;
    case DisplayKind.FilterScope:
      return (
        <FilterScope
          className={css.fill}
          type={label(DisplayProp.FilterType)}
          cutoff={raw(DisplayProp.Cutoff)}
          resonance={raw(DisplayProp.Resonance)}
        />
      );
    case DisplayKind.Lfo1Scope:
    case DisplayKind.Lfo2Scope:
      return (
        <LfoScope
          className={css.fill}
          shape={label(DisplayProp.Shape)}
          rate={raw(DisplayProp.Rate)}
          steps={steps()}
          smooth={smooth()}
        />
      );
    case DisplayKind.StepLfoScope:
      return <LfoScope className={css.fill} shape="STEP" rate={0} steps={steps()} smooth={smooth()} />;
    case DisplayKind.WavePicker:
      return (
        <WaveDisplay
          toneType={ToneType.PcmSynth}
          fieldOf={(s) => s.pcmsPartial}
          value={(field) => partialValue(partial, field)}
          onChange={onChange}
          marking={marking}
        />
      );
    default:
      return null;
  }
}

/** One Display of PARTIAL MIX: a pair's Structure, drawn as its signal path. */
export function PmtDisplay({ kind, pmt }: { kind: DisplayKind; pmt: PcmSynthPmt }) {
  const slots = displaySlots(kind, ToneType.PcmSynth);
  const structure = slots.find((s) => s.prop === DisplayProp.Structure);
  const first = slots.find((s) => s.prop === DisplayProp.First)?.fixed;
  if (!structure || first === undefined) return null;
  const value = pmtValue(pmt, slotField(structure, structure.pcmsPmt));
  return <StructureDiagram className={css.fill} structure={value} first={first} />;
}
