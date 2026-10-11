//! `pack` commands: list a Pack file, and export User memory to one.
//!
//! The Pack codec and the export state machine live in
//! `integral-core::librarian`; this module parses arguments, talks MIDI and
//! prints. Types and their flags come from [`USER_MEMORY`].

use std::path::Path;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use clap::builder::PossibleValuesParser;
use clap::{Arg, ArgAction, ArgMatches, Args, Command, FromArgMatches, ValueEnum};
use integral_core::address::DataSize;
use integral_core::device::DeviceState;
use integral_core::device_spec::{USER_MEMORY, UserMemorySpec};
use integral_core::librarian::export::{EntryStatus, ExportAction, ExportSource, PackExport};
use integral_core::librarian::pack::{Pack, PackEntry, ToneRef};
use integral_core::svd::{ChunkType, tone_category_name};
use integral_core::{params, sysex};
use serde::Serialize;

use crate::device::IntegraDevice;

/// Output format of `pack list`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ListFormat {
    /// Human-readable text.
    Text,
    /// One JSON document.
    Json,
    /// Markdown tables, one per type.
    Markdown,
}

/// The type of a User memory spec.
pub(crate) fn kind_of(spec: &UserMemorySpec) -> ChunkType {
    ChunkType::from_cli_name(spec.cli_name).expect("every User memory type is a chunk type")
}

/// Accepts the type names of [`USER_MEMORY`].
pub fn type_parser() -> PossibleValuesParser {
    PossibleValuesParser::new(USER_MEMORY.iter().map(|m| m.cli_name))
}

/// Parse `5,7-9` into 0-indexed slots, each within `1..=count`.
fn parse_slots(list: &str, count: u16) -> Result<Vec<u16>, String> {
    let mut slots = Vec::new();
    for item in list.split(',').map(str::trim) {
        let (lo, hi) = item.split_once('-').unwrap_or((item, item));
        let num = |s: &str| {
            s.trim()
                .parse::<u16>()
                .ok()
                .filter(|n| (1..=count).contains(n))
                .ok_or_else(|| format!("`{item}`: slots are 1-{count}"))
        };
        let (lo, hi) = (num(lo)?, num(hi)?);
        if lo > hi {
            return Err(format!("`{item}`: empty range"));
        }
        slots.extend(lo - 1..hi);
    }
    Ok(slots)
}

/// The User slots picked with one `--<type> SLOTS` flag per
/// [`USER_MEMORY`] type.
#[derive(Debug, Clone, Default)]
pub struct UserSlots(pub Vec<ExportSource>);

impl FromArgMatches for UserSlots {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        let sources = USER_MEMORY
            .iter()
            .flat_map(|spec| {
                let kind = kind_of(spec);
                matches
                    .get_many::<Vec<u16>>(spec.cli_name)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .map(move |&slot| ExportSource::User { kind, slot })
            })
            .collect();
        Ok(Self(sources))
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        *self = Self::from_arg_matches(matches)?;
        Ok(())
    }
}

impl Args for UserSlots {
    fn augment_args(cmd: Command) -> Command {
        USER_MEMORY.iter().fold(cmd, |cmd, spec| {
            let count = spec.slot_count;
            cmd.arg(
                Arg::new(spec.cli_name)
                    .long(spec.cli_name)
                    .value_name("SLOTS")
                    .action(ArgAction::Append)
                    .value_parser(move |s: &str| parse_slots(s, count))
                    .help(format!(
                        "User {} slots, e.g. 5,7-9 (1-{count})",
                        kind_of(spec)
                    )),
            )
        })
    }

    fn augment_args_for_update(cmd: Command) -> Command {
        Self::augment_args(cmd)
    }
}

/// One entry as listed.
#[derive(Debug, Serialize)]
struct Listed {
    id: String,
    #[serde(rename = "type")]
    kind: &'static str,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category_name: Option<&'static str>,
    /// For a Studio Set, the User tones its Parts use, as `(part, id)`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tones: Vec<PartTone>,
}

