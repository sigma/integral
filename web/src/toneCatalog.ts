/**
 * Central tone catalog service.
 *
 * Owns the cache and orchestrates catalog page fetches. Each response
 * entry is routed to the correct cache bucket by its self-identified
 * (msb, lsb, pc). Subscribers are notified whenever their bank's
 * bucket receives new entries.
 */

import type { IntegraService } from "./integra";
import type { ToneBank } from "./toneBanks";
import { factoryTonesJson } from "../pkg/integral_wasm.js";

export interface ToneEntry {
  msb: number;
  lsb: number;
  pc: number;
  name: string;
}

type Listener = (entries: ToneEntry[]) => void;

export function bankKey(bank: ToneBank): string {
  return `${bank.msb}:${bank.lsbs.join(",")}`;
}

export class ToneCatalog {
  /** Deduplicated entries per bank key, keyed by "lsb:pc". */
  private cache = new Map<string, Map<string, ToneEntry>>();
  /** Listeners per bank key. */
  private listeners = new Map<string, Set<Listener>>();
  /** Banks whose fetch completed (all pages done). */
  private complete = new Set<string>();
  /** Banks currently being fetched. */
  private fetching = new Set<string>();

  constructor(private service: IntegraService) {}

  /** Get cached entries for a bank (empty array if not yet fetched). */
  get(bank: ToneBank): ToneEntry[] {
    const bucket = this.cache.get(bankKey(bank));
    return bucket ? [...bucket.values()] : [];
  }

  /** Whether all pages for this bank have been fetched. */
  isComplete(bank: ToneBank): boolean {
    return this.complete.has(bankKey(bank));
  }

  /** Whether this bank is currently being fetched. */
  isLoading(bank: ToneBank): boolean {
    return this.fetching.has(bankKey(bank));
  }

  /**
   * Subscribe to cache updates for a bank. The listener is called
   * with the full deduplicated entry list whenever new entries land.
   * Returns an unsubscribe function.
   */
  subscribe(bank: ToneBank, listener: Listener): () => void {
    const key = bankKey(bank);
    let set = this.listeners.get(key);
    if (!set) {
      set = new Set();
      this.listeners.set(key, set);
    }
    set.add(listener);
    return () => set!.delete(listener);
  }

  /**
   * Ensure all pages for a bank are fetched. Idempotent — calling
   * again for a complete or in-progress bank is a no-op.
   *
   * For preset banks with factory data, pre-populates instantly and
   * skips the MIDI fetch. For user banks, fetches dynamically.
   */
  fetch(bank: ToneBank): void {
    const key = bankKey(bank);
    if (this.complete.has(key) || this.fetching.has(key)) return;

    // Pre-populate from factory data.
    const factoryCount = this.populateFactory(bank, key);
    if (factoryCount > 0) {
      this.notify(key);
    }

    // If factory data fully covers this bank, skip MIDI fetch.
    const expectedCount = bank.lsbs.length * 128;
    if (factoryCount >= expectedCount) {
      this.complete.add(key);
      return;
    }

    // Otherwise, fetch dynamically (user banks, partial coverage).
    this.fetching.add(key);
    this.fetchPages(bank, key);
  }

  // -----------------------------------------------------------------------

  /** Pre-fill cache from compiled-in factory data. Returns count added. */
  private populateFactory(bank: ToneBank, key: string): number {
    let count = 0;
    for (const lsb of bank.lsbs) {
      const json = factoryTonesJson(bank.msb, lsb);
      const entries: ToneEntry[] = JSON.parse(json);
      for (const e of entries) {
        if (this.addEntry(key, e)) count++;
      }
    }
    return count;
  }

  private async fetchPages(bank: ToneBank, key: string): Promise<void> {
    const lsbSet = new Set(bank.lsbs);

    for (const lsb of bank.lsbs) {
      for (const start of [0, 64]) {
        const page = await this.service.requestToneCatalogPage(
          bank.msb,
          lsb,
          start,
          64,
        );

        let added = false;
        for (const e of page) {
          // Route by identity: only accept entries belonging to this bank.
          if (e.msb !== bank.msb || !lsbSet.has(e.lsb)) continue;
          added = this.addEntry(key, e) || added;
        }
        if (added) this.notify(key);
      }
    }

    this.fetching.delete(key);
    this.complete.add(key);
    this.notify(key);
  }

  /** Insert an entry into the bank's bucket. Returns true if new. */
  private addEntry(
    key: string,
    e: { msb: number; lsb: number; pc: number; name: string },
  ): boolean {
    let bucket = this.cache.get(key);
    if (!bucket) {
      bucket = new Map();
      this.cache.set(key, bucket);
    }
    const ek = `${e.lsb}:${e.pc}`;
    if (bucket.has(ek)) return false;
    bucket.set(ek, { msb: e.msb, lsb: e.lsb, pc: e.pc, name: e.name });
    return true;
  }

  private notify(key: string): void {
    const listeners = this.listeners.get(key);
    if (!listeners || listeners.size === 0) return;
    const bucket = this.cache.get(key);
    if (!bucket) return;
    const entries = [...bucket.values()];
    for (const listener of listeners) {
      listener(entries);
    }
  }
}
