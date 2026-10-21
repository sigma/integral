//! Embeds the plugin's web build (`web/dist-plugin`) in the plugin binary.
//!
//! `just build-web-plugin` produces that directory (`vite build --mode
//! plugin`: no service worker, base `/`). This script writes a sorted table
//! of `(path, bytes)` pairs to `$OUT_DIR/web_assets.rs`, one `include_bytes!`
//! per file, which `src/webview/assets.rs` serves over the `integral://`
//! protocol.
//!
//! Without the directory the table is empty and the editor shows a page
//! saying how to build it, so `cargo build` and clippy need no Node.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

/// The plugin web build, relative to this crate.
const WEB_DIST: &str = "../../web/dist-plugin";

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("set by cargo"));
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("set by cargo"));
    let dist = manifest.join(WEB_DIST);
    println!("cargo:rerun-if-changed={WEB_DIST}");

    let mut files = Vec::new();
    if dist.is_dir() {
        collect(&dist, &dist, &mut files);
    } else {
        println!(
            "cargo:warning=web/dist-plugin is missing: the plugin editor will only say how to \
             build it. Run `just build-web-plugin` first."
        );
    }
    files.sort();

    let mut table = String::from("/// The plugin web build, sorted by path.\n");
    table.push_str("pub(crate) static ASSETS: &[(&str, &[u8])] = &[\n");
    for (rel, abs) in &files {
        writeln!(table, "    ({rel:?}, include_bytes!({abs:?})),").expect("write to String");
    }
    table.push_str("];\n");
    fs::write(out.join("web_assets.rs"), table).expect("write web_assets.rs");
}

/// Every file under `dir`, as (path relative to `root` with `/` separators,
/// absolute path).
fn collect(root: &Path, dir: &Path, files: &mut Vec<(String, String)>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect(root, &path, files);
        } else {
            let rel = path.strip_prefix(root).expect("under root");
            let rel = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            let abs = path
                .canonicalize()
                .unwrap_or_else(|e| panic!("canonicalize {}: {e}", path.display()));
            files.push((rel, abs.to_string_lossy().into_owned()));
        }
    }
}
