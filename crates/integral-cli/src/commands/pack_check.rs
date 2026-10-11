//! `pack check`: read Pack files offline and total their Conversion
//! reports, to measure how much of a corpus of Foreign patches converts
//! cleanly. No device and no MIDI port.
//!
//! Files that `Pack::read` rejects are skipped and counted. Only a path
//! that can't be read at all is an error.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::hash::Hash;
use std::path::{Path, PathBuf};

use anyhow::Result;
use integral_core::librarian::foreign::wave_map::ForeignWave;
use integral_core::librarian::foreign::{
    ConversionItem, ConversionReport, FileItem, ForeignEffect, PackRead, Skipped, SourceModel,
    lossy_count,
};
use integral_core::librarian::pack::Pack;
use serde::Serialize;

use super::expansion::OutputFormat;
use super::pack::entry_id;

/// One converted entry and its Conversion report.
#[derive(Debug, Serialize)]
struct Tone {
    id: String,
    name: String,
    /// The model the entry was made on, when its bytes tell.
    source: Option<SourceModel>,
    report: ConversionReport,
}

/// One file `Pack::read` accepted.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileCheck {
    path: String,
    entries: usize,
    clean: usize,
    lossy: usize,
    /// Foreign patches and entries the file holds but that don't convert:
    /// the [`FileItem::Skipped`] units that would have been entries.
    rejected: usize,
    file_report: Vec<FileItem>,
    tones: Vec<Tone>,
}

/// A file `Pack::read` rejected, or a path that can't be read.
#[derive(Debug, Serialize)]
struct PathError {
    path: String,
    reason: String,
}

/// How often something occurs.
#[derive(Debug, Serialize)]
struct Counted<T> {
    item: T,
    count: usize,
}

/// The clean and lossy entries made on one source model.
#[derive(Debug, Serialize)]
struct ModelCount {
    /// The model, or `None` when the bytes don't tell.
    model: Option<SourceModel>,
    entries: usize,
    clean: usize,
    lossy: usize,
}

/// The totals over every file read.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Totals {
    files: usize,
    skipped_files: usize,
    unreadable_paths: usize,
    /// Skipped files per reason, most frequent first.
    skipped_reasons: Vec<Counted<String>>,
    entries: usize,
    clean: usize,
    lossy: usize,
    rejected: usize,
    /// Clean entries out of all entries, or `None` without entries.
    clean_percent: Option<f64>,
    /// Entries per source model, in model order, the unknown last.
    models: Vec<ModelCount>,
    /// The top unmapped Waves, by the number of partials using them.
    unmapped_waves: Vec<Counted<ForeignWave>>,
    /// The top dropped effects, by the number of tones using them.
    dropped_effects: Vec<Counted<ForeignEffect>>,
    /// Every Conversion report item kind, most frequent first.
    item_kinds: Vec<Counted<String>>,
}

/// The whole check.
#[derive(Debug, Serialize)]
struct Check {
    files: Vec<FileCheck>,
    skipped: Vec<PathError>,
    unreadable: Vec<PathError>,
    totals: Totals,
}

/// Every file under `path` (or `path` itself), in name order. Paths that
/// can't be read go to `unreadable`.
fn collect(path: &Path, files: &mut Vec<PathBuf>, unreadable: &mut Vec<PathError>) {
    let mut error = |e: std::io::Error| {
        unreadable.push(PathError {
            path: path.display().to_string(),
            reason: e.to_string(),
        });
    };
    let meta = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(e) => return error(e),
    };
    if !meta.is_dir() {
        files.push(path.to_path_buf());
        return;
    }
    let children: std::io::Result<Vec<PathBuf>> =
        std::fs::read_dir(path).and_then(|dir| dir.map(|e| e.map(|e| e.path())).collect());
    match children {
        Ok(mut children) => {
            children.sort();
            for child in children {
                collect(&child, files, unreadable);
            }
        }
        Err(e) => error(e),
    }
}

/// Whether a skipped unit would have been a Pack entry.
fn is_rejected(kind: Skipped) -> bool {
    matches!(
        kind,
        Skipped::Patch | Skipped::Rhythm | Skipped::MultiPartial | Skipped::Incomplete
    )
}

/// Check one file that `Pack::read` accepted.
fn check_file(path: &Path, read: PackRead) -> FileCheck {
    let PackRead {
        pack,
        file_report,
        reports,
        sources,
        ..
    } = read;
    let tones: Vec<Tone> = reports
        .into_iter()
        .zip(sources)
        .enumerate()
        .map(|(i, (report, source))| Tone {
            id: entry_id(&pack, i),
            name: pack.entries[i].name(),
            source,
            report,
        })
        .collect();
    let clean = tones.iter().filter(|t| lossy_count(&t.report) == 0).count();
    let rejected = file_report
        .iter()
        .map(|item| match *item {
            FileItem::Skipped { kind, count } if is_rejected(kind) => count,
            _ => 0,
        })
        .sum();
    FileCheck {
        path: path.display().to_string(),
        entries: tones.len(),
        clean,
        lossy: tones.len() - clean,
        rejected,
        file_report,
        tones,
    }
}

