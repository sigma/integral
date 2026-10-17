//! Integration tests for `integral pack check`: run the binary offline over
//! the Foreign patch fixtures of `integral-core`.

use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::{Value, json};

/// `crates/integral-core/tests/fixtures/foreign`.
fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../integral-core/tests/fixtures/foreign")
}

fn pack_check(args: &[&str], paths: &[PathBuf]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_integral"))
        .args(["pack", "check"])
        .args(args)
        .args(paths)
        .output()
        .expect("failed to run integral")
}

fn json_check(paths: &[PathBuf]) -> (Output, Value) {
    let out = pack_check(&["--format", "json"], paths);
    let value = serde_json::from_slice(&out.stdout).expect("JSON output");
    (out, value)
}

#[test]
fn totals_the_fixtures_and_skips_what_is_not_a_pack() {
    let (out, check) = json_check(&[fixtures()]);
    assert!(out.status.success(), "{out:?}");
    let totals = &check["totals"];
    // The JV and XV `.syx` fixtures read; their `SOURCE.md` files don't.
    assert_eq!(totals["files"], 2);
    assert_eq!(totals["skippedFiles"], 2);
    assert_eq!(totals["unreadablePaths"], 0);
    assert_eq!(
        totals["skippedReasons"],
        json!([{ "item": "not an SVD, SysEx or MIDI file", "count": 2 }])
    );
    // 4 JV-1080 Patches and 1 XV-5080 Patch. Two JV Patches only miss
    // Studio Set settings (`notApplied`), so they are clean.
    assert_eq!(totals["entries"], 5);
    assert_eq!(totals["lossy"], 3);
    assert_eq!(totals["cleanPercent"], 40.0);
    assert_eq!(
        totals["unmappedWaves"],
        json!([{ "item": { "group": { "type": "PcmCard", "id": 3 }, "number": 30 }, "count": 1 }])
    );
    // Every fixture effect converts.
    assert_eq!(totals["droppedEffects"], json!([]));
    assert_eq!(
        totals["itemKinds"][0],
        json!({ "item": "notApplied", "count": 9 })
    );

    // Per file, in path order, with each tone's full report.
    let files = check["files"].as_array().unwrap();
    let counts: Vec<_> = files
        .iter()
        .map(|f| {
            (
                f["entries"].clone(),
                f["lossy"].clone(),
                f["rejected"].clone(),
            )
        })
        .collect();
    assert_eq!(
        counts,
        [
            (json!(4), json!(2), json!(0)),
            (json!(1), json!(1), json!(0))
        ]
    );
    // Clean and lossy per source model, the unknown (XV) last.
    assert_eq!(
        totals["models"],
        json!([
            { "model": "jv1080", "entries": 4, "clean": 2, "lossy": 2 },
            { "model": null, "entries": 1, "clean": 0, "lossy": 1 },
        ])
    );
    // A JV Patch tells its model; an XV one doesn't.
    assert_eq!(files[0]["tones"][0]["source"], "jv1080");
    let united = &files[1]["tones"][0];
    assert_eq!(united["source"], Value::Null);
    assert_eq!(united["id"], "pcm-synth:1");
    assert_eq!(united["name"], "United !");
    assert_eq!(united["report"][0]["type"], "paramDropped");
}

#[test]
fn prints_files_and_totals_as_text() {
    let out = pack_check(&[], &[fixtures().join("xv/srx03ap2-001.syx")]);
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "ENTRIES  CLEAN  LOSSY  REJECTED  FILE");
    let fields: Vec<&str> = lines[1].split_whitespace().collect();
    assert_eq!(fields[..4], ["1", "0", "1", "0"]);
    assert!(fields[4].ends_with("srx03ap2-001.syx"), "{}", lines[1]);
    assert!(
        text.contains("Files: 1 read, 0 skipped, 0 unreadable"),
        "{text}"
    );
    assert!(
        text.contains("Entries: 1, clean 0 (0.0%), lossy 1; rejected 0"),
        "{text}"
    );
    assert!(
        text.contains("      1  unknown model: clean 0, lossy 1\n"),
        "{text}"
    );
}

#[test]
fn fails_only_when_a_path_cannot_be_read() {
    let missing = fixtures().join("missing");
    let (out, check) = json_check(&[missing, fixtures().join("xv")]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot read"));
    // The readable paths are still checked.
    assert_eq!(check["totals"]["unreadablePaths"], 1);
    assert_eq!(check["totals"]["files"], 1);
}
