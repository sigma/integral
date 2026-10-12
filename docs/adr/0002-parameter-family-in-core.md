# Parameter family and its palette are declared in core

Every parameter's **Parameter family** is a required `ParameterFamily` field on its `integral-core` definition: on `FieldSpec` for the tone engines, on `FxParamDef` for MFX, Chorus and Reverb, and on `ParamDef` in `param_registry` for the mixer EQ and Comp. The palette (`ParameterFamily::colour()`) sits beside the enum in `integral-core::family`. The VST reads both directly and the web reads them through wasm, so web and VST draw one family in one colour, and no view holds a colour literal. Decided in [Where Parameter families are declared](https://github.com/sigma/integral/issues/157).

## Consequences

- Core now holds presentation: the palette is not device data, but core is the only crate both surfaces already share. A tokens file built into both, or a TS copy guarded by a test, were rejected as plumbing or a second source of truth.
- The field is required, never `Option`: the exhaustive spec matches force a classification for every new field. A parameter that fits no family gets an explicit variant, not a `None`.
- MFX families are emitted as literals by `docs/research/mfx-param-gen.mjs` from the rules and exceptions in `docs/research/mfx-param-families.md`; after generation the table, not the rules, is the source of truth.
- Comp gains `ParamDef` entries, so the Comp+EQ knobs have one source like every other control.
- Sections are screen layout, not device data: a `SectionPanel` names its family in the view, and its colour still comes from the palette. MFX skins lose their colour props; EQ band hues go with them.
- Family is never inferred from a section or a parameter name at runtime: Pan sits in AMP, so section and family can disagree.
