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
