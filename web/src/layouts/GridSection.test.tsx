import { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  DisplayKind,
  MixerField,
  MixerScopeKind,
  PcmsPartialField,
  Relevance,
  SndNoteField,
  WasmDeviceState,
  mixer_field_spec,
  pcms_partial_count,
  pcms_partial_spec,
  snd_note_spec,
} from "../../pkg/integral_wasm.js";
import { paramFormatter, paramMeta } from "../paramDefaults";
import { CELL, FOOTPRINT } from "../synth-ui/grid";
import { cleanup, click, render, typeInto } from "../test/dom";
import { expectFillsFootprint, stackedHeight } from "../test/grid";
import { arrowUp, dt1Drainer, slider } from "../test/toneService";
import { specBinding } from "./binding";
import { enumFields, mixerFields } from "./fields";
import { GridScreen } from "./GridScreen";
import { BORDER, DISPLAY_INSET, INSET } from "./geometry";
import { GridSection, type Binding } from "./GridSection";
import { EQ_LAYOUT } from "./mixer/eq";
import type { ScreenLayout, SectionLayout } from "./types";

afterEach(cleanup);

/** Binds a mixer field to its core range, at its default. */
function bindMixer(field: MixerField): Binding {
  const meta = paramMeta(field);
  return { ...meta, value: meta.defaultValue, format: paramFormatter(field), onChange: () => {} };
}

/** The element wrapping the Control bound to `field`. */
function wrapper(container: HTMLElement, field: MixerField): HTMLElement {
  const el = container.querySelector<HTMLElement>(`[data-field="${field}"]`);
  if (!el) throw new Error(`no Control for ${MixerField[field]}`);
  return el;
}

describe("GridSection draws the pilot mixer EQ", () => {
  // Every px below is what the pilot renderer (prototype/grid-pilot) drew, the knobs and
  // Silkscreen a row higher since the switch moved into the title (#405), and blank rows
  // under the last knob to match the staggered Chorus and Reverb Sections.
  const draw = () => render(<GridSection layout={EQ_LAYOUT} bind={bindMixer} />).container;

  it("at the pilot's size, title row included", () => {
    const root = draw().firstElementChild as HTMLElement;
    expect(root.style.width).toBe("96px");
    expect(root.style.height).toBe("432px");
    const [border, title] = [...root.children] as HTMLElement[];
    expect(border!.style.inset).toBe("4px");
    expect(title!.textContent).toBe("EQ");
    expect([title!.style.left, title!.style.top, title!.style.width, title!.style.height]).toEqual([
      "4px",
      "4px",
      "88px",
      "20px",
    ]);
  });

  // The left column's knobs clear the border their LED rings reach.
  const SIDE = INSET + BORDER.width;
  it.each([
    [MixerField.EqHighFreq, SIDE, 24],
    [MixerField.EqHighGain, 48, 72],
    [MixerField.EqMidFreq, SIDE, 120],
    [MixerField.EqMidGain, 48, 168],
    [MixerField.EqMidQ, SIDE, 216],
    [MixerField.EqLowGain, 48, 264],
    [MixerField.EqLowFreq, SIDE, 312],
  ])("places field %i at (%ipx, %ipx)", (field, left, top) => {
    const el = wrapper(draw(), field);
    expect([el.style.left, el.style.top]).toEqual([`${left}px`, `${top}px`]);
  });

  it("draws each Control at its footprint, unscaled, less the border it clears", () => {
    const container = draw();
    const knob = wrapper(container, MixerField.EqMidQ).firstElementChild as HTMLElement;
    expect([knob.style.width, knob.style.height]).toEqual([`${FOOTPRINT.knobSm.w * 24 - SIDE}px`, `${FOOTPRINT.knobSm.h * 24}px`]);
  });

  it("names knobs as the layout says", () => {
    expect(wrapper(draw(), MixerField.EqMidQ).querySelector("span")!.textContent).toBe("Q");
  });

  /** The EQ with its switch at `value`, recording what the title toggle writes. */
  const drawSwitch = (value: number, relevance?: (f: MixerField) => Relevance) => {
    const writes: number[] = [];
    const bind = (field: MixerField): Binding =>
      field === MixerField.EqSwitch ? { ...bindMixer(field), value, onChange: (v) => writes.push(v) } : bindMixer(field);
    const container = render(<GridSection layout={EQ_LAYOUT} bind={bind} relevance={relevance} />).container;
    const title = (container.firstElementChild as HTMLElement).children[1] as HTMLElement;
    return { title, button: title.querySelector("button")!, writes };
  };

  it("draws its switch as a power toggle in the title, beside the title's text, and not in the body", () => {
    const { title, button } = drawSwitch(1);
    expect(title.textContent).toBe("EQ");
    expect(button.closest("[data-field]")!.getAttribute("data-field")).toBe(String(MixerField.EqSwitch));
    expect(button.querySelector("svg")).not.toBeNull();
    expect(title.parentElement!.querySelectorAll(`[data-field="${MixerField.EqSwitch}"]`)).toHaveLength(1);
  });

  it("lights the title toggle while on, says what a click does, and writes the other state", () => {
    const on = drawSwitch(1);
    expect(on.button.getAttribute("aria-pressed")).toBe("true");
    expect(on.button.title).toBe("Switch EQ off");
    act(() => on.button.click());
    expect(on.writes).toEqual([0]);
    cleanup();
    const off = drawSwitch(0);
    expect(off.button.getAttribute("aria-pressed")).toBe("false");
    expect(off.button.title).toBe("Switch EQ on");
    act(() => off.button.click());
    expect(off.writes).toEqual([1]);
  });

  it("never dims the title toggle", () => {
    const { button } = drawSwitch(0, () => Relevance.Irrelevant);
    const host = button.closest<HTMLElement>("[data-field]")!;
    expect(host.dataset.inactive).toBeUndefined();
    expect(button.closest("[inert]")).toBeNull();
  });

  it("draws the zig-zag rules and band names, clipped to the border", () => {
    const svg = [...draw().querySelectorAll("svg")].find((s) => !s.closest("button"))!;
    expect(svg.style.clipPath).toBe("inset(5px round 6px)");
    expect([...svg.querySelectorAll("polyline")].map((p) => p.getAttribute("points"))).toEqual([
      "0,108 24,108 72,156 96,156",
      "0,300 24,300 72,252 96,252",
    ]);
    expect([...svg.querySelectorAll("text")].map((t) => [t.textContent, t.getAttribute("x"), t.getAttribute("y")])).toEqual([
      ["HI", "72", "60"],
      ["MID", "24", "204"],
      ["LO", "72", "348"],
    ]);
  });

  it("draws a band name in the band style and a title-style group name in the Section title's type", () => {
    const layout: SectionLayout<MixerField> = {
      ...EQ_LAYOUT,
      silkscreen: [
        { kind: "label", text: "HI", x: 3, y: 1 },
        { kind: "label", text: "OSC", x: 1, y: 0.5, style: "title" },
      ],
    };
    const texts = [...render(<GridSection layout={layout} bind={bindMixer} />).container.querySelectorAll("svg text")];
    const classOf = (name: string) => texts.find((t) => t.textContent === name)!.getAttribute("class");
    expect(classOf("HI")).toMatch(/groupName/);
    expect(classOf("OSC")).toMatch(/groupTitle/);
  });
});

