//! The `kit-binding` scenario: the side a Kit plugin is not bound to
//! writes nothing, though a host can still automate it (steps 2 and 3 of
//! check 9 in the live runbook — the checks Live cannot reach, because it
//! lists only the parameters touched in Configure and each editor hides
//! the other side's controls).
//!
//! A PCM Drum and an SN Drum instance are each bound to the whole kit: a
//! host edit of a key parameter writes nothing, while one of a kit-wide
//! parameter does (the control that the instance is live). The same
//! instance is then bound to one key, as the host's Key parameter binds
//! it: a kit-wide edit writes nothing, and a key edit writes at that key's
//! address.
//!
//! The fake device is given a kit of each type on the Part. A real device
//! has to hold one there already, or the run is refused; the rig puts back
//! the blocks it found, the bound key's included.

use std::path::Path;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use integral_core::address::{Address, DataSize};
use integral_core::host_params::{Anchor, Binding, HostParam, PluginKind};
use integral_core::params::TONE_SELECTION_SIZE;
use serde_json::{Map, Value, json};

use crate::family::{self, Bank, Params};
use crate::host;
use crate::report::Report;
use crate::rig::{Device, Rig, Write, bytes_of_address};

/// The Kit plugins, whose lists have the two sides.
const KINDS: [PluginKind; 2] = [PluginKind::PcmDrum, PluginKind::SnDrum];
/// The Part the instances are bound to.
const PART: u8 = 0;
/// The key the key-bound half binds to: within both kit types' range, as
/// the device spec's `pcm_drum_keys` and `sn_drum_keys` say.
const KEY: u8 = 36;
/// The program of the kits the fake device holds.
const PROGRAM: u8 = 0;
/// The name of the fake device's kits.
const TONE_NAME: &str = "Init Tone";
/// Blocks the host processes after it changes a parameter, which is when
/// the plugin would send the change.
const PROCESSED: usize = 8;
/// The silence that ends a read or a write.
const QUIET: Duration = Duration::from_secs(1);
/// The longest a wait may take.
const LIMIT: Duration = Duration::from_secs(60);
/// How long to watch for a write that must not come.
const WATCH: Duration = Duration::from_millis(2000);

/// Runs the scenario; `Ok(true)` when every assertion held.
pub fn run(bundle: &Path, device: &Device, report: &Arc<Report>) -> Result<bool> {
    let mut ok = true;
    for kind in KINDS {
        ok &= one(bundle, device, kind, report)
            .with_context(|| format!("the {} instance", kind.name()))?;
    }
    Ok(ok)
}

