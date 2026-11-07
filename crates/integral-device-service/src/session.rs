//! Session IDs: one per time the link comes up.

use std::collections::hash_map::RandomState;
use std::fmt;
use std::hash::{BuildHasher, Hasher};

/// Identifies one device session: the span from a link coming up to it going
/// down. A client that sees the ID change knows the device may have changed
/// under it (a drop, a power cycle, a service restart).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(pub u64);

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// Where session IDs come from. Injected so tests are deterministic.
pub trait SessionIdSource: Send {
    /// An ID that differs from every one this source made before and, for a
    /// real source, from those of earlier service processes.
    fn next_id(&mut self) -> SessionId;
}

/// Random IDs, from the per-process random keys of the standard library's
/// hasher. No clock, no environment, no extra dependency.
#[derive(Debug, Default)]
pub struct RandomSessionIds {
    last: Option<SessionId>,
}

impl SessionIdSource for RandomSessionIds {
    fn next_id(&mut self) -> SessionId {
        loop {
            let id = SessionId(RandomState::new().build_hasher().finish());
            if self.last != Some(id) {
                self.last = Some(id);
                return id;
            }
        }
    }
}

/// IDs counting up from a start, for tests.
#[derive(Debug)]
#[cfg(feature = "test-util")]
pub struct SequentialSessionIds(pub u64);

#[cfg(feature = "test-util")]
impl SessionIdSource for SequentialSessionIds {
    fn next_id(&mut self) -> SessionId {
        self.0 += 1;
        SessionId(self.0)
    }
}
