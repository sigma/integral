// The SN Synth partial boxes put each visualizer over the Controls it
// describes (#427): OSC stacks its Wave scope over its pitch envelope, over
// the envelope's faders; AMP's knobs sit over its envelope, over its faders;
// the LFOs' knobs sit right, under their scopes. FILTER stacks its response
// scope, its knobs, its envelope and its faders, five knobs wide (#430).
import { describe, expect, it } from "vitest";
import { DisplayKind, SnsPartialField as F } from "../../../pkg/integral_wasm.js";
import { FOOTPRINT } from "../../synth-ui/grid";
import { allElements, itemRect } from "../geometry";
import type { CellRect, ElementLayout, SectionLayout } from "../types";
import {
  OSC_VARIATION_GROUP,
  OSC_WAVE_ALTERNATIVE,
  OSC_WAVE_GROUP,
  SNS_AMP_LAYOUT,
  SNS_BOXES,
  SNS_FILTER_LAYOUT,
  SNS_LFO_LAYOUT,
  SNS_MOD_LFO_LAYOUT,
  SNS_OSC_LAYOUT,
} from "./partial";

type Layout = SectionLayout<F>;

/** The element of `layout` editing `field`. */
function control(layout: Layout, field: F): ElementLayout<F> {
  const el = allElements(layout.items).find((e) => e.kind !== "display" && e.field === field);
  if (!el) throw new Error(`no Control for field ${F[field]}`);
  return el;
}

/** The Cells of `layout`'s `display`. */
function display(layout: Layout, kind: DisplayKind): CellRect {
  const el = allElements(layout.items).find((e) => e.kind === "display" && e.display === kind);
  if (!el) throw new Error(`no ${DisplayKind[kind]} Display`);
  return itemRect(el);
}

/** The Cells of `layout`'s `alternatives` group `group`. */
function group(layout: Layout, name: string): CellRect {
  const item = layout.items.find((i) => i.kind === "alternatives" && i.group === name);
  if (!item) throw new Error(`no ${name} group`);
  return itemRect(item);
}

const rect = (layout: Layout, field: F) => itemRect(control(layout, field));
const right = (r: CellRect) => r.x + r.w;
const bottom = (r: CellRect) => r.y + r.h;
/** Whether `a` and `b` share a row. */
const rowsOverlap = (a: CellRect, b: CellRect) => a.y < bottom(b) && b.y < bottom(a);

