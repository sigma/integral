//! The host parameter lists: snapshots of each list's (ID, min, max) so a
//! change shows in review (ADR 0008), and the properties a host relies on.
//!
//! After an intended change, regenerate the snapshots with
//! `just regen-host-params` and review the diff: a changed ID or range moves
//! saved automation. The hashing of the IDs is tested in `integral-vst`,
//! through nih-plug's own wrapper.

use std::collections::HashSet;
use std::path::PathBuf;

use integral_core::fx_params::{
    CHORUS_TYPE_HOST_NAMES, FxParamDef, REVERB_TYPE_HOST_NAMES, chorus_params, reverb_params,
};
use integral_core::host_params::{PluginKind, snapshot};
use integral_core::mfx::MFX_PARAM_COUNT;
use integral_core::mfx_params::{MFX_TYPE_NAMES, mfx_type_def};

/// The checked-in snapshot of `kind`'s list.
fn snapshot_path(kind: PluginKind) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots/host_params")
        .join(format!("{kind:?}.txt"))
}

#[test]
fn lists_match_their_snapshots() {
    for kind in PluginKind::ALL {
        let actual = snapshot(&kind.host_params());
        let path = snapshot_path(kind);
        let expected = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{}: {e}; run just regen-host-params", path.display()));
        if let Some((line, (want, got))) = expected
            .lines()
            .zip(actual.lines())
            .enumerate()
            .find(|(_, (want, got))| want != got)
        {
            panic!(
                "{kind:?}: line {} differs: snapshot {want:?}, list {got:?}; run just regen-host-params and review the diff",
                line + 1
            );
        }
        assert_eq!(
            expected.lines().count(),
            actual.lines().count(),
            "{kind:?}: the list and its snapshot differ in length; run just regen-host-params and review the diff"
        );
    }
}

#[test]
fn ids_and_names_are_unique_within_a_list() {
    for kind in PluginKind::ALL {
        let list = kind.host_params();
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        for p in &list {
            assert!(ids.insert(&p.id), "{kind:?}: ID {} twice", p.id);
            assert!(names.insert(&p.name), "{kind:?}: name {} twice", p.name);
        }
    }
}

#[test]
fn ranges_and_defaults_are_consistent_and_format() {
    for kind in PluginKind::ALL {
        for p in kind.host_params() {
            assert!(p.min < p.max, "{kind:?} {}: {}..={}", p.id, p.min, p.max);
            assert!(
                (p.min..=p.max).contains(&p.default),
                "{kind:?} {}: default {} outside {}..={}",
                p.id,
                p.default,
                p.min,
                p.max
            );
            for raw in [p.min, p.default, p.max] {
                assert!(!p.format.format(raw).is_empty(), "{kind:?} {}", p.id);
            }
            assert!(!p.group.is_empty(), "{kind:?} {}", p.id);
            assert!(
                p.group.split('/').all(|segment| !segment.trim().is_empty()),
                "{kind:?} {}: group {:?}",
                p.id,
                p.group
            );
        }
    }
}

/// Whether an ID is an effect type's parameter (`mfx/<type>/<slot>`), as
/// opposed to a fixed field (`mfx/head/<offset>`).
fn is_effect_type_param(id: &str) -> bool {
    id.split('/')
        .nth(1)
        .is_some_and(|second| second.chars().all(|c| c.is_ascii_digit()))
}

/// ADR 0007's sizes count the MFX as the device's 32 generic slots; here
/// every type's parameters are listed instead.
#[test]
fn sizes_follow_adr_0007() {
    let tone_size = |kind: PluginKind| {
        let list = kind.host_params();
        let effect = list.iter().filter(|p| is_effect_type_param(&p.id)).count();
        list.len() - effect + MFX_PARAM_COUNT
    };
    assert_eq!(tone_size(PluginKind::SnSynth), 233);
    assert_eq!(tone_size(PluginKind::SnAcoustic), 96);
    assert_eq!(tone_size(PluginKind::PcmSynth), 705);
    let key = |kind: PluginKind| {
        kind.host_params()
            .iter()
            .filter(|p| p.id.contains("/key/"))
            .count()
    };
    assert_eq!(key(PluginKind::PcmDrum), 143);
}

#[test]
fn ids_are_relative_to_the_binding() {
    for kind in PluginKind::ALL {
        for p in kind.host_params() {
            assert!(
                !p.id.starts_with("part") || kind == PluginKind::StudioSet,
                "{}",
                p.id
            );
            if kind != PluginKind::StudioSet {
                assert!(!p.name.contains("Part "), "{}", p.name);
            }
        }
    }
    let id_of = |kind: PluginKind, name: &str| {
        kind.host_params()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{kind:?}: no {name}"))
            .id
    };
    assert_eq!(id_of(PluginKind::StudioSet, "Part 7 Level"), "part07/0x09");
    assert_eq!(
        id_of(PluginKind::StudioSet, "Part 7 EQ Switch"),
        "part07/eq/0x00"
    );
    assert_eq!(id_of(PluginKind::StudioSet, "Solo Part"), "common/0x3F");
    assert_eq!(
        id_of(PluginKind::StudioSet, "Chorus Type"),
        "chorus/head/0x00"
    );
    assert_eq!(
        id_of(PluginKind::StudioSet, "Reverb Type"),
        "reverb/head/0x00"
    );
    assert_eq!(
        id_of(PluginKind::StudioSet, "Master EQ Mid Q"),
        "mastereq/0x04"
    );
    assert_eq!(
        id_of(PluginKind::PcmSynth, "Partial 2 Level"),
        "pcms/partial2/0x00"
    );
    let kit = PluginKind::PcmDrum.host_params();
    assert!(kit.iter().any(|p| p.id.starts_with("pcmd/key/")));
    assert!(
        kit.iter()
            .all(|p| !p.name.starts_with("Key ") || p.id.contains("/key/"))
    );
}

#[test]
fn the_studio_set_has_no_system_parameter() {
    let list = PluginKind::StudioSet.host_params();
    assert!(list.iter().all(|p| p.name != "Master Level"));
    assert!(list.iter().any(|p| p.name.starts_with("Master EQ")));
}

#[test]
fn a_tempo_synced_parameter_lists_its_switch_value_and_note() {
    let ids = |kind: PluginKind| -> HashSet<String> {
        kind.host_params().into_iter().map(|p| p.id).collect()
    };
    let (studio_set, tone) = (ids(PluginKind::StudioSet), ids(PluginKind::SnAcoustic));
    let mut synced = 0;
    let mut check = |ids: &HashSet<String>, block: &str, ty: usize, defs: &[FxParamDef]| {
        for def in defs.iter().filter(|d| d.sync.is_some()) {
            let sync = def.sync.unwrap();
            for slot in [sync.switch_index, def.index, sync.note_index] {
                assert!(
                    ids.contains(&format!("{block}/{ty}/{slot}")),
                    "{block} {ty} {slot}"
                );
            }
            synced += 1;
        }
    };
    for ty in 0..MFX_TYPE_NAMES.len() {
        check(&tone, "mfx", ty, mfx_type_def(ty as u8).unwrap().params);
    }
    for ty in 0..CHORUS_TYPE_HOST_NAMES.len() {
        check(&studio_set, "chorus", ty, chorus_params(ty as u8));
    }
    for ty in 0..REVERB_TYPE_HOST_NAMES.len() {
        check(&studio_set, "reverb", ty, reverb_params(ty as u8));
    }
    assert!(synced > 20);
}
