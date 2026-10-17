//! `pack import`: plan where a Pack's entries land, then store them.
//!
//! Planning and storing live in `integral-core::librarian`; this module
//! reads what the plan needs (User slot names, loaded Expansions), prints
//! the plan, and drives the import. Planning only reads: [`store`], the only
//! place that writes to the device, runs after the `--dry-run` return.
//!
//! `pack reinit` reuses the same reads, undo export and driver to store
//! INIT into chosen User slots.

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use clap::Args;
use integral_core::catalog::{
    CatalogEntry, build_studio_set_catalog_request, build_tone_catalog_request, parse_catalog_entry,
};
use integral_core::device_spec::{INTEGRA7, USER_MEMORY, UserMemorySpec};
use integral_core::expansion_slots::ExpansionSlots;
use integral_core::librarian::export::{ExportSource, PackExport};
use integral_core::librarian::foreign::{PackFormat, PackRead, lossy_count};
use integral_core::librarian::import::{EntryStatus, ImportAction, PackImport};
use integral_core::librarian::pack::Pack;
use integral_core::librarian::plan::{ImportPlan, Origin, Placement, plan_read};
use integral_core::svd::ChunkType;
use integral_core::sysex;
use serde_json::{Value, json};

use super::expansion::{OutputFormat, read_slots};
use super::pack::{
    Throttle, entry_id, lossy_marker, parse_slots, read_pack, report_json, run_export, type_parser,
    warn_file_report,
};
use crate::device::IntegraDevice;

/// Exit code when some entries were stored and others not.
pub const EXIT_PARTIAL: u8 = 2;

/// `pack import` options.
#[derive(Debug, Clone, Args)]
pub struct ImportOptions {
    /// Import only these entries, e.g. studio-set:1,sn-synth:2.
    #[arg(long, value_name = "IDS", value_delimiter = ',')]
    only: Vec<String>,
    /// Send an entry to this User slot of its type, free or not.
    #[arg(long, value_name = "ID=SLOT", value_parser = parse_to)]
    to: Vec<(String, u16)>,
    /// The Part that loads a lone tone, and that stores tones.
    #[arg(long, value_name = "N", default_value_t = 1,
          value_parser = clap::value_parser!(u8).range(1..=i64::from(INTEGRA7.part_count)))]
    part: u8,
    /// Also apply a lone Foreign patch's Chorus and Reverb to the Temporary
    /// Studio Set. Only for a single entry loaded into Temporary.
    #[arg(long)]
    apply_effects: bool,
    /// Print the plan and stop. Only reads from the device.
    #[arg(long)]
    dry_run: bool,
    /// Overwrite used slots without asking.
    #[arg(long, short = 'y')]
    yes: bool,
    /// Where to export the slots about to be overwritten (default:
    /// UNDO-<file>-<unix time>.SVD next to FILE).
    #[arg(long, value_name = "PATH", conflicts_with = "no_undo")]
    undo_file: Option<PathBuf>,
    /// Don't export the slots about to be overwritten.
    #[arg(long)]
    no_undo: bool,
    #[arg(long, short = 'f', value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

/// `pack reinit` options.
#[derive(Debug, Clone, Args)]
pub struct ReinitOptions {
    /// The User memory type.
    #[arg(value_parser = type_parser())]
    r#type: String,
    /// The User slots, e.g. 113 or 1-4,7.
    slots: String,
    /// The Part borrowed to store tones; restored afterwards.
    #[arg(long, value_name = "N", default_value_t = 1,
          value_parser = clap::value_parser!(u8).range(1..=i64::from(INTEGRA7.part_count)))]
    part: u8,
    /// Re-init without asking.
    #[arg(long, short = 'y')]
    yes: bool,
    /// Where to export the used slots first (default:
    /// UNDO-reinit-<type>-<unix time>.SVD in the current directory).
    #[arg(long, value_name = "PATH", conflicts_with = "no_undo")]
    undo_file: Option<PathBuf>,
    /// Don't export the used slots first.
    #[arg(long)]
    no_undo: bool,
}

impl ReinitOptions {
    /// The type, and its picked slots (0-indexed), sorted, each once.
    fn slots(&self) -> Result<(ChunkType, Vec<u16>)> {
        let kind = ChunkType::from_cli_name(&self.r#type).expect("type_parser takes only types");
        let mut slots = parse_slots(&self.slots, spec(kind).slot_count).map_err(|e| anyhow!(e))?;
        slots.sort_unstable();
        slots.dedup();
        Ok((kind, slots))
    }
}

fn parse_to(s: &str) -> Result<(String, u16), String> {
    let (id, slot) = s
        .split_once('=')
        .ok_or_else(|| format!("`{s}`: expected ID=SLOT, e.g. sn-synth:1=41"))?;
    let slot = slot
        .parse::<u16>()
        .ok()
        .filter(|&n| n >= 1)
        .ok_or_else(|| format!("`{s}`: SLOT is a User slot number from 1"))?;
    Ok((id.to_string(), slot))
}

fn spec(kind: ChunkType) -> &'static UserMemorySpec {
    USER_MEMORY
        .iter()
        .find(|m| m.cli_name == kind.cli_name())
        .expect("every Pack entry type has User memory")
}