describe("SN Synth OSC", () => {
  const env = display(SNS_OSC_LAYOUT, DisplayKind.PitchEnvelope);
  const envFaders = [F.OscPitchEnvAttack, F.OscPitchEnvDecay, F.OscPitchEnvDepth].map((f) => rect(SNS_OSC_LAYOUT, f));

  it("sits its pitch envelope directly above A, D and Dep, spanning them", () => {
    expect({ from: env.x, to: right(env) }).toEqual({ from: envFaders[0]!.x, to: right(envFaders[2]!) });
    for (const f of envFaders) expect(f.y).toBe(bottom(env));
  });

  it("stacks the Wave's scope, or its PCM picker, above the pitch envelope, as wide", () => {
    const wave = group(SNS_OSC_LAYOUT, OSC_WAVE_GROUP);
    expect({ x: wave.x, w: wave.w }).toEqual({ x: env.x, w: env.w });
    expect(bottom(wave)).toBeLessThanOrEqual(env.y);
    const scope = display(SNS_OSC_LAYOUT, DisplayKind.OscScope);
    expect({ x: scope.x, w: scope.w }).toEqual({ x: env.x, w: env.w });
    const pcm = SNS_OSC_LAYOUT.items.find((i) => i.kind === "alternatives" && i.group === OSC_WAVE_GROUP);
    expect(pcm?.kind === "alternatives" && pcm.options.map((o) => o.name)).toEqual([
      OSC_WAVE_ALTERNATIVE.scope,
      OSC_WAVE_ALTERNATIVE.pcm,
    ]);
  });

  it("puts the Wave dropdown and the Var knob to the scope's left, on its rows", () => {
    const scope = display(SNS_OSC_LAYOUT, DisplayKind.OscScope);
    const waveDropdown = rect(SNS_OSC_LAYOUT, F.OscWave);
    const variation = group(SNS_OSC_LAYOUT, OSC_VARIATION_GROUP);
    for (const r of [waveDropdown, variation]) {
      expect(right(r)).toBeLessThanOrEqual(scope.x);
      expect(rowsOverlap(r, scope)).toBe(true);
    }
    expect(right(waveDropdown)).toBeLessThanOrEqual(variation.x);
  });

  it("keeps Pitch, HPF and Detune over PWM, PW and Shift", () => {
    const pairs = [
      [F.OscPitch, F.OscPwModDepth],
      [F.HpfCutoff, F.OscPulseWidth],
      [F.OscDetune, F.PwShift],
    ] as const;
    for (const [knob, fader] of pairs) {
      const [k, f] = [rect(SNS_OSC_LAYOUT, knob), rect(SNS_OSC_LAYOUT, fader)];
      expect(k.x).toBe(f.x);
      expect(bottom(k)).toBe(f.y);
    }
  });

  it("draws a rule between the PWM, PW and Shift faders and the A, D and Dep faders", () => {
    const shift = rect(SNS_OSC_LAYOUT, F.PwShift);
    const attack = envFaders[0]!;
    const rules = (SNS_OSC_LAYOUT.silkscreen ?? []).filter((s) => s.kind === "rule");
    const between = rules.filter((r) => {
      const xs = r.points.map(([x]) => x);
      const ys = r.points.map(([, y]) => y);
      const vertical = xs.every((x) => x === xs[0]);
      return (
        vertical &&
        xs[0]! >= right(shift) &&
        xs[0]! <= attack.x &&
        Math.min(...ys) <= attack.y &&
        Math.max(...ys) >= bottom(attack)
      );
    });
    expect(between).toHaveLength(1);
  });
});

describe("SN Synth FILTER", () => {
  const scope = display(SNS_FILTER_LAYOUT, DisplayKind.FilterScope);
  const env = display(SNS_FILTER_LAYOUT, DisplayKind.FilterEnvelope);
  const knobFields = [F.FilterCutoff, F.FilterResonance, F.FilterKeyfollow, F.FilterEnvVelSens, F.AftertouchCutoff];
  const faderFields = [F.FilterEnvAttack, F.FilterEnvDecay, F.FilterEnvSustain, F.FilterEnvRelease, F.FilterEnvDepth];
  const knobs = knobFields.map((f) => control(SNS_FILTER_LAYOUT, f));
  const faders = faderFields.map((f) => rect(SNS_FILTER_LAYOUT, f));

  it("is five knobs wide, past its free first column", () => {
    expect(SNS_FILTER_LAYOUT.w).toBe(1 + 5 * FOOTPRINT.knobSm.w);
    expect(knobs.map((k) => k.kind)).toEqual(knobs.map(() => "knob"));
    const rects = knobs.map(itemRect);
    expect({ from: rects[0]!.x, to: right(rects[4]!) }).toEqual({ from: 1, to: SNS_FILTER_LAYOUT.w });
  });

  it("stacks the response scope, the knobs in one row, the envelope and the faders, top to bottom", () => {
    const rects = knobs.map(itemRect);
    expect(new Set(rects.map((r) => r.y)).size).toBe(1);
    expect(bottom(scope)).toBeLessThanOrEqual(rects[0]!.y);
    expect(bottom(rects[0]!)).toBeLessThanOrEqual(env.y);
    for (const f of faders) expect(f.y).toBe(bottom(env));
  });

  it("aligns each knob over a fader, and spans the envelope across A to Dep", () => {
    expect(knobs.map((k) => itemRect(k).x)).toEqual(faders.map((f) => f.x));
    expect({ from: env.x, to: right(env) }).toEqual({ from: faders[0]!.x, to: right(faders[4]!) });
  });

  it("stacks the Mode and Slope dropdowns at the scope's left, on its rows", () => {
    const [mode, slope] = [F.FilterMode, F.FilterSlope].map((f) => rect(SNS_FILTER_LAYOUT, f));
    for (const r of [mode!, slope!]) {
      expect(right(r)).toBeLessThanOrEqual(scope.x);
      expect(rowsOverlap(r, scope)).toBe(true);
    }
    expect(mode!.x).toBe(slope!.x);
    expect(bottom(mode!)).toBeLessThanOrEqual(slope!.y);
    expect(right(scope)).toBe(SNS_FILTER_LAYOUT.w);
  });
});

