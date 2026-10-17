import { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import css from "./PwaUpdateBanner.module.css";
import { PwaUpdateBanner } from "./PwaUpdateBanner";
import { button, cleanup, click, render } from "./test/dom";

interface RegisterOptions {
  onNeedRefresh?: () => void;
}

// The virtual module's registration and its updater, controllable per test.
const { register, updateSW } = vi.hoisted(() => ({
  register: vi.fn<(options: RegisterOptions) => void>(),
  updateSW: vi.fn<() => Promise<void>>(() => Promise.resolve()),
}));

vi.mock("virtual:pwa-register", () => ({
  registerSW: (options: RegisterOptions) => {
    register(options);
    return updateSW;
  },
}));

afterEach(cleanup);

/** Report a waiting update through the last registration's callback. */
function needRefresh(): void {
  const last = register.mock.calls[register.mock.calls.length - 1];
  act(() => last?.[0].onNeedRefresh?.());
}

describe("PwaUpdateBanner", () => {
  it("registers the service worker and stays hidden until an update is ready", () => {
    const { container } = render(<PwaUpdateBanner />);
    expect(register).toHaveBeenCalledOnce();
    expect(container.textContent).not.toContain("New version available");
    expect(updateSW).not.toHaveBeenCalled();
  });

  it("shows the banner when a new version is waiting; Reload applies it", () => {
    const { container } = render(<PwaUpdateBanner />);

    needRefresh();
    expect(container.querySelector(`.${css.banner}`)).not.toBeNull();

    click(button(container, "Reload"));
    expect(updateSW).toHaveBeenCalledOnce();
  });

  it("Dismiss hides the banner without applying the update", () => {
    const { container } = render(<PwaUpdateBanner />);

    needRefresh();
    click(button(container, "Dismiss"));
    expect(container.textContent).not.toContain("New version available");
    expect(updateSW).not.toHaveBeenCalled();
  });

  it("shows again for a newer update after a Dismiss", () => {
    const { container } = render(<PwaUpdateBanner />);

    needRefresh();
    click(button(container, "Dismiss"));
    needRefresh();
    expect(container.querySelector(`.${css.banner}`)).not.toBeNull();
  });
});
