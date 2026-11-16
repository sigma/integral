//! Integration tests for `integral init`'s argument checks: each bad
//! argument fails before a port is opened, so they run offline. The port
//! pattern matches no device, in case a check were to slip.

use std::process::{Command, Output};

const NO_PORT: &str = "--port=integral-test-no-such-port";

fn init(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_integral"))
        .args(["init", NO_PORT])
        .args(args)
        .output()
        .expect("failed to run integral")
}

/// Run `init` with `args`, expect it to fail and return its stderr.
fn refused(args: &[&str]) -> String {
    let out = init(args);
    assert!(!out.status.success(), "{args:?}: {out:?}");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(!stderr.contains("Using MIDI port"), "{args:?}: {stderr}");
    stderr
}

#[test]
fn part_targets_need_a_part() {
    for target in ["part", "tone", "partial"] {
        let stderr = refused(&[target]);
        assert!(stderr.contains("needs --part N"), "{target}: {stderr}");
    }
}

#[test]
fn whole_set_targets_refuse_a_part() {
    for target in ["studio-set", "sound-ctrl"] {
        let stderr = refused(&[target, "--part", "1"]);
        assert!(stderr.contains("takes no --part"), "{target}: {stderr}");
    }
}

#[test]
fn part_is_range_checked() {
    for part in ["0", "17"] {
        let stderr = refused(&["part", "--part", part]);
        assert!(stderr.contains("--part"), "{part}: {stderr}");
    }
}

#[test]
fn partial_and_key_exclude_each_other() {
    let stderr = refused(&["partial", "--part", "16", "--partial", "1", "--key", "36"]);
    assert!(stderr.contains("cannot be used with"), "{stderr}");
}

#[test]
fn partial_needs_a_partial_or_a_key() {
    let stderr = refused(&["partial", "--part", "16"]);
    assert!(
        stderr.contains("--partial N") && stderr.contains("--key N"),
        "{stderr}"
    );
}

#[test]
fn key_only_applies_to_partial() {
    let stderr = refused(&["tone", "--part", "16", "--key", "36"]);
    assert!(stderr.contains("only apply to init partial"), "{stderr}");
}

#[test]
fn unknown_target_is_refused() {
    let stderr = refused(&["everything"]);
    assert!(stderr.contains("invalid value"), "{stderr}");
}
