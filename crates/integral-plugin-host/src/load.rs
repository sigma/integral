//! The `load` scenario: load every plugin class of a bundle without opening
//! an editor, and check it against the parameter list core declares for it.
//!
//! A class is paired with its kind by name (`PluginKind::name`), and checked
//! against it: the same number of parameters, with the same names, in the
//! same order. A class no kind names, or a kind with no class, fails. Each class's state then
//! round-trips into a fresh instance (what a host does when it saves and
//! reopens a project). Load, save and reopen are timed, which is what the
//! largest list (PCM Synth, about 1,400 parameters) is checked against in
//! the Live runbook.

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Result, anyhow};
use integral_core::host_params::PluginKind;
use serde_json::json;

use crate::family::declared_names;
use crate::host;
use crate::report::Report;

/// Runs the scenario over every class, or only the one named `only`;
/// `Ok(true)` when every assertion held.
pub fn run(bundle: &Path, only: Option<&str>, report: &Arc<Report>) -> Result<bool> {
    let classes = host::classes(bundle)?;
    report.emit(
        "bundle",
        json!({
            "bundle": bundle.display().to_string(),
            "classes": classes.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        }),
    );
    let mut ok = true;
    for kind in PluginKind::ALL {
        ok &= report.check(
            "the bundle exports the plugin",
            classes.iter().filter(|c| c.name == kind.name()).count() == 1,
            json!({ "plugin": kind.name() }),
        );
    }
    for class in &classes {
        let Some(kind) = PluginKind::ALL.into_iter().find(|k| k.name() == class.name) else {
            ok &= report.check(
                "every class is a plugin of the family",
                false,
                json!({ "class": class.name }),
            );
            continue;
        };
        if only.is_some_and(|name| name != class.name) {
            continue;
        }
        ok &= check_class(bundle, &class.name, &class.class_id, kind, report)?;
    }
    Ok(ok)
}

/// Load, list, save and reopen one class.
fn check_class(
    bundle: &Path,
    name: &str,
    class_id: &str,
    kind: PluginKind,
    report: &Report,
) -> Result<bool> {
    let started = Instant::now();
    let plugin = host::load_class(bundle, class_id)?;
    let load_ms = millis(started);

    let params = plugin
        .get_parameters()
        .map_err(|e| anyhow!("listing {name}'s parameters: {e}"))?;
    let wanted = declared_names(kind);
    let mut ok = report.check(
        "the plugin declares its list",
        params.len() == wanted.len() && params.iter().zip(&wanted).all(|(p, w)| p.name == *w),
        json!({
            "plugin": name, "kind": format!("{kind:?}"),
            "declared": params.len(), "wanted": wanted.len(), "load_ms": load_ms,
        }),
    );

    let started = Instant::now();
    let saved = plugin
        .save_state()
        .map_err(|e| anyhow!("saving {name}'s state: {e}"))?;
    let save_ms = millis(started);

    let started = Instant::now();
    let mut fresh = host::load_class(bundle, class_id)?;
    fresh
        .load_state(&saved)
        .map_err(|e| anyhow!("restoring {name}'s state: {e}"))?;
    let reopen_ms = millis(started);
    let resaved = fresh
        .save_state()
        .map_err(|e| anyhow!("saving {name}'s restored state: {e}"))?;
    ok &= report.check(
        "the state reopens unchanged",
        resaved == saved,
        json!({
            "plugin": name, "bytes": saved.len(),
            "save_ms": save_ms, "reopen_ms": reopen_ms,
        }),
    );
    Ok(ok)
}

fn millis(since: Instant) -> f64 {
    (since.elapsed().as_secs_f64() * 1e4).round() / 10.0
}
