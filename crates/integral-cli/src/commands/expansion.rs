//! `expansion` commands: list the Expansions and load them into the
//! Expansion slots A–D.
//!
//! The catalog, slot staging, load plan and load state machine all live in
//! `integral-core`; this module parses arguments, talks MIDI and prints.

use std::io::{IsTerminal, Write};
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use clap::ValueEnum;
use integral_core::address::DataSize;
use integral_core::device_spec::INTEGRA7;
use integral_core::expansion_load::{self, LoadAction, LoadMachine, LoadOutcome, LoadPlan};
use integral_core::expansion_slots::{self, ExpansionSlots, SLOT_COUNT, SLOT_NAMES};
use integral_core::expansions::{self, EXPANSIONS, Expansion};
use integral_core::{params, sysex};
use serde_json::{Value, json};

use crate::device::IntegraDevice;

/// Exit code when a load was sent but failed or timed out.
pub const EXIT_LOAD_FAILED: u8 = 2;

/// Name of an empty Expansion slot, as typed and printed.
pub const OFF_NAME: &str = "OFF";

/// Size of a Part's Tone Bank Select MSB + LSB.
const BANK_SELECT_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x02);

/// Output format of the `expansion` commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text.
    Text,
    /// One JSON document on stdout.
    Json,
}

/// One requested slot change: put `expansion` (or OFF when `None`) in `slot`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Assignment {
    /// Slot index, 0 = A.
    pub slot: usize,
    /// The Expansion to load, or `None` for OFF.
    pub expansion: Option<&'static Expansion>,
}

/// Whether `e` takes every slot (ExPCM).
fn fills_all(e: &Expansion) -> bool {
    usize::from(e.slot_cost) == SLOT_COUNT
}

/// Parse a slot letter (`A`–`D`, case-insensitive) into a slot index.
fn parse_slot(s: &str) -> Result<usize> {
    let mut chars = s.chars();
    let slot = match (chars.next(), chars.next()) {
        (Some(c), None) => SLOT_NAMES.iter().position(|n| n.eq_ignore_ascii_case(&c)),
        _ => None,
    };
    let letters: String = SLOT_NAMES.iter().collect();
    slot.with_context(|| format!("unknown Expansion slot `{s}` (expected one of {letters})"))
}

/// Parse a catalog name or `OFF` (case-insensitive).
fn parse_name(s: &str) -> Result<Option<&'static Expansion>> {
    if s.eq_ignore_ascii_case(OFF_NAME) {
        return Ok(None);
    }
    expansions::by_name(s).map(Some).with_context(|| {
        let names: Vec<&str> = EXPANSIONS.iter().map(|e| e.name).collect();
        format!(
            "unknown Expansion `{s}` (expected {OFF_NAME} or one of {})",
            names.join(", ")
        )
    })
}

/// Parse `expansion load` arguments: `SLOT=NAME` assignments, or a
/// slot-filling Expansion (ExPCM) on its own.
///
/// Returns the assignments sorted by slot.
pub fn parse_assignments(args: &[String]) -> Result<Vec<Assignment>> {
    if args.is_empty() {
        bail!("nothing to load: pass SLOT=NAME assignments (e.g. A=SRX-01) or ExPCM");
    }
    let mut out: Vec<Assignment> = Vec::with_capacity(args.len());
    for arg in args {
        let assignment = match arg.split_once('=') {
            None => {
                let e = parse_name(arg)?
                    .with_context(|| format!("`{arg}` needs a slot, e.g. A={OFF_NAME}"))?;
                if !fills_all(e) {
                    bail!("`{arg}` needs a slot, e.g. A={}", e.name);
                }
                if args.len() > 1 {
                    bail!("{} fills all four slots; pass it alone", e.name);
                }
                Assignment {
                    slot: 0,
                    expansion: Some(e),
                }
            }
            Some((slot, name)) => {
                let slot = parse_slot(slot)?;
                let expansion = parse_name(name)?;
                if let Some(e) = expansion.filter(|e| fills_all(e)) {
                    bail!(
                        "{} fills all four slots; load it on its own: `expansion load {}`",
                        e.name,
                        e.name
                    );
                }
                if out.iter().any(|a| a.slot == slot) {
                    bail!("slot {} is assigned twice", SLOT_NAMES[slot]);
                }
                if let Some(e) = expansion
                    && out.iter().any(|a| a.expansion == expansion)
                {
                    bail!("{} is assigned to two slots", e.name);
                }
                Assignment { slot, expansion }
            }
        };
        out.push(assignment);
    }
    out.sort_by_key(|a| a.slot);
    Ok(out)
}

