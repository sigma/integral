//! What an instance can tell its editor besides its connection status.
//!
//! The editor shows each notice the instance reports and hides it when the
//! instance stops reporting it. The recall
//! ([`recall`](super::recall)) reports [`Notice::Recalling`]; [`Instance::set_fit`](super::Instance::set_fit)
//! reports [`Notice::TypeMismatch`] and the connection
//! ([`Tracker`](super::link::Tracker)) [`Notice::Duplicate`].

use std::sync::atomic::{AtomicU8, Ordering};

pub use integral_core::notice::Notice;

/// The bit that stands for `notice` in a set of them.
const fn bit(notice: Notice) -> u8 {
    1 << notice as u8
}

/// The notices an instance reports, which one thread sets and any other
/// reads, wait-free.
#[derive(Debug, Default)]
pub struct NoticeCell(AtomicU8);

impl NoticeCell {
    /// Report `notice` while `on`, and stop reporting it otherwise.
    pub fn set(&self, notice: Notice, on: bool) {
        if on {
            self.0.fetch_or(bit(notice), Ordering::AcqRel);
        } else {
            self.0.fetch_and(!bit(notice), Ordering::AcqRel);
        }
    }

    /// The notices being reported, in [`Notice::ALL`] order.
    pub fn get(&self) -> Vec<Notice> {
        let bits = self.0.load(Ordering::Acquire);
        Notice::ALL
            .into_iter()
            .filter(|notice| bits & bit(*notice) != 0)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_cell_reports_nothing() {
        assert_eq!(NoticeCell::default().get(), []);
    }

    #[test]
    fn notices_are_set_and_cleared_one_by_one_in_listing_order() {
        let cell = NoticeCell::default();
        cell.set(Notice::Duplicate, true);
        cell.set(Notice::Recalling, true);
        assert_eq!(cell.get(), [Notice::Recalling, Notice::Duplicate]);
        cell.set(Notice::Recalling, false);
        cell.set(Notice::TypeMismatch, false);
        assert_eq!(cell.get(), [Notice::Duplicate]);
    }

    #[test]
    fn notices_serialize_in_camel_case_with_their_text() {
        let json = serde_json::to_string(&Notice::TypeMismatch).unwrap();
        assert_eq!(json, r#""typeMismatch""#);
        assert!(Notice::ALL.iter().all(|n| !n.text().is_empty()));
    }
}
