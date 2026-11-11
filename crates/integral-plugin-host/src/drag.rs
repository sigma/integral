//! The `drag` scenario: open the editor, drag a slider with real pointer
//! events, and check the plugin reported balanced `beginEdit` /
//! `performEdit` / `endEdit` on the matching host parameter, and nothing
//! else.
//!
//! The host runs the Device service itself, as the other scenarios do: with
//! no service to connect to, the editor shows only its connection panel and
//! there is no slider to drag.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow, bail};
use integral_core::host_params::PluginKind;
use serde_json::json;
use vst3_host::ParameterEdit;

use crate::report::Report;
use crate::rig::{Device, Rig};
use crate::{driver, family, gesture, host};

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
    /// The plugin class to load (by name); the Studio Set if none.
    pub class: Option<String>,
    /// Where screenshots go.
    pub out_dir: PathBuf,
}

/// The side of the slider away from `value`, so a drag there must move it.
fn far_side(value: f64) -> f64 {
    if value >= 0.5 { 0.1 } else { 0.9 }
}

/// Runs the scenario; `Ok(true)` when every assertion held.
pub fn run(
    bundle: &Path,
    device: &Device,
    options: &Options,
    report: &Arc<Report>,
) -> Result<bool> {
    let kind = PluginKind::StudioSet;
    // shortcut: Studio Set only; a Tone or Kit class needs its Part to hold a
    // tone of its type first (see `recall`), add that when a drag needs one.
    if options.class.as_deref().is_some_and(|c| c != kind.name()) {
        bail!("drag drives the {} only", kind.name());
    }
    // Held until the editor is closed: the plugin talks to it throughout.
    let mut rig = Rig::start(device, family::studio_set_touched(), report)?;
    rig.bring_up()?;
    let plugin = host::load(bundle, Some(kind.name()))?;
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

    let mut edits: Vec<ParameterEdit> = Vec::new();
    let (mut host, session) = driver::open_editor(plugin, report, &options.out_dir, &mut edits)?;
    let control = options
        .control
        .clone()
        .unwrap_or_else(|| options.param.clone());
    let to = options.to.unwrap_or_else(|| far_side(before));
    let (result, driven) = driver::drive(&host, report, &mut edits, move || {
        session.drag_slider(&control, to)
    });
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
