import { describe, expect, it } from "vitest";
import { PluginKind, ToneType } from "../pkg/integral_wasm.js";
import { deviceSpec } from "./deviceSpec";
import {
  acceptsBinding,
  drumScope,
  kitKeys,
  lockedKey,
  pluginDrumScope,
  pluginPages,
  pluginToneType,
} from "./pluginPages";

const TONE_PLUGINS = [
  [PluginKind.SnSynth, ToneType.SnSynth],
  [PluginKind.SnAcoustic, ToneType.SnAcoustic],
  [PluginKind.PcmSynth, ToneType.PcmSynth],
] as const;

const KIT_PLUGINS = [
  [PluginKind.SnDrum, ToneType.SnDrum],
  [PluginKind.PcmDrum, ToneType.PcmDrum],
] as const;

describe("the pages of a plugin", () => {
  it("are the Mixer and Surround for the Studio Set plugin", () => {
    expect(pluginPages(PluginKind.StudioSet)).toEqual(["mixer", "surround"]);
    expect(pluginToneType(PluginKind.StudioSet)).toBeUndefined();
  });

  it.each([...TONE_PLUGINS, ...KIT_PLUGINS])(
    "are the tone editor, for the type it edits, for plugin %i",
    (kind, toneType) => {
      expect(pluginPages(kind)).toEqual(["toneEdit"]);
      expect(pluginToneType(kind)).toBe(toneType);
    },
  );

  it("never include the pages of another instance or of the browser alone", () => {
    for (const kind of [PluginKind.StudioSet, ...TONE_PLUGINS.map(([k]) => k), ...KIT_PLUGINS.map(([k]) => k)]) {
      for (const page of ["sequencer", "expansions", "library"] as const) {
        expect(pluginPages(kind)).not.toContain(page);
      }
    }
  });
});

describe("a Kit plugin's drum scope", () => {
  it("is the whole kit without a key, and that key with one", () => {
    expect(drumScope({ part: 2 })).toBe("kit");
    expect(drumScope({ part: 2, key: 38 })).toEqual({ key: 38 });
    expect(lockedKey("kit")).toBeUndefined();
    expect(lockedKey(undefined)).toBeUndefined();
    expect(lockedKey({ key: 38 })).toBe(38);
  });

  it.each(KIT_PLUGINS.map(([kind]) => kind))("applies to plugin %i, and to no other", (kind) => {
    expect(pluginDrumScope(kind, { part: 1 })).toBe("kit");
    expect(pluginDrumScope(kind, { part: 1, key: 40 })).toEqual({ key: 40 });
    expect(pluginDrumScope(PluginKind.StudioSet, {})).toBeUndefined();
    for (const [tone] of TONE_PLUGINS) expect(pluginDrumScope(tone, { part: 1 })).toBeUndefined();
  });

  it("lists the keys of the kit type", () => {
    expect(kitKeys(PluginKind.SnDrum)).toEqual(deviceSpec().sn_drum_keys);
    expect(kitKeys(PluginKind.PcmDrum)).toEqual(deviceSpec().pcm_drum_keys);
    expect(kitKeys(PluginKind.SnSynth)).toBeUndefined();
    expect(kitKeys(PluginKind.StudioSet)).toBeUndefined();
  });
});

describe("the Bindings a plugin accepts, by core's rule", () => {
  it("are no Part for the Studio Set plugin", () => {
    expect(acceptsBinding(PluginKind.StudioSet, {})).toBe(true);
    expect(acceptsBinding(PluginKind.StudioSet, { part: 0 })).toBe(false);
  });

  it("are a Part for a Tone plugin, never a key", () => {
    const last = deviceSpec().part_count - 1;
    expect(acceptsBinding(PluginKind.PcmSynth, { part: last })).toBe(true);
    expect(acceptsBinding(PluginKind.PcmSynth, { part: last + 1 })).toBe(false);
    expect(acceptsBinding(PluginKind.PcmSynth, {})).toBe(false);
    expect(acceptsBinding(PluginKind.PcmSynth, { part: 0, key: 36 })).toBe(false);
  });

  it("are a Part, and the whole kit or a key of the kit type, for a Kit plugin", () => {
    const { first, last } = deviceSpec().pcm_drum_keys;
    expect(acceptsBinding(PluginKind.PcmDrum, { part: 0 })).toBe(true);
    expect(acceptsBinding(PluginKind.PcmDrum, { part: 0, key: first })).toBe(true);
    expect(acceptsBinding(PluginKind.PcmDrum, { part: 0, key: last + 1 })).toBe(false);
    expect(acceptsBinding(PluginKind.PcmDrum, { key: first })).toBe(false);
  });
});
