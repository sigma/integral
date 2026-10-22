# Layouts are TypeScript data in the web app

Every **Layout** is a TypeScript object in `web/src/layouts/`: one module per Section, plus one per screen. A Section's layout places its Controls, Displays, `alternatives` and Section Silkscreen, and says which edge carries its title. A variant set maps each key to a Section layout inside one footprint, and sits beside the runtime generator for keys it doesn't cover. A screen's layout places its Sections on the Grid and declares its Lanes and Screen Silkscreen. Coordinates are plain numbers: whole Cells for Sections, Controls and Displays, and halves for Silkscreen. Decided in [Author layouts: form and home](https://github.com/sigma/integral/issues/332).

## Consequences

- **Rust structs and data files were rejected.** Once the plugin runs the web app ([ADR 0004](0004-plugin-ui-in-a-webview.md)), the web renderer is the only consumer, and a Rust or RON/TOML/JSON home would only add a crossing into it. This applies [ADR 0002](0002-parameter-family-in-core.md)'s split: screen layout lives in the web, device data in core.
- **A layout names what core defines and defines nothing itself.**
  - Tone fields are named by their wasm enums (`SnsPartialField.OscPitch`), so a renamed field fails `tsc`.
  - The mixer registry's string ids (`part.{i}.chorus_send`, `eq.low_gain`) give way to an index-free wasm enum.
  - MFX, Chorus and Reverb params stay as type plus name from core's generated tables. They are checked by a test, not by the compiler: generating a TS copy of the names would be a second source of truth.
- **Displays and relevance come from core.** A layout names a Display by a core-exported id and never lists its fields. It gives each Control `inactive: "dim" | "hide"`, and core decides when a field is inactive. A layout holds no predicates.
- **Alignment is checked by a test, not by review.** One vitest suite walks every layout and every variant key, with the keys enumerated from core, and runs in CI through `just test-web`. It checks:
  - whole-Cell and half-Cell placement;
  - no overlap outside an `alternatives` group;
  - everything inside its Section or the variant footprint;
  - Display minimum footprints;
  - every parameter of a variant key placed;
  - Lanes left uncovered;
  - arrow endpoints that exist, arrows the Lanes (or a Section's free Cells) can route, and no two glyphs in one Cell;
  - FX names that exist;
  - no two options of a list or switch that read the same once cut to its width.
- **Positions are plain numbers, not branded types.** The validator enforces the lattice, so the layout files read as plain data.
- **No visual-regression tests in this spec.** Positions are data, so the validator proves alignment and the pilot proves the renderer. Screenshot tests come back only if the renderer drifts.