/// The index into `pack.entries` of the entry with id `<type>:<n>`.
fn entry_index(pack: &Pack, entry_id: &str) -> Result<usize> {
    let (kind, n) = entry_id
        .split_once(':')
        .and_then(|(t, n)| Some((ChunkType::from_cli_name(t)?, n.parse::<usize>().ok()?)))
        .filter(|&(_, n)| n >= 1)
        .with_context(|| format!("`{entry_id}`: expected an id like sn-synth:1"))?;
    pack.entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind() == kind)
        .nth(n - 1)
        .map(|(i, _)| i)
        .with_context(|| format!("no entry {entry_id} in the Pack"))
}

/// Plan the import, then apply `--only` and `--to`.
fn plan(
    read: &PackRead,
    names: &[CatalogEntry],
    loaded: Option<&ExpansionSlots>,
    opts: &ImportOptions,
) -> Result<ImportPlan> {
    let pack = &read.pack;
    let mut plan = plan_read(read, Origin::of(read), names, loaded);
    if !opts.only.is_empty() {
        let only = opts
            .only
            .iter()
            .map(|id| entry_index(pack, id))
            .collect::<Result<Vec<_>>>()?;
        // Leave everything out, then take the kept entries back, so they get
        // the lowest free slots.
        if (0..pack.entries.len()).any(|i| !only.contains(&i)) {
            for i in 0..pack.entries.len() {
                plan.exclude(i)?;
            }
            for i in (0..pack.entries.len()).filter(|i| only.contains(i)) {
                plan.include(i)?;
            }
        }
    }
    for (id, slot) in &opts.to {
        let i = entry_index(pack, id)?;
        if plan.placements()[i] == Placement::Excluded {
            bail!("--to {id}: the entry is left out by --only");
        }
        plan.assign(i, slot - 1)
            .with_context(|| format!("--to {id}={slot}"))?;
    }
    if opts.apply_effects && plan.placements() != [Placement::Temporary] {
        bail!("--apply-effects: only a single entry loaded into Temporary takes effects");
    }
    Ok(plan)
}

/// The catalog queries reading the User slot names of `kinds`, each with
/// the number of entries it returns. All are RQ1s.
fn name_requests(device_id: u8, kinds: &[ChunkType]) -> Vec<(Vec<u8>, usize)> {
    let mut requests = Vec::new();
    for &kind in kinds {
        let memory = spec(kind);
        for first in (0..memory.slot_count).step_by(64) {
            let count = (memory.slot_count - first).min(64) as u8;
            let (lsb, pc) = ((first / 128) as u8, (first % 128) as u8);
            let request = if kind == ChunkType::StudioSet {
                build_studio_set_catalog_request(device_id, pc, count)
            } else {
                build_tone_catalog_request(device_id, memory.bank_msb, lsb, pc, count)
            };
            requests.push((request, count.into()));
        }
    }
    requests
}

/// Read the User slot names of `kinds`.
fn read_names(
    dev: &mut IntegraDevice,
    throttle: &mut Throttle,
    kinds: &[ChunkType],
) -> Result<Vec<CatalogEntry>> {
    let msbs: Vec<u8> = kinds.iter().map(|&k| spec(k).bank_msb).collect();
    let mut names = Vec::new();
    for (request, count) in name_requests(dev.device_id(), kinds) {
        throttle.send(dev, request)?;
        let mut got = 0;
        while got < count {
            match dev.rx().recv_timeout(dev.timeout()) {
                Ok(raw) => {
                    if let Some(entry) = sysex::parse_dt1(&raw)
                        .ok()
                        .and_then(|dt1| parse_catalog_entry(&dt1.data))
                        .filter(|e| msbs.contains(&e.bank_msb))
                    {
                        names.push(entry);
                        got += 1;
                    }
                }
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => bail!("MIDI input disconnected"),
            }
        }
    }
    Ok(names)
}

/// The current name of User slot `slot` of `kind`, if read.
fn slot_name(names: &[CatalogEntry], kind: ChunkType, slot: u16) -> Option<&str> {
    let bs = spec(kind).bank_select(slot)?;
    names
        .iter()
        .find(|e| (e.bank_msb, e.bank_lsb, e.pc) == (bs.msb, bs.lsb, bs.pc))
        .map(|e| e.name.as_str())
}

