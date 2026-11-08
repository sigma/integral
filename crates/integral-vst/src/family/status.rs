//! The one status an instance reports about its connection.

use std::sync::atomic::{AtomicU8, Ordering};

use serde::Serialize;

/// What an instance can say about its connection to the Device service and,
/// through it, the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[repr(u8)]
pub enum Status {
    /// The service is not running (or not reachable); the instance keeps
    /// retrying. Where every instance starts.
    ServiceNotRunning = 0,
    /// The service is there but its link to the device is down.
    DeviceOffline = 1,
    /// The service is there and the device answers.
    Connected = 2,
    /// The service speaks another protocol version, or is older than the
    /// requests the plugin needs (it refused `Bind` or `Claim` as malformed).
    /// The client does not retry: the service or the plugin has to be
    /// updated.
    ServiceIncompatible = 3,
}

impl Status {
    /// The text the editor shows.
    pub const fn text(self) -> &'static str {
        match self {
            Self::ServiceNotRunning => "Device service not running",
            Self::DeviceOffline => "Device offline",
            Self::Connected => "Connected",
            Self::ServiceIncompatible => "Device service version mismatch",
        }
    }

    fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::DeviceOffline,
            2 => Self::Connected,
            3 => Self::ServiceIncompatible,
            _ => Self::ServiceNotRunning,
        }
    }
}

/// A [`Status`] that one thread sets and any other reads, wait-free.
#[derive(Debug, Default)]
pub struct StatusCell(AtomicU8);

impl StatusCell {
    /// The current status.
    pub fn get(&self) -> Status {
        Status::from_u8(self.0.load(Ordering::Acquire))
    }

    /// Report `status`.
    pub fn set(&self, status: Status) {
        self.0.store(status as u8, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_cell_says_the_service_is_not_running() {
        assert_eq!(StatusCell::default().get(), Status::ServiceNotRunning);
    }

    #[test]
    fn every_status_reads_back() {
        let cell = StatusCell::default();
        for status in [
            Status::ServiceNotRunning,
            Status::DeviceOffline,
            Status::Connected,
            Status::ServiceIncompatible,
        ] {
            cell.set(status);
            assert_eq!(cell.get(), status);
        }
    }
}
