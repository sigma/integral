//! Rewrites the checked-in snapshots of the host parameter lists
//! (`tests/snapshots/host_params`). Run it with `just regen-host-params`
//! after an intended change and review the diff: a changed ID or range
//! moves saved automation (ADR 0008).

use std::fs;
use std::path::PathBuf;

use integral_core::host_params::{PluginKind, snapshot};

fn main() -> std::io::Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/host_params");
    fs::create_dir_all(&dir)?;
    for kind in PluginKind::ALL {
        fs::write(
            dir.join(format!("{kind:?}.txt")),
            snapshot(&kind.host_params()),
        )?;
    }
    Ok(())
}
