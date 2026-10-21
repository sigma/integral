/**
 * The INTEGRA-7's Wave catalog, read from the core device spec via wasm.
 *
 * A {@link WaveSource} binds one Wave Group to its Waves, families and
 * Stereo pairs. Sources are built once per group and cached.
 */

import {
  WaveGroup,
  waveFamilies,
  waveGroupOf,
  waveGroupWavesJson,
  waveGroups,
  waveName,
  waveStereoPartner,
} from "../pkg/integral_wasm.js";
import { expansionCatalog, type Expansion } from "./expansionCatalog";

/** One named Wave of a Wave Group. */
export interface Wave {
  /** Wave Number (1…N; 0 is OFF). */
  number: number;
  /** Wave name. */
  name: string;
  /** Family the name belongs to (its first word, or "BPM loops"). */
  family: string;
}

/** The Waves of one Wave Group, and what core knows about them. */
export interface WaveSource {
  /** Group identifier: `INT`, `SRX-01`…`SRX-12`, `SN-Synth`. */
  id: string;
  /** Wave Group Type and ID to write, or null for SN Synth. */
  params: readonly [number, number] | null;
  /** The SRX Expansion providing the Waves, or null. */
  expansion: Expansion | null;
  /** Every Wave, in Wave Number order (OFF excluded). */
  waves: readonly Wave[];
  /** Distinct families, in catalog order. */
  families: readonly string[];
  /** Name of Wave `number` ("OFF" for 0), or null past the end. */
  name: (number: number) => string | null;
  /** The R Wave Number paired with L Wave `number`, or null. */
  stereoPartner: (number: number) => number | null;
}

const cache = new Map<string, WaveSource>();

/** The source of `group`; takes ownership of the wasm object. */
function sourceOf(group: WaveGroup): WaveSource {
  const id = group.id;
  const cached = cache.get(id);
  if (cached) {
    group.free();
    return cached;
  }
  const params = group.params();
  const expansionName = group.expansionName;
  const source: WaveSource = {
    id,
    params: params ? [params[0]!, params[1]!] : null,
    expansion: expansionCatalog().find((e) => e.name === expansionName) ?? null,
    waves: JSON.parse(waveGroupWavesJson(group)) as Wave[],
    families: waveFamilies(group),
    name: (n) => waveName(group, n) ?? null,
    stereoPartner: (n) => waveStereoPartner(group, n) ?? null,
  };
  cache.set(id, source);
  return source;
}

/** The source selected by a Wave Group Type and ID, or null if reserved. */
export function waveSourceOf(type: number, id: number): WaveSource | null {
  const group = waveGroupOf(type, id);
  return group ? sourceOf(group) : null;
}

let pcmSources: WaveSource[] | null = null;

/** The groups a PCM partial or WMT picks from (INT, SRX-01…12). */
export function pcmWaveSources(): readonly WaveSource[] {
  pcmSources ??= waveGroups().map(sourceOf);
  return pcmSources;
}