/// `counts` as a list, most frequent first, ties in label order, cut to
/// `top`.
fn ranked<T: Hash + Eq>(
    counts: HashMap<T, usize>,
    label: impl Fn(&T) -> String,
    top: usize,
) -> Vec<Counted<T>> {
    let mut list: Vec<(String, Counted<T>)> = counts
        .into_iter()
        .map(|(item, count)| (label(&item), Counted { item, count }))
        .collect();
    list.sort_by(|(a, x), (b, y)| y.count.cmp(&x.count).then_with(|| a.cmp(b)));
    list.into_iter().take(top).map(|(_, c)| c).collect()
}

/// The kind of a Conversion report item: its JSON `type` tag.
fn kind(item: &ConversionItem) -> String {
    serde_json::to_value(item)
        .ok()
        .and_then(|v| Some(v.get("type")?.as_str()?.to_owned()))
        .unwrap_or_default()
}

/// The totals of `files`, with `top` unmapped Waves and dropped effects.
fn totals(files: &[FileCheck], skipped: &[PathError], unreadable: usize, top: usize) -> Totals {
    let mut waves = HashMap::new();
    let mut effects = HashMap::new();
    let mut kinds = HashMap::new();
    let mut models: Vec<ModelCount> = Vec::new();
    for tone in files.iter().flat_map(|f| &f.tones) {
        let at = match models.iter().position(|m| m.model == tone.source) {
            Some(at) => at,
            None => {
                models.push(ModelCount {
                    model: tone.source,
                    entries: 0,
                    clean: 0,
                    lossy: 0,
                });
                models.len() - 1
            }
        };
        let m = &mut models[at];
        m.entries += 1;
        if lossy_count(&tone.report) == 0 {
            m.clean += 1;
        } else {
            m.lossy += 1;
        }
    }
    models.sort_by_key(|m| (m.model.is_none(), m.model));
    for item in files.iter().flat_map(|f| &f.tones).flat_map(|t| &t.report) {
        *kinds.entry(kind(item)).or_default() += 1;
        match item {
            ConversionItem::WaveUnmapped { source, .. } => *waves.entry(*source).or_default() += 1,
            ConversionItem::EffectDropped { source } => *effects.entry(*source).or_default() += 1,
            _ => {}
        }
    }
    let mut reasons = HashMap::new();
    for s in skipped {
        *reasons.entry(s.reason.clone()).or_default() += 1;
    }
    let sum = |field: fn(&FileCheck) -> usize| files.iter().map(field).sum::<usize>();
    let (entries, clean) = (sum(|f| f.entries), sum(|f| f.clean));
    Totals {
        files: files.len(),
        skipped_files: skipped.len(),
        unreadable_paths: unreadable,
        skipped_reasons: ranked(reasons, Clone::clone, usize::MAX),
        entries,
        clean,
        lossy: entries - clean,
        rejected: sum(|f| f.rejected),
        clean_percent: (entries > 0).then(|| 100.0 * clean as f64 / entries as f64),
        models,
        unmapped_waves: ranked(waves, ToString::to_string, top),
        dropped_effects: ranked(effects, ToString::to_string, top),
        item_kinds: ranked(kinds, Clone::clone, usize::MAX),
    }
}

/// Read every file under `paths` and check it.
fn check(paths: &[PathBuf], top: usize) -> Check {
    let mut files = Vec::new();
    let mut unreadable = Vec::new();
    for path in paths {
        collect(path, &mut files, &mut unreadable);
    }
    let mut checked = Vec::new();
    let mut skipped = Vec::new();
    for path in files {
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) => {
                unreadable.push(PathError {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                });
                continue;
            }
        };
        match Pack::read(&bytes) {
            Ok(read) => checked.push(check_file(&path, read)),
            Err(e) => skipped.push(PathError {
                path: path.display().to_string(),
                reason: e.to_string(),
            }),
        }
    }
    let totals = totals(&checked, &skipped, unreadable.len(), top);
    Check {
        files: checked,
        skipped,
        unreadable,
        totals,
    }
}

/// A ranked list under `title`, one `count  label` line each.
fn render_ranked<T>(
    out: &mut String,
    title: &str,
    list: &[Counted<T>],
    label: impl Fn(&T) -> String,
) {
    let _ = writeln!(out, "\n{title}:");
    if list.is_empty() {
        out.push_str("  none\n");
    }
    for c in list {
        let _ = writeln!(out, "{:>7}  {}", c.count, label(&c.item));
    }
}