/** A Section of three switches listed out of reading order, plus a group and a Display, all clear of its border. */
const SCRAMBLED: SectionLayout<MixerField> = {
  title: "Part {part}",
  titleEdge: "side",
  w: 5,
  h: 5,
  fields: mixerFields([MixerScopeKind.Part]),
  items: [
    { kind: "switch", field: MixerField.EqMidQ, x: 2, y: 1, inactive: "hide" },
    { kind: "switch", field: MixerField.EqLowGain, x: 2, y: 0 },
    { kind: "switch", field: MixerField.EqSwitch, x: 0, y: 1 },
    {
      kind: "alternatives",
      group: "sends",
      x: 0,
      y: 0,
      w: 2,
      h: 1,
      options: [
        { name: "low", items: [{ kind: "switch", field: MixerField.EqLowFreq, x: 0, y: 0 }] },
        { name: "high", items: [{ kind: "switch", field: MixerField.EqHighFreq, x: 0, y: 0 }] },
      ],
    },
    { kind: "display", display: DisplayKind.CompEnvelope, x: 0, y: 2, w: 4, h: 2 },
  ],
};

/** Field ids in DOM order. */
const fieldOrder = (root: ParentNode): string[] =>
  [...root.querySelectorAll<HTMLElement>("[data-field]")].map((el) => el.dataset.field ?? "");