/// Runs the scenario for one Kit plugin.
fn one(bundle: &Path, device: &Device, kind: PluginKind, report: &Report) -> Result<bool> {
    let (kit, bound_key) = (Binding::of_part(PART), Binding::of_part(PART).with_key(KEY));
    let mut touched = family::tone_blocks(kind, PART)?;
    // The key-bound half writes in the bound key's block, which
    // `tone_read_blocks` leaves out of a kit's read-back.
    touched.blocks.push(key_block(kind, &bound_key)?);
    let mut rig = Rig::start(device, touched, report)?;
    if let Some(memory) = rig.memory() {
        family::hold_defaults(memory, kind, &kit);
        family::hold_defaults(memory, kind, &bound_key);
        family::hold_tone(memory, kind, PART, Bank::Preset, PROGRAM, TONE_NAME)?;
    }
    rig.bring_up()?;
    // The precondition: the Part holds the plugin's own type (a kit).
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
    let Some((kit_wide, key)) = params.kit_sides(&bound_key) else {
        return Ok(report.check(
            "a Kit plugin has a parameter of each side",
            false,
            json!({ "plugin": kind.name() }),
        ));
    };
    let mut ok = report.check(
        "a Kit plugin has a parameter of each side",
        true,
        json!({ "plugin": kind.name(), "kit-wide": params.host(kit_wide).name,
                "key": params.host(key).name }),
    );
    let held_wide = device_value(&rig, &params, kit_wide, &kit);
    let held_key = device_value(&rig, &params, key, &bound_key);

    // Bound to the whole kit: a key parameter writes nothing.
    rig.clear();
    edit(&params, &mut plugin, key, held_key)?;
    sleep(WATCH);
    ok &= no_dt1(
        &rig,
        report,
        "kit-bound key",
        "a kit-bound instance writes nothing for a key parameter",
        json!({ "plugin": kind.name(), "parameter": params.host(key).name }),
    );

    // A kit-wide parameter still writes: the control that the instance is
    // live, and only the key's side is inert.
    rig.clear();
    let value = edit(&params, &mut plugin, kit_wide, held_wide)?;
    let writes = rig
        .wait_quiet((0, 1), QUIET, LIMIT)
        .context("the kit-wide edit reaching the device")?;
    rig.report_writes(report, "kit-bound kit-wide", &writes);
    let at = params.host(kit_wide).device_address(&kit)?;
    ok &= report.check(
        "a kit-bound instance writes a kit-wide edit",
        one_dt1_of(&writes, &params, kit_wide, &kit, value),
        json!({ "plugin": kind.name(), "parameter": params.host(kit_wide).name,
                "address": bytes_of_address(&at), "value": value, "dt1s": writes.len() }),
    );

    // The same instance bound to one key, as the host's Key parameter binds
    // it: the rebind reads the key's block.
    rig.clear();
    family::bind_as_host(&mut plugin, kind, &bound_key)?;
    family::process_blocks(&mut plugin, PROCESSED)?;
    rig.wait_quiet((1, 0), QUIET, LIMIT)
        .context("the instance reading the bound key")?;
    let state = plugin
        .save_state()
        .map_err(|e| anyhow!("saving state: {e}"))?;
    let binding = family::state_fields(&state)?.get("binding").cloned();
    report.emit(
        "bound",
        json!({ "plugin": kind.name(), "binding": binding, "key": KEY }),
    );
    ok &= report.check(
        "the Key parameter binds the instance to the key",
        binding
            .as_deref()
            .is_some_and(|text| bound_to(text, PART, KEY)),
        json!({ "plugin": kind.name(), "binding": binding, "key": KEY }),
    );

    // Bound to the key: a kit-wide parameter writes nothing.
    rig.clear();
    edit(&params, &mut plugin, kit_wide, Some(value))?;
    sleep(WATCH);
    ok &= no_dt1(
        &rig,
        report,
        "key-bound kit-wide",
        "a key-bound instance writes nothing for a kit-wide parameter",
        json!({ "plugin": kind.name(), "parameter": params.host(kit_wide).name }),
    );

    // The key's own parameter writes, at the bound key's address.
    rig.clear();
    let value = edit(&params, &mut plugin, key, held_key)?;
    let writes = rig
        .wait_quiet((0, 1), QUIET, LIMIT)
        .context("the key edit reaching the device")?;
    rig.report_writes(report, "key-bound key", &writes);
    let at = params.host(key).device_address(&bound_key)?;
    ok &= report.check(
        "a key-bound instance writes a key edit at the key's address",
        one_dt1_of(&writes, &params, key, &bound_key, value),
        json!({ "plugin": kind.name(), "parameter": params.host(key).name,
                "address": bytes_of_address(&at), "value": value, "dt1s": writes.len() }),
    );
    Ok(ok)
}

/// Sets `index` to a value the host does not hold — and, where the device's
/// is known, one the device does not hold either, so the service does not
/// take the write for one it need not send — and processes blocks: when the
/// plugin would send the change. The value it set.
///
/// # Errors
///
/// The plugin would not list its parameters, refused the value or failed
/// to process.
fn edit(
    params: &Params,
    plugin: &mut vst3_host::Plugin,
    index: usize,
    device: Option<i32>,
) -> Result<i32> {
    let now = params.values(plugin)?[index];
    let value = away_from(params.host(index), now, device);
    params.set(plugin, index, value)?;
    family::process_blocks(plugin, PROCESSED)?;
    Ok(value)
}

/// A value of `host` other than `now`, and other than `device`'s where it
/// is known: an end of the range that is neither, or the middle when `now`
/// and `device` hold the two ends between them (the sides this scenario
/// edits have at least three values).
fn away_from(host: &HostParam, now: i32, device: Option<i32>) -> i32 {
    if let Some(value) = [host.min, host.max]
        .into_iter()
        .find(|v| *v != now && device.is_none_or(|held| *v != held))
    {
        return value;
    }
    (host.min + host.max) / 2
}

/// The value the device held at parameter `index`'s address for `binding`
/// when the link came up, where the rig read the bytes; the plugin's own
/// writes are the only ones after, and each sets what it sets.
fn device_value(rig: &Rig, params: &Params, index: usize, binding: &Binding) -> Option<i32> {
    let host = params.host(index);
    let at = host.device_address(binding).ok()?;
    host.decode(&rig.held_at(at, host.width)?)
}

