// The drum kits' LEVEL / PAN strip: one arrangement, from one builder, in both kits.
import { describe, expect, it } from "vitest";
import { PcmdPartialField, SndNoteField } from "../../../pkg/integral_wasm.js";
import type { FieldKey } from "../fields";
import { allElements, itemRect } from "../geometry";
import type { SectionLayout } from "../types";
import { PCMD_OUTPUT_LAYOUT } from "./pcmDrum";
import { SND_OUTPUT_LAYOUT } from "./snDrum";

/** Each kit's strip and the fields it places. */
const KITS: { name: string; layout: SectionLayout<FieldKey>; pan: FieldKey; sends: FieldKey[]; level: FieldKey }[] = [
  {
    name: "PCM Drum",
    layout: PCMD_OUTPUT_LAYOUT,
    pan: PcmdPartialField.Pan,
    sends: [PcmdPartialField.ChorusSend, PcmdPartialField.ReverbSend],
    level: PcmdPartialField.OutputLevel,
  },
  {
    name: "SN Drum",
    layout: SND_OUTPUT_LAYOUT,
    pan: SndNoteField.Pan,
    sends: [SndNoteField.ChorusSend, SndNoteField.ReverbSend],
    level: SndNoteField.Level,
  },
];

/** The item placing `field` in `layout`, and its Cells. */
function at(layout: SectionLayout<FieldKey>, field: FieldKey) {
  const item = allElements(layout.items).find((i) => "field" in i && i.field === field);
  if (!item) throw new Error(`no field ${String(field)}`);
  return { item, rect: itemRect(item) };
}

/** The fader's length, in Cells, in `kit`'s strip. */
const faderH = (kit: (typeof KITS)[number]) => at(kit.layout, kit.level).rect.h;

describe("the drum kits' LEVEL / PAN strip", () => {
  it.each(KITS)("$name: Pan, the sends side by side under it, then the scaled fader to the strip's foot", (kit) => {
    const { layout } = kit;
    expect(layout.w).toBe(4);
    const pan = at(layout, kit.pan).rect;
    const sends = kit.sends.map((f) => at(layout, f).rect);
    expect(new Set(sends.map((s) => s.y)).size).toBe(1);
    for (const s of sends) expect(s.y).toBeGreaterThanOrEqual(pan.y + pan.h);
    // Side by side: neither send overlaps the other's columns.
    const [a, b] = sends as [(typeof sends)[0], (typeof sends)[0]];
    expect(a.x + a.w <= b.x || b.x + b.w <= a.x).toBe(true);
    const knobsFoot = Math.max(pan.y + pan.h, ...sends.map((s) => s.y + s.h));
    const fader = at(layout, kit.level);
    expect(fader.rect.y).toBeGreaterThanOrEqual(knobsFoot);
    expect(fader.rect.y + fader.rect.h).toBe(layout.h);
    expect(fader.item.kind === "fader" && fader.item.scale).toBe(true);
  });

  it("is one arrangement in both kits, SN Drum's fader as long as PCM Drum's at least", () => {
    const [pcmd, snd] = KITS as [(typeof KITS)[0], (typeof KITS)[0]];
    const shape = (kit: (typeof KITS)[number]) =>
      [kit.pan, ...kit.sends, kit.level].map((f) => {
        const { x, y, w } = at(kit.layout, f).rect;
        return { x, y, w };
      });
    expect(shape(snd)).toEqual(shape(pcmd));
    expect(faderH(snd)).toBeGreaterThanOrEqual(faderH(pcmd));
  });
});