#[derive(Debug, Serialize)]
struct PartTone {
    part: u8,
    id: String,
}

/// The `<type>:<n>` id of the `index`th (0-based) entry of `kind`.
pub(crate) fn id(kind: ChunkType, index: usize) -> String {
    format!("{}:{}", kind.cli_name(), index + 1)
}

/// The entries with their ids, filtered.
fn listed(
    pack: &Pack,
    type_filter: Option<&str>,
    category: Option<u8>,
    name: Option<&str>,
) -> Vec<Listed> {
    let name = name.map(str::to_lowercase);
    USER_MEMORY
        .iter()
        .map(kind_of)
        .filter(|k| type_filter.is_none_or(|t| t == k.cli_name()))
        .flat_map(|kind| pack.of_kind(kind).enumerate())
        .filter(|(_, e)| category.is_none() || e.category() == category)
        .filter(|(_, e)| {
            name.as_ref()
                .is_none_or(|n| e.name().to_lowercase().contains(n))
        })
        .map(|(i, e): (usize, &PackEntry)| Listed {
            id: id(e.kind(), i),
            kind: e.kind().cli_name(),
            name: e.name(),
            category: e.category(),
            category_name: e.category().and_then(tone_category_name),
            tones: (1..)
                .zip(e.tone_refs())
                .filter_map(|(part, r)| match r {
                    ToneRef::Pack { kind, index } => Some(PartTone {
                        part,
                        id: id(kind, index.into()),
                    }),
                    ToneRef::Bank { .. } => None,
                })
                .collect(),
        })
        .collect()
}

impl Listed {
    fn category_display(&self) -> String {
        match (self.category_name, self.category) {
            (Some(n), _) => n.to_string(),
            (None, Some(c)) => format!("?{c}"),
            (None, None) => String::new(),
        }
    }

    fn tones_display(&self) -> String {
        let tones: Vec<_> = self
            .tones
            .iter()
            .map(|t| format!("{}={}", t.part, t.id))
            .collect();
        tones.join(" ")
    }
}

fn render_list(entries: &[Listed], format: ListFormat) -> Result<String> {
    let mut out = String::new();
    match format {
        ListFormat::Json => out = serde_json::to_string_pretty(entries)? + "\n",
        ListFormat::Text => {
            for e in entries {
                let line = format!("{:<16} {:<16} {}", e.id, e.name, e.category_display());
                out += line.trim_end();
                out += "\n";
                if !e.tones.is_empty() {
                    out += &format!("{:16} Parts: {}\n", "", e.tones_display());
                }
            }
        }
        ListFormat::Markdown => {
            out += "| Id | Name | Category | User tones (Part=Id) |\n";
            out += "|----|------|----------|----------------------|\n";
            for e in entries {
                out += &format!(
                    "| {} | {} | {} | {} |\n",
                    e.id,
                    e.name,
                    e.category_display(),
                    e.tones_display()
                );
            }
        }
    }
    Ok(out)
}

/// `pack list`: print a Pack file's entries.
pub fn list(
    path: &Path,
    type_filter: Option<&str>,
    category: Option<u8>,
    name: Option<&str>,
    format: ListFormat,
) -> Result<()> {
    let bytes =
        std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let pack =
        Pack::from_svd(&bytes).with_context(|| format!("failed to parse {}", path.display()))?;
    print!(
        "{}",
        render_list(&listed(&pack, type_filter, category, name), format)?
    );
    Ok(())
}

/// The tone type of Part `part`'s (0-indexed) current tone.
fn part_tone_kind(dev: &mut IntegraDevice, part: u8) -> Result<ChunkType> {
    let addr = params::part_address(part, params::part::TONE_BANK_MSB);
    let msb = dev.request_data(&addr, &DataSize::ONE)?[0];
    let tone_type = params::tone_type_from_bank_msb(msb);
    USER_MEMORY
        .iter()
        .find(|m| m.tone_type.is_some() && m.tone_type == tone_type)
        .map(kind_of)
        .with_context(|| {
            format!(
                "Part {}'s tone (bank MSB {msb}) has no known type",
                part + 1
            )
        })
}

