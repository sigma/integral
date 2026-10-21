//! The plugin's editor runs the web UI in a webview (ADR 0004).
//!
//! - [`assets`]: the embedded web build and how each path is served.
//! - [`ipc`]: the page ↔ plugin message codec.
//! - [`link`]: the device link behind the plugin transport, the seam the
//!   Device service of ADR 0006 replaces.
//! - [`bridge`]: page messages to link calls, and back.
//! - `editor` (macOS): the nih-plug editor, a wry webview in a baseview
//!   window.

pub(crate) mod assets;
pub(crate) mod bridge;
#[cfg(target_os = "macos")]
pub(crate) mod editor;
pub(crate) mod ipc;
pub(crate) mod link;
