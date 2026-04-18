//! Platform-agnostic MIDI request/response bridge.
//!
//! This module provides [`RequestTracker`], which manages pending RQ1 requests
//! and matches them against incoming DT1 responses by address key. It extracts
//! the core request/response pairing logic that was previously embedded in the
//! web frontend's TypeScript code, making it reusable across WASM and VST3
//! targets.

use std::collections::HashMap;

/// Convert a 4-byte SysEx address to a colon-separated hex string key.
///
/// This is used for matching RQ1 requests to DT1 responses.
///
/// # Examples
///
/// ```
/// use integral_core::midi_bridge::address_key;
/// assert_eq!(address_key(&[0x19, 0x01, 0x00, 0x00]), "19:01:00:00");
/// ```
pub fn address_key(addr: &[u8]) -> String {
    addr.iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// A pending RQ1 request awaiting a DT1 response.
struct PendingRequest {
    /// Monotonic timestamp (in milliseconds) when the request was sent.
    sent_at: f64,
}

/// Tracks pending RQ1 requests and matches them against incoming DT1 responses.
///
/// This struct is platform-agnostic: it accepts monotonic timestamps as `f64`
/// milliseconds so callers can provide `performance.now()` (WASM),
/// `std::time::Instant` (native), or any other clock source.
pub struct RequestTracker {
    /// Pending requests keyed by address string, with timeout timestamp.
    pending: HashMap<String, PendingRequest>,
}

impl RequestTracker {
    /// Create a new, empty request tracker.
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
        }
    }

    /// Register a pending RQ1 request for the given address key.
    ///
    /// If a request with the same address key already exists, it is replaced.
    pub fn register_request(&mut self, address_key: String, now_ms: f64) {
        self.pending
            .insert(address_key, PendingRequest { sent_at: now_ms });
    }

    /// Check if an incoming DT1 matches a pending request.
    ///
    /// Returns `true` if the address key matched a pending request (which is
    /// then removed). Returns `false` if no matching request was found.
    pub fn match_response(&mut self, address_key: &str, _now_ms: f64) -> bool {
        self.pending.remove(address_key).is_some()
    }

    /// Remove all requests older than `timeout_ms` relative to `now_ms`.
    pub fn expire_old(&mut self, now_ms: f64, timeout_ms: f64) {
        self.pending
            .retain(|_, req| (now_ms - req.sent_at) < timeout_ms);
    }

    /// Returns `true` if there are any pending requests.
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Returns the number of pending requests.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

impl Default for RequestTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_match() {
        let mut tracker = RequestTracker::new();
        let key = address_key(&[0x19, 0x01, 0x00, 0x00]);

        tracker.register_request(key.clone(), 1000.0);
        assert!(tracker.has_pending());
        assert_eq!(tracker.pending_count(), 1);

        // First match succeeds.
        assert!(tracker.match_response(&key, 1050.0));
        // Second match fails — already consumed.
        assert!(!tracker.match_response(&key, 1100.0));
        assert!(!tracker.has_pending());
    }

    #[test]
    fn timeout_expiry() {
        let mut tracker = RequestTracker::new();
        let key = address_key(&[0x18, 0x00, 0x00, 0x00]);

        tracker.register_request(key.clone(), 1000.0);
        assert_eq!(tracker.pending_count(), 1);

        // Not yet expired at 2999ms with 2000ms timeout.
        tracker.expire_old(2999.0, 2000.0);
        assert_eq!(tracker.pending_count(), 1);

        // Expired at 3000ms with 2000ms timeout.
        tracker.expire_old(3000.0, 2000.0);
        assert_eq!(tracker.pending_count(), 0);
    }

    #[test]
    fn no_match_wrong_address() {
        let mut tracker = RequestTracker::new();
        let key_a = address_key(&[0x19, 0x01, 0x00, 0x00]);
        let key_b = address_key(&[0x18, 0x00, 0x20, 0x00]);

        tracker.register_request(key_a.clone(), 1000.0);

        // Matching with a different address fails.
        assert!(!tracker.match_response(&key_b, 1050.0));
        // Original is still pending.
        assert!(tracker.has_pending());
        assert_eq!(tracker.pending_count(), 1);

        // Correct address still works.
        assert!(tracker.match_response(&key_a, 1100.0));
    }

    #[test]
    fn address_key_format() {
        assert_eq!(address_key(&[0x19, 0x01, 0x00, 0x00]), "19:01:00:00");
        assert_eq!(address_key(&[0x00, 0x00, 0x00, 0x04]), "00:00:00:04");
        assert_eq!(address_key(&[0xFF, 0xAB, 0x0C, 0x7F]), "ff:ab:0c:7f");
    }
}
