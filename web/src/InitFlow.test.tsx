import { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ToneType } from "../pkg/integral_wasm.js";
import { deviceSpec } from "./deviceSpec";
import { InitGuard, InitProvider, useInit } from "./InitFlow";
import type { InitScope } from "./init";
import { LINK_LOST_ERROR, type IntegraService } from "./integra";
import { button, cleanup, click, render } from "./test/dom";
import { fakeInitService } from "./test/initService";

afterEach(cleanup);

/** An "Init…" item for `scope`, and what Part 3's tone editor would key on. */
function Item({ scope }: { scope: InitScope }) {
  const init = useInit();
  return (
    <>
      <button onClick={() => init.request(scope)}>Init…</button>
      <span data-testid="rereads">{init.rereads(2)}</span>
    </>
  );
}

function setup(scope: InitScope) {
  const fake = fakeInitService();
  const reload = vi.fn();
  const tree = (service: IntegraService | null) => (
    <InitProvider service={service} mixer={{ reload }}>
      <Item scope={scope} />
      <InitGuard area={{ kind: "studio-set" }}>
        <div data-testid="studioSet" />
      </InitGuard>
      <InitGuard area={{ kind: "part", part: 2 }}>
        <div data-testid="part3" />
      </InitGuard>
      <InitGuard area={{ kind: "tone", part: 2 }}>
        <div data-testid="tone3" />
      </InitGuard>
      <InitGuard area={{ kind: "part", part: 5 }}>
        <div data-testid="part6" />
      </InitGuard>
    </InitProvider>
  );
  const { container, rerender } = render(tree(fake.service));
  const guard = (id: string) => container.querySelector(`[data-testid="${id}"]`)!.parentElement!;
  const rereads = () => container.querySelector('[data-testid="rereads"]')!.textContent;
  /** The link drops: App renders the provider again without a service. */
  const disconnect = () => rerender(tree(null));
  return { ...fake, reload, container, guard, rereads, disconnect };
}

