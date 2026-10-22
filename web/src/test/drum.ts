/** Helpers for the drum kit editor tests. */

/** The cell of `key` (its MIDI number) in a drum editor's key grid. */
export function drumKey(scope: ParentNode, key: number): Element {
  const number = [...scope.querySelectorAll("span")].find((s) => s.textContent === String(key));
  const cell = number?.parentElement;
  if (!cell) throw new Error(`no drum key ${key}`);
  return cell;
}

/**
 * Every Control on the page outside a Grid Section, the MFX placement aside:
 * it hosts `MfxPanel`, a CSS Section with its own selects and send knobs.
 */
export function controlsOffGrid(scope: ParentNode): Element[] {
  return [...scope.querySelectorAll('[role="slider"], [role="radiogroup"], select')].filter(
    (c) => c.closest("[data-grid-section]") === null && c.closest('[data-section="mfx"]') === null,
  );
}

/**
 * Where the kit-wide KIT Section sits: the key matrix's row holding it (the
 * one holding the key grid too), and the Grid screen of key Sections holding
 * it, if any.
 */
export function kitPlacement(scope: ParentNode, key: number): { besideKeys: boolean; inKeyScreen: boolean } {
  const kit = scope.querySelector('[data-grid-section="KIT"]');
  if (!kit) throw new Error("no KIT Section");
  const row = kit.closest("[data-drum-matrix]");
  return {
    besideKeys: row !== null && row === drumKey(scope, key).closest("[data-drum-matrix]"),
    inKeyScreen: kit.closest("[data-grid-screen]") !== null,
  };
}
