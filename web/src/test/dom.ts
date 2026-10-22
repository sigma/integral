/** Minimal DOM helpers for component tests (React + jsdom, no extra deps). */

import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";

const mounted: { root: Root; container: HTMLElement }[] = [];

/** Render `node` into a fresh container attached to the document. */
export function render(node: ReactNode): {
  container: HTMLElement;
  rerender: (node: ReactNode) => void;
} {
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  mounted.push({ root, container });
  act(() => root.render(node));
  return {
    container,
    rerender: (next) => act(() => root.render(next)),
  };
}

/** Unmount everything `render` mounted and clear the document. */
export function cleanup(): void {
  for (const { root, container } of mounted.splice(0)) {
    act(() => root.unmount());
    container.remove();
  }
  document.body.innerHTML = "";
}

/** Click `element`. */
export function click(element: Element): void {
  act(() => {
    element.dispatchEvent(
      new MouseEvent("click", { bubbles: true, cancelable: true }),
    );
  });
}

/** Dispatch pointer event `type` at (`x`, `y`) on `target`. */
export function pointer(target: Element, type: string, x: number, y: number): void {
  act(() => {
    // jsdom has no PointerEvent; React only reads the type and coordinates.
    const event = new MouseEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y });
    Object.defineProperty(event, "pointerId", { value: 1 });
    target.dispatchEvent(event);
  });
}

/** Drag the slider `knob` (a knob or fader's `role="slider"`) up by `px` pixels. */
export function dragUp(knob: Element, px: number): void {
  // jsdom has no pointer capture.
  (knob as Element & { setPointerCapture: () => void }).setPointerCapture = () => {};
  pointer(knob, "pointerdown", 0, 100);
  pointer(knob, "pointermove", 0, 100 - px);
  pointer(knob, "pointerup", 0, 100 - px);
}

/** Type `value` into a React-controlled input. */
export function typeInto(input: HTMLInputElement, value: string): void {
  const setter = Object.getOwnPropertyDescriptor(
    HTMLInputElement.prototype,
    "value",
  )?.set;
  act(() => {
    setter?.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

/** The button whose text or accessible label is `name`. */
export function button(scope: ParentNode, name: string): HTMLButtonElement {
  const found = [...scope.querySelectorAll("button")].find(
    (b) =>
      b.textContent?.trim() === name || b.getAttribute("aria-label") === name,
  );
  if (!found) throw new Error(`no button "${name}"`);
  return found;
}

/** The option of the open listbox in `scope` whose text is `name`. */
export function option(scope: ParentNode, name: string): HTMLElement {
  const found = [...scope.querySelectorAll<HTMLElement>('[role="option"]')].find(
    (o) => o.textContent === name,
  );
  if (!found) throw new Error(`no option "${name}"`);
  return found;
}

/** Text of every option of the open listbox in `scope`. */
export function options(scope: ParentNode): string[] {
  return [...scope.querySelectorAll('[role="option"]')].map(
    (o) => o.textContent ?? "",
  );
}
