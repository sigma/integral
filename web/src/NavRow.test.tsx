import { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { NavRow, browserPages } from "./NavRow";
import type { PageTab } from "./pluginPages";
import type { IntegraService } from "./integra";
import type { ToneCatalog } from "./toneCatalog";
import type { UseExpansionsResult } from "./useExpansions";
import type { UseMixerResult } from "./useMixer";
import { defaultMixerState, type MixerState } from "./types";
import { button, cleanup, click, render } from "./test/dom";
import { deviceSpec } from "./deviceSpec";
import { InitProvider } from "./InitFlow";

afterEach(cleanup);

const expansions = {
  unloadedExpansionOf: () => null,
} as unknown as UseExpansionsResult;

/** The row over a mixer in `state`, on `tab`, with the browser's pages. */
function row({
  state = { ...defaultMixerState(), loading: false },
  tab = "mixer",
  sequencer = true,
  locked = false,
  service = {} as IntegraService,
  settingsOpen = false,
}: {
  state?: MixerState;
  tab?: PageTab;
  sequencer?: boolean;
  locked?: boolean;
  service?: IntegraService;
  settingsOpen?: boolean;
} = {}) {
  const selectPart = vi.fn();
  const onTab = vi.fn();
  const reload = vi.fn();
  const onToggleSettings = vi.fn();
  const onStudioSetSettings = vi.fn();
  const mixer = { state, selectPart, reload } as unknown as UseMixerResult;
  const { container } = render(
    <InitProvider service={service} mixer={mixer}>
      <NavRow
        mixer={mixer}
        service={service}
        catalog={{} as ToneCatalog}
        expansions={expansions}
        onLoadExpansion={() => {}}
        pages={browserPages(sequencer)}
        tab={tab}
        onTab={onTab}
        transport={sequencer ? <button>Panic</button> : null}
        locked={locked}
        partSettings={{ open: settingsOpen, onToggle: onToggleSettings }}
        onStudioSetSettings={onStudioSetSettings}
      />
    </InitProvider>,
  );
  return { container, selectPart, onTab, reload, onToggleSettings, onStudioSetSettings };
}

/** The row's scope groups, by name. */
const groups = (scope: ParentNode) =>
  [...scope.querySelectorAll('nav [role="group"]')].map((g) => g.getAttribute("aria-label"));

function group(scope: ParentNode, name: string): HTMLElement {
  const found = scope.querySelector<HTMLElement>(`nav [role="group"][aria-label="${name}"]`);
  if (!found) throw new Error(`no "${name}" group`);
  return found;
}

/** The page buttons of `scope`, as shown. */
const pages = (scope: ParentNode) =>
  [...scope.querySelectorAll("button[data-page]")].map((b) => b.getAttribute("data-page"));

describe("the navigation row", () => {
  it("nests the scopes left to right: Device, Studio Set, Part, Tone", () => {
    const { container } = row();
    expect(groups(container)).toEqual(["Device", "Studio Set", "Part", "Tone"]);
  });

  it("puts each page in the scope that decides what it shows", () => {
    const { container } = row();
    expect(pages(group(container, "Device"))).toEqual(["system", "expansions", "library"]);
    expect(pages(group(container, "Studio Set"))).toEqual(["mixer", "surround", "routing", "sequencer"]);
    expect(pages(group(container, "Part"))).toEqual([]);
    expect(pages(group(container, "Tone"))).toEqual(["toneEdit"]);
  });

  it("calls the tone editor Edit in the Tone group, and Tone Edit to a screen reader", () => {
    const { container } = row();
    const edit = group(container, "Tone").querySelector("button[data-page]")!;
    expect(edit.textContent).toBe("Tone Edit");
    const shown = [...edit.childNodes].filter(
      (n) => !(n instanceof HTMLElement && n.hasAttribute("data-scope-name")),
    );
    expect(shown.map((n) => n.textContent).join("")).toBe("Edit");
  });

  it("leaves out the Sequencer until there is one", () => {
    const { container } = row({ sequencer: false });
    expect(pages(group(container, "Studio Set"))).toEqual(["mixer", "surround", "routing"]);
    expect(container.textContent).not.toContain("Panic");
  });

  it("opens a page from its group, and marks the page shown", () => {
    const { container, onTab } = row({ tab: "surround" });
    click(button(group(container, "Tone"), "Tone Edit"));
    expect(onTab).toHaveBeenCalledWith("toneEdit");
    const current = container.querySelectorAll('[aria-current="page"]');
    expect([...current].map((b) => b.getAttribute("data-page"))).toEqual(["surround"]);
  });

  it("holds each control in its scope: the Studio Set's SAVE, the Part's PREVIEW, the tone's SAVE", () => {
    const { container } = row();
    expect(button(group(container, "Studio Set"), "SAVE").title).toContain("studio set");
    expect(button(group(container, "Part"), "PREVIEW")).toBeTruthy();
    expect(button(group(container, "Tone"), "SAVE").title).toContain("tone");
    expect(group(container, "Studio Set").textContent).toContain("Panic");
  });

  it("opens the Studio Set settings drawer from ⚙ in the Studio Set group", () => {
    const { container, onStudioSetSettings } = row();
    click(button(group(container, "Studio Set"), "Studio Set settings"));
    expect(onStudioSetSettings).toHaveBeenCalledTimes(1);
  });

  it("shows the selected Part's receive channel", () => {
    const base = defaultMixerState();
    const parts = base.parts.map((p, i) => (i === 2 ? { ...p, receiveChannel: 9 } : p));
    const { container } = row({ state: { ...base, loading: false, parts, selectedPart: 2 } });
    expect(group(container, "Part").textContent).toContain("ch 10");
  });

  it("shows no channel before the mixer is read, while the Parts hold defaults", () => {
    const { container } = row({ state: { ...defaultMixerState(), loading: true } });
    expect(group(container, "Part").textContent).not.toContain("ch ");
  });

  it("selects a Part from the Part group", () => {
    const { container, selectPart } = row();
    const select = group(container, "Part").querySelector("select")!;
    expect(select.options).toHaveLength(deviceSpec().part_count);
    act(() => {
      select.value = "4";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(selectPart).toHaveBeenCalledWith(4);
  });

  it("locks the Studio Set, Part and Tone controls during a Pack import, not the pages or the transport", () => {
    const { container } = row({ locked: true });
    const inert = (el: Element) => el.closest("[inert]") !== null;
    for (const name of ["Studio Set", "Tone"]) {
      expect(inert(button(group(container, name), "SAVE"))).toBe(true);
    }
    expect(inert(button(group(container, "Studio Set"), "Studio Set settings"))).toBe(true);
    expect(inert(button(group(container, "Part"), "PREVIEW"))).toBe(true);
    expect(inert(group(container, "Part").querySelector("select")!)).toBe(true);
    for (const page of container.querySelectorAll("button[data-page]")) {
      expect(inert(page)).toBe(false);
    }
    expect(inert(button(container, "Panic"))).toBe(false);
    expect(inert(button(group(container, "Part"), "Part settings"))).toBe(true);
  });

  it("toggles the Part settings drawer from ⚙ in the Part group, pressed while it is open", () => {
    const { container, onToggleSettings } = row();
    const gear = button(group(container, "Part"), "Part settings");
    expect(gear.textContent).toBe("⚙");
    expect(gear.getAttribute("aria-pressed")).toBe("false");
    click(gear);
    expect(onToggleSettings).toHaveBeenCalledTimes(1);
    cleanup();
    const open = row({ settingsOpen: true });
    expect(button(group(open.container, "Part"), "Part settings").getAttribute("aria-pressed")).toBe("true");
  });

  it("puts the Studio Set's Init… right after its SAVE", () => {
    const { container } = row();
    const controls = group(container, "Studio Set").querySelectorAll("button:not([data-page])");
    const labels = [...controls].map((b) => b.textContent);
    expect(labels.indexOf("Init…")).toBe(labels.indexOf("SAVE") + 1);
  });

  it("runs a Studio Set Init once confirmed, holds the scopes' controls until \"finished\", then re-reads the mixer", async () => {
    let finish: (error?: string) => void = () => {};
    const initialize = vi.fn(
      () =>
        new Promise<string | undefined>((resolve) => {
          finish = resolve;
        }),
    );
    const { container, reload } = row({ service: { initialize } as unknown as IntegraService });
    click(button(group(container, "Studio Set"), "Init…"));
    const dialog = document.querySelector('[role="dialog"]')!;
    expect(dialog.textContent).toContain("the Temporary Studio Set");
    expect(initialize).not.toHaveBeenCalled();
    click(button(dialog, "Init"));
    expect(initialize).toHaveBeenCalledWith({ kind: "studio-set" });

    const busy = (el: Element) => el.closest('[inert][aria-busy="true"]') !== null;
    expect(busy(button(group(container, "Studio Set"), "Init…"))).toBe(true);
    expect(busy(button(group(container, "Studio Set"), "SAVE"))).toBe(true);
    expect(busy(group(container, "Part").querySelector("select")!)).toBe(true);
    expect(busy(button(group(container, "Tone"), "SAVE"))).toBe(true);
    // The pages and the transport stay live.
    for (const page of container.querySelectorAll("button[data-page]")) {
      expect(page.closest("[inert]")).toBeNull();
    }
    expect(button(container, "Panic").closest("[inert]")).toBeNull();
    expect(reload).not.toHaveBeenCalled();

    await act(async () => finish());
    expect(reload).toHaveBeenCalledOnce();
    expect(container.querySelector("[inert]")).toBeNull();
  });

  it("locks the Studio Set's Init… during a Pack import", () => {
    const { container } = row({ locked: true });
    expect(button(group(container, "Studio Set"), "Init…").closest("[inert]")).not.toBeNull();
  });
});
