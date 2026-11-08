//! Window-only screenshots. Never the full screen: it can catch unrelated
//! private content.

use std::path::Path;
use std::process::Command;

/// Captures the window with the given window-server id to `path` (PNG,
/// without the shadow). Returns whether a file was written.
pub fn window(id: u32, path: &Path) -> bool {
    Command::new("/usr/sbin/screencapture")
        .args(["-x", "-o", "-l"])
        .arg(id.to_string())
        .arg(path)
        .status()
        .is_ok_and(|status| status.success())
        && path.exists()
}
