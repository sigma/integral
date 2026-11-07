//! What a plugin instance can tell its editor besides its connection status.
//!
//! The editor shows each notice the instance reports and hides it when the
//! instance stops reporting it. It lives here, not in the plugin, so that the
//! test host (`integral-plugin-host`) can look for the very text the editor
//! shows.

/// Something the editor shows beside the status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
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
            Self::TypeMismatch => {
                "The bound Part holds another tone type, so this plugin does nothing until it holds one of its own"
            }
            Self::Duplicate => {
                "Another instance of this plugin is bound to this target: both work, and the last to write wins"
            }
        }
    }
}
