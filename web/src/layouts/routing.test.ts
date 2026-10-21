// Arrow routing (#355): routes are computed from the layout, deterministic,
// on the half-Cell lattice, through Lanes (between Sections) or free Cells
// (inside one).
import { describe, expect, it } from "vitest";
import { MixerField, MixerScopeKind } from "../../pkg/integral_wasm.js";
import { mixerFields } from "./fields";
import { routeScreenArrow, routeSectionArrow, route, sidePoint, type Anchor } from "./routing";
import type { CellRect, ScreenLayout, SectionLayout, Side } from "./types";

/** A section outline of `w`×`h` content Cells, title on top. */
const frame = (w: number, h: number) => ({ title: "S", titleEdge: "top" as const, w, h });

describe("sidePoint", () => {
  const rect: CellRect = { x: 2, y: 1, w: 3, h: 4 };
  it.each<[Side, [number, number], [number, number]]>([
    // The middle Cell of an odd side; the upper/left middle of an even one.
    ["top", [3.5, 1], [3, 0]],
    ["right", [5, 2.5], [5, 2]],
    ["bottom", [3.5, 5], [3, 5]],
    ["left", [2, 2.5], [1, 2]],
  ])("leaves the %s side at a Cell centre, into the Cell outside it", (side, point, cell) => {
    expect(sidePoint({ rect, side })).toEqual({ point, cell });
  });
});

/** Sections `a` and `b` side by side, with Lanes above, between and below them. */
const RING: ScreenLayout = {
  name: "ring",
  w: 9,
  h: 6,
  sections: [
    { id: "a", section: frame(4, 3), x: 0, y: 1 },
    { id: "b", section: frame(4, 3), x: 5, y: 1 },
  ],
  lanes: [
    { x: 0, y: 0, w: 9, h: 1 },
    { x: 4, y: 1, w: 1, h: 4 },
    { x: 0, y: 5, w: 9, h: 1 },
  ],
  silkscreen: [],
};

/** The route of an arrow from `a`'s `from` side to `b`'s `to` side on `screen`. */
const across = (from: Side, to: Side, screen: ScreenLayout = RING) =>
  routeScreenArrow(screen, { kind: "arrow", from: { section: "a", side: from }, to: { section: "b", side: to } });

describe("routeScreenArrow", () => {
  it("runs straight across a 1-Cell Lane between facing sides", () => {
    expect(across("right", "left")).toEqual({
      points: [[4, 2.5], [5, 2.5]],
      glyphAt: [4.5, 2.5],
    });
  });

  it("elbows through a Lane between Sections at different heights", () => {
    const screen: ScreenLayout = {
      name: "elbow",
      w: 9,
      h: 8,
      sections: [
        { id: "a", section: frame(4, 3), x: 0, y: 0 },
        { id: "b", section: frame(4, 5), x: 5, y: 2 },
      ],
      lanes: [{ x: 4, y: 0, w: 1, h: 8 }],
      silkscreen: [],
    };
    expect(across("right", "left", screen)).toEqual({
      points: [[4, 1.5], [4.5, 1.5], [4.5, 4.5], [5, 4.5]],
      glyphAt: [4.5, 4.5],
    });
  });

  it("leaves and enters by the top", () => {
    expect(across("top", "top")?.points).toEqual([[1.5, 1], [1.5, 0.5], [6.5, 0.5], [6.5, 1]]);
  });

  it("leaves and enters by the bottom", () => {
    expect(across("bottom", "bottom")?.points).toEqual([[1.5, 5], [1.5, 5.5], [6.5, 5.5], [6.5, 5]]);
  });

  it("turns from one side to another with the fewest bends", () => {
    expect(across("top", "left")?.points).toEqual([[1.5, 1], [1.5, 0.5], [4.5, 0.5], [4.5, 2.5], [5, 2.5]]);
    expect(across("bottom", "left")?.points).toEqual([[1.5, 5], [1.5, 5.5], [4.5, 5.5], [4.5, 2.5], [5, 2.5]]);
  });

  it("climbs the Lane between to reach a top side", () => {
    expect(across("right", "top")?.points).toEqual([[4, 2.5], [4.5, 2.5], [4.5, 0.5], [6.5, 0.5], [6.5, 1]]);
  });

  it("is deterministic", () => {
    expect(across("top", "bottom")).toEqual(across("top", "bottom"));
  });

  it("finds no route out of a side with no Lane next to it", () => {
    expect(across("left", "left")).toBeNull();
  });

  it("finds no route between Lanes that don't touch", () => {
    const cut: ScreenLayout = { ...RING, lanes: [RING.lanes[0]!, RING.lanes[2]!] };
    expect(across("top", "bottom", cut)).toBeNull();
  });

  it("finds no route for an unknown Section", () => {
    expect(routeScreenArrow(RING, { kind: "arrow", from: { section: "a", side: "top" }, to: { section: "z", side: "top" } })).toBeNull();
  });
});