/// Sends through `DeviceState`'s throttled queue, raw so that DT1s to the
/// same address are not coalesced.
pub(crate) struct Throttle {
    queue: DeviceState,
    start: Instant,
}

impl Throttle {
    pub(crate) fn new(device_id: u8) -> Self {
        Self {
            queue: DeviceState::new(device_id),
            start: Instant::now(),
        }
    }

    /// Send `bytes` once the throttle allows.
    pub(crate) fn send(&mut self, dev: &mut IntegraDevice, bytes: Vec<u8>) -> Result<()> {
        self.queue.send_raw("pack", bytes);
        loop {
            match self
                .queue
                .drain(self.start.elapsed().as_secs_f64() * 1000.0)
            {
                Some(bytes) => return dev.conn_out_mut().send(&bytes).context("failed to send"),
                None => std::thread::sleep(Duration::from_millis(2)),
            }
        }
    }
}

/// Drive `export` to its Pack: send each RQ1, feed the DT1s back, and time
/// out a block after `dev.timeout()` of silence.
pub(crate) fn run_export(
    dev: &mut IntegraDevice,
    throttle: &mut Throttle,
    export: &mut PackExport,
) -> Result<Pack> {
    let mut deadline = Instant::now();
    let mut reported = Vec::new();
    loop {
        let action = export.next_action();
        for (source, status) in export.progress() {
            if status == EntryStatus::Read && !reported.contains(&source) {
                eprintln!("Read {source}");
                reported.push(source);
            }
        }
        match action {
            ExportAction::Done(result) => return Ok(result?),
            ExportAction::Read(rq1) => {
                throttle.send(dev, rq1)?;
                deadline = Instant::now() + dev.timeout();
            }
            ExportAction::Wait => {
                match dev
                    .rx()
                    .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                {
                    Ok(raw) => {
                        if let Ok(dt1) = sysex::parse_dt1(&raw)
                            && export.on_read(&dt1.address, &dt1.data)
                        {
                            deadline = Instant::now() + dev.timeout();
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => export.on_timeout(),
                    Err(RecvTimeoutError::Disconnected) => bail!("MIDI input disconnected"),
                }
            }
        }
    }
}

/// `pack export`: read the chosen User slots (and Temporary sources) into
/// a Pack and write it to `out`. Only reads from the device.
#[allow(clippy::too_many_arguments)]
pub fn export(
    port_pattern: &str,
    device_id: Option<u8>,
    timeout: Duration,
    out: &Path,
    slots: UserSlots,
    current_part: Option<u8>,
    current_set: bool,
    force: bool,
) -> Result<()> {
    if out.exists() && !force {
        bail!("{} exists; pass --force to overwrite it", out.display());
    }
    let mut sources = slots.0;
    if current_set {
        sources.push(ExportSource::StudioSet);
    }
    if sources.is_empty() && current_part.is_none() {
        bail!("nothing to export: pick User slots, --current-part or --current-set");
    }
    let mut dev = IntegraDevice::connect_detecting(port_pattern, device_id, timeout)?.quiet();
    if let Some(n) = current_part {
        let part = n - 1;
        let kind = part_tone_kind(&mut dev, part)?;
        sources.push(ExportSource::PartTone { kind, part });
    }
    let mut export = PackExport::new(dev.device_id(), sources)?;
    let mut throttle = Throttle::new(dev.device_id());
    let pack = run_export(&mut dev, &mut throttle, &mut export)?;
    let bytes = pack.to_svd()?;
    std::fs::write(out, &bytes).with_context(|| format!("failed to write {}", out.display()))?;
    println!(
        "Exported {} entries to {}",
        pack.entries.len(),
        out.display()
    );
    for source in export.auto_included() {
        println!("Added for a Studio Set: {source}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use integral_core::parse_helpers::encode_ascii_name;

    use super::*;

    fn entry(kind: ChunkType, name: &str) -> PackEntry {
        let spec = USER_MEMORY
            .iter()
            .find(|m| m.cli_name == kind.cli_name())
            .unwrap();
        let mut blocks: Vec<Vec<u8>> = spec
            .blocks(spec.temporary_base(0))
            .map(|(_, s)| vec![0; s.value() as usize])
            .collect();
        let len = kind.name_length().unwrap();
        blocks[0][..len].copy_from_slice(&encode_ascii_name(name, len));
        PackEntry::new(kind, None, blocks).unwrap()
    }

    fn pack() -> Pack {
        let mut set = entry(ChunkType::StudioSet, "My Set");
        let tone = |index| ToneRef::Pack {
            kind: ChunkType::SnSynthTone,
            index,
        };
        set.set_tone_ref(0, tone(1)).unwrap();
        set.set_tone_ref(4, tone(0)).unwrap();
        Pack {
            entries: vec![
                set,
                entry(ChunkType::SnSynthTone, "Lead"),
                entry(ChunkType::SnSynthTone, "Pad"),
                entry(ChunkType::PcmDrumKit, "Kit"),
            ],
        }
    }

    #[test]
    fn parses_slot_lists() {
        assert_eq!(parse_slots("5,7-9", 64), Ok(vec![4, 6, 7, 8]));
        assert_eq!(parse_slots("1", 1), Ok(vec![0]));
        for bad in ["0", "65", "9-7", "x", "1-", ""] {
            assert!(parse_slots(bad, 64).is_err(), "{bad}");
        }
    }

    #[test]
    fn slot_flags_come_from_the_spec() {
        let cmd = UserSlots::augment_args(Command::new("t"));
        let m = cmd
            .try_get_matches_from([
                "t",
                "--sn-synth",
                "1-2",
                "--studio-set",
                "3",
                "--sn-synth",
                "5",
            ])
            .unwrap();
        let sns = |slot| ExportSource::User {
            kind: ChunkType::SnSynthTone,
            slot,
        };
        let set = ExportSource::User {
            kind: ChunkType::StudioSet,
            slot: 2,
        };
        assert_eq!(
            UserSlots::from_arg_matches(&m).unwrap().0,
            [set, sns(0), sns(1), sns(4)]
        );
        let cmd = UserSlots::augment_args(Command::new("t"));
        assert!(cmd.try_get_matches_from(["t", "--pcm-drum", "33"]).is_err());
    }

    #[test]
    fn lists_ids_and_set_references() {
        let text = render_list(&listed(&pack(), None, None, None), ListFormat::Text).unwrap();
        assert_eq!(
            text,
            "studio-set:1     My Set\n\
             \x20                Parts: 1=sn-synth:2 5=sn-synth:1\n\
             sn-synth:1       Lead             No assign\n\
             sn-synth:2       Pad              No assign\n\
             pcm-drum:1       Kit\n"
        );
    }

    #[test]
    fn filters_keep_ids() {
        let entries = listed(&pack(), Some("sn-synth"), None, Some("pa"));
        let ids: Vec<_> = entries.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["sn-synth:2"]);
        let json = render_list(&entries, ListFormat::Json).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(doc[0]["type"], "sn-synth");
        assert_eq!(doc[0]["name"], "Pad");
    }

    #[test]
    fn lists_a_written_file() {
        let bytes = pack().to_svd().unwrap();
        let read = Pack::from_svd(&bytes).unwrap();
        let entries = listed(&read, Some("studio-set"), None, None);
        assert_eq!(entries[0].tones_display(), "1=sn-synth:2 5=sn-synth:1");
    }
}