fn destination(kind: ChunkType, placement: Placement, part: u8) -> String {
    match placement {
        Placement::User(slot) => format!("User {}", slot + 1),
        Placement::Temporary if kind == ChunkType::StudioSet => "Temporary Studio Set".into(),
        Placement::Temporary => format!("Temporary Part {part}"),
        Placement::Unplaced => "unplaced".into(),
        Placement::Excluded => "excluded".into(),
    }
}

fn status_name(status: EntryStatus) -> &'static str {
    match status {
        EntryStatus::Pending => "not stored",
        EntryStatus::Storing => "storing",
        EntryStatus::Stored => "stored",
        EntryStatus::Loaded => "loaded",
        EntryStatus::Failed => "failed",
        EntryStatus::Excluded => "excluded",
    }
}

/// What a run tells the user: the plan, and once run, each entry's status.
struct Report<'a> {
    file: &'a Path,
    format: PackFormat,
    pack: &'a Pack,
    plan: &'a ImportPlan,
    names: &'a [CatalogEntry],
    part: u8,
    statuses: Option<Vec<EntryStatus>>,
    undo: Option<PathBuf>,
    error: Option<String>,
}

impl Report<'_> {
    fn overwritten(&self, i: usize) -> Option<String> {
        let kind = self.pack.entries[i].kind();
        match self.plan.placements()[i] {
            Placement::User(slot) if self.plan.overwrites().any(|o| o == i) => {
                Some(slot_name(self.names, kind, slot).unwrap_or("?").to_string())
            }
            _ => None,
        }
    }

    /// What the file held that is not in the Pack, then the plan's
    /// warnings.
    fn warnings(&self) -> Vec<String> {
        let file = self.plan.file_report().iter().map(ToString::to_string);
        let plan = self.plan.warnings().map(|w| {
            let entry = w.entry();
            let name = self.pack.entries[entry].name();
            format!("{} {name}: {w}", entry_id(self.pack, entry))
        });
        file.chain(plan).collect()
    }

    fn text_plan(&self) -> String {
        let kind = self.format.label(self.plan.origin());
        let mut out = format!(
            "{} ({kind}, {} {}):\n",
            self.file.display(),
            self.pack.entries.len(),
            if self.pack.entries.len() == 1 {
                "entry"
            } else {
                "entries"
            }
        );
        let mut excluded = 0;
        for (i, e) in self.pack.entries.iter().enumerate() {
            let placement = self.plan.placements()[i];
            if placement == Placement::Excluded {
                excluded += 1;
                continue;
            }
            let dest = destination(e.kind(), placement, self.part);
            let mut line = format!(
                "  {:<16} {:<16} -> {dest}",
                entry_id(self.pack, i),
                e.name()
            );
            if let Some(name) = self.overwritten(i) {
                line += &format!(" (overwrites \"{name}\")");
            }
            if let Some(model) = self.plan.source(i) {
                line += &format!(" ({model})");
            }
            let report = self.plan.report(i);
            let lossy = lossy_count(report);
            if lossy > 0 {
                line += &format!(" [{}]", lossy_marker(lossy));
            }
            out += &line;
            out += "\n";
            for item in report {
                out += &format!("      {item}\n");
            }
        }
        if excluded > 0 {
            out += &format!("  ({excluded} left out)\n");
        }
        for w in self.warnings() {
            out += &format!("Warning: {w}\n");
        }
        out
    }

    fn text_result(&self) -> String {
        let mut out = String::new();
        if let Some(statuses) = &self.statuses {
            for (i, &s) in statuses.iter().enumerate() {
                if s != EntryStatus::Excluded {
                    out += &format!("{:<16} {}\n", entry_id(self.pack, i), status_name(s));
                }
            }
        }
        if let Some(undo) = &self.undo {
            out += &format!("Undo file: {}\n", undo.display());
        }
        if let Some(e) = &self.error {
            out += &format!("Error: {e}\n");
        }
        out
    }

    fn json(&self, result: &str) -> Value {
        let entries: Vec<Value> = self
            .pack
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let destination = match self.plan.placements()[i] {
                    Placement::User(slot) => json!({ "user": slot + 1 }),
                    Placement::Temporary if e.kind() == ChunkType::StudioSet => {
                        json!("temporary-studio-set")
                    }
                    Placement::Temporary => json!({ "temporary_part": self.part }),
                    Placement::Unplaced => json!("unplaced"),
                    Placement::Excluded => json!("excluded"),
                };
                json!({
                    "id": entry_id(self.pack, i),
                    "name": e.name(),
                    "destination": destination,
                    "overwrites": self.overwritten(i),
                    "source": self.plan.source(i),
                    "status": self.statuses.as_ref().map(|s| status_name(s[i])),
                    "report": report_json(self.plan.report(i)),
                })
            })
            .collect();
        json!({
            "file": self.file,
            "entries": entries,
            "file_report": report_json(self.plan.file_report()),
            "warnings": self.warnings(),
            "undo_file": self.undo,
            "result": result,
            "error": self.error,
        })
    }
}

