//! Tone catalog cache with deduplication.
//!
//! The catalog owns a per-bank cache of tone entries keyed by `(lsb, pc)`.
//! Entries are routed by their self-identified `(msb, lsb, pc)` — not by
//! which request produced them — so concurrent or overlapping fetches
//! cannot contaminate each other's buckets.
//!
//! The catalog is a pure data structure: it does not perform I/O.  The
//! host drives fetching by calling [`ToneCatalog::next_request`] and
//! feeding responses via [`ToneCatalog::add_entry`].

use std::collections::{HashMap, HashSet};

use crate::catalog;
use crate::tone_banks::{self, ToneBank};

/// A tone entry with its self-identified bank coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToneEntry {
    pub msb: u8,
    pub lsb: u8,
    pub pc: u8,
    pub name: String,
}

/// Cache key for a tone bank: `"msb:lsb1,lsb2,..."`.
fn bank_key(bank: &ToneBank) -> String {
    let lsbs: Vec<String> = bank.lsbs.iter().map(|l| l.to_string()).collect();
    format!("{}:{}", bank.msb, lsbs.join(","))
}

/// Per-bank fetch progress.
struct BankFetch {
    /// Which (lsb, start) pages have been requested.
    pages_requested: HashSet<(u8, u8)>,
    /// All (lsb, start) pages we need to fetch.
    all_pages: Vec<(u8, u8)>,
    /// Whether all pages have been received.
    complete: bool,
}

impl BankFetch {
    fn new(bank: &ToneBank) -> Self {
        let all_pages: Vec<(u8, u8)> = bank
            .lsbs
            .iter()
            .flat_map(|&lsb| [(lsb, 0), (lsb, 64)])
            .collect();
        Self {
            pages_requested: HashSet::new(),
            all_pages,
            complete: false,
        }
    }
}

/// Tone catalog cache.
pub struct ToneCatalog {
    /// Deduplicated entries per bank key, keyed by `"lsb:pc"`.
    cache: HashMap<String, HashMap<String, ToneEntry>>,
    /// Fetch progress per bank key.
    fetches: HashMap<String, BankFetch>,
    /// Banks whose fetch completed.
    complete: HashSet<String>,
}

impl ToneCatalog {
    /// Create an empty catalog.
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            fetches: HashMap::new(),
            complete: HashSet::new(),
        }
    }

    /// Get cached entries for a bank (empty vec if not yet fetched).
    pub fn get(&self, bank: &ToneBank) -> Vec<&ToneEntry> {
        let key = bank_key(bank);
        match self.cache.get(&key) {
            Some(bucket) => bucket.values().collect(),
            None => Vec::new(),
        }
    }

    /// Whether all pages for this bank have been fetched.
    pub fn is_complete(&self, bank: &ToneBank) -> bool {
        self.complete.contains(&bank_key(bank))
    }

    /// Whether this bank has an active (incomplete) fetch.
    pub fn is_loading(&self, bank: &ToneBank) -> bool {
        let key = bank_key(bank);
        self.fetches.contains_key(&key) && !self.complete.contains(&key)
    }

    /// Start fetching a bank.  Idempotent if already complete or in progress.
    pub fn start_fetch(&mut self, bank: &ToneBank) {
        let key = bank_key(bank);
        if self.complete.contains(&key) || self.fetches.contains_key(&key) {
            return;
        }
        self.fetches.insert(key, BankFetch::new(bank));
    }

    /// Get the next catalog RQ1 to send for the given bank, if any.
    ///
    /// Returns `None` if all pages have been requested or the bank is
    /// complete.  The caller should send the returned bytes via SysEx.
    pub fn next_request(&mut self, bank: &ToneBank, device_id: u8) -> Option<Vec<u8>> {
        let key = bank_key(bank);
        let fetch = self.fetches.get_mut(&key)?;
        if fetch.complete {
            return None;
        }

        for &(lsb, start) in &fetch.all_pages {
            if !fetch.pages_requested.contains(&(lsb, start)) {
                fetch.pages_requested.insert((lsb, start));
                return Some(catalog::build_tone_catalog_request(
                    device_id, bank.msb, lsb, start, 64,
                ));
            }
        }
        None
    }

    /// Feed a catalog entry.  Routes to the correct bucket by identity.
    ///
    /// Returns the bank key if the entry was new (for notification).
    pub fn add_entry(&mut self, entry: ToneEntry) -> Option<String> {
        // Find which bank this entry belongs to.
        let bank = tone_banks::find_bank(entry.msb, entry.lsb)?;
        let key = bank_key(bank);

        let bucket = self.cache.entry(key.clone()).or_default();
        let ek = format!("{}:{}", entry.lsb, entry.pc);
        if bucket.contains_key(&ek) {
            return None; // Already have it.
        }
        bucket.insert(ek, entry);
        Some(key)
    }

    /// Mark a page as received and check if the bank is now complete.
    ///
    /// Call this after processing all entries from a `requestToneCatalogPage`
    /// response.  Returns `true` if the bank just became complete.
    pub fn mark_page_received(&mut self, bank: &ToneBank, lsb: u8, start: u8) -> bool {
        let key = bank_key(bank);
        if let Some(fetch) = self.fetches.get_mut(&key) {
            fetch.pages_requested.insert((lsb, start));
            if fetch.pages_requested.len() >= fetch.all_pages.len() {
                fetch.complete = true;
                self.complete.insert(key);
                return true;
            }
        }
        false
    }
}

