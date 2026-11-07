/**
 * What a plugin's editor shows (ADR 0007): only what its instance owns.
 *
 * The Studio Set plugin owns the mixer, so it shows the Mixer page (with
 * Chorus, Reverb and the Master EQ) and the Surround page. A Tone or Kit
 * plugin owns one Part's tone of its type, so it shows the tone editor,
 * scoped to its Binding.
 */

import {
  PluginKind,
  ToneType,
  plugin_kind_accepts,
  plugin_kind_tone_type,
} from "../pkg/integral_wasm.js";
import { deviceSpec } from "./deviceSpec";
import type { PluginBinding } from "./plugin";

/** The pages of the app. */
export type PageTab = "mixer" | "surround" | "toneEdit" | "sequencer" | "expansions" | "library";

/** The pages a plugin of `kind` shows, the first being where it opens. */
export function pluginPages(kind: PluginKind): readonly PageTab[] {
  return kind === PluginKind.StudioSet ? ["mixer", "surround"] : ["toneEdit"];
}

/** The tone type a Tone or Kit plugin edits; `undefined` for the Studio Set plugin. */
export function pluginToneType(kind: PluginKind): ToneType | undefined {
  return plugin_kind_tone_type(kind);
}

/** What of a drum kit a Kit plugin's Binding leaves its editor: the whole kit, or one key. */
export type DrumScope = "kit" | { key: number };

/** The drum scope of a Kit plugin bound to `binding`. */
export function drumScope(binding: PluginBinding): DrumScope {
  return binding.key === undefined ? "kit" : { key: binding.key };
}

/** The drum scope of a plugin of `kind` bound to `binding`; `undefined` for a plugin that is no Kit plugin. */
export function pluginDrumScope(kind: PluginKind, binding: PluginBinding): DrumScope | undefined {
  const type = pluginToneType(kind);
  return type === ToneType.SnDrum || type === ToneType.PcmDrum ? drumScope(binding) : undefined;
}

/** The keys a Kit plugin of `kind` can be bound to; `undefined` for a plugin that is no Kit plugin. */
export function kitKeys(kind: PluginKind): { first: number; last: number } | undefined {
  const type = pluginToneType(kind);
  if (type === ToneType.SnDrum) return deviceSpec().sn_drum_keys;
  if (type === ToneType.PcmDrum) return deviceSpec().pcm_drum_keys;
  return undefined;
}

/** The one key a drum editor edits under `scope`, if it is one key's. */
export function lockedKey(scope: DrumScope | undefined): number | undefined {
  return typeof scope === "object" ? scope.key : undefined;
}

/** Whether a plugin of `kind` can be bound to `binding` (core's rule). */
export function acceptsBinding(kind: PluginKind, binding: PluginBinding): boolean {
  return plugin_kind_accepts(kind, binding.part, binding.key);
}