/// Apply `assignments` to `current`. Unnamed slots keep their Expansion;
/// naming an Expansion already in another slot moves it (old slot → OFF).
pub fn stage(current: &ExpansionSlots, assignments: &[Assignment]) -> ExpansionSlots {
    assignments
        .iter()
        .fold(*current, |slots, a| slots.with(a.slot, a.expansion))
}

/// Display name of one slot's content.
fn slot_name(slot: Option<&Expansion>) -> &str {
    slot.map_or(OFF_NAME, |e| e.name)
}

/// The slot header line, e.g. `A SRX-03  B SRX-05  C SRX-07  D SRX-08`;
/// `?` per slot when unknown.
pub fn slots_line(slots: Option<&ExpansionSlots>) -> String {
    SLOT_NAMES
        .iter()
        .enumerate()
        .map(|(i, letter)| {
            let name = slots.map_or("?", |s| slot_name(s.0[i]));
            format!("{letter} {name}")
        })
        .collect::<Vec<_>>()
        .join("  ")
}

/// Slot letters holding `e`: `-` when not loaded, `A-D` for a run.
fn slot_column(slots: &ExpansionSlots, e: &Expansion) -> String {
    let held: Vec<char> = SLOT_NAMES
        .iter()
        .zip(slots.0)
        .filter(|(_, s)| *s == Some(e))
        .map(|(letter, _)| *letter)
        .collect();
    match held.as_slice() {
        [] => "-".to_string(),
        [one] => one.to_string(),
        [first, .., last] => format!("{first}-{last}"),
    }
}

/// Slots as a JSON object `{"A": "SRX-03", ...}`.
fn slots_json(slots: &ExpansionSlots) -> Value {
    SLOT_NAMES
        .iter()
        .zip(slots.0)
        .map(|(letter, s)| (letter.to_string(), Value::from(slot_name(s))))
        .collect::<serde_json::Map<_, _>>()
        .into()
}

/// The `expansion list` output for `slots` (`None` when unknown).
pub fn render_list(slots: Option<&ExpansionSlots>, format: OutputFormat) -> String {
    match format {
        OutputFormat::Text => {
            const HEADER: &str = "Expansion";
            let width = EXPANSIONS
                .iter()
                .map(|e| e.name.len())
                .fold(HEADER.len(), usize::max);
            let mut out = slots_line(slots);
            out.push_str(&format!("\n\n{HEADER:width$}  Slot\n"));
            for e in &EXPANSIONS {
                let column = slots.map_or("?".to_string(), |s| slot_column(s, e));
                out.push_str(&format!("{:width$}  {column}\n", e.name));
            }
            out
        }
        OutputFormat::Json => {
            let expansions: Vec<Value> = EXPANSIONS
                .iter()
                .map(|e| {
                    let held = slots.map(|s| {
                        SLOT_NAMES
                            .iter()
                            .zip(s.0)
                            .filter(|(_, slot)| *slot == Some(e))
                            .map(|(letter, _)| letter.to_string())
                            .collect::<Vec<_>>()
                    });
                    json!({
                        "name": e.name,
                        "loaded": held.as_ref().map(|h| !h.is_empty()),
                        "slots": held,
                    })
                })
                .collect();
            let doc = json!({
                "slots": slots.map(slots_json),
                "expansions": expansions,
            });
            format!("{doc:#}\n")
        }
    }
}