describe("GridSection", () => {
  it("puts Controls in the DOM row-major, so focus goes row by row", () => {
    const { container } = render(<GridSection layout={SCRAMBLED} bind={bindMixer} />);
    expect(fieldOrder(container)).toEqual(
      [MixerField.EqLowFreq, MixerField.EqLowGain, MixerField.EqSwitch, MixerField.EqMidQ].map(String),
    );
  });

  it("draws the first alternative by default, and the one the caller picks", () => {
    const first = render(<GridSection layout={SCRAMBLED} bind={bindMixer} />).container;
    expect(fieldOrder(first)).toContain(String(MixerField.EqLowFreq));
    expect(fieldOrder(first)).not.toContain(String(MixerField.EqHighFreq));
    const picked = render(<GridSection layout={SCRAMBLED} bind={bindMixer} alternative={() => "high"} />).container;
    expect(fieldOrder(picked)).toContain(String(MixerField.EqHighFreq));
    expect(fieldOrder(picked)).not.toContain(String(MixerField.EqLowFreq));
  });

  it("keeps every Control active by default", () => {
    const { container } = render(<GridSection layout={SCRAMBLED} bind={bindMixer} />);
    expect(container.querySelector("[data-inactive]")).toBeNull();
    expect(container.querySelector("[inert]")).toBeNull();
  });

  it("dims or hides an irrelevant Control in its Cells, inert", () => {
    const irrelevant = (f: MixerField) =>
      f === MixerField.EqMidQ || f === MixerField.EqSwitch ? Relevance.Irrelevant : Relevance.Active;
    const { container } = render(<GridSection layout={SCRAMBLED} bind={bindMixer} relevance={irrelevant} />);
    const hidden = wrapper(container, MixerField.EqMidQ);
    const dimmed = wrapper(container, MixerField.EqSwitch);
    expect(hidden.dataset.inactive).toBe("hide");
    expect(dimmed.dataset.inactive).toBe("dim");
    for (const el of [hidden, dimmed]) {
      expect(el.hasAttribute("inert")).toBe(true);
      expect([el.style.width, el.style.height]).toEqual(["48px", "24px"]);
    }
    expect(wrapper(container, MixerField.EqLowGain).hasAttribute("inert")).toBe(false);
  });

  it("dims a switched-off Control and a Display in their Cells, still editable", () => {
    const { container } = render(
      <GridSection
        layout={SCRAMBLED}
        bind={bindMixer}
        relevance={(f) =>
          f === MixerField.EqLowGain || f === MixerField.EqMidQ ? Relevance.SwitchedOff : Relevance.Active
        }
        displayRelevance={() => Relevance.SwitchedOff}
      />,
    );
    const dimmed = wrapper(container, MixerField.EqLowGain);
    // Laid out to hide, it dims all the same: hidden, it couldn't be edited.
    const shown = wrapper(container, MixerField.EqMidQ);
    const display = container.querySelector<HTMLElement>("[data-display]")!;
    for (const el of [dimmed, shown, display]) {
      expect(el.dataset.inactive).toBe("dim");
      expect(el.hasAttribute("inert")).toBe(false);
    }
  });

  it("goes inert on an irrelevant Display", () => {
    const { container } = render(
      <GridSection layout={SCRAMBLED} bind={bindMixer} displayRelevance={() => Relevance.Irrelevant} />,
    );
    expect(container.querySelector("[data-display]")!.hasAttribute("inert")).toBe(true);
  });

  it("fills the title from the Scope, 1-based", () => {
    const { container } = render(<GridSection layout={SCRAMBLED} bind={bindMixer} scope={{ part: 2 }} />);
    expect(container.querySelector("[data-grid-section]")!.getAttribute("data-grid-section")).toBe("Part 3");
  });

  it("hands each Display its rectangle in px", () => {
    const sizes: string[] = [];
    render(
      <GridSection
        layout={SCRAMBLED}
        bind={bindMixer}
        renderDisplay={(kind, { width, height }) => {
          sizes.push(`${DisplayKind[kind]} ${width}×${height}`);
          return null;
        }}
      />,
    );
    expect(sizes).toEqual(["CompEnvelope 96×48"]);
  });
});

describe("GridSection edge padding", () => {
  /** Side-titled, so its top, right and bottom edges have no title: every element sits on one. */
  const EDGES: SectionLayout<MixerField> = {
    title: "Edges",
    titleEdge: "side",
    w: 4,
    h: 5,
    fields: mixerFields([MixerScopeKind.Part]),
    items: [
      { kind: "switch", field: MixerField.EqSwitch, x: 2, y: 0 },
      { kind: "knob", field: MixerField.EqLowGain, x: 0, y: 0 },
      { kind: "display", display: DisplayKind.CompEnvelope, x: 0, y: 3, w: 4, h: 2 },
    ],
  };
  const draw = () =>
    render(
      <GridSection
        layout={EDGES}
        bind={bindMixer}
        renderDisplay={(_, { width, height }) => <div data-size={`${width}×${height}`} />}
      />,
    ).container;
  /** An element's box, px from the Section's origin: [left, top, right, bottom]. */
  const boxOf = (el: HTMLElement) => {
    const [l, t, w, h] = [el.style.left, el.style.top, el.style.width, el.style.height].map(parseFloat) as [
      number,
      number,
      number,
      number,
    ];
    return [l, t, l + w, t + h];
  };

  it("draws an element in an edge Cell inset on that edge, never over the border", () => {
    const container = draw();
    const root = container.firstElementChild as HTMLElement;
    const [W, H] = [parseFloat(root.style.width), parseFloat(root.style.height)];
    // Inside of the border's line: INSET in, plus the line (the accent on top).
    const inside = [INSET + BORDER.width, INSET + BORDER.accent, W - INSET - BORDER.width, H - INSET - BORDER.width];
    const elements = [...container.querySelectorAll<HTMLElement>("[data-field], [data-display]")];
    expect(elements).toHaveLength(3);
    for (const el of elements) {
      const [l, t, r, b] = boxOf(el);
      expect(l).toBeGreaterThanOrEqual(inside[0]!);
      expect(t).toBeGreaterThanOrEqual(inside[1]!);
      expect(r).toBeLessThanOrEqual(inside[2]!);
      expect(b).toBeLessThanOrEqual(inside[3]!);
    }
    // Each edge loses exactly the border's clearance, and only on border Cells.
    expect(boxOf(wrapper(container, MixerField.EqSwitch))).toEqual([72, 7, 115, 24]);
    const display = container.querySelector<HTMLElement>("[data-display]")!;
    expect(boxOf(display)).toEqual([24, 72, 115, 115]);
  });

  it("draws the Control and the Display to fill the padded box", () => {
    const container = draw();
    const toggle = wrapper(container, MixerField.EqSwitch).firstElementChild as HTMLElement;
    expect([toggle.style.width, toggle.style.height]).toEqual(["43px", "17px"]);
    // Behind the side title a knob's left edge is no border: it loses its top, keeps its width.
    const knob = wrapper(container, MixerField.EqLowGain).firstElementChild as HTMLElement;
    expect([knob.style.width, knob.style.height]).toEqual(["48px", "65px"]);
    expect(container.querySelector("[data-size]")!.getAttribute("data-size")).toBe("91×43");
  });
});

