//! The `drag` scenario: open the editor, drag a slider with real pointer
//! events, and check the plugin reported balanced `beginEdit` /
//! `performEdit` / `endEdit` on the matching host parameter, and nothing
//! else.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use serde_json::json;
use vst3_host::ParameterEdit;

use crate::driver::{self, Session};
use crate::report::Report;
use crate::{gesture, host};

/// How many parameters to list: enough to see the plugin's shape without a
/// line per parameter when a plugin declares thousands.
const LISTED_PARAMETERS: usize = 16;

/// Scenario inputs.
pub struct Options {
    /// Host parameter name that the dragged control must move.
    pub param: String,
    /// Accessibility label of the control (exact, else substring); defaults
    /// to `param`.
    pub control: Option<String>,
    /// Where on the slider to release, as a fraction of its width. Default:
    /// the far side of the parameter's current value, so it always moves.
    pub to: Option<f64>,
    /// The plugin class to load (by name); the bundle's first if none.
    pub class: Option<String>,
    /// Where screenshots go.
    pub out_dir: PathBuf,
}

/// The side of the slider away from `value`, so a drag there must move it.
fn far_side(value: f64) -> f64 {
    if value >= 0.5 { 0.1 } else { 0.9 }
}

/// Runs the scenario; `Ok(true)` when every assertion held.
pub fn run(bundle: &Path, options: &Options, report: &Arc<Report>) -> Result<bool> {
    let plugin = host::load(bundle, options.class.as_deref())?;
    let params = plugin
        .get_parameters()
        .map_err(|e| anyhow!("listing parameters: {e}"))?;
    report.emit(
        "plugin",
        json!({ "bundle": bundle.display().to_string(), "parameters": params.len() }),
    );
    for p in params.iter().take(LISTED_PARAMETERS) {
        report.emit(
            "parameter",
            json!({ "id": p.id, "name": p.name, "value": p.value }),
        );
    }
    // Fail before opening any window when there is nothing to drag.
    let Some(target) = params.iter().find(|p| p.name == options.param) else {
        report.check(
            "plugin declares the host parameter",
            false,
            json!({ "wanted": options.param, "declared": params.len() }),
        );
        return Ok(false);
    };
    let (target_id, before) = (target.id, target.value);
    // Integer and list parameters quantise: allow half a step.
    let tolerance = match target.step_count {
        n if n > 0 => 0.5 / f64::from(n),
        _ => 1e-6,
    };
    report.check(
        "plugin declares the host parameter",
        true,
        json!({ "name": options.param, "id": target_id }),
    );

    std::fs::create_dir_all(&options.out_dir)?;
    let mut host = host::Host::open(plugin)?;
    let mut edits: Vec<ParameterEdit> = Vec::new();

    // Let the window show and come to the front. Edits the plugin reports
    // while loading count: a spurious one must fail the run.
    let mut until = Instant::now() + Duration::from_millis(500);
    host.run(
        || Instant::now() < until,
        Duration::ZERO,
        |e| driver::record_edit(report, &mut edits, e),
    )?;
    host.activate();
    until = Instant::now() + Duration::from_millis(500);
    host.run(
        || Instant::now() < until,
        Duration::ZERO,
        |e| driver::record_edit(report, &mut edits, e),
    )?;

    let (window_id, via) = host.editor_window_id()?;
    let session = Session {
        pid: i32::try_from(std::process::id())?,
        window_id,
        via,
        shared: host.shared(),
        report: Arc::clone(report),
        out_dir: options.out_dir.clone(),
    };
    let control = options
        .control
        .clone()
        .unwrap_or_else(|| options.param.clone());
    let to = options.to.unwrap_or_else(|| far_side(before));
    let worker = thread::spawn(move || session.drag_slider(&control, to));

    let result = host.run(
        || !worker.is_finished(),
        Duration::from_millis(600),
        |e| driver::record_edit(report, &mut edits, e),
    );
    let driven = worker.join().map_err(|_| anyhow!("the driver panicked"))?;
    let after = host.plugin().get_parameter(target_id);
    host.close();
    result?;
    let driven = driven?;

    report.emit(
        "driven",
        json!({ "window": driven.window.id, "control_frame": driver::rect(driven.frame) }),
    );
    let mut ok = true;
    match gesture::check(&edits, target_id) {
        Ok(s) => {
            ok &= report.check(
                "balanced beginEdit/performEdit/endEdit on the dragged parameter",
                true,
                json!({ "id": target_id, "begins": s.begins, "changes": s.changes, "ends": s.ends }),
            );
            let value = after.as_ref().ok().copied();
            let moved = value.is_some_and(|v| (v - before).abs() > tolerance);
            let agrees =
                matches!((value, s.last_value), (Some(v), Some(l)) if (v - l).abs() <= tolerance);
            ok &= report.check(
                "parameter value moved and matches the last performEdit",
                moved && agrees,
                json!({ "before": before, "after": value, "last_performed": s.last_value, "tolerance": tolerance }),
            );
        }
        Err(why) => {
            ok &= report.check(
                "balanced beginEdit/performEdit/endEdit on the dragged parameter",
                false,
                json!({ "id": target_id, "why": why, "edits": edits.len() }),
            );
        }
    }
    Ok(ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_target_is_on_the_far_side_of_the_value() {
        assert!(far_side(0.8) < 0.5);
        assert!(far_side(0.2) > 0.5);
        assert!(far_side(0.5) < 0.5);
    }
}
