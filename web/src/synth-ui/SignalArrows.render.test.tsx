import { afterEach, describe, expect, it } from "vitest";
import { SignalArrows } from "./SignalArrows";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

describe("SignalArrows", () => {
  it("draws on first mount, measuring its own parent", () => {
    // Regression: it used to read a parent ref, which isn't attached yet when
    // its own layout effect runs, so a fixed edge list never drew.
    const { container } = render(
      <div style={{ position: "relative" }}>
        <div data-node="a" />
        <div data-node="b" />
        <SignalArrows edges={[{ from: "a", to: "b" }]} />
      </div>,
    );
    expect(container.querySelectorAll("path[marker-end]")).toHaveLength(1);
  });
});
