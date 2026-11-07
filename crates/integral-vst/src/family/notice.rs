//! What an instance can tell its editor besides its connection status.
//!
//! The editor shows each notice the instance reports and hides it when the
//! instance stops reporting it. Nothing sets one yet: the recall (#458)
//! reports [`Notice::Recalling`], and the mismatch and duplicate checks
//! (#455) the other two.

use std::sync::atomic::{AtomicU8, Ordering};

use serde::Serialize;

/// Something the editor shows beside the status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Notice {
    /// The project's state is being pushed to the device (ADR 0009).
    Recalling,
    /// The bound Part holds another tone type than the plugin's: the plugin
    /// stays inert (ADR 0007).
    TypeMismatch,
    /// Another instance is bound to the same target.
    Duplicate,
}

impl Notice {
    /// Every notice, in the order the editor lists them.
    pub const ALL: [Self; 3] = [Self::Recalling, Self::TypeMismatch, Self::Duplicate];

    /// The text the editor shows.
    pub const fn text(self) -> &'static str {
        match self {
            Self::Recalling => "Recalling\u{2026}",
            Self::TypeMismatch => "The bound Part holds another tone type",
            Self::Duplicate => "Another instance is bound to this target",
        }
    }

    const fn bit(self) -> u8 {
        1 << self as u8
    }
}

/// The notices an instance reports, which one thread sets and any other
/// reads, wait-free.
#[derive(Debug, Default)]
pub struct NoticeCell(AtomicU8);

impl NoticeCell {
    /// Report `notice` while `on`, and stop reporting it otherwise.
    pub fn set(&self, notice: Notice, on: bool) {
        if on {
            self.0.fetch_or(notice.bit(), Ordering::AcqRel);
        } else {
            self.0.fetch_and(!notice.bit(), Ordering::AcqRel);
        }
    }

    /// The notices being reported, in [`Notice::ALL`] order.
    pub fn get(&self) -> Vec<Notice> {
        let bits = self.0.load(Ordering::Acquire);
        Notice::ALL
            .into_iter()
            .filter(|notice| bits & notice.bit() != 0)
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