describe("GridSection adjacent Displays", () => {
  /** Top-titled, its Displays clear of the border: two side by side, and one alone below. */
  const DISPLAYS: SectionLayout<MixerField> = {
    title: "Displays",
    titleEdge: "top",
    w: 10,
    h: 6,
    fields: mixerFields([MixerScopeKind.Part]),
    items: [
      { kind: "display", display: DisplayKind.CompEnvelope, x: 1, y: 0, w: 4, h: 2 },
      { kind: "display", display: DisplayKind.CompTransferCurve, x: 5, y: 0, w: 3, h: 3 },
      { kind: "display", display: DisplayKind.FilterScope, x: 1, y: 3, w: 3, h: 2 },
    ],
  };
  const draw = () =>
    render(
      <GridSection
        layout={DISPLAYS}
        bind={bindMixer}
        renderDisplay={(_, { width, height }) => <div data-size={`${width}×${height}`} />}
      />,
    ).container;
  const display = (container: HTMLElement, kind: DisplayKind) =>
    container.querySelector<HTMLElement>(`[data-display="${kind}"]`)!;
  /** `el`'s [left, right] edges, px. */
  const span = (el: HTMLElement): [number, number] => {
    const left = parseFloat(el.style.left);
    return [left, left + parseFloat(el.style.width)];
  };

  it("draws two Displays side by side each inset on the edge they share, a separator in the gap", () => {
    const container = draw();
    const shared = 5 * CELL;
    const [, envRight] = span(display(container, DisplayKind.CompEnvelope));
    const [curveLeft] = span(display(container, DisplayKind.CompTransferCurve));
    expect([envRight, curveLeft]).toEqual([shared - DISPLAY_INSET, shared + DISPLAY_INSET]);
    // Each still fills its Cells on its other edges.
    expect(span(display(container, DisplayKind.CompEnvelope))[0]).toBe(CELL);
    expect(span(display(container, DisplayKind.CompTransferCurve))[1]).toBe(8 * CELL);
    const separators = [...container.querySelectorAll<HTMLElement>("[data-display-separator]")];
    expect(separators).toHaveLength(1);
    // Centred on the shared edge (its CSS centres the rule's line there), so inside the gap.
    expect(parseFloat(separators[0]!.style.left)).toBe(shared);
    // Down the rows both Displays cover: the title row, then two rows.
    expect([separators[0]!.style.top, separators[0]!.style.height]).toEqual([`${CELL}px`, `${2 * CELL}px`]);
  });

  it("hands each inset Display its narrower size", () => {
    const sizes = [...draw().querySelectorAll("[data-size]")].map((el) => el.getAttribute("data-size"));
    expect(sizes).toEqual([`${4 * CELL - DISPLAY_INSET}×${2 * CELL}`, `${3 * CELL - DISPLAY_INSET}×${3 * CELL}`, `${3 * CELL}×${2 * CELL}`]);
  });

  it("draws a Display with no Display beside it at its full width", () => {
    expect(span(display(draw(), DisplayKind.FilterScope))).toEqual([CELL, 4 * CELL]);
  });
});

