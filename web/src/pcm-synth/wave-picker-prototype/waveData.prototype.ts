// PROTOTYPE ONLY (throwaway). Wave name data + naming heuristics for the
// Wave picker variants. Not for main: the real data belongs in integral-core.
import data from "./waves.prototype.json";

export interface WaveGroup {
  /** Wave Group ID: 0 = INT, n = SRX-n. */
  id: number;
  name: string;
  /** Wave Number n is waves[n - 1]. */
  waves: string[];
}

export const GROUPS: WaveGroup[] = data.groups;

/** Wave Group Type the device expects for a Wave Group ID. */
export function groupTypeFor(id: number): number {
  return id === 0 ? 0 : 1;
}

export function findGroup(type: number, id: number): WaveGroup | undefined {
  return GROUPS.find((g) => g.id === id && groupTypeFor(g.id) === type);
}

/** Display name for a Group even when the prototype has no names for it. */
export function groupLabel(type: number, id: number): string {
  if (type === 0) return "INT";
  return `SRX-${String(id).padStart(2, "0")}`;
}

export function waveName(group: WaveGroup | undefined, n: number): string {
  if (n === 0) return "OFF";
  return group?.waves[n - 1] ?? `#${n}`;
}

export function pad4(n: number): string {
  return String(n).padStart(4, "0");
}

function isBoundary(s: string, i: number): boolean {
  return i < 0 || i >= s.length || s[i] === " ";
}

/**
 * Stereo pair heuristic: a and b differ in exactly one character, a has "L"
 * and b has "R" there, and that character is a whole token or the last
 * character ("StGrand pA L", "StGrand L pA", "080:BladeBtL").
 */
export function isLRPair(a: string, b: string): boolean {
  if (a.length !== b.length) return false;
  let diff = -1;
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) {
      if (diff !== -1) return false;
      diff = i;
    }
  }
  if (diff === -1 || a[diff] !== "L" || b[diff] !== "R") return false;
  return diff === a.length - 1 || (isBoundary(a, diff - 1) && isBoundary(a, diff + 1));
}

/** If Wave n is the L half of a Stereo pair, its R Wave Number; else null. */
export function rPartnerOf(group: WaveGroup | undefined, n: number): number | null {
  if (!group || n < 1) return null;
  const a = group.waves[n - 1];
  const b = group.waves[n];
  return a !== undefined && b !== undefined && isLRPair(a, b) ? n + 1 : null;
}

/** Name of a pair with its L/R token replaced by "L/R". */
export function pairLabel(name: string): string {
  for (let i = name.length - 1; i >= 0; i--) {
    if (name[i] === "L" && (i === name.length - 1 || (isBoundary(name, i - 1) && isBoundary(name, i + 1)))) {
      return `${name.slice(0, i)}L/R${name.slice(i + 1)}`;
    }
  }
  return name;
}

/** One row of a list where Stereo pairs are merged. */
export interface WaveEntry {
  /** Wave Number L (the only number for a mono Wave). */
  l: number;
  /** Wave Number R for a pair, 0 for mono. */
  r: number;
  label: string;
}

export function mergedEntries(group: WaveGroup): WaveEntry[] {
  const out: WaveEntry[] = [];
  for (let n = 1; n <= group.waves.length; n++) {
    const name = group.waves[n - 1] ?? "";
    const r = rPartnerOf(group, n);
    if (r !== null) {
      out.push({ l: n, r, label: pairLabel(name) });
      n++;
    } else {
      out.push({ l: n, r: 0, label: name });
    }
  }
  return out;
}

/** "080:BladeBtL" / "75:BoomRvBel": a BPM-prefixed phrase. */
export function isBpmName(name: string): boolean {
  const colon = name.indexOf(":");
  if (colon < 2 || colon > 3) return false;
  for (let i = 0; i < colon; i++) {
    const c = name.charCodeAt(i);
    if (c < 48 || c > 57) return false;
  }
  return true;
}

export const BPM_FAMILY = "BPM loops";

/** Derived "family" of a name: BPM loops, else its first space-separated word. */
export function familyOf(name: string): string {
  if (isBpmName(name)) return BPM_FAMILY;
  const sp = name.indexOf(" ");
  return sp === -1 ? name : name.slice(0, sp);
}

/** Families with at least `min` members, largest first. */
export function families(group: WaveGroup, min = 4): { name: string; count: number }[] {
  const counts = new Map<string, number>();
  for (const w of group.waves) {
    const f = familyOf(w);
    counts.set(f, (counts.get(f) ?? 0) + 1);
  }
  return [...counts]
    .filter(([, c]) => c >= min)
    .sort((a, b) => b[1] - a[1])
    .map(([name, count]) => ({ name, count }));
}

export function matches(name: string, query: string): boolean {
  return name.toLowerCase().includes(query.trim().toLowerCase());
}
