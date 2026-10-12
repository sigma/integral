import { afterEach, describe, expect, it } from "vitest";
import { ADSREnvelope } from "./ADSREnvelope";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

const stage = (label: string, value: number) => ({ label, value, min: 0, max: 127, defaultValue: 64, onChange: () => {} });

describe("ADSREnvelope", () => {
  it("draws an A/R envelope: rise, hold, fall, with two faders", () => {
    const { container } = render(<ADSREnvelope attack={stage("Atk", 0)} release={stage("Rel", 127)} />);
    expect([...container.querySelectorAll('[role="slider"]')].map((s) => s.getAttribute("aria-label"))).toEqual(["Atk", "Rel"]);
    // Instant attack, hold to 0.25, full-length release.
    expect(container.querySelector("path")!.getAttribute("d")).toBe("M 0 1 L 0 0 L 0.25 0 L 0.6 1");
  });
});
