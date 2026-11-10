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

mod param_index {
    use integral_core::address::Address;
    use integral_core::device_spec::INTEGRA7;
    use integral_core::host_params::{Anchor, Binding, EffectKind, ParamIndex};

    use super::PluginKind;

    /// The binding a test gives a parameter that needs one.
    fn binding_for(anchor: Anchor) -> Binding {
        match anchor {
            Anchor::Absolute => Binding::ANY,
            Anchor::Part => Binding::of_part(5),
            Anchor::Key(tone_type) => {
                let first = match tone_type {
                    integral_core::params::ToneType::SnDrum => INTEGRA7.sn_drum_keys.first,
                    _ => INTEGRA7.pcm_drum_keys.first,
                };
                Binding::of_part(5).with_key(first + 3)
            }
        }
    }

    #[test]
    fn every_parameter_is_found_again_at_its_address() {
        let index = ParamIndex::new();
        for plugin in PluginKind::ALL {
            for (position, param) in plugin.host_params().iter().enumerate() {
                let handle = index.find(plugin, &param.id).expect("found by ID");
                assert_eq!(handle.index as usize, position, "{}", param.id);
                let binding = binding_for(param.anchor);
                let address = param.device_address(&binding).expect("address");
                let found = index.locate(&address);
                let hit = found
                    .iter()
                    .find(|l| l.handle == handle)
                    .unwrap_or_else(|| panic!("{plugin:?} {} not at {address}", param.id));
                assert_eq!(hit.binding, binding, "{plugin:?} {}", param.id);
            }
        }
    }

