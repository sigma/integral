import { afterEach, describe, expect, it } from "vitest";
import { DisplayKind, MixerField, MixerScopeKind } from "../../pkg/integral_wasm.js";
import { paramFormatter, paramMeta } from "../paramDefaults";
import { FOOTPRINT } from "../synth-ui/grid";
import { cleanup, render } from "../test/dom";
import { expectFillsFootprint } from "../test/grid";
import { mixerFields } from "./fields";
import { GridScreen } from "./GridScreen";
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
  // Every px below is what the pilot renderer (prototype/grid-pilot) drew.
  const draw = () => render(<GridSection layout={EQ_LAYOUT} bind={bindMixer} />).container;

  it("at the pilot's size, title row included", () => {
    const root = draw().firstElementChild as HTMLElement;
    expect(root.style.width).toBe("96px");
    expect(root.style.height).toBe("408px");
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

  it.each([
    [MixerField.EqSwitch, 24, 24],
    [MixerField.EqHighFreq, 0, 48],
    [MixerField.EqHighGain, 48, 96],
    [MixerField.EqMidFreq, 0, 144],
    [MixerField.EqMidGain, 48, 192],
    [MixerField.EqMidQ, 0, 240],
    [MixerField.EqLowGain, 48, 288],
    [MixerField.EqLowFreq, 0, 336],
  ])("places field %i at (%ipx, %ipx)", (field, left, top) => {
    const el = wrapper(draw(), field);
    expect([el.style.left, el.style.top]).toEqual([`${left}px`, `${top}px`]);
  });

  it("draws each Control at its footprint, unscaled", () => {
    const container = draw();
    expectFillsFootprint(wrapper(container, MixerField.EqSwitch).firstElementChild as HTMLElement, FOOTPRINT.switch);
    expectFillsFootprint(wrapper(container, MixerField.EqMidQ).firstElementChild as HTMLElement, FOOTPRINT.knobSm);
  });

  it("names knobs as the layout says, and hides the switch's name", () => {
    const container = draw();
    expect(wrapper(container, MixerField.EqMidQ).querySelector("span")!.textContent).toBe("Q");
    // Core names it "EQ Switch"; only its state shows.
    expect(wrapper(container, MixerField.EqSwitch).textContent).not.toContain("EQ");
  });

  it("draws the zig-zag rules and band names, clipped to the border", () => {
    const svg = draw().querySelector("svg")!;
    expect(svg.style.clipPath).toBe("inset(5px round 6px)");
    expect([...svg.querySelectorAll("polyline")].map((p) => p.getAttribute("points"))).toEqual([
      "0,132 24,132 72,180 96,180",
      "0,324 24,324 72,276 96,276",
    ]);
    expect([...svg.querySelectorAll("text")].map((t) => [t.textContent, t.getAttribute("x"), t.getAttribute("y")])).toEqual([
      ["HI", "72", "84"],
      ["MID", "24", "228"],
      ["LO", "72", "372"],
    ]);
  });
});

/** A Section of three switches listed out of reading order, plus a group and a Display. */
const SCRAMBLED: SectionLayout<MixerField> = {
  title: "Part {part}",
  titleEdge: "side",
  w: 4,
  h: 4,
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

  it("dims or hides an inactive Control in its Cells, inert", () => {
    const { container } = render(
      <GridSection layout={SCRAMBLED} bind={bindMixer} isActive={(f) => f !== MixerField.EqMidQ && f !== MixerField.EqSwitch} />,
    );
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
