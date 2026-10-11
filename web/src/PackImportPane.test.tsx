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
  "MFX type 6 dropped, MFX set to THRU",
  "Chorus CHORUS, level 127 not applied; set it in the Studio Set",
  "Reverb SRV HALL, level 127 not applied; set it in the Studio Set",
];

/** A converted lossy tone, then a clean one: both go to User slots. */
function entries(): ImportEntry[] {
  const type = libraryType("pcm-synth");
  const entry = (index: number, name: string, report: string[]): ImportEntry => ({
    index,
    info: { kind: type.kind, index, name, toneRefs: [] },
    type,
    origin: `#${index + 1}`,
    placement: { user: index },
    report,
  });
  return [entry(0, "United !", REPORT), entry(1, "Clean", [])];
}

function noop(): void {}

/** The pane on an opened file, showing reports as the hook does. */
function Opened({ warnings }: { warnings: string[] }) {
  const [reportShown, setReportShown] = useState<ReadonlySet<number>>(new Set());
  const imp: UseLibraryImportResult = {
    file: { name: "dump.syx", label: pack.label },
    opening: false,
    openError: null,
    entries: entries(),
    warnings,
    unplaced: 0,
    overwrites: [],
    temporary: false,
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
    start: noop,
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
    expect(badges.map((b) => b.textContent)).toEqual(["lossy · 3"]);
    expect(reportItems(container)).toEqual([]);

    const badge = button(container, "lossy · 3");
    click(badge);
    expect(badge.getAttribute("aria-expanded")).toBe("true");
    expect(reportItems(container)).toEqual(REPORT);

    click(badge);
    expect(reportItems(container)).toEqual([]);
  });

  it("keeps lossy entries included", () => {
    const { container } = render(<Opened warnings={[]} />);
    const boxes = [...container.querySelectorAll<HTMLInputElement>("input[type=checkbox]")];
    expect(boxes.map((b) => b.checked)).toEqual([true, true]);
    expect(button(container, "Import 2…").disabled).toBe(false);
  });

  it("lists file-level items with the warnings", () => {
    const fileReport = plan.fileReport();
    expect(fileReport).toEqual(["1 other message skipped"]);
    expect(plan.reports()).toEqual([]);

    const { container } = render(<Opened warnings={fileReport} />);
    const warnings = [...container.querySelectorAll("li")].map((li) => li.textContent);
    expect(warnings).toContain("1 other message skipped");
  });
});