/// The exit code for the final statuses: 0 when every entry not excluded
/// was stored (or loaded), 1 when the device was left untouched, else 2.
fn exit_code(statuses: &[EntryStatus]) -> u8 {
    use EntryStatus::*;
    if statuses
        .iter()
        .all(|s| matches!(s, Stored | Loaded | Excluded))
    {
        0
    } else if statuses
        .iter()
        .any(|s| matches!(s, Stored | Loaded | Failed))
    {
        EXIT_PARTIAL
    } else {
        1
    }
}

/// Ask `question` with a y/N prompt on a TTY; elsewhere refuse with
/// `refusal`. `what` names the cancelled command.
fn confirm(question: &str, refusal: &str, what: &str) -> Result<()> {
    if !std::io::stdin().is_terminal() {
        bail!("{refusal}");
    }
    eprint!("{question} [y/N] ");
    std::io::stderr().flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    match line.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => Ok(()),
        _ => bail!("{what} cancelled"),
    }
}

/// `UNDO-<stem>-<unix time>.SVD`.
fn undo_name(stem: &str) -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    format!("UNDO-{stem}-{secs}.SVD")
}

/// `UNDO-<stem>-<unix time>.SVD` next to `file`.
fn default_undo_path(file: &Path) -> PathBuf {
    let stem = file.file_stem().unwrap_or_default().to_string_lossy();
    file.with_file_name(undo_name(&stem))
}

/// Export `sources` to `path`.
fn export_undo(
    dev: &mut IntegraDevice,
    throttle: &mut Throttle,
    sources: Vec<ExportSource>,
    path: &Path,
) -> Result<()> {
    if path.exists() {
        bail!("{} exists; pick another --undo-file", path.display());
    }
    let mut export = PackExport::new(dev.device_id(), sources)?;
    let undo = run_export(dev, throttle, &mut export)?;
    std::fs::write(path, undo.to_svd()?)
        .with_context(|| format!("failed to write {}", path.display()))
}

/// Run `import` to its end, printing `<status> <line(i)>` as entry `i`
/// finishes. Returns the final statuses, and the error that stopped it, if
/// any.
fn drive(
    dev: &mut IntegraDevice,
    throttle: &mut Throttle,
    import: &mut PackImport,
    line: &dyn Fn(usize) -> String,
) -> (Vec<EntryStatus>, Option<String>) {
    let mut seen = import.progress().to_vec();
    let mut deadline = Instant::now();
    let result: Result<Option<String>> = (|| {
        loop {
            let action = import.next_action();
            for (i, (&now, was)) in import.progress().iter().zip(&mut seen).enumerate() {
                if now != *was
                    && matches!(
                        now,
                        EntryStatus::Stored | EntryStatus::Loaded | EntryStatus::Failed
                    )
                {
                    eprintln!("{} {}", status_name(now), line(i));
                }
                *was = now;
            }
            match action {
                ImportAction::Done(r) => return Ok(r.error.map(|e| e.to_string())),
                ImportAction::Send(bytes) => throttle.send(dev, bytes)?,
                ImportAction::Read(rq1) => {
                    throttle.send(dev, rq1)?;
                    deadline = Instant::now() + dev.timeout();
                }
                ImportAction::Wait => {
                    match dev
                        .rx()
                        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                    {
                        Ok(raw) => {
                            if let Ok(dt1) = sysex::parse_dt1(&raw)
                                && import.on_read(&dt1.address, &dt1.data)
                            {
                                deadline = Instant::now() + dev.timeout();
                            }
                        }
                        Err(RecvTimeoutError::Timeout) => import.on_timeout(),
                        Err(RecvTimeoutError::Disconnected) => bail!("MIDI input disconnected"),
                    }
                }
                ImportAction::AwaitWrite { timeout_ms } => {
                    let until = Instant::now() + Duration::from_secs_f64(timeout_ms / 1000.0);
                    loop {
                        match dev
                            .rx()
                            .recv_timeout(until.saturating_duration_since(Instant::now()))
                        {
                            Ok(raw) => {
                                if let Ok(dt1) = sysex::parse_dt1(&raw)
                                    && import.on_write_reply(&dt1.address, &dt1.data)
                                {
                                    break;
                                }
                            }
                            Err(RecvTimeoutError::Timeout) => {
                                import.on_timeout();
                                break;
                            }
                            Err(RecvTimeoutError::Disconnected) => {
                                bail!("MIDI input disconnected")
                            }
                        }
                    }
                }
            }
        }
    })();
    let error = result.unwrap_or_else(|e| Some(format!("{e:#}")));
    (import.progress().to_vec(), error)
}

