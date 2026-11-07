//! The `binding-mismatch` scenario (ADR 0007): an instance bound to a Part
//! that comes to hold another tone type shows the mismatch and writes nothing.
//!
//! A PCM Synth plugin is bound to a Part that holds a PCM Synth tone. Another
//! writer then selects an SN Synth tone on the Part, as a raw client of the
//! service would (the device's front panel, the web app).
//!
//! Two parts:
//!
//! - **Headless**, through the service's link: as a control, the instance
//!   writes a host edit while the Part holds its own type. After the other
//!   writer's selection it writes nothing for a host edit, even though the
//!   host is processing.
//! - **The editor**, through Accessibility: the editor shows the text of
//!   [`Notice::TypeMismatch`]. It opens a window, so `--no-editor` skips it.
//!
//! The fake device is given a PCM Synth tone on the Part. A real device has
//! to hold one there already, or the run is refused; the rig puts back the
//! tone it found.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use integral_core::host_params::{Binding, PluginKind};
use integral_core::notice::Notice;
use integral_core::params::TONE_SELECTION_SIZE;
use serde_json::json;
use vst3_host::ParameterEdit;

use crate::family::{self, Bank, Params};
use crate::report::Report;
use crate::rig::{Device, Rig};
use crate::{driver, host};

/// The Part the plugin is bound to.
const PART: u8 = 0;
/// The program of the tones the fake device and the other writer use.
const PROGRAM: u8 = 0;
/// The name of the fake device's tone.
const TONE_NAME: &str = "Init Tone";
/// Blocks the host processes after it changes a parameter, which is when the
/// plugin would send the change.
const PROCESSED: usize = 8;
/// The silence that ends a read or a write.
const QUIET: Duration = Duration::from_secs(1);
/// The longest a wait may take.
const LIMIT: Duration = Duration::from_secs(60);
/// How long to watch for a write that must not come.
const WATCH: Duration = Duration::from_millis(2000);
/// The screenshot of the editor showing the notice.
const SHOT: &str = "binding-mismatch.png";

/// Scenario inputs.
pub struct Options {
    /// Skip the editor: run the headless assertions alone.
    pub no_editor: bool,
    /// Where the screenshot goes.
    pub out_dir: PathBuf,
}

/// Runs the scenario; `Ok(true)` when every assertion held.
pub fn run(
    bundle: &Path,
    device: &Device,
    options: &Options,
    report: &Arc<Report>,
) -> Result<bool> {
    let (kind, other) = (PluginKind::PcmSynth, PluginKind::SnSynth);
    let binding = Binding::of_part(PART);
    let mut rig = Rig::start(device, family::tone_blocks(kind, PART)?, report)?;
    if let Some(memory) = rig.memory() {
        family::hold_defaults(memory, kind, &binding);
        family::hold_tone(memory, kind, PART, Bank::Preset, PROGRAM, TONE_NAME)?;
    }
    rig.bring_up()?;
    // The precondition: the Part holds the plugin's own type.
    let held = rig.held(
        family::select_address(PART),
        TONE_SELECTION_SIZE.value() as usize,
    )?;
    family::require_type(kind, [held[0], held[1], held[2]])?;

    let class = host::classes(bundle)?
        .into_iter()
        .find(|c| c.name == kind.name())
        .ok_or_else(|| anyhow!("the bundle has no {} class", kind.name()))?;
    let mut plugin = host::load_class(bundle, &class.class_id)?;
    let params = Params::of(kind, &plugin)?;
    family::settle(&mut plugin)?;
    rig.wait_quiet((1, 0), QUIET, LIMIT)
        .context("the instance adopting the device")?;
    let mut ok = true;
    let editable = params.editable(&binding, 1);
    ok &= report.check(
        "the plugin has a parameter to edit",
        !editable.is_empty(),
        json!({ "plugin": kind.name() }),
    );
    let Some(&index) = editable.first() else {
        return Ok(false);
    };

    // The control: on a Part of its own type an edit goes out.
    let now = params.values(&plugin)?[index];
    params.set(&mut plugin, index, params.other_than(index, now))?;
    family::process_blocks(&mut plugin, PROCESSED)?;
    let writes = rig.wait_quiet((1, 1), QUIET, LIMIT)?;
    ok &= report.check(
        "a plugin on a Part of its own type writes an edit",
        writes.len() == 1,
        json!({ "plugin": kind.name(), "dt1s": writes.len() }),
    );

    // Another writer selects a tone of another type on the Part.
    let (msb, lsb) = family::bank(other, Bank::Preset)?;
    rig.write_as_other(&family::select_address(PART), &[msb, lsb, PROGRAM])?;
    rig.wait_quiet((0, 0), 2 * QUIET, LIMIT)?;
    rig.clear();

    // The host edits a parameter (automation, a drag): nothing goes out.
    let now = params.values(&plugin)?[index];
    params.set(&mut plugin, index, params.other_than(index, now))?;
    family::process_blocks(&mut plugin, PROCESSED)?;
    sleep(WATCH);
    let writes = rig.writes();
    rig.report_writes(report, "mismatch", &writes);
    ok &= report.check(
        "a host edit on the mismatched instance writes nothing",
        writes.is_empty(),
        json!({ "plugin": kind.name(), "dt1s": writes.len() }),
    );

    if options.no_editor {
        report.emit("editor", json!({ "skipped": "--no-editor" }));
        return Ok(ok);
    }

    // The editor shows the mismatch.
    let mut edits: Vec<ParameterEdit> = Vec::new();
    let (mut editor, session) = driver::open_editor(plugin, report, &options.out_dir, &mut edits)?;
    let text = Notice::TypeMismatch.text();
    let (pumped, shown) = driver::drive(&editor, report, &mut edits, move || {
        session.wait_for_text(text, SHOT)
    });
    editor.close();
    pumped?;
    // That it did not show is a failed assertion; failing to look is an abort.
    ok &= report.check(
        "the editor shows the mismatch",
        shown?,
        json!({ "text": text }),
    );
    ok &= report.check(
        "opening the editor writes nothing either",
        rig.writes().is_empty(),
        json!({ "dt1s": rig.writes().len() }),
    );
    Ok(ok)
}