describe("InitFlow", () => {
  it("confirms first, naming what resets and that User slots stay", () => {
    const { container, initialize } = setup({ kind: "part", part: 2 });
    click(button(container, "Init…"));
    const dialog = container.querySelector('[role="dialog"]')!;
    expect(dialog.getAttribute("aria-label")).toBe("Init Part 3");
    expect(dialog.textContent).toContain("This resets Part 3's settings.");
    expect(dialog.textContent).toContain("stored User slots aren't touched");
    expect(initialize).not.toHaveBeenCalled();
  });

  it("names a drum key by its note", () => {
    const { container } = setup({ kind: "partial", part: 2, toneType: ToneType.PcmDrum, partial: 36 });
    click(button(container, "Init…"));
    expect(container.textContent).toContain("This resets key C2 (36) of Part 3's kit.");
  });

  it("cancel sends nothing", () => {
    const { container, initialize } = setup({ kind: "part", part: 2 });
    click(button(container, "Init…"));
    click(button(container, "Cancel"));
    expect(container.querySelector('[role="dialog"]')).toBeNull();
    expect(initialize).not.toHaveBeenCalled();
  });

  it("keeps the area it resets busy and inert until \"finished\"", async () => {
    const { container, initialize, guard, finish } = setup({ kind: "part", part: 2 });
    click(button(container, "Init…"));
    click(button(container, "Init"));
    expect(initialize).toHaveBeenCalledWith({ kind: "part", part: 2 });
    expect(container.querySelector('[role="dialog"]')).toBeNull();
    expect(guard("part3").hasAttribute("inert")).toBe(true);
    expect(guard("part3").getAttribute("aria-busy")).toBe("true");
    // The Part holds its tone.
    expect(guard("tone3").hasAttribute("inert")).toBe(true);
    // Another Part, and the rest of the Studio Set, stay live.
    expect(guard("part6").hasAttribute("inert")).toBe(false);
    expect(guard("studioSet").hasAttribute("inert")).toBe(false);
    await finish();
    expect(guard("part3").hasAttribute("inert")).toBe(false);
    expect(guard("part3").getAttribute("aria-busy")).toBe("false");
  });

  it("re-reads a Part's whole block and remounts its tone editor after a Part Init", async () => {
    const { container, reReadPart, reReadPartBlock, reload, rereads, finish } = setup({ kind: "part", part: 2 });
    click(button(container, "Init…"));
    click(button(container, "Init"));
    expect(reReadPartBlock).not.toHaveBeenCalled();
    expect(rereads()).toBe("0");
    await finish();
    // The dump and the settings past it: the Part settings drawer shows them.
    expect(reReadPartBlock).toHaveBeenCalledWith(2);
    expect(reReadPart).not.toHaveBeenCalled();
    expect(reload).not.toHaveBeenCalled();
    expect(rereads()).toBe("1");
  });

  it("re-reads the whole mixer after a Studio Set Init", async () => {
    const { container, reload, rereads, guard, finish } = setup({ kind: "studio-set" });
    click(button(container, "Init…"));
    click(button(container, "Init"));
    // A Studio Set holds every Part.
    expect(guard("part6").hasAttribute("inert")).toBe(true);
    await finish();
    expect(reload).toHaveBeenCalledOnce();
    expect(rereads()).toBe("1");
  });

  it("re-reads every Part after a Sound Ctrl Init, and leaves the tone editor", async () => {
    const { container, reReadPart, rereads, finish } = setup({ kind: "sound-ctrl" });
    click(button(container, "Init…"));
    click(button(container, "Init"));
    await finish();
    expect(reReadPart).toHaveBeenCalledTimes(deviceSpec().part_count);
    expect(rereads()).toBe("0");
  });

  it("only remounts the tone editor after a Partial Init", async () => {
    const { container, reReadPart, rereads, finish } = setup({
      kind: "partial",
      part: 2,
      toneType: ToneType.SnSynth,
      partial: 1,
    });
    click(button(container, "Init…"));
    expect(container.textContent).toContain("This resets Partial 2 of Part 3's tone.");
    click(button(container, "Init"));
    await finish();
    expect(reReadPart).not.toHaveBeenCalled();
    expect(rereads()).toBe("1");
  });

  it("a tone's Init leaves its Part's other settings live", () => {
    const { container, guard } = setup({ kind: "tone", part: 2 });
    click(button(container, "Init…"));
    click(button(container, "Init"));
    expect(guard("tone3").hasAttribute("inert")).toBe(true);
    expect(guard("part3").hasAttribute("inert")).toBe(false);
  });

  it("a timeout ends the busy state, says so, and reads the area back", async () => {
    const { container, reReadPartBlock, guard, rereads, finish } = setup({ kind: "part", part: 2 });
    click(button(container, "Init…"));
    click(button(container, "Init"));
    await finish("the device didn't confirm the Init");
    expect(guard("part3").hasAttribute("inert")).toBe(false);
    const alert = container.querySelector('[role="alert"]')!;
    expect(alert.textContent).toBe(
      "Init Part 3 failed: the device didn't confirm the Init. It is read again from the device.",
    );
    // The device may have reset it all the same.
    expect(reReadPartBlock).toHaveBeenCalledWith(2);
    expect(rereads()).toBe("1");
    click(button(container, "Close"));
    expect(container.querySelector('[role="alertdialog"]')).toBeNull();
  });

  it("a lost link ends the busy state and says so once the shell is gone", async () => {
    const { container, reReadPart, reload, guard, finish, disconnect } = setup({ kind: "studio-set" });
    click(button(container, "Init…"));
    click(button(container, "Init"));
    expect(guard("part3").hasAttribute("inert")).toBe(true);
    // The service's teardown ends the run with the link lost.
    disconnect();
    await finish(LINK_LOST_ERROR);
    expect(guard("part3").hasAttribute("inert")).toBe(false);
    expect(container.querySelector('[role="alert"]')!.textContent).toBe(
      `Init Studio Set failed: ${LINK_LOST_ERROR}. What it resets is unknown until it is read again.`,
    );
    expect(reReadPart).not.toHaveBeenCalled();
    expect(reload).not.toHaveBeenCalled();
  });

  it("a service that refuses to start says why", async () => {
    const fake = fakeInitService();
    fake.initialize.mockRejectedValueOnce(new Error("A save is still in progress"));
    const { container } = render(
      <InitProvider service={fake.service} mixer={{ reload: () => {} }}>
        <Item scope={{ kind: "studio-set" }} />
      </InitProvider>,
    );
    click(button(container, "Init…"));
    await act(async () => click(button(container, "Init")));
    expect(container.querySelector('[role="alert"]')!.textContent).toBe(
      "Init Studio Set failed: A save is still in progress. Nothing was reset.",
    );
  });

  it("a request outside a provider throws", () => {
    let caught: unknown;
    function Bare() {
      const init = useInit();
      return (
        <button
          onClick={() => {
            try {
              init.request({ kind: "studio-set" });
            } catch (e) {
              caught = e;
            }
          }}
        >
          Init…
        </button>
      );
    }
    const { container } = render(<Bare />);
    click(button(container, "Init…"));
    expect(caught).toBeInstanceOf(Error);
  });
});