fn render_text(check: &Check) -> String {
    let mut out = String::new();
    if !check.files.is_empty() {
        out.push_str("ENTRIES  CLEAN  LOSSY  REJECTED  FILE\n");
    }
    for f in &check.files {
        let _ = writeln!(
            out,
            "{:>7}  {:>5}  {:>5}  {:>8}  {}",
            f.entries, f.clean, f.lossy, f.rejected, f.path
        );
        for item in &f.file_report {
            let _ = writeln!(out, "{:36}- {item}", "");
        }
    }
    let t = &check.totals;
    let _ = writeln!(
        out,
        "\nFiles: {} read, {} skipped, {} unreadable",
        t.files, t.skipped_files, t.unreadable_paths
    );
    for r in &t.skipped_reasons {
        let _ = writeln!(out, "{:>7}  skipped: {}", r.count, r.item);
    }
    let percent = t
        .clean_percent
        .map_or_else(|| "-".to_owned(), |p| format!("{p:.1}%"));
    let _ = writeln!(
        out,
        "Entries: {}, clean {} ({percent}), lossy {}; rejected {}",
        t.entries, t.clean, t.lossy, t.rejected
    );
    for m in &t.models {
        let model = m
            .model
            .map_or_else(|| "unknown model".to_owned(), |m| m.to_string());
        let _ = writeln!(
            out,
            "{:>7}  {model}: clean {}, lossy {}",
            m.entries, m.clean, m.lossy
        );
    }
    render_ranked(
        &mut out,
        "Top unmapped Waves (partials)",
        &t.unmapped_waves,
        ToString::to_string,
    );
    render_ranked(
        &mut out,
        "Top dropped effects (tones)",
        &t.dropped_effects,
        ToString::to_string,
    );
    render_ranked(&mut out, "Item kinds", &t.item_kinds, Clone::clone);
    out
}

/// `pack check`: print the check of every file under `paths`. Returns the
/// exit code: 0, or 1 when a path can't be read.
pub fn run(paths: &[PathBuf], top: usize, format: OutputFormat) -> Result<u8> {
    let check = check(paths, top);
    for e in &check.unreadable {
        eprintln!("Error: cannot read {}: {}", e.path, e.reason);
    }
    match format {
        OutputFormat::Text => print!("{}", render_text(&check)),
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&check)?),
    }
    Ok(u8::from(!check.unreadable.is_empty()))
}

#[cfg(test)]
mod tests {
    use integral_core::librarian::foreign::PackFormat;

    use super::*;

    #[test]
    fn counts_rejected_units_but_not_other_skipped_messages() {
        let read = PackRead {
            file_report: vec![
                FileItem::Skipped {
                    kind: Skipped::Rhythm,
                    count: 3,
                },
                FileItem::Skipped {
                    kind: Skipped::MultiPartial,
                    count: 2,
                },
                FileItem::Skipped {
                    kind: Skipped::System,
                    count: 1,
                },
                FileItem::Skipped {
                    kind: Skipped::Other,
                    count: 9,
                },
                FileItem::Duplicates { count: 4 },
            ],
            format: PackFormat::Sysex,
            pack: Pack::default(),
            reports: vec![],
            sources: vec![],
        };
        let file = check_file(Path::new("x.syx"), read);
        assert_eq!((file.entries, file.rejected), (0, 5));
    }

    #[test]
    fn an_entry_with_only_not_applied_items_is_clean() {
        let read = Pack::read(include_bytes!(
            "../../../integral-core/tests/fixtures/foreign/xv/srx03ap2-001.syx"
        ))
        .unwrap();
        let mut not_applied = read.clone();
        not_applied.reports[0].retain(|i| matches!(i, ConversionItem::NotApplied { .. }));
        assert!(!not_applied.reports[0].is_empty());
        let file = check_file(Path::new("x.syx"), not_applied);
        assert_eq!((file.clean, file.lossy), (1, 0));
        let file = check_file(Path::new("x.syx"), read);
        assert_eq!((file.clean, file.lossy), (0, 1));
    }

    #[test]
    fn ranks_by_count_then_label_and_keeps_the_top() {
        let counts = HashMap::from([("b", 2), ("a", 2), ("c", 5), ("d", 1)]);
        let ranked: Vec<_> = ranked(counts, |s| (*s).to_owned(), 3)
            .into_iter()
            .map(|c| (c.item, c.count))
            .collect();
        assert_eq!(ranked, [("c", 5), ("a", 2), ("b", 2)]);
    }

    #[test]
    fn names_item_kinds_by_their_json_tag() {
        let item = ConversionItem::NotApplied {
            what: "Tempo 90".into(),
        };
        assert_eq!(kind(&item), "notApplied");
    }
}
