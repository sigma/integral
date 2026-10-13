import { afterEach, describe, expect, it } from "vitest";
import { ParameterFamily } from "../../pkg/integral_wasm.js";
import { SectionPanel } from "./SectionPanel";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

/** The panel's top border and header underline colours. */
function accents(container: HTMLElement): string[] {
  const panel = container.firstElementChild as HTMLElement;
  return [
    panel.style.borderTopColor,
    (panel.firstElementChild as HTMLElement).style.borderBottomColor,
  ];
}

describe("SectionPanel", () => {
  it("colours its top border and header underline in its family's colour", () => {
    const { container } = render(
      <SectionPanel label="AMP" family={ParameterFamily.Amp}>
        body
      </SectionPanel>,
    );
    expect(accents(container)).toEqual([
      "var(--family-amp)",
      "var(--family-amp)",
    ]);
  });

  it("draws a named non-family section colour", () => {
    const { container } = render(
      <SectionPanel label="DRUM KEY" sectionColour="drum">
        body
      </SectionPanel>,
    );
    const [top, underline] = accents(container);
    expect(top).not.toBe("");
    expect(top).not.toMatch(/family/);
    expect(underline).toBe(top);
  });

  it("takes its colour only from a family or a named section colour", () => {
    // Type-level checks: tsc fails if any line stops being an error.
    // prettier-ignore
    {
      // @ts-expect-error -- a section has no colour-literal prop
      void (<SectionPanel label="AMP" family={ParameterFamily.Amp} accentColor="#fff">x</SectionPanel>);
      // @ts-expect-error -- a section colour is a name, not a literal
      void (<SectionPanel label="AMP" sectionColour="#fff">x</SectionPanel>);
      // @ts-expect-error -- a section needs a family or a section colour
      void (<SectionPanel label="AMP">x</SectionPanel>);
      // @ts-expect-error -- not both
      void (<SectionPanel label="AMP" family={ParameterFamily.Amp} sectionColour="drum">x</SectionPanel>);
    }
  });
});