describe("GridSection knob dials", () => {
  /** Three small knobs: one on the left border, one inside, one tempo-syncable. */
  const KNOBS: SectionLayout<MixerField> = {
    title: "K",
    titleEdge: "top",
    w: 8,
    h: 3,
    fields: mixerFields([MixerScopeKind.Part]),
    items: [
      { kind: "knob", field: MixerField.EqLowGain, x: 0, y: 0 },
      { kind: "knob", field: MixerField.EqMidGain, x: 3, y: 0 },
      { kind: "knob", field: MixerField.EqHighGain, x: 5, y: 0 },
    ],
  };
  const dial = (container: HTMLElement, field: MixerField) => wrapper(container, field).querySelector("svg")!.getAttribute("width");

  /** The High Gain knob tempo-syncable, in `noteMode`, its readout split as core would. */
  const synced =
    (noteMode: boolean, onChange: (noteMode: boolean) => void = () => {}) =>
    (field: MixerField): Binding =>
      field === MixerField.EqHighGain
        ? {
            ...bindMixer(field),
            format: () => "200 ms",
            sync: { labels: ["ms", "♪"], formatNumber: () => (noteMode ? "1/8" : "200"), noteMode, onChange },
          }
        : bindMixer(field);
  /** A knob's root: its name, dial and readout, stacked. */
  const knobRoot = (container: HTMLElement, field: MixerField) => wrapper(container, field).firstElementChild!;

  it("draws every knob of a size in a Section at one dial, the smallest any fits", () => {
    const { container } = render(<GridSection layout={KNOBS} bind={bindMixer} />);
    const sides = [MixerField.EqLowGain, MixerField.EqMidGain, MixerField.EqHighGain].map((f) => dial(container, f));
    expect(new Set(sides).size).toBe(1);
    // The knob on the border gives up its edge padding.
    expect(Number(sides[0])).toBeLessThan(FOOTPRINT.knobSm.w * 24 - INSET);
  });

  it.each([false, true])("draws a synced knob (note mode %s) at the dial and height of an unsynced one", (noteMode) => {
    const plain = render(<GridSection layout={KNOBS} bind={bindMixer} />).container;
    const { container } = render(<GridSection layout={KNOBS} bind={synced(noteMode)} />);
    for (const f of [MixerField.EqMidGain, MixerField.EqHighGain]) {
      expect(dial(container, f), MixerField[f]).toBe(dial(plain, MixerField.EqMidGain));
      const root = knobRoot(container, f);
      // Name, dial and readout: no line under the readout.
      expect(root.children.length, MixerField[f]).toBe(3);
      expect(stackedHeight([...root.children]), MixerField[f]).toBe(stackedHeight([...knobRoot(plain, MixerField.EqMidGain).children]));
    }
  });

  it("reads a synced knob as its number then its unit, the unit a button toggling sync", () => {
    const onChange = vi.fn();
    const { container } = render(<GridSection layout={KNOBS} bind={synced(false, onChange)} />);
    const readout = knobRoot(container, MixerField.EqHighGain).lastElementChild!;
    expect(readout.textContent).toBe("200 ms");
    const unit = readout.querySelector("button")!;
    expect(unit.getAttribute("aria-pressed")).toBe("false");
    expect(unit.title).toMatch(/note/);
    click(unit);
    expect(onChange).toHaveBeenCalledWith(true);

    const note = render(<GridSection layout={KNOBS} bind={synced(true, onChange)} />).container;
    const noteReadout = knobRoot(note, MixerField.EqHighGain).lastElementChild!;
    expect(noteReadout.textContent).toBe("1/8 ♪");
    click(noteReadout.querySelector("button")!);
    expect(onChange).toHaveBeenLastCalledWith(false);
  });
});