impl Default for ToneCatalog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_bank() -> &'static ToneBank {
        tone_banks::find_bank(89, 64).unwrap() // SN Acoustic Preset
    }

    #[test]
    fn empty_catalog() {
        let cat = ToneCatalog::new();
        let bank = test_bank();
        assert!(cat.get(bank).is_empty());
        assert!(!cat.is_complete(bank));
        assert!(!cat.is_loading(bank));
    }

    #[test]
    fn start_fetch_makes_loading() {
        let mut cat = ToneCatalog::new();
        let bank = test_bank();
        cat.start_fetch(bank);
        assert!(cat.is_loading(bank));
        assert!(!cat.is_complete(bank));
    }

    #[test]
    fn add_entry_routes_by_identity() {
        let mut cat = ToneCatalog::new();
        let bank = test_bank();
        cat.start_fetch(bank);

        let result = cat.add_entry(ToneEntry {
            msb: 89,
            lsb: 64,
            pc: 0,
            name: "Piano 1".to_string(),
        });
        assert!(result.is_some());
        assert_eq!(cat.get(bank).len(), 1);
    }

    #[test]
    fn deduplicates_entries() {
        let mut cat = ToneCatalog::new();
        let bank = test_bank();
        cat.start_fetch(bank);

        let entry = ToneEntry {
            msb: 89,
            lsb: 64,
            pc: 0,
            name: "Piano 1".to_string(),
        };
        cat.add_entry(entry.clone());
        let result = cat.add_entry(entry);
        assert!(result.is_none()); // Duplicate.
        assert_eq!(cat.get(bank).len(), 1);
    }

    #[test]
    fn wrong_bank_entry_ignored() {
        let mut cat = ToneCatalog::new();
        let bank = test_bank(); // MSB=89
        cat.start_fetch(bank);

        // Entry from MSB=95 (SN Synth) — should go to SN Synth bucket, not SN Acoustic.
        let result = cat.add_entry(ToneEntry {
            msb: 95,
            lsb: 64,
            pc: 0,
            name: "Synth 1".to_string(),
        });
        // It IS added (to SN Synth's bucket), but SN Acoustic's bucket is empty.
        assert!(result.is_some());
        assert_eq!(cat.get(bank).len(), 0);
    }

    #[test]
    fn next_request_returns_pages_in_order() {
        let mut cat = ToneCatalog::new();
        let bank = test_bank(); // lsbs = [64, 65]
        cat.start_fetch(bank);

        // Should have 4 pages: (64,0), (64,64), (65,0), (65,64)
        assert!(cat.next_request(bank, 0x10).is_some());
        assert!(cat.next_request(bank, 0x10).is_some());
        assert!(cat.next_request(bank, 0x10).is_some());
        assert!(cat.next_request(bank, 0x10).is_some());
        assert!(cat.next_request(bank, 0x10).is_none()); // All requested.
    }

    #[test]
    fn mark_page_completes_bank() {
        let mut cat = ToneCatalog::new();
        let bank = test_bank(); // lsbs = [64, 65] → 4 pages
        cat.start_fetch(bank);

        assert!(!cat.mark_page_received(bank, 64, 0));
        assert!(!cat.mark_page_received(bank, 64, 64));
        assert!(!cat.mark_page_received(bank, 65, 0));
        assert!(cat.mark_page_received(bank, 65, 64)); // Last page → complete.
        assert!(cat.is_complete(bank));
        assert!(!cat.is_loading(bank));
    }
}