/// Confirm, export the undo file, and store. The only part of `pack
/// import` that writes to the device.
fn store(
    dev: &mut IntegraDevice,
    throttle: &mut Throttle,
    opts: &ImportOptions,
    report: &mut Report,
) -> Result<()> {
    let overwrites = report.plan.overwrites().count();
    if overwrites > 0 {
        if !opts.yes {
            confirm(
                &format!("Overwrite {overwrites} used slots?"),
                &format!(
                    "the import overwrites {overwrites} used slots; pass --yes to import anyway"
                ),
                "import",
            )?;
        }
        if !opts.no_undo {
            let path = opts
                .undo_file
                .clone()
                .unwrap_or_else(|| default_undo_path(report.file));
            let (pack, plan) = (report.pack, report.plan);
            let sources = plan
                .overwrites()
                .filter_map(|i| match plan.placements()[i] {
                    Placement::User(slot) => Some(ExportSource::User {
                        kind: pack.entries[i].kind(),
                        slot,
                    }),
                    _ => None,
                })
                .collect();
            export_undo(dev, throttle, sources, &path)
                .context("undo export failed; nothing was stored")?;
            eprintln!("Saved the overwritten slots to {}", path.display());
            report.undo = Some(path);
        }
    }
    let mut import = PackImport::new(
        dev.device_id(),
        report.pack,
        report.plan,
        opts.part - 1,
        opts.apply_effects,
    )?;
    let line = |i: usize| {
        let e = &report.pack.entries[i];
        let dest = destination(e.kind(), report.plan.placements()[i], report.part);
        format!("{} {} -> {dest}", entry_id(report.pack, i), e.name())
    };
    let (statuses, error) = drive(dev, throttle, &mut import, &line);
    report.statuses = Some(statuses);
    report.error = error;
    Ok(())
}

/// `pack import`: plan, print the plan, and unless `--dry-run`, store.
///
/// Returns the exit code: 0 when everything was stored, 1 when nothing
/// was, [`EXIT_PARTIAL`] otherwise. `Err` means nothing was stored.
pub fn import(
    port_pattern: &str,
    device_id: Option<u8>,
    timeout: Duration,
    file: &Path,
    opts: &ImportOptions,
) -> Result<u8> {
    let read = read_pack(file)?;
    let pack = &read.pack;
    if pack.entries.is_empty() {
        warn_file_report(file, &read);
        bail!("{} holds nothing to import", file.display());
    }

    let mut dev = IntegraDevice::connect_detecting(port_pattern, device_id, timeout)?.quiet();
    let mut throttle = Throttle::new(dev.device_id());
    let mut kinds = Vec::new();
    for e in &pack.entries {
        if !kinds.contains(&e.kind()) {
            kinds.push(e.kind());
        }
    }
    let names = read_names(&mut dev, &mut throttle, &kinds).context("could not read slot names")?;
    let loaded = read_slots(&mut dev)
        .inspect_err(|e| eprintln!("Warning: {e:#}; Expansion checks skipped"))
        .ok();
    let plan = plan(&read, &names, loaded.as_ref(), opts)?;
    if opts.apply_effects && !plan.effects(0).is_empty() {
        match dev.read_surround_enabled() {
            Ok(true) => eprintln!(
                "Warning: Motional Surround is ON, which mutes the applied Reverb; switch it off to hear it"
            ),
            Ok(false) => {}
            Err(e) => eprintln!("Warning: {e:#}; Motional Surround check skipped"),
        }
    }

    let text = opts.format == OutputFormat::Text;
    let mut report = Report {
        file,
        format: read.format,
        pack,
        plan: &plan,
        names: &names,
        part: opts.part,
        statuses: None,
        undo: None,
        error: None,
    };
    if text {
        print!("{}", report.text_plan());
    }
    let unplaced: Vec<String> = plan.unplaced().map(|i| entry_id(pack, i)).collect();
    if !unplaced.is_empty() {
        if !text {
            println!("{:#}", report.json("unplaced"));
        }
        return Err(anyhow!(
            "no destination for {}; free slots, pick them with --to, or leave entries out with --only",
            unplaced.join(", ")
        ));
    }
    if opts.dry_run {
        if !text {
            println!("{:#}", report.json("dry-run"));
        }
        return Ok(0);
    }

    store(&mut dev, &mut throttle, opts, &mut report)?;
    let code = exit_code(report.statuses.as_deref().unwrap_or_default());
    if text {
        print!("{}", report.text_result());
    } else {
        let result = ["stored", "failed", "partial"][usize::from(code)];
        println!("{:#}", report.json(result));
    }
    Ok(code)
}