describe("GridSection array-field elements", () => {
  const PART = 1;
  const PARTIAL = 0;
  const LEVELS = PcmsPartialField.TvaEnvLevel;
  /** A fader per PCM Synth TVA envelope level, L1–L3: one per element of one array field. */
  const AMP: SectionLayout<PcmsPartialField> = {
    title: "AMP",
    titleEdge: "top",
    w: 8,
    h: 5,
    fields: enumFields("PcmsPartialField", PcmsPartialField, pcms_partial_spec, { count: pcms_partial_count }),
    items: [0, 1, 2].map((index) => ({ kind: "fader", field: LEVELS, index, x: 1 + 2 * index, y: 0, h: 4, label: `L${index + 1}` })),
  };

  /** Binds each element to a real core, writing it through the core's element setter. */
  function draw(device: WasmDeviceState) {
    const values = [10, 20, 30];
    const bind = (field: PcmsPartialField, index = 0): Binding =>
      specBinding(`level ${index + 1}`, pcms_partial_spec(field), values[index]!, (v) =>
        device.setPcmsPartialParam(PART, PARTIAL, field, index, v),
      );
    const relevance = vi.fn((_field: PcmsPartialField, _index?: number) => Relevance.Active);
    const { container } = render(<GridSection layout={AMP} bind={bind} relevance={relevance} />);
    return { container, relevance };
  }

  it("binds each fader to its element's value", () => {
    const { container, relevance } = draw(new WasmDeviceState(0x10));
    expect(["L1", "L2", "L3"].map((l) => slider(container, l).getAttribute("aria-valuenow"))).toEqual(["10", "20", "30"]);
    expect(relevance.mock.calls).toEqual([[LEVELS, 0], [LEVELS, 1], [LEVELS, 2]]);
    const levels = [...container.querySelectorAll(`[data-field="${LEVELS}"]`)].map((el) => el.getAttribute("data-index"));
    expect(levels).toEqual(["0", "1", "2"]);
  });

  it("moving the L2 fader writes only TVA envelope level L2", () => {
    const device = new WasmDeviceState(0x10);
    const sent = dt1Drainer(device);
    const { container } = draw(device);
    arrowUp(slider(container, "L2"));

    /** What a fresh core sends for element `index` set to `value`. */
    const write = (index: number, value: number) => {
      const fresh = new WasmDeviceState(0x10);
      const drain = dt1Drainer(fresh);
      fresh.setPcmsPartialParam(PART, PARTIAL, LEVELS, index, value);
      return drain();
    };
    const got = sent();
    expect(got).toEqual(write(1, 21));
    expect(got).toHaveLength(1);
    expect(got[0]!.data).toEqual([21]);
    // Not the field's first element, nor its last: the address is L2's own.
    expect(got[0]!.address).not.toEqual(write(0, 21)[0]!.address);
    expect(got[0]!.address).not.toEqual(write(2, 21)[0]!.address);
  });
});

describe("GridSection tempo sync", () => {
  /** One knob and one fader over mixer fields, both bound as tempo-syncable. */
  const SYNCED: SectionLayout<MixerField> = {
    title: "T",
    titleEdge: "top",
    w: 6,
    h: 4,
    fields: mixerFields([MixerScopeKind.Part]),
    items: [
      { kind: "knob", field: MixerField.EqLowGain, x: 1, y: 0 },
      { kind: "fader", field: MixerField.EqMidGain, x: 3, y: 0, h: 4 },
    ],
  };

  it("keeps the free/note toggle inside each Control's footprint, as its readout's unit", () => {
    const onChange = vi.fn();
    const bind = (field: MixerField): Binding => ({
      ...bindMixer(field),
      sync: { labels: ["ms", "♪"], formatNumber: String, noteMode: false, onChange },
    });
    const { container } = render(<GridSection layout={SYNCED} bind={bind} />);
    const knob = wrapper(container, MixerField.EqLowGain);
    const fader = wrapper(container, MixerField.EqMidGain);
    expectFillsFootprint(knob.firstElementChild as HTMLElement, FOOTPRINT.knobSm);
    expectFillsFootprint(fader.firstElementChild as HTMLElement, { w: FOOTPRINT.faderW, h: 4 });
    for (const el of [knob, fader]) {
      const unit = [...el.querySelectorAll("button")].find((b) => b.textContent === "ms");
      act(() => unit?.click());
    }
    expect(onChange.mock.calls).toEqual([[true], [true]]);
  });

  it("draws no toggle for a Control without sync", () => {
    const { container } = render(<GridSection layout={SYNCED} bind={bindMixer} />);
    expect(container.querySelector("button")).toBeNull();
  });
});

describe("GridScreen", () => {
  const frame = { title: "S", titleEdge: "top" as const, w: 2, h: 1 };
  const screen: ScreenLayout = {
    name: "screen",
    w: 4,
    h: 4,
    sections: [
      { id: "lower", section: frame, x: 0, y: 2 },
      { id: "right", section: frame, x: 2, y: 0 },
      { id: "left", section: frame, x: 0, y: 0 },
    ],
    lanes: [],
    silkscreen: [],
  };
  const one = (field: MixerField): SectionLayout<MixerField> => ({
    ...frame,
    fields: mixerFields([MixerScopeKind.Part]),
    items: [{ kind: "switch", field, x: 0, y: 0 }],
  });
  const FIELD = { left: MixerField.EqSwitch, right: MixerField.EqLowGain, lower: MixerField.EqMidQ } as const;

  it("puts Sections in the DOM in screen order, so focus goes Section by Section", () => {
    const { container } = render(
      <GridScreen layout={screen}>
        {(p) => <GridSection layout={one(FIELD[p.id as keyof typeof FIELD])} bind={bindMixer} />}
      </GridScreen>,
    );
    const ids = [...container.querySelectorAll<HTMLElement>("[data-section]")].map((el) => el.dataset.section);
    expect(ids).toEqual(["left", "right", "lower"]);
    expect(fieldOrder(container)).toEqual([FIELD.left, FIELD.right, FIELD.lower].map(String));
    const lower = container.querySelector<HTMLElement>('[data-section="lower"]')!;
    expect([lower.style.left, lower.style.top, lower.style.width, lower.style.height]).toEqual([
      "0px",
      "48px",
      "48px",
      "48px",
    ]);
  });
});

