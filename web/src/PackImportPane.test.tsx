import { useState } from "react";
import { afterAll, afterEach, describe, expect, it } from "vitest";
import { Pack, planImport } from "../pkg/integral_wasm.js";
import { libraryType } from "./librarian";
import { PackImportPane } from "./PackImportPane";
import { button, cleanup, click, render } from "./test/dom";
import { toggled, type ImportEntry, type UseLibraryImportResult } from "./useLibraryImport";

afterEach(cleanup);

/** A Universal Identity Request: a SysEx dump with nothing to import. */
const IDENTITY_REQUEST = new Uint8Array([0xf0, 0x7e, 0x7f, 0x06, 0x01, 0xf7]);
const pack = Pack.read(IDENTITY_REQUEST);
const plan = planImport(pack, [], undefined);
afterAll(() => {
  plan.free();
  pack.free();
});

/** The Conversion report of the XV fixture's Patch, as the core words it. */
const REPORT = [
  "Partial 3: MFX bypass dropped",
  "Partial 4: MFX bypass dropped",
  "Chorus CHORUS, level 127 lives in the Studio Set",
  "Reverb SRV HALL, level 127 lives in the Studio Set",
];

/** Settings a clean conversion leaves to the Studio Set. */
const NOT_APPLIED = ["Tempo 90 lives in the Studio Set"];

/** A converted lossy tone, a clean one with Studio Set items, then a clean
 * one: all go to User slots. */
function entries(): ImportEntry[] {
  const type = libraryType("pcm-synth");
  const entry = (index: number, name: string, report: string[], lossy: number, source: string | null = null): ImportEntry => ({
    index,
    info: { kind: type.kind, index, name, toneRefs: [] },
    type,
    origin: `#${index + 1}`,
    placement: { user: index },
    report,
    lossy,
    // Only the XV fixture's Patch has a Chorus/Reverb to apply.
    effects: report === REPORT,
    source,
  });
  return [entry(0, "United !", REPORT, 2), entry(1, "Tempo", NOT_APPLIED, 0, "JV-1080"), entry(2, "Clean", [], 0)];
}

function noop(): void {}

/** The pane on an opened file, showing reports as the hook does. With
 * `audition`, only the lossy tone, loaded into Temporary. */
function Opened({
  warnings,
  audition = false,
  start = noop,
}: {
  warnings: string[];
  audition?: boolean;
  start?: UseLibraryImportResult["start"];
}) {
  const [reportShown, setReportShown] = useState<ReadonlySet<number>>(new Set());
  const imp: UseLibraryImportResult = {
    file: { name: "dump.syx", label: pack.label },
    opening: false,
    openError: null,
    entries: audition
      ? entries()
          .slice(0, 1)
          .map((e) => ({ ...e, placement: "temporary" as const }))
      : entries(),
    warnings,
    unplaced: 0,
    overwrites: [],
    temporary: audition,
    focused: null,
    focus: noop,
    reportShown,
    toggleReport: (entry) => setReportShown((shown) => toggled(shown, entry)),
    open: noop,
    close: noop,
    assign: noop,
    setIncluded: noop,
    run: null,
    busy: false,
    locked: false,
    start,
    cancel: noop,
  };
  const type = libraryType("pcm-synth");
  return (
    <PackImportPane
      libraryImport={imp}
      slots={[]}
      slotsType={type}
      part={0}
      onFocus={noop}
      busy={false}
    />
  );
}

function reportItems(container: HTMLElement): string[] {
  return [...container.querySelectorAll("ul")]
    .filter((ul) => ul.className.includes("conversion"))
    .flatMap((ul) => [...ul.querySelectorAll("li")].map((li) => li.textContent ?? ""));
}

describe("PackImportPane", () => {
  it("names a SysEx dump as such", () => {
    const { container } = render(<Opened warnings={[]} />);
    expect(container.textContent).toContain("dump.syx · SysEx dump");
  });

  it("marks only the lossy entry, and expands it to its items", () => {
    const { container } = render(<Opened warnings={[]} />);
    const badges = [...container.querySelectorAll("button")].filter((b) =>
      b.textContent?.startsWith("lossy"),
    );
    expect(badges.map((b) => b.textContent)).toEqual(["lossy · 2"]);
    expect(reportItems(container)).toEqual([]);

    const badge = button(container, "lossy · 2");
    click(badge);
    expect(badge.getAttribute("aria-expanded")).toBe("true");
    expect(reportItems(container)).toEqual(REPORT);

    click(badge);
    expect(reportItems(container)).toEqual([]);
  });

  it("lists the items of a clean entry without a lossy badge", () => {
    const { container } = render(<Opened warnings={[]} />);
    click(button(container, "notes · 1"));
    expect(reportItems(container)).toEqual(NOT_APPLIED);
  });

  it("names the model a Foreign patch was made on", () => {
    const { container } = render(<Opened warnings={[]} />);
    const sources = [...container.querySelectorAll("span")].filter((s) => s.className.includes("source"));
    expect(sources.map((s) => s.textContent)).toEqual(["JV-1080"]);
  });

  it("keeps lossy entries included", () => {
    const { container } = render(<Opened warnings={[]} />);
    const boxes = [...container.querySelectorAll<HTMLInputElement>("input[type=checkbox]")];
    expect(boxes.map((b) => b.checked)).toEqual([true, true, true]);
    expect(button(container, "Import 3…").disabled).toBe(false);
  });

  it("lists file-level items with the warnings", () => {
    const fileReport = plan.fileReport();
    expect(fileReport).toEqual(["1 other message skipped"]);
    expect(plan.reports()).toEqual([]);

    const { container } = render(<Opened warnings={fileReport} />);
    const warnings = [...container.querySelectorAll("li")].map((li) => li.textContent);
    expect(warnings).toContain("1 other message skipped");
  });

  it("applies a Foreign patch's Chorus/Reverb on audition only when ticked", () => {
    const starts: boolean[] = [];
    const start = (_undo: boolean, applyEffects: boolean): void => {
      starts.push(applyEffects);
    };
    expect(
      render(<Opened warnings={[]} />).container.textContent,
    ).not.toContain("Apply its Chorus/Reverb");
    cleanup();

    const { container } = render(<Opened warnings={[]} audition start={start} />);
    const box = [...container.querySelectorAll<HTMLInputElement>("input[type=checkbox]")].find((b) =>
      b.parentElement?.textContent?.includes("Apply its Chorus/Reverb to the Temporary Studio Set"),
    );
    if (!box) throw new Error("no Chorus/Reverb checkbox");
    expect(box.checked).toBe(false);
    const load = button(container, "Load into Part 1");
    click(load);
    click(box);
    click(load);
    expect(starts).toEqual([false, true]);
  });
});
