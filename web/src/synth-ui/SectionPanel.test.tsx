import { afterEach, describe, expect, it } from "vitest";
import { ParameterFamily } from "../../pkg/integral_wasm.js";
import { SectionPanel } from "./SectionPanel";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

describe("SectionPanel", () => {
  it("colours its top border and header underline in its family's colour", () => {
    const { container } = render(
      <SectionPanel label="AMP" accentColor="red" family={ParameterFamily.Amp}>
        body
      </SectionPanel>,
    );
    const panel = container.firstElementChild as HTMLElement;
    expect(panel.style.borderTopColor).toBe("var(--family-amp)");
    expect(
      (panel.firstElementChild as HTMLElement).style.borderBottomColor,
    ).toBe("var(--family-amp)");
  });
});