/** The `d` of each arrow path in `root`, in DOM order. */
const arrowPaths = (root: ParentNode): (string | null)[] =>
  [...root.querySelectorAll("path[data-arrow]")].map((p) => p.getAttribute("d"));

describe("GridSection Silkscreen arrows", () => {
  // Two knobs across the top of a side-titled Section; free Cells below them.
  const layout: SectionLayout<MixerField> = {
    title: "S",
    titleEdge: "side",
    w: 4,
    h: 5,
    fields: mixerFields([MixerScopeKind.Part]),
    items: [
      { kind: "knob", field: MixerField.EqLowGain, x: 0, y: 0 },
      { kind: "knob", field: MixerField.EqMidGain, x: 2, y: 0 },
    ],
    silkscreen: [
      { kind: "arrow", from: { field: MixerField.EqLowGain, side: "bottom" }, to: { field: MixerField.EqMidGain, side: "bottom" } },
    ],
  };

  it("routes an arrow between elements from the layout, offset past the title", () => {
    const { container } = render(<GridSection layout={layout} bind={bindMixer} />);
    // Content starts one Cell (24px) right of the side title; routes run on Cell centres.
    expect(arrowPaths(container)).toEqual(["M 36 72 L 36 84 L 84 84 L 84 72"]);
  });

  it("draws a glyph centred in the Cell next to the target", () => {
    const glyphed: SectionLayout<MixerField> = {
      ...layout,
      silkscreen: [{ kind: "arrow", from: { field: MixerField.EqLowGain, side: "bottom" }, to: { field: MixerField.EqMidGain, side: "bottom" }, glyph: "R" }],
    };
    const { container } = render(<GridSection layout={glyphed} bind={bindMixer} />);
    const circle = container.querySelector("circle")!;
    expect([circle.getAttribute("cx"), circle.getAttribute("cy")]).toEqual(["84", "84"]);
    expect(container.querySelector("[data-glyph]")!.textContent).toBe("R");
    // The route stops at the glyph's ring; a lead runs from it into the target.
    expect(arrowPaths(container)).toEqual(["M 36 72 L 36 84 L 77 84", "M 84 77 L 84 72"]);
  });

  it("draws no arrow whose endpoint is in an alternative not drawn", () => {
    const alt: SectionLayout<MixerField> = {
      ...layout,
      items: [
        layout.items[0]!,
        {
          kind: "alternatives",
          group: "g",
          x: 2,
          y: 0,
          w: 2,
          h: 3,
          options: [
            { name: "a", items: [{ kind: "knob", field: MixerField.EqMidGain, x: 2, y: 0 }] },
            { name: "b", items: [{ kind: "knob", field: MixerField.EqMidQ, x: 2, y: 0 }] },
          ],
        },
      ],
    };
    expect(arrowPaths(render(<GridSection layout={alt} bind={bindMixer} />).container)).toHaveLength(1);
    expect(arrowPaths(render(<GridSection layout={alt} bind={bindMixer} alternative={() => "b"} />).container)).toHaveLength(0);
  });
});