describe("SN Synth AMP", () => {
  const env = display(SNS_AMP_LAYOUT, DisplayKind.AmpEnvelope);
  const release = rect(SNS_AMP_LAYOUT, F.AmpEnvRelease);

  it("draws Level as a fader right of R, on the same rows", () => {
    const level = control(SNS_AMP_LAYOUT, F.AmpLevel);
    expect(level.kind).toBe("fader");
    const r = itemRect(level);
    expect(r.x).toBeGreaterThanOrEqual(right(release));
    expect({ y: r.y, h: r.h }).toEqual({ y: release.y, h: release.h });
    // The envelope spans A to R, not Level.
    expect(right(env)).toBe(right(release));
  });

  it("puts Gain, Pan, Vel Sns, Aft Lvl and KeyF in one row of knobs above the envelope, over the faders", () => {
    const knobs = [F.WaveGain, F.AmpPan, F.AmpVelSens, F.AftertouchLevel, F.AmpLevelKeyfollow].map((f) =>
      control(SNS_AMP_LAYOUT, f),
    );
    expect(knobs.map((k) => k.kind)).toEqual(knobs.map(() => "knob"));
    const rects = knobs.map(itemRect);
    expect(new Set(rects.map((r) => r.y)).size).toBe(1);
    expect(rects.map((r) => r.x)).toEqual([...rects.map((r) => r.x)].sort((a, b) => a - b));
    expect(bottom(rects[0]!)).toBeLessThanOrEqual(env.y);
    expect(bottom(env)).toBe(release.y);
  });
});

describe.each([
  ["MOD LFO", SNS_MOD_LFO_LAYOUT, DisplayKind.ModLfoScope, F.ModLfoShape, [F.ModLfoRate, F.ModLfoRateControl]],
  ["LFO", SNS_LFO_LAYOUT, DisplayKind.LfoScope, F.LfoShape, [F.LfoRate, F.LfoFadeTime]],
] as const)("SN Synth %s", (_, layout, scopeKind, shape, knobFields) => {
  const knobs = knobFields.map((f) => rect(layout, f));
  const scope = display(layout, scopeKind);

  it("aligns its knobs to the right, under the scope", () => {
    expect(Math.max(...knobs.map(right))).toBe(layout.w);
    for (const k of knobs) expect(k.y).toBeGreaterThanOrEqual(bottom(scope));
  });

  it("keeps the Shape dropdown at the left", () => {
    const picker = control(layout, shape);
    expect(picker.kind).toBe("picker");
    expect(right(itemRect(picker))).toBeLessThanOrEqual(Math.min(...knobs.map((k) => k.x)));
  });
});

describe("SN Synth partial row", () => {
  it("keeps every box one height, its faders on the same rows", () => {
    expect(new Set(SNS_BOXES.map((b) => b.layout.h)).size).toBe(1);
    const faders = SNS_BOXES.flatMap((b) => allElements(b.layout.items).filter((e) => e.kind === "fader").map(itemRect));
    expect(new Set(faders.map((f) => `${f.y}+${f.h}`))).toEqual(new Set([`${faders[0]!.y}+${faders[0]!.h}`]));
    // The faders fill the bottom of every box.
    for (const f of faders) expect(bottom(f)).toBe(SNS_BOXES[0].layout.h);
  });
});