describe("route", () => {
  const cells = new Set(["1,0", "2,0", "3,0"]);
  const free = (x: number, y: number) => cells.has(`${x},${y}`);
  const from: Anchor = { rect: { x: 0, y: 0, w: 1, h: 1 }, side: "right" };

  it("runs through free Cells only", () => {
    const to: Anchor = { rect: { x: 4, y: 0, w: 1, h: 1 }, side: "left" };
    expect(route(free, from, to)?.points).toEqual([[1, 0.5], [4, 0.5]]);
    expect(route(() => false, from, to)).toBeNull();
  });

  it("takes the route with the fewest bends among equally short ones", () => {
    // A staircase (four bends) and an L (two), both six Cells long.
    const open = new Set(["1,0", "2,0", "2,1", "3,1", "3,2", "4,2"]);
    const to: Anchor = { rect: { x: 5, y: 2, w: 1, h: 1 }, side: "left" };
    const stairs = route((x, y) => open.has(`${x},${y}`), from, to);
    expect(stairs?.points).toHaveLength(6);
    for (const c of ["1,1", "1,2", "2,2"]) open.add(c);
    expect(route((x, y) => open.has(`${x},${y}`), from, to)?.points).toHaveLength(4);
  });

  it("turns into a target whose side faces across the run", () => {
    const below: Anchor = { rect: { x: 2, y: 1, w: 1, h: 1 }, side: "top" };
    expect(route(free, from, below)?.points).toEqual([[1, 0.5], [2.5, 0.5], [2.5, 1]]);
  });
});

describe("routeSectionArrow", () => {
  // Two knobs across the top; free Cells below them.
  const layout: SectionLayout<MixerField> = {
    ...frame(4, 5),
    fields: mixerFields([MixerScopeKind.Part]),
    items: [
      { kind: "knob", field: MixerField.EqLowGain, x: 0, y: 0 },
      { kind: "knob", field: MixerField.EqMidGain, x: 2, y: 0 },
    ],
  };

  it("routes between elements through the Section's free Cells", () => {
    const r = routeSectionArrow(layout, {
      kind: "arrow",
      from: { field: MixerField.EqLowGain, side: "bottom" },
      to: { field: MixerField.EqMidGain, side: "bottom" },
    });
    expect(r?.points).toEqual([[0.5, 3], [0.5, 3.5], [2.5, 3.5], [2.5, 3]]);
  });

  it("finds no route out of a side another element covers", () => {
    const blocked = routeSectionArrow(layout, {
      kind: "arrow",
      from: { field: MixerField.EqLowGain, side: "right" },
      to: { field: MixerField.EqMidGain, side: "bottom" },
    });
    expect(blocked).toBeNull();
  });

  it("finds no route out of the Section", () => {
    const out = routeSectionArrow(layout, {
      kind: "arrow",
      from: { field: MixerField.EqLowGain, side: "top" },
      to: { field: MixerField.EqMidGain, side: "bottom" },
    });
    expect(out).toBeNull();
  });

  it("draws nothing for an endpoint not among the elements drawn", () => {
    const r = routeSectionArrow(
      layout,
      { kind: "arrow", from: { field: MixerField.EqLowGain, side: "bottom" }, to: { field: MixerField.EqMidGain, side: "bottom" } },
      [],
    );
    expect(r).toBeNull();
  });
});