    #[test]
    fn a_binding_moves_a_tone_parameter_to_its_part_and_key() {
        let index = ParamIndex::new();
        let handle = index
            .find(PluginKind::PcmSynth, "pcms/common/0x0E")
            .expect("param");
        let param = index.get(handle);
        let part0 = param.device_address(&Binding::of_part(0)).unwrap();
        let part3 = param.device_address(&Binding::of_part(3)).unwrap();
        assert_ne!(part0, part3);
        assert_eq!(part0, param.address);
        assert!(param.device_address(&Binding::ANY).is_err());
        assert!(
            param
                .device_address(&Binding::of_part(INTEGRA7.part_count))
                .is_err()
        );
        let key = PluginKind::PcmDrum
            .host_params()
            .into_iter()
            .find(|p| matches!(p.anchor, Anchor::Key(_)))
            .expect("a key parameter");
        assert!(
            key.device_address(&Binding::of_part(0)).is_err(),
            "needs a key"
        );
        let a = key
            .device_address(&Binding::of_part(0).with_key(INTEGRA7.pcm_drum_keys.first))
            .unwrap();
        let b = key
            .device_address(&Binding::of_part(0).with_key(INTEGRA7.pcm_drum_keys.first + 1))
            .unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn values_encode_and_decode_through_width_and_bias() {
        let index = ParamIndex::new();
        for plugin in PluginKind::ALL {
            for param in plugin.host_params() {
                for value in [param.min, param.max, (param.min + param.max) / 2] {
                    let bytes = param.encode(value);
                    assert_eq!(bytes.len(), param.width, "{}", param.id);
                    assert!(bytes.iter().all(|b| *b < 0x80), "{}", param.id);
                    assert_eq!(param.decode(&bytes), Some(value), "{}", param.id);
                }
            }
        }
        let _ = (index, Address::new(0, 0, 0, 0));
    }

    #[test]
    fn a_write_at_a_parameters_address_sets_it_to_the_value() {
        let index = ParamIndex::new();
        for plugin in PluginKind::ALL {
            for param in plugin.host_params() {
                let handle = index.find(plugin, &param.id).expect("found by ID");
                let binding = binding_for(param.anchor);
                let address = param.device_address(&binding).expect("address");
                // The effect type the parameter is for is the active one.
                let active =
                    |kind: EffectKind| param.effect.filter(|e| e.kind == kind).map(|e| e.type_id);
                for value in [param.min, param.max] {
                    let writes =
                        index.writes_at(plugin, &binding, &address, &param.encode(value), active);
                    assert!(
                        writes
                            .iter()
                            .any(|w| w.handle == handle && w.value == value),
                        "{plugin:?} {}: {value} not seen in {writes:?}",
                        param.id
                    );
                }
            }
        }
    }

    #[test]
    fn a_write_belongs_to_the_binding_it_lands_in() {
        let index = ParamIndex::new();
        let handle = index
            .find(PluginKind::PcmSynth, "pcms/common/0x0E")
            .expect("param");
        let param = index.get(handle);
        let at = param.device_address(&Binding::of_part(3)).unwrap();
        let data = param.encode(param.max);
        let none = |_| None;
        let seen = |binding: Binding, plugin| index.writes_at(plugin, &binding, &at, &data, none);
        assert_eq!(seen(Binding::of_part(3), PluginKind::PcmSynth).len(), 1);
        // Another Part, another plugin's list, or a key: nothing.
        assert!(seen(Binding::of_part(4), PluginKind::PcmSynth).is_empty());
        assert!(seen(Binding::of_part(3), PluginKind::SnSynth).is_empty());
        // A key owns only a key's parameters, which a Tone list has none of.
        assert!(seen(Binding::of_part(3).with_key(40), PluginKind::PcmSynth).is_empty());
    }

    #[test]
    fn a_kit_plugin_hears_its_kit_or_its_key_and_not_the_other() {
        let index = ParamIndex::new();
        let list = PluginKind::PcmDrum.host_params();
        let key_param = list
            .iter()
            .find(|p| matches!(p.anchor, Anchor::Key(_)))
            .expect("a key parameter");
        let kit_param = list
            .iter()
            .find(|p| p.anchor == Anchor::Part && p.effect.is_none())
            .expect("a kit parameter");
        let key = INTEGRA7.pcm_drum_keys.first + 2;
        let by_key = Binding::of_part(1).with_key(key);
        let by_kit = Binding::of_part(1);
        let hears = |binding: &Binding, param: &integral_core::host_params::HostParam| {
            let at = param.device_address(binding).or_else(|_| {
                // The address the other role would write, for the same Part.
                param.device_address(&Binding::of_part(1).with_key(key))
            });
            let at = at.expect("address");
            !index
                .writes_at(
                    PluginKind::PcmDrum,
                    binding,
                    &at,
                    &param.encode(param.min),
                    |_| None,
                )
                .is_empty()
        };
        assert!(hears(&by_key, key_param));
        assert!(hears(&by_kit, kit_param));
        assert!(
            !hears(&by_kit, key_param),
            "a kit binding has no key parameters"
        );
        assert!(
            !hears(&by_key, kit_param),
            "a key binding has no kit parameters"
        );
    }

    #[test]
    fn an_effect_slot_belongs_to_the_active_type_only() {
        let index = ParamIndex::new();
        let a = index.find(PluginKind::SnSynth, "mfx/1/1").expect("param");
        let param = index.get(a);
        let EffectKind::Mfx = param.effect.expect("effect").kind else {
            panic!("an MFX slot");
        };
        let binding = Binding::of_part(0);
        let at = param.device_address(&binding).unwrap();
        let data = param.encode(0);
        let of_type = |active: Option<u8>| {
            index
                .writes_at(PluginKind::SnSynth, &binding, &at, &data, |k| {
                    (k == EffectKind::Mfx).then_some(active).flatten()
                })
                .into_iter()
                .map(|w| index.get(w.handle).id.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(of_type(Some(1)), ["mfx/1/1"]);
        assert_eq!(of_type(Some(2)), ["mfx/2/1"]);
        assert!(of_type(None).is_empty(), "an unknown type sets nothing");
    }

    #[test]
    fn effect_slots_share_an_address_across_types() {
        let index = ParamIndex::new();
        let a = index.find(PluginKind::SnSynth, "mfx/1/1").expect("param");
        let address = index.get(a).device_address(&Binding::of_part(0)).unwrap();
        let types: Vec<_> = index
            .locate(&address)
            .into_iter()
            .filter(|l| index.get(l.handle).id.ends_with("/1"))
            .collect();
        assert!(types.len() > 1);
    }

    #[test]
    fn an_effect_types_parameters_say_where_the_type_is() {
        let index = ParamIndex::new();
        for plugin in PluginKind::ALL {
            for param in plugin.host_params() {
                let Some(effect) = param.effect else { continue };
                let binding = binding_for(param.anchor);
                let at = param
                    .effect_type_address(&binding)
                    .expect("an effect parameter")
                    .expect("address");
                let type_param = index
                    .locate(&at)
                    .into_iter()
                    .find(|l| l.handle.plugin == plugin)
                    .unwrap_or_else(|| panic!("{} type byte not a parameter", param.id));
                let def = index.get(type_param.handle);
                assert_eq!(def.width, 1, "{}", param.id);
                assert!(def.max >= i32::from(effect.type_id), "{}", param.id);
            }
        }
    }
}

/// Raw values worth trying in `p`'s range: both ends, the default and the
/// middle.
fn probes(p: &integral_core::host_params::HostParam) -> [i32; 4] {
    [p.min, p.default, p.min + (p.max - p.min) / 2, p.max]
}

#[test]
fn a_values_text_parses_back_to_a_value_with_the_same_text() {
    for kind in PluginKind::ALL {
        for p in kind.host_params() {
            for raw in probes(&p) {
                let text = p.format.format(raw);
                let parsed = p
                    .parse(&text)
                    .unwrap_or_else(|| panic!("{kind:?} {}: {text:?} does not parse", p.id));
                assert_eq!(
                    p.format.format(parsed),
                    text,
                    "{kind:?} {}: {text:?} parsed as {parsed}",
                    p.id
                );
                if raw == parsed {
                    continue;
                }
                // Only a text two values share may parse as the other one,
                // and then as the lowest.
                assert!(parsed < raw, "{kind:?} {}: {text:?}", p.id);
            }
        }
    }
}

#[test]
fn typed_text_may_drop_case_spaces_and_the_unit() {
    use integral_core::fx_params::FxUnit;
    use integral_core::host_params::HostFormat;
    use integral_core::param_registry::ParamFormat;

    let pan = HostFormat::Field(ParamFormat::Pan);
    assert_eq!(pan.parse("l 64", 0, 127), Some(0));
    assert_eq!(pan.parse("C", 0, 127), Some(64));
    assert_eq!(pan.parse("63r", 0, 127), Some(127));

    let signed = HostFormat::Field(ParamFormat::SignedCenter(64));
    assert_eq!(signed.parse("-64", 0, 127), Some(0));
    assert_eq!(signed.parse("+63", 0, 127), Some(127));
    assert_eq!(signed.parse("63", 0, 127), Some(127));

    let db = HostFormat::Fx(FxUnit::Decibels);
    let three = db.parse("+3dB", -15, 15).expect("+3dB");
    assert_eq!(db.format(three), "+3 dB");
    assert_eq!(db.parse("+3", -15, 15), Some(three));
    assert_eq!(db.parse("3 db", -15, 15), Some(three));
}

#[test]
fn mixer_eq_text_carries_its_unit_and_reads_back() {
    use integral_core::host_params::HostFormat;
    use integral_core::param_registry::{MixerField, mixer_param};

    let host = |field| {
        let def = mixer_param(field);
        (HostFormat::Field(def.format), def.min, def.max)
    };
    let (gain, min, max) = host(MixerField::EqMidGain);
    assert_eq!(gain.format(18), "+3 dB");
    for text in ["+3 dB", "+3dB", "+3", "3 db"] {
        assert_eq!(gain.parse(text, min, max), Some(18), "{text}");
    }
    let (out, min, max) = host(MixerField::CompOutputGain);
    assert_eq!(out.parse("+6 dB", min, max), Some(6));
    let (freq, min, max) = host(MixerField::EqMidFreq);
    assert_eq!(freq.format(7), "1.0 kHz");
    for text in ["1.0 kHz", "1khz", "1"] {
        assert_eq!(freq.parse(text, min, max), Some(7), "{text}");
    }
    assert_eq!(freq.parse("400 Hz", min, max), Some(3));
}

#[test]
fn mixer_comp_times_carry_ms_and_read_back() {
    use integral_core::host_params::HostFormat;
    use integral_core::param_registry::{MixerField, mixer_param};

    let host = |field| {
        let def = mixer_param(field);
        (HostFormat::Field(def.format), def.min, def.max)
    };
    let (attack, min, max) = host(MixerField::CompAttack);
    assert_eq!(attack.format(15), "0.50 ms");
    for text in ["0.50 ms", "0.5ms", "0.5", ".5 MS"] {
        assert_eq!(attack.parse(text, min, max), Some(15), "{text}");
    }
    assert_eq!(attack.parse("50 ms", min, max), Some(max));
    assert_eq!(attack.parse("0.5 s", min, max), None);
    let (release, min, max) = host(MixerField::CompRelease);
    assert_eq!(release.format(4), "1 ms");
    for text in ["1 ms", "1ms", "1", "1.0"] {
        assert_eq!(release.parse(text, min, max), Some(4), "{text}");
    }
    assert_eq!(release.parse("2000 ms", min, max), Some(max));
}

#[test]
fn text_no_value_has_does_not_parse() {
    use integral_core::host_params::HostFormat;
    use integral_core::param_registry::ParamFormat;

    let signed = HostFormat::Field(ParamFormat::SignedCenter(64));
    assert_eq!(signed.parse("", 0, 127), None);
    assert_eq!(signed.parse("loud", 0, 127), None);
    assert_eq!(signed.parse("64000", 0, 127), None);
    // The unit must be the value's, once given.
    let pct = HostFormat::Field(ParamFormat::Percent);
    assert_eq!(pct.parse("50", 0, 100), Some(50));
    assert_eq!(pct.parse("50hz", 0, 100), None);
}

#[test]
fn a_new_instance_has_a_target_per_part_except_the_studio_set_plugin() {
    use integral_core::device_spec::INTEGRA7;
    use integral_core::host_params::Binding;

    assert_eq!(PluginKind::StudioSet.targets(), [Binding::ANY]);
    for kind in PluginKind::ALL.into_iter().skip(1) {
        let targets = kind.targets();
        assert_eq!(targets.len(), usize::from(INTEGRA7.part_count));
        assert_eq!(targets[0], Binding::of_part(0));
        assert!(targets.iter().all(|t| t.key.is_none()));
    }
}

#[test]
fn a_plugin_accepts_the_bindings_it_can_have() {
    use integral_core::host_params::Binding;

    let part = Binding::of_part(3);
    assert!(PluginKind::StudioSet.accepts(&Binding::ANY));
    assert!(!PluginKind::StudioSet.accepts(&part));
    for tone in [
        PluginKind::SnSynth,
        PluginKind::SnAcoustic,
        PluginKind::PcmSynth,
    ] {
        assert!(tone.accepts(&part) && tone.accepts(&Binding::of_part(15)));
        assert!(!tone.accepts(&Binding::ANY));
        assert!(!tone.accepts(&Binding::of_part(16)));
        assert!(
            !tone.accepts(&part.with_key(40)),
            "a Tone plugin has no key"
        );
    }
    // The key ranges of the device spec: SN Drum 27-88, PCM Drum 21-108.
    assert!(PluginKind::SnDrum.accepts(&part) && PluginKind::PcmDrum.accepts(&part));
    assert!(!PluginKind::SnDrum.accepts(&Binding::ANY));
    assert!(PluginKind::SnDrum.accepts(&part.with_key(27)));
    assert!(PluginKind::SnDrum.accepts(&part.with_key(88)));
    assert!(!PluginKind::SnDrum.accepts(&part.with_key(26)));
    assert!(!PluginKind::SnDrum.accepts(&part.with_key(89)));
    assert!(PluginKind::PcmDrum.accepts(&part.with_key(21)));
    assert!(PluginKind::PcmDrum.accepts(&part.with_key(108)));
    assert!(!PluginKind::PcmDrum.accepts(&part.with_key(20)));
    assert!(!PluginKind::PcmDrum.accepts(&part.with_key(109)));
}

#[test]
fn plugin_names_are_unique() {
    let names: HashSet<_> = PluginKind::ALL.iter().map(|k| k.name()).collect();
    assert_eq!(names.len(), PluginKind::ALL.len());
}

#[test]
fn only_tone_and_kit_plugins_have_binding_params_and_only_kits_a_key() {
    use integral_core::device_spec::INTEGRA7;
    assert!(PluginKind::StudioSet.binding_params().is_empty());
    for kind in PluginKind::ALL.into_iter().skip(1) {
        let params = kind.binding_params();
        let kit = matches!(kind, PluginKind::SnDrum | PluginKind::PcmDrum);
        assert_eq!(params.len(), if kit { 2 } else { 1 }, "{kind:?}");
        assert_eq!(
            (params[0].min, params[0].max),
            (1, i32::from(INTEGRA7.part_count))
        );
    }
    let key = PluginKind::SnDrum.binding_params()[1];
    let keys = INTEGRA7.sn_drum_keys;
    assert_eq!(key.max, i32::from(keys.last - keys.first) + 1);
}

#[test]
fn binding_param_ids_are_not_device_address_ids() {
    let device: HashSet<_> = PluginKind::ALL
        .iter()
        .flat_map(|k| k.host_params())
        .map(|p| p.id)
        .collect();
    for kind in PluginKind::ALL {
        for param in kind.binding_params() {
            assert!(!device.contains(param.id), "{}", param.id);
            // An address ID ends in a hex offset; these do not.
            assert!(!param.id.contains("/0x"), "{}", param.id);
        }
    }
}

#[test]
fn every_accepted_binding_round_trips_through_its_raw_values() {
    use integral_core::host_params::{Binding, binding_of};
    for kind in PluginKind::ALL.into_iter().skip(1) {
        let params = kind.binding_params();
        let mut bindings = kind.targets();
        if params.len() == 2 {
            let (first, last) = (params[1].raw_of(&Binding::ANY), params[1].max);
            assert_eq!(first, 0);
            bindings.extend((1..=last).map(|n| {
                let part = Binding::of_part(3);
                // The nth key, found by the inverse.
                binding_of(&params, &[4, n])
                    .key
                    .map_or(part, |key| part.with_key(key))
            }));
        }
        for binding in bindings {
            assert!(kind.accepts(&binding), "{kind:?} {binding:?}");
            let raw: Vec<i32> = params.iter().map(|p| p.raw_of(&binding)).collect();
            assert_eq!(binding_of(&params, &raw), binding, "{kind:?}");
        }
    }
}

#[test]
fn binding_param_text_reads_back() {
    for kind in PluginKind::ALL.into_iter().skip(1) {
        for param in kind.binding_params() {
            for raw in param.min..=param.max {
                assert_eq!(param.parse(&param.format(raw)), Some(raw), "{}", param.id);
            }
            assert_eq!(param.parse(&param.format(param.max + 1)), None);
        }
    }
    let key = PluginKind::PcmDrum.binding_params()[1];
    assert_eq!(key.format(0), "Whole kit");
    assert_eq!(key.parse("whole kit"), Some(0));
    assert_eq!(key.format(1), "21");
}
