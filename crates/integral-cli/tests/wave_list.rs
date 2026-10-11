//! Integration tests for `integral wave list`: run the binary offline.

use std::process::{Command, Output};

use integral_core::waves::{self, WaveGroup};
use serde_json::Value;

fn wave_list(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_integral"))
        .args(["wave", "list"])
        .args(args)
        .output()
        .expect("failed to run integral")
}

fn stdout(args: &[&str]) -> String {
    let out = wave_list(args);
    assert!(out.status.success(), "{args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

fn fields(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}

#[test]
fn no_argument_lists_every_group_with_its_count() {
    let text = stdout(&[]);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), WaveGroup::ALL.len());

    let int_count = waves::waves(WaveGroup::Int).len().to_string();
    assert_eq!(fields(lines[0]), ["INT", int_count.as_str()]);

    let srx07_count = waves::waves(WaveGroup::Srx(7)).len().to_string();
    let srx07 = lines.iter().find(|l| l.starts_with("SRX-07")).unwrap();
    assert_eq!(
        fields(srx07),
        ["SRX-07", srx07_count.as_str(), "Ultimate", "Keys"]
    );

    assert!(lines[lines.len() - 1].starts_with("SN-Synth"));
}

#[test]
fn group_lists_padded_numbers_and_names() {
    let text = stdout(&["srx-07"]);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), waves::waves(WaveGroup::Srx(7)).len());
    assert_eq!(lines[0], "0001 StGrand L pA");
    assert_eq!(lines[1], "0002 StGrand R pA");
}

#[test]
fn all_prefixes_each_wave_with_its_group() {
    let text = stdout(&["--all"]);
    let total: usize = WaveGroup::ALL.iter().map(|&g| waves::waves(g).len()).sum();
    assert_eq!(text.lines().count(), total);

    let grand: Vec<&str> = text.lines().filter(|l| l.contains("StGrand")).collect();
    assert!(grand.contains(&"INT 0001 StGrand pA L"), "{grand:?}");
    assert!(grand.contains(&"SRX-07 0001 StGrand L pA"), "{grand:?}");
}

#[test]
fn json_documents() {
    let doc: Value = serde_json::from_str(&stdout(&["--format", "json"])).unwrap();
    let groups = doc["groups"].as_array().unwrap();
    assert_eq!(groups.len(), WaveGroup::ALL.len());
    assert_eq!(groups[0]["id"], "INT");
    assert!(groups[0]["title"].is_null());
    assert_eq!(groups[7]["id"], "SRX-07");
    assert_eq!(groups[7]["title"], "Ultimate Keys");
    assert_eq!(groups[7]["waves"], waves::waves(WaveGroup::Srx(7)).len());

    let doc: Value = serde_json::from_str(&stdout(&["INT", "--format", "json"])).unwrap();
    assert_eq!(doc["group"], "INT");
    assert_eq!(doc["waves"][0]["number"], 1);
    assert_eq!(doc["waves"][0]["name"], "StGrand pA L");

    let doc: Value = serde_json::from_str(&stdout(&["--all", "--format", "json"])).unwrap();
    assert_eq!(doc["groups"][13]["group"], "SN-Synth");
    assert_eq!(
        doc["groups"][13]["waves"].as_array().unwrap().len(),
        waves::waves(WaveGroup::SnSynth).len()
    );
}

#[test]
fn unknown_group_fails_and_lists_valid_ids() {
    let out = wave_list(&["SRX-13"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("unknown Wave Group `SRX-13`"), "{err}");
    for id in WaveGroup::ALL.map(WaveGroup::id) {
        assert!(err.contains(id), "{id} missing from {err}");
    }
}