describe("GridScreen Silkscreen arrows", () => {
  /** A section outline of `w`×`h` content Cells, title on top. */
  const frame = (w: number, h: number) => ({ title: "S", titleEdge: "top" as const, w, h });
  /** `a` above left, `b` lower right, a Lane between, and arrows `a` → `b`. */
  const screen = (silkscreen: ScreenLayout["silkscreen"]): ScreenLayout => ({
    name: "elbow",
    w: 9,
    h: 8,
    sections: [
      { id: "a", section: frame(4, 3), x: 0, y: 0 },
      { id: "c", section: frame(4, 3), x: 0, y: 4 },
      { id: "b", section: frame(4, 5), x: 5, y: 2 },
    ],
    lanes: [{ x: 4, y: 0, w: 1, h: 8 }],
    silkscreen,
  });
  const draw = (layout: ScreenLayout) => render(<GridScreen layout={layout}>{() => null}</GridScreen>).container;

  it("routes an elbowed arrow between two Sections through a Lane, border to border", () => {
    const container = draw(screen([{ kind: "arrow", from: { section: "a", side: "right" }, to: { section: "b", side: "left" } }]));
    // Lattice (4, 1.5) → (4.5, 1.5) → (4.5, 4.5) → (5, 4.5), each end reaching the
    // border drawn INSET (4px) inside its Section.
    expect(arrowPaths(container)).toEqual(["M 92 36 L 108 36 L 108 108 L 124 108"]);
    const svg = container.querySelector<SVGSVGElement>("svg[data-screen-silkscreen]")!;
    expect([svg.getAttribute("width"), svg.getAttribute("height")]).toEqual(["216", "192"]);
    expect(svg.getAttribute("aria-hidden")).toBe("true");
  });

  it("draws one glyph where arrows into one target meet", () => {
    const container = draw(
      screen([
        { kind: "arrow", from: { section: "a", side: "right" }, to: { section: "b", side: "left" }, glyph: "+" },
        { kind: "arrow", from: { section: "c", side: "right" }, to: { section: "b", side: "left" }, glyph: "+" },
      ]),
    );
    expect(container.querySelectorAll("[data-glyph]")).toHaveLength(1);
    expect(arrowPaths(container)).toEqual([
      "M 92 36 L 108 36 L 108 101",
      "M 92 132 L 108 132 L 108 115",
      "M 115 108 L 124 108",
    ]);
  });

  it("draws a lead into each target, even when their glyph Cells coincide", () => {
    // a → b and b → a both put their glyph in Lane Cell (4, 2); a validator error, but each target still gets its lead.
    const facing: ScreenLayout = {
      ...screen([]),
      sections: [
        { id: "a", section: frame(4, 4), x: 0, y: 0 },
        { id: "b", section: frame(4, 4), x: 5, y: 0 },
      ],
      silkscreen: [
        { kind: "arrow", from: { section: "a", side: "right" }, to: { section: "b", side: "left" }, glyph: "+" },
        { kind: "arrow", from: { section: "b", side: "left" }, to: { section: "a", side: "right" }, glyph: "+" },
      ],
    };
    const leads = arrowPaths(draw(facing)).slice(2);
    expect(leads).toEqual(["M 115 60 L 124 60", "M 101 60 L 92 60"]);
  });

  it("draws nothing for an arrow the Lanes can't carry", () => {
    const container = draw(screen([{ kind: "arrow", from: { section: "a", side: "top" }, to: { section: "b", side: "left" } }]));
    expect(arrowPaths(container)).toEqual([]);
  });
});

describe("specBinding", () => {
  it("carries the 0 dB mark a field's core spec declares, and none where it declares none", () => {
    const bind = (field: MixerField) => specBinding("", mixer_field_spec(field), 0, () => {});
    expect(bind(MixerField.PartLevel).zeroDb).toBe(mixer_field_spec(MixerField.PartLevel).zeroDb);
    expect(bind(MixerField.PartLevel).zeroDb).toBeDefined();
    expect(bind(MixerField.PartChorusSend).zeroDb).toBeUndefined();
  });
});

describe("GridSection picker", () => {
  const fields = enumFields("SndNoteField", SndNoteField, snd_note_spec);
  const layout = (inactive?: "dim" | "hide"): SectionLayout<SndNoteField> => ({
    title: "TONE",
    titleEdge: "top",
    w: 8,
    h: 2,
    fields,
    items: [{ kind: "picker", field: SndNoteField.InstNumber, x: 1, y: 0, label: "Inst", inactive }],
  });
  const draw = (value: number, onChange = vi.fn(), relevance = Relevance.Active, inactive?: "dim" | "hide") => {
    const bind = (field: SndNoteField) => specBinding("Inst Number", snd_note_spec(field), value, onChange);
    const { container } = render(<GridSection layout={layout(inactive)} bind={bind} relevance={() => relevance} />);
    return container.querySelector<HTMLElement>(`[data-field="${SndNoteField.InstNumber}"]`)!;
  };
  /** The names the open list shows. */
  const listed = () => [...document.querySelectorAll('[role="option"]')].map((o) => o.textContent ?? "");

  it("fills 6×1 and shows the bound field's option name, from core", () => {
    const el = draw(1);
    expect([el.style.left, el.style.top]).toEqual(["24px", "24px"]);
    expectFillsFootprint(el.firstElementChild as HTMLElement, FOOTPRINT.picker);
    expect(el.querySelector("button")!.textContent).toBe(snd_note_spec(SndNoteField.InstNumber).formatValue(1));
  });

  it("filters the list by typed text and writes the chosen option", () => {
    const onChange = vi.fn();
    const el = draw(0, onChange);
    click(el.querySelector("button")!);
    // Every Inst core names, OFF first.
    expect(listed()[0]).toBe("OFF");
    expect(listed()).toContain("Studio Kick");

    typeInto(document.querySelector("input")!, "telephone");
    expect(listed()).toEqual(["Telephone 1", "Telephone 2", "Telephone 3"]);
    click([...document.querySelectorAll<HTMLElement>('[role="option"]')].find((o) => o.textContent === "Telephone 3")!);
    expect(onChange).toHaveBeenCalledWith(294);
  });

  it.each(["dim", "hide"] as const)("goes inert in place when irrelevant (%s)", (inactive) => {
    const el = draw(1, vi.fn(), Relevance.Irrelevant, inactive);
    expect(el.dataset.inactive).toBe(inactive);
    expect(el.hasAttribute("inert")).toBe(true);
    expect([el.style.left, el.style.top]).toEqual(["24px", "24px"]);
  });
});