/// `pack reinit`: print the picked slots' names, confirm, export the used
/// ones, and store INIT into them all.
///
/// Returns the exit code, as [`import`] does.
pub fn reinit(
    port_pattern: &str,
    device_id: Option<u8>,
    timeout: Duration,
    opts: &ReinitOptions,
) -> Result<u8> {
    let (kind, slots) = opts.slots()?;
    let memory = spec(kind);
    let mut dev = IntegraDevice::connect_detecting(port_pattern, device_id, timeout)?.quiet();
    let mut throttle = Throttle::new(dev.device_id());
    let names =
        read_names(&mut dev, &mut throttle, &[kind]).context("could not read slot names")?;

    let name = |slot| slot_name(&names, kind, slot);
    println!("{} slots to re-init:", kind.cli_name());
    for &slot in &slots {
        println!("  User {:<4} {}", slot + 1, name(slot).unwrap_or("?"));
    }
    // An unread name counts as used.
    let used: Vec<u16> = slots
        .iter()
        .copied()
        .filter(|&s| !name(s).is_some_and(|n| memory.is_free(n)))
        .collect();
    if !opts.yes {
        let n = slots.len();
        confirm(
            &format!("Re-init {n} slots ({} used)?", used.len()),
            &format!("the re-init overwrites {n} slots; pass --yes to re-init anyway"),
            "re-init",
        )?;
    }
    if !used.is_empty() && !opts.no_undo {
        let path = opts
            .undo_file
            .clone()
            .unwrap_or_else(|| undo_name(&format!("reinit-{}", kind.cli_name())).into());
        let sources = used
            .iter()
            .map(|&slot| ExportSource::User { kind, slot })
            .collect();
        export_undo(&mut dev, &mut throttle, sources, &path)
            .context("undo export failed; nothing was stored")?;
        eprintln!("Saved the used slots to {}", path.display());
    }

    let mut import = PackImport::reinit(dev.device_id(), kind, &slots, opts.part - 1)?;
    let line = |i: usize| format!("INIT -> User {}", slots[i] + 1);
    let (statuses, error) = drive(&mut dev, &mut throttle, &mut import, &line);
    for (&slot, &s) in slots.iter().zip(&statuses) {
        println!("User {:<4} {}", slot + 1, status_name(s));
    }
    if let Some(e) = error {
        println!("Error: {e}");
    }
    Ok(exit_code(&statuses))
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use integral_core::librarian::foreign::SourceModel;
    use integral_core::librarian::pack::PackEntry;

    use super::*;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        opts: ImportOptions,
    }

    fn opts(args: &[&str]) -> Result<ImportOptions, clap::Error> {
        Cli::try_parse_from(std::iter::once("t").chain(args.iter().copied())).map(|c| c.opts)
    }

    fn entry(kind: ChunkType, slot: Option<u16>) -> PackEntry {
        let memory = spec(kind);
        let blocks = memory
            .blocks(memory.temporary_base(0))
            .map(|(_, s)| vec![0; s.value() as usize])
            .collect();
        PackEntry::new(kind, slot, blocks).unwrap()
    }

    /// Two SN Synth tones and a PCM Drum Kit, from a sound export.
    fn pack() -> Pack {
        Pack {
            entries: vec![
                entry(ChunkType::SnSynthTone, None),
                entry(ChunkType::SnSynthTone, None),
                entry(ChunkType::PcmDrumKit, None),
            ],
        }
    }

    /// `pack` as read from an SVD file.
    fn read_of(pack: Pack) -> PackRead {
        PackRead {
            format: PackFormat::Svd,
            pack,
            file_report: vec![],
            reports: vec![],
            sources: vec![],
            effects: vec![],
        }
    }

    /// SN Synth slots 1-3 free, the rest used; PCM Drum slots all used.
    fn names() -> Vec<CatalogEntry> {
        let sns = (0..8).map(|pc| CatalogEntry {
            bank_msb: spec(ChunkType::SnSynthTone).bank_msb,
            bank_lsb: 0,
            pc,
            name: if pc < 3 { "INIT TONE" } else { "Used" }.into(),
        });
        let kits = (0..32).map(|pc| CatalogEntry {
            bank_msb: spec(ChunkType::PcmDrumKit).bank_msb,
            bank_lsb: 0,
            pc,
            name: "Kit".into(),
        });
        sns.chain(kits).collect()
    }

    #[test]
    fn parses_options() {
        let o = opts(&["--only", "sn-synth:1,pcm-drum:1", "--to", "pcm-drum:1=5"]).unwrap();
        assert_eq!(o.only, ["sn-synth:1", "pcm-drum:1"]);
        assert_eq!(o.to, [("pcm-drum:1".to_string(), 5)]);
        assert_eq!((o.part, o.dry_run, o.yes), (1, false, false));
        for bad in [
            &["--to", "sn-synth:1"][..],
            &["--to", "sn-synth:1=0"],
            &["--part", "17"],
            &["--undo-file", "u.svd", "--no-undo"],
        ] {
            assert!(opts(bad).is_err(), "{bad:?}");
        }
    }

    fn reinit_opts(args: &[&str]) -> Result<ReinitOptions, clap::Error> {
        #[derive(Parser)]
        struct Cli {
            #[command(flatten)]
            opts: ReinitOptions,
        }
        Cli::try_parse_from(std::iter::once("t").chain(args.iter().copied())).map(|c| c.opts)
    }

    #[test]
    fn parses_reinit_slots_and_ranges() {
        let slots = |args: &[&str]| reinit_opts(args).unwrap().slots().unwrap();
        assert_eq!(
            slots(&["pcm-synth", "113"]),
            (ChunkType::PcmSynthTone, vec![112])
        );
        assert_eq!(
            slots(&["sn-synth", "1-4"]),
            (ChunkType::SnSynthTone, vec![0, 1, 2, 3])
        );
        // Sorted, each slot once.
        assert_eq!(slots(&["studio-set", "9,2-3,3"]).1, [1, 2, 8]);
        let o = reinit_opts(&["sn-synth", "1"]).unwrap();
        assert_eq!((o.part, o.yes, o.no_undo), (1, false, false));

        for bad in [
            &["bogus", "1"][..],
            &["sn-synth"],
            &["sn-synth", "1", "--part", "17"],
            &["sn-synth", "1", "--undo-file", "u.svd", "--no-undo"],
        ] {
            assert!(reinit_opts(bad).is_err(), "{bad:?}");
        }
        for bad in [
            &["sn-synth", "0"][..],
            &["sn-synth", "4-2"],
            &["pcm-drum", "33"],
        ] {
            assert!(reinit_opts(bad).unwrap().slots().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn resolves_ids() {
        let pack = pack();
        assert_eq!(entry_index(&pack, "sn-synth:2").unwrap(), 1);
        assert_eq!(entry_index(&pack, "pcm-drum:1").unwrap(), 2);
        for bad in [
            "sn-synth:3",
            "sn-synth:0",
            "studio-set:1",
            "bogus:1",
            "sn-synth",
        ] {
            assert!(entry_index(&pack, bad).is_err(), "{bad}");
        }
        assert_eq!(entry_id(&pack, 1), "sn-synth:2");
    }

    #[test]
    fn only_excludes_and_to_assigns() {
        let read = read_of(pack());
        let plan_with = |args: &[&str]| plan(&read, &names(), None, &opts(args).unwrap());

        let default = plan_with(&[]).unwrap();
        assert_eq!(
            default.placements(),
            [Placement::User(0), Placement::User(1), Placement::Unplaced]
        );

        let only = plan_with(&["--only", "sn-synth:2"]).unwrap();
        assert_eq!(
            only.placements(),
            [Placement::Excluded, Placement::User(0), Placement::Excluded]
        );

        let to = plan_with(&["--to", "pcm-drum:1=5"]).unwrap();
        assert_eq!(to.placements()[2], Placement::User(4));
        assert_eq!(to.overwrites().collect::<Vec<_>>(), [2]);
        assert_eq!(to.unplaced().count(), 0);

        assert!(plan_with(&["--to", "pcm-drum:1=33"]).is_err());
        assert!(plan_with(&["--only", "sn-synth:1", "--to", "pcm-drum:1=5"]).is_err());
        // Effects apply only to a lone entry, loaded into Temporary.
        assert!(plan_with(&["--apply-effects"]).is_err());
        let lone = read_of(Pack {
            entries: vec![entry(ChunkType::PcmSynthTone, None)],
        });
        let apply = opts(&["--apply-effects"]).unwrap();
        let audition = plan(&lone, &[], None, &apply).unwrap();
        assert_eq!(audition.placements(), [Placement::Temporary]);
    }

    #[test]
    fn backup_entries_keep_their_slots() {
        let read = read_of(Pack {
            entries: vec![
                entry(ChunkType::SnSynthTone, Some(2)),
                entry(ChunkType::SnSynthTone, Some(5)),
            ],
        });
        assert_eq!(Origin::of(&read), Origin::Backup);
        let plan = plan(&read, &names(), None, &opts(&[]).unwrap()).unwrap();
        assert_eq!(plan.placements(), [Placement::User(2), Placement::Unplaced]);
    }

    /// The XV fixture: one Patch, converted with a lossy report.
    const UNITED: &[u8] =
        include_bytes!("../../../integral-core/tests/fixtures/foreign/xv/srx03ap2-001.syx");

    fn united_report<'a>(read: &'a PackRead, plan: &'a ImportPlan) -> Report<'a> {
        Report {
            file: Path::new("srx03ap2-001.syx"),
            format: read.format,
            pack: &read.pack,
            plan,
            names: &[],
            part: 1,
            statuses: None,
            undo: None,
            error: None,
        }
    }

    #[test]
    fn json_plan_carries_the_conversion_report() {
        let read = Pack::read(UNITED).unwrap();
        // No Expansion loaded: the tone's SRX-03 Waves warn.
        let loaded = ExpansionSlots::default();
        let plan = plan(&read, &[], Some(&loaded), &opts(&[]).unwrap()).unwrap();
        let doc = united_report(&read, &plan).json("dry-run");

        let entry = &doc["entries"][0];
        assert_eq!(entry["id"], "pcm-synth:1");
        assert_eq!(entry["name"], "United !");
        assert_eq!(entry["destination"], json!({ "temporary_part": 1 }));
        assert_eq!(
            entry["report"],
            json!([
                {
                    "type": "paramDropped",
                    "at": { "partial": 3 },
                    "param": "MFX bypass",
                    "message": "Partial 3: MFX bypass dropped",
                },
                {
                    "type": "paramDropped",
                    "at": { "partial": 4 },
                    "param": "MFX bypass",
                    "message": "Partial 4: MFX bypass dropped",
                },
                {
                    "type": "notApplied",
                    "what": "Reverb SRV HALL, level 127",
                    "message": "Reverb SRV HALL, level 127 lives in the Studio Set",
                },
            ])
        );
        assert_eq!(doc["file_report"], json!([]));
        assert_eq!(
            doc["warnings"],
            json!(["pcm-synth:1 United !: plays Waves of SRX-03, not loaded"])
        );
        assert_eq!(doc["result"], "dry-run");
    }

    #[test]
    fn text_plan_lists_items_under_their_entry_and_file_items_as_warnings() {
        let skipped = identity_request();
        let mut read = Pack::read(&[UNITED, &skipped].concat()).unwrap();
        assert_eq!(read.file_report.len(), 1);
        read.format = PackFormat::Midi;
        let plan = plan(&read, &[], None, &opts(&[]).unwrap()).unwrap();
        assert_eq!(
            united_report(&read, &plan).text_plan(),
            "srx03ap2-001.syx (MIDI file, 1 entry):\n\
             \x20 pcm-synth:1      United !         -> Temporary Part 1 [lossy · 2]\n\
             \x20     Partial 3: MFX bypass dropped\n\
             \x20     Partial 4: MFX bypass dropped\n\
             \x20     Reverb SRV HALL, level 127 lives in the Studio Set\n\
             Warning: 1 other message skipped\n"
        );
    }

    #[test]
    fn text_plan_names_the_source_model() {
        let mut read = Pack::read(UNITED).unwrap();
        read.sources[0] = Some(SourceModel::Jv1080);
        let plan = plan(&read, &[], None, &opts(&[]).unwrap()).unwrap();
        let report = united_report(&read, &plan);
        assert!(
            report
                .text_plan()
                .contains("United !         -> Temporary Part 1 (JV-1080) [lossy · 2]\n"),
            "{}",
            report.text_plan()
        );
        assert_eq!(report.json("dry-run")["entries"][0]["source"], "jv1080");
    }

    /// A message no reader takes: a Universal Identity Request.
    fn identity_request() -> Vec<u8> {
        vec![0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7]
    }

    #[test]
    fn maps_statuses_to_exit_codes() {
        use EntryStatus::*;
        assert_eq!(exit_code(&[Stored, Excluded, Stored]), 0);
        assert_eq!(exit_code(&[Loaded]), 0);
        assert_eq!(exit_code(&[Pending, Pending]), 1);
        assert_eq!(exit_code(&[Pending, Excluded]), 1);
        assert_eq!(exit_code(&[Stored, Pending]), EXIT_PARTIAL);
        assert_eq!(exit_code(&[Failed, Pending]), EXIT_PARTIAL);
    }

    /// A dry run sends the identity request, the slot name reads and the
    /// Expansion slot read: reads only, never a DT1.
    #[test]
    fn dry_run_reads_only() {
        let kinds: Vec<ChunkType> = USER_MEMORY
            .iter()
            .filter_map(|m| ChunkType::from_cli_name(m.cli_name))
            .collect();
        let requests = name_requests(0x10, &kinds);
        let slots: u16 = USER_MEMORY.iter().map(|m| m.slot_count).sum();
        assert_eq!(
            requests.iter().map(|(_, n)| *n).sum::<usize>(),
            usize::from(slots)
        );
        let expansion = integral_core::expansion_slots::build_read_request(0x10);
        for msg in requests.iter().map(|(r, _)| r).chain([&expansion]) {
            assert_eq!(msg[6], sysex::CMD_RQ1, "{msg:02X?}");
        }
    }

    #[test]
    fn undo_file_sits_next_to_the_pack() {
        let path = default_undo_path(Path::new("/x/I7SL.SVD"));
        let name = path.file_name().unwrap().to_string_lossy();
        assert_eq!(path.parent(), Some(Path::new("/x")));
        assert!(
            name.starts_with("UNDO-I7SL-") && name.ends_with(".SVD"),
            "{name}"
        );
    }
}
