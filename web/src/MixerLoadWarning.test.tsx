import { afterEach, describe, expect, it } from "vitest";
import { MixerLoadWarning } from "./MixerLoadWarning";
import { cleanup, click, render } from "./test/dom";

afterEach(cleanup);

describe("MixerLoadWarning", () => {
  it("says how many blocks failed and retries on click", () => {
    let retried = 0;
    const el = render(
      <MixerLoadWarning failed={["Chorus", "Part 4"]} onRetry={() => retried++} />,
    ).container;
    expect(el.textContent).toContain("2 blocks failed to load");
    expect(el.querySelector("[title]")?.getAttribute("title")).toBe(
      "Chorus, Part 4",
    );
    click(el.querySelector("button")!);
    expect(retried).toBe(1);
  });

  it("uses the singular for one block", () => {
    const el = render(<MixerLoadWarning failed={["Reverb"]} onRetry={() => {}} />)
      .container;
    expect(el.textContent).toContain("1 block failed to load");
  });
});
