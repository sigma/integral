// Every SN Synth and PCM Synth envelope Display spans exactly the columns of
// its envelope's faders, depth included where the box has one (#415). A
// re-layout of these boxes keeps this test passing.
import { describe, expect, it } from "vitest";
import {
  DisplayKind,
  SnsPartialField,
  ToneType,
  type PcmsPartialField,
  type WasmDisplaySlot,
} from "../../pkg/integral_wasm.js";
import { displaySlots } from "./displays";
import type { FieldKey } from "./fields";
import { allElements, itemRect } from "./geometry";
import { PCMS_BOXES } from "./pcm-synth/sections";
import { SNS_AMP_LAYOUT, SNS_FILTER_LAYOUT, SNS_OSC_LAYOUT } from "./sn-synth/partial";
import type { ControlLayout, SectionLayout } from "./types";

/** An envelope Display in a box, and the fields of the faders it describes. */
interface EnvelopeCase<F extends FieldKey> {
  name: string;
  layout: SectionLayout<F>;
  display: DisplayKind;
  /** The fields its faders edit: those core binds to the Display, and the box's depth fader, if any. */
  fields: readonly F[];
}

/** The fields of the block `fieldOf` reads that core binds to `display` in the `toneType` editor. */
function boundFields<F>(display: DisplayKind, toneType: ToneType, fieldOf: (s: WasmDisplaySlot) => F | undefined): F[] {
  return displaySlots(display, toneType).flatMap((s) => {
    const f = fieldOf(s);
    return f === undefined ? [] : [f];
  });
}

const sns = (name: string, layout: SectionLayout<SnsPartialField>, display: DisplayKind, depth?: SnsPartialField) => ({
  name: `SN Synth ${name}`,
  layout,
  display,
  fields: [...boundFields(display, ToneType.SnSynth, (s) => s.snsPartial), ...(depth === undefined ? [] : [depth])],
});

const pcms = (name: string, layout: SectionLayout<PcmsPartialField>, display: DisplayKind) => ({
  name: `PCM Synth ${name}`,
  layout,
  display,
  fields: boundFields(display, ToneType.PcmSynth, (s) => s.pcmsPartial),
});

const CASES: readonly EnvelopeCase<FieldKey>[] = [
  sns("OSC", SNS_OSC_LAYOUT, DisplayKind.PitchEnvelope, SnsPartialField.OscPitchEnvDepth),
  sns("FILTER", SNS_FILTER_LAYOUT, DisplayKind.FilterEnvelope, SnsPartialField.FilterEnvDepth),
  sns("AMP", SNS_AMP_LAYOUT, DisplayKind.AmpEnvelope),
  pcms("PITCH", PCMS_BOXES.pitch.layout, DisplayKind.PitchEnvelope),
  pcms("FILTER", PCMS_BOXES.tvf.layout, DisplayKind.FilterEnvelope),
  pcms("AMP", PCMS_BOXES.tva.layout, DisplayKind.AmpEnvelope),
];

describe("SN Synth and PCM Synth envelope Displays", () => {
  it.each(CASES.map((c) => [c.name, c] as const))("%s: the curve spans exactly its faders' columns", (_, c) => {
    const elements = allElements(c.layout.items);
    const curves = elements.filter((el) => el.kind === "display" && el.display === c.display);
    expect(curves).toHaveLength(1);
    const faders = elements.filter(
      (el): el is ControlLayout<FieldKey> => el.kind === "fader" && c.fields.includes(el.field),
    );
    // Every field of the envelope has its fader in the box, so a lost fader can't shrink the span.
    expect(new Set(faders.map((f) => f.field))).toEqual(new Set(c.fields));
    const rects = faders.map(itemRect);
    const span = {
      from: Math.min(...rects.map((r) => r.x)),
      to: Math.max(...rects.map((r) => r.x + r.w)),
    };
    const curve = itemRect(curves[0]!);
    expect({ from: curve.x, to: curve.x + curve.w }).toEqual(span);
  });

  it("counts a box's depth fader: SN Synth OSC's pitch envelope spans A, D and Dep", () => {
    const osc = CASES[0]!;
    const labels = allElements(osc.layout.items)
      .filter((el) => el.kind === "fader" && osc.fields.includes(el.field))
      .map((el) => (el.kind === "fader" ? el.label : undefined));
    expect(labels).toEqual(["A", "D", "Dep"]);
  });
});