/// Reports the DT1s the rig has seen under `phase`, and checks there are
/// none: the inert side wrote nothing. `detail` says what was edited.
fn no_dt1(rig: &Rig, report: &Report, phase: &str, name: &str, detail: Value) -> bool {
    let writes = rig.writes();
    rig.report_writes(report, phase, &writes);
    let mut line = match detail {
        Value::Object(fields) => fields,
        _ => Map::new(),
    };
    line.insert("dt1s".into(), json!(writes.len()));
    report.check(name, writes.is_empty(), Value::Object(line))
}

/// Whether `writes` is exactly one DT1 of parameter `index` for `binding`,
/// holding `value`.
fn one_dt1_of(
    writes: &[Write],
    params: &Params,
    index: usize,
    binding: &Binding,
    value: i32,
) -> bool {
    let host = params.host(index);
    host.device_address(binding).is_ok_and(|at| {
        writes.len() == 1 && writes[0].address == at && host.decode(&writes[0].data) == Some(value)
    })
}

/// The run of the bound key's block its parameters lie in, to read and
/// write back as part of the tone's: the smallest block that covers every
/// key parameter's bytes.
///
/// # Errors
///
/// The plugin of `kind` has no key parameter under `binding`, or the run
/// does not fit the device's addresses.
fn key_block(kind: PluginKind, binding: &Binding) -> Result<(Address, DataSize)> {
    let mut span: Option<(u32, u32)> = None;
    for host in kind.host_params() {
        let Anchor::Key(_) = host.anchor else {
            continue;
        };
        let at = host.device_address(binding)?;
        let (start, end) = (at.value(), at.value() + host.width as u32);
        span = Some(match span {
            None => (start, end),
            Some((s, e)) => (s.min(start), e.max(end)),
        });
    }
    let (start, end) = span.ok_or_else(|| anyhow!("{} has no key parameter", kind.name()))?;
    Ok((
        Address::from_value(start).ok_or_else(|| anyhow!("the key's block is out of range"))?,
        DataSize::from_value(end - start)
            .ok_or_else(|| anyhow!("the key's block is too long for the device"))?,
    ))
}

/// Whether `text`, the Binding as the plugin saves it, is Part `part`
/// bound to key `key`.
fn bound_to(text: &str, part: u8, key: u8) -> bool {
    let Ok(saved) = serde_json::from_str::<Value>(text) else {
        return false;
    };
    saved.get("part") == Some(&json!(part)) && saved.get("key") == Some(&json!(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The saved Binding names the Part and the key as JSON numbers, or is
    /// not a binding of that key.
    #[test]
    fn a_saved_binding_names_its_part_and_key() {
        assert!(bound_to(r#"{"part":0,"key":36}"#, 0, 36));
        assert!(!bound_to(r#"{"part":0}"#, 0, 36));
        assert!(!bound_to(r#"{"part":1,"key":36}"#, 0, 36));
        assert!(!bound_to("null", 0, 36));
        assert!(!bound_to("not json", 0, 36));
    }

    /// The value an edit lands on is one neither the host nor the device
    /// holds, wherever they hold the range's ends between them.
    #[test]
    fn an_edit_lands_on_a_value_neither_the_host_nor_the_device_holds() {
        let host = PluginKind::PcmDrum
            .host_params()
            .into_iter()
            .find(|h| h.max - h.min >= 2)
            .expect("a parameter with three values");
        for now in [host.min, host.max] {
            let device = [host.min, host.max]
                .into_iter()
                .find(|v| *v != now)
                .expect("the other end");
            let value = away_from(&host, now, Some(device));
            assert!(value != now && value != device, "{value}");
        }
        assert_ne!(away_from(&host, host.min, None), host.min);
    }

    /// The key's block covers every key parameter of both kits, and nothing
    /// of the kit-wide parameters.
    #[test]
    fn the_key_block_covers_the_key_parameters_alone() {
        for kind in KINDS {
            let binding = Binding::of_part(PART).with_key(KEY);
            let (at, size) = key_block(kind, &binding).expect("a key block");
            let (start, end) = (at.value(), at.value() + size.value());
            for host in kind.host_params() {
                let covered = |h: &HostParam| {
                    h.device_address(&binding)
                        .is_ok_and(|a| (start..end).contains(&a.value()))
                };
                match host.anchor {
                    Anchor::Key(_) => assert!(covered(&host), "{} {}", kind.name(), host.name),
                    _ => assert!(!covered(&host), "{} {}", kind.name(), host.name),
                }
            }
        }
    }
}