/// The plan in text: one `A: SRX-03 → SRX-01` line per changed slot, then
/// the Parts going silent.
pub fn render_plan(plan: &LoadPlan) -> String {
    let mut out = String::new();
    for change in &plan.changes {
        out.push_str(&format!(
            "{}: {} → {}\n",
            SLOT_NAMES[change.slot],
            slot_name(change.from),
            slot_name(change.to)
        ));
    }
    if plan.silenced_parts.is_empty() {
        out.push_str("No Part goes silent.\n");
    } else {
        out.push_str(&format!(
            "Parts going silent: {}\n",
            join_parts(&plan.silenced_parts)
        ));
    }
    out
}

fn join_parts(parts: &[u8]) -> String {
    parts
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// The plan as JSON, with the run's `result` and final `slots`.
fn plan_json(
    current: &ExpansionSlots,
    target: &ExpansionSlots,
    plan: &LoadPlan,
    result: &str,
    slots: Option<&ExpansionSlots>,
) -> Value {
    let changes: Vec<Value> = plan
        .changes
        .iter()
        .map(|c| {
            json!({
                "slot": SLOT_NAMES[c.slot].to_string(),
                "from": slot_name(c.from),
                "to": slot_name(c.to),
            })
        })
        .collect();
    json!({
        "current": slots_json(current),
        "target": slots_json(target),
        "changes": changes,
        "silenced_parts": plan.silenced_parts,
        "result": result,
        "slots": slots.map(slots_json),
    })
}

/// Read the Expansion slots.
fn read_slots(dev: &mut IntegraDevice) -> Result<ExpansionSlots> {
    let request = expansion_slots::build_read_request(dev.device_id());
    dev.conn_out_mut()
        .send(&request)
        .context("failed to send the Expansion slot read")?;
    let deadline = Instant::now() + dev.timeout();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let raw = dev.rx().recv_timeout(remaining).map_err(|e| match e {
            RecvTimeoutError::Timeout => anyhow!("no answer to the Expansion slot read"),
            RecvTimeoutError::Disconnected => anyhow!("MIDI input disconnected"),
        })?;
        if let Ok(dt1) = sysex::parse_dt1(&raw)
            && let Some(reply) = expansion_slots::parse_read_reply(&dt1.address, &dt1.data)
        {
            return reply.context("unexpected Expansion slot reply");
        }
    }
}

/// Read every Part's Tone Bank Select MSB/LSB, Part 1 first.
fn read_part_banks(dev: &mut IntegraDevice) -> Result<Vec<(u8, u8)>> {
    (0..INTEGRA7.part_count)
        .map(|part| {
            let addr = params::part_address(part, params::part::TONE_BANK_MSB);
            let data = dev
                .request_data(&addr, &BANK_SELECT_SIZE)
                .with_context(|| format!("could not read Part {}'s tone", part + 1))?;
            match data[..] {
                [msb, lsb, ..] => Ok((msb, lsb)),
                _ => bail!("short tone bank reply for Part {}", part + 1),
            }
        })
        .collect()
}

/// `expansion list`: print the slots, then the catalog with a slot column.
/// The catalog prints even when the device doesn't answer.
pub fn list(
    port_pattern: &str,
    device_id: Option<u8>,
    timeout: Duration,
    format: OutputFormat,
) -> Result<()> {
    let slots = IntegraDevice::connect_detecting(port_pattern, device_id, timeout)
        .and_then(|dev| read_slots(&mut dev.quiet()));
    let slots = match slots {
        Ok(slots) => Some(slots),
        Err(e) => {
            eprintln!("Warning: {e:#}; loaded state unknown");
            None
        }
    };
    print!("{}", render_list(slots.as_ref(), format));
    Ok(())
}

/// Ask for confirmation when Parts would go silent: on a TTY, a y/N prompt;
/// otherwise refuse.
fn confirm_silenced(parts: &[u8]) -> Result<()> {
    let parts = join_parts(parts);
    if !std::io::stdin().is_terminal() {
        bail!("Parts {parts} would go silent; pass --yes to load anyway");
    }
    eprint!("Parts {parts} will go silent. Load anyway? [y/N] ");
    std::io::stderr().flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    match line.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => Ok(()),
        _ => bail!("load cancelled"),
    }
}

/// Drive the core load machine to its outcome. `Err` only when the load
/// request itself couldn't be sent.
fn run_load(
    dev: &mut IntegraDevice,
    current: ExpansionSlots,
    target: ExpansionSlots,
) -> Result<LoadOutcome> {
    let mut machine = LoadMachine::new(dev.device_id(), current, target);
    let start = Instant::now();
    let now_ms = || start.elapsed().as_secs_f64() * 1000.0;
    loop {
        match machine.next_action(now_ms()) {
            LoadAction::Done(outcome) => return Ok(outcome),
            LoadAction::Send(bytes) => {
                dev.conn_out_mut()
                    .send(&bytes)
                    .context("failed to send the Expansion load")?;
                eprintln!("Loading; all sound stops until done...");
            }
            LoadAction::Poll(bytes) => {
                if let Err(e) = dev.conn_out_mut().send(&bytes) {
                    eprintln!("Warning: failed to send slot read: {e}");
                }
            }
            LoadAction::Wait(ms) => {
                let wait = Duration::from_secs_f64(ms.max(0.0) / 1000.0);
                match dev.rx().recv_timeout(wait) {
                    Ok(raw) => {
                        let Ok(dt1) = sysex::parse_dt1(&raw) else {
                            continue;
                        };
                        let now = now_ms();
                        let handled = machine.on_dt1(&dt1.address, &dt1.data, now);
                        let is_slot_read =
                            expansion_slots::parse_read_reply(&dt1.address, &dt1.data).is_some();
                        if handled && !is_slot_read && machine.outcome().is_none() {
                            eprintln!("Load started");
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => {
                        eprintln!("Error: MIDI input disconnected during the load");
                        return Ok(LoadOutcome::TimedOut);
                    }
                }
            }
        }
    }
}

/// `expansion load`: plan, confirm, load, and report.
///
/// Returns the process exit code: 0 on success or no-op, or
/// [`EXIT_LOAD_FAILED`] when the load was sent but failed or timed out.
/// `Err` (exit 1) means nothing was sent.
pub fn load(
    port_pattern: &str,
    device_id: Option<u8>,
    timeout: Duration,
    args: &[String],
    dry_run: bool,
    yes: bool,
    format: OutputFormat,
) -> Result<u8> {
    let assignments = parse_assignments(args)?;
    let mut dev = IntegraDevice::connect_detecting(port_pattern, device_id, timeout)?.quiet();
    let current = read_slots(&mut dev).context("could not read the Expansion slots")?;
    let target = stage(&current, &assignments);
    let banks = read_part_banks(&mut dev)?;
    let plan = expansion_load::plan(&current, &target, &banks);
    let text = format == OutputFormat::Text;
    let finish = |result: &str, slots: Option<&ExpansionSlots>| {
        if text {
            println!("{}", slots_line(slots));
        } else {
            println!("{:#}", plan_json(&current, &target, &plan, result, slots));
        }
    };

    if plan.changes.is_empty() {
        eprintln!("Nothing to load: the slots already hold that.");
        finish("no-op", Some(&current));
        return Ok(0);
    }
    if text {
        print!("{}", render_plan(&plan));
    }
    if dry_run {
        if !text {
            finish("dry-run", None);
        }
        return Ok(0);
    }
    if !plan.silenced_parts.is_empty() && !yes {
        confirm_silenced(&plan.silenced_parts)?;
    }

    let started = Instant::now();
    match run_load(&mut dev, current, target)? {
        LoadOutcome::NoOp => {
            finish("no-op", Some(&current));
            Ok(0)
        }
        LoadOutcome::Loaded(slots) => {
            eprintln!("Load done in {:.1} s", started.elapsed().as_secs_f64());
            finish("loaded", Some(&slots));
            Ok(0)
        }
        LoadOutcome::Failed(slots) => {
            eprintln!("Load failed: the slots don't hold the requested Expansions");
            finish("failed", Some(&slots));
            Ok(EXIT_LOAD_FAILED)
        }
        LoadOutcome::TimedOut => {
            eprintln!(
                "Load timed out after {:.0} s",
                started.elapsed().as_secs_f64()
            );
            let slots = read_slots(&mut dev)
                .inspect_err(|e| eprintln!("Warning: {e:#}; slots unknown"))
                .ok();
            finish("timed-out", slots.as_ref());
            Ok(EXIT_LOAD_FAILED)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn slots(names: [&str; SLOT_COUNT]) -> ExpansionSlots {
        ExpansionSlots(names.map(expansions::by_name))
    }

    fn names(slots: ExpansionSlots) -> [&'static str; SLOT_COUNT] {
        slots.0.map(|s| s.map_or(OFF_NAME, |e| e.name))
    }

    fn current() -> ExpansionSlots {
        slots(["SRX-03", "SRX-05", "SRX-07", "SRX-08"])
    }

    fn staged(list: &[&str]) -> [&'static str; SLOT_COUNT] {
        names(stage(&current(), &parse_assignments(&args(list)).unwrap()))
    }

    fn error(list: &[&str]) -> String {
        format!("{:#}", parse_assignments(&args(list)).unwrap_err())
    }

    #[test]
    fn parses_assignments_case_insensitively() {
        assert_eq!(
            parse_assignments(&args(&["c=exsn3", "A=SRX-01", "d=off"])).unwrap(),
            [
                Assignment {
                    slot: 0,
                    expansion: expansions::by_name("SRX-01"),
                },
                Assignment {
                    slot: 2,
                    expansion: expansions::by_name("ExSN3"),
                },
                Assignment {
                    slot: 3,
                    expansion: None,
                },
            ]
        );
    }

    #[test]
    fn unnamed_slots_keep_their_expansion() {
        assert_eq!(
            staged(&["A=SRX-01", "C=ExSN3"]),
            ["SRX-01", "SRX-05", "ExSN3", "SRX-08"]
        );
    }

    #[test]
    fn naming_a_loaded_expansion_moves_it() {
        assert_eq!(staged(&["A=SRX-07"]), ["SRX-07", "SRX-05", "OFF", "SRX-08"]);
    }

    #[test]
    fn swaps_in_either_order() {
        let swapped = ["SRX-07", "SRX-05", "SRX-03", "SRX-08"];
        assert_eq!(staged(&["A=SRX-07", "C=SRX-03"]), swapped);
        assert_eq!(staged(&["C=SRX-03", "A=SRX-07"]), swapped);
    }

    #[test]
    fn off_empties_a_slot() {
        assert_eq!(staged(&["b=OFF"]), ["SRX-03", "OFF", "SRX-07", "SRX-08"]);
    }

    #[test]
    fn expcm_fills_all_slots() {
        assert_eq!(staged(&["expcm"]), ["ExPCM"; SLOT_COUNT]);
        let expcm = slots(["ExPCM"; SLOT_COUNT]);
        let a = parse_assignments(&args(&["A=SRX-01"])).unwrap();
        assert_eq!(names(stage(&expcm, &a)), ["SRX-01", "OFF", "OFF", "OFF"]);
    }

    #[test]
    fn rejects_bad_arguments() {
        for (list, needle) in [
            (&[][..], "nothing to load"),
            (&["A=SRX-13"], "unknown Expansion `SRX-13`"),
            (&["E=SRX-01"], "unknown Expansion slot `E`"),
            (&["AB=SRX-01"], "unknown Expansion slot `AB`"),
            (&["=SRX-01"], "unknown Expansion slot ``"),
            (&["SRX-01"], "needs a slot"),
            (&["OFF"], "needs a slot"),
            (&["bogus"], "unknown Expansion `bogus`"),
            (&["A=ExPCM"], "fills all four slots"),
            (&["ExPCM", "A=SRX-01"], "fills all four slots"),
            (&["A=SRX-01", "a=SRX-02"], "slot A is assigned twice"),
            (&["A=SRX-01", "B=srx-01"], "SRX-01 is assigned to two slots"),
        ] {
            let err = error(list);
            assert!(err.contains(needle), "{list:?}: {err}");
        }
    }

    #[test]
    fn two_slots_may_both_go_off() {
        assert_eq!(
            staged(&["A=OFF", "B=OFF"]),
            ["OFF", "OFF", "SRX-07", "SRX-08"]
        );
    }

    #[test]
    fn plan_reports_changes_and_silenced_parts() {
        let target = stage(
            &current(),
            &parse_assignments(&args(&["A=SRX-01", "C=ExSN3"])).unwrap(),
        );
        let part_banks = [(93, 2), (93, 4), (89, 64), (93, 11)];
        let plan = expansion_load::plan(&current(), &target, &part_banks);
        assert_eq!(
            render_plan(&plan),
            "A: SRX-03 → SRX-01\nC: SRX-07 → ExSN3\nParts going silent: 1, 4\n"
        );
        let unchanged = expansion_load::plan(&current(), &current(), &part_banks);
        assert_eq!(render_plan(&unchanged), "No Part goes silent.\n");
    }

    #[test]
    fn slots_line_shows_names_off_and_unknown() {
        assert_eq!(
            slots_line(Some(&current())),
            "A SRX-03  B SRX-05  C SRX-07  D SRX-08"
        );
        assert_eq!(
            slots_line(Some(&slots(["ExSN1", "OFF", "OFF", "OFF"]))),
            "A ExSN1  B OFF  C OFF  D OFF"
        );
        assert_eq!(slots_line(None), "A ?  B ?  C ?  D ?");
    }

    #[test]
    fn list_marks_slots_and_unknown_state() {
        let text = render_list(Some(&current()), OutputFormat::Text);
        let row = |name: &str| {
            text.lines()
                .find(|l| l.starts_with(name))
                .map(|l| l.split_whitespace().collect::<Vec<_>>())
                .unwrap()
        };
        assert!(text.starts_with("A SRX-03  B SRX-05  C SRX-07  D SRX-08\n"));
        assert_eq!(row("SRX-05"), ["SRX-05", "B"]);
        assert_eq!(row("SRX-01"), ["SRX-01", "-"]);
        assert_eq!(text.lines().count(), 3 + EXPANSIONS.len());

        let expcm = render_list(Some(&slots(["ExPCM"; 4])), OutputFormat::Text);
        assert!(
            expcm
                .lines()
                .any(|l| l.split_whitespace().eq(["ExPCM", "A-D"]))
        );

        let unknown = render_list(None, OutputFormat::Text);
        assert!(unknown.starts_with("A ?  B ?  C ?  D ?\n"));
        assert!(
            unknown
                .lines()
                .any(|l| l.split_whitespace().eq(["SRX-01", "?"]))
        );
    }

    #[test]
    fn list_json() {
        let doc: Value =
            serde_json::from_str(&render_list(Some(&current()), OutputFormat::Json)).unwrap();
        assert_eq!(doc["slots"]["C"], "SRX-07");
        assert_eq!(doc["expansions"][2]["name"], "SRX-03");
        assert_eq!(doc["expansions"][2]["loaded"], true);
        assert_eq!(doc["expansions"][2]["slots"], json!(["A"]));
        assert_eq!(doc["expansions"][0]["loaded"], false);

        let doc: Value = serde_json::from_str(&render_list(None, OutputFormat::Json)).unwrap();
        assert!(doc["slots"].is_null());
        assert!(doc["expansions"][0]["loaded"].is_null());
        assert_eq!(
            doc["expansions"].as_array().unwrap().len(),
            EXPANSIONS.len()
        );
    }
}
