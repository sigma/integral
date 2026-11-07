//! A plugin's parameters as nih-plug declares them, and the state it saves
//! beside them.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use integral_core::host_params::{HostParam, PluginKind};
use nih_plug::prelude::*;

use super::binding::PERSIST_KEY;
use super::instance::Instance;

/// The state key the editor's size is saved under.
const EDITOR_SIZE_KEY: &str = "editor-size";

/// The editor's size before the page asks for its own, logical px.
const DEFAULT_EDITOR_SIZE: (u32, u32) = (1280, 820);

/// A plugin kind's host parameters over their raw range, and what is saved
/// with them: the editor's size and the instance's Binding.
pub struct FamilyParams {
    /// The editor window's size in logical px, as the page last set it.
    pub editor_size: Arc<RwLock<(u32, u32)>>,
    /// The instance whose Binding is saved, and whose bank holds the
    /// parameters.
    pub instance: Arc<Instance>,
}

impl FamilyParams {
    /// The parameters of `kind`, and a new [`Instance`] of it.
    pub fn new(kind: PluginKind) -> Self {
        Self {
            editor_size: Arc::new(RwLock::new(DEFAULT_EDITOR_SIZE)),
            instance: Arc::new(Instance::new(kind)),
        }
    }

    /// The host parameters with their nih-plug counterparts, in list order.
    pub fn list(&self) -> &[(HostParam, IntParam)] {
        self.instance.bank.list()
    }
}

// SAFETY: the pointers `param_map` returns point into the instance's bank,
// which `self` keeps alive, and the bank's list is never resized.
unsafe impl Params for FamilyParams {
    fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
        self.list()
            .iter()
            .map(|(host, param)| (host.id.clone(), param.as_ptr(), host.group.clone()))
            .collect()
    }

    fn serialize_fields(&self) -> BTreeMap<String, String> {
        let size = *self.editor_size.read().expect("size lock");
        BTreeMap::from([
            (
                EDITOR_SIZE_KEY.to_string(),
                serde_json::to_string(&size).expect("a pair of integers"),
            ),
            (PERSIST_KEY.to_string(), self.instance.binding.to_json()),
        ])
    }

    fn deserialize_fields(&self, serialized: &BTreeMap<String, String>) {
        if self.instance.replacing() {
            // The device's side replacing the values through the wrapper's
            // state restore: no host restore, and the Binding is the same.
            return;
        }
        if let Some(size) = serialized
            .get(EDITOR_SIZE_KEY)
            .and_then(|text| serde_json::from_str(text).ok())
        {
            *self.editor_size.write().expect("size lock") = size;
        }
        // A state was restored, so the host has had its say: settle, even
        // if the Binding in it is missing or one the plugin cannot have.
        self.instance.settle();
        if let Some(text) = serialized.get(PERSIST_KEY)
            && self
                .instance
                .binding
                .restore_json(self.instance.kind(), text)
        {
            self.instance.mark_restored();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use integral_core::host_params::Binding;

    fn params(kind: PluginKind) -> FamilyParams {
        FamilyParams::new(kind)
    }

    #[test]
    fn every_host_parameter_is_declared_in_list_order() {
        for kind in PluginKind::ALL {
            let declared = params(kind).param_map();
            let list = kind.host_params();
            assert_eq!(declared.len(), list.len(), "{kind:?}");
            for ((id, _, group), host) in declared.iter().zip(&list) {
                assert_eq!((id, group), (&host.id, &host.group), "{kind:?}");
            }
        }
    }

    #[test]
    fn declared_ranges_names_and_defaults_are_the_lists() {
        for kind in PluginKind::ALL {
            let params = params(kind);
            for (host, param) in params.list() {
                assert_eq!(param.name(), host.name);
                assert_eq!(param.default_plain_value(), host.default);
                assert_eq!(param.preview_plain(0.0), host.min, "{}", host.id);
                assert_eq!(param.preview_plain(1.0), host.max, "{}", host.id);
                assert!(
                    param.flags().is_empty(),
                    "{} is plainly automatable",
                    host.id
                );
            }
        }
    }

    #[test]
    fn value_text_is_the_lists_format_both_ways() {
        for kind in PluginKind::ALL {
            let params = params(kind);
            for (host, param) in params.list() {
                for raw in [host.min, host.default, host.max] {
                    let normalized = param.preview_normalized(raw);
                    let text = param.normalized_value_to_string(normalized, false);
                    assert_eq!(text, host.format.format(raw), "{}", host.id);
                    let back = param
                        .string_to_normalized_value(&text)
                        .unwrap_or_else(|| panic!("{}: {text:?} does not parse", host.id));
                    assert_eq!(
                        host.format.format(param.preview_plain(back)),
                        text,
                        "{}",
                        host.id
                    );
                }
            }
        }
    }

    #[test]
    fn parameters_do_not_smooth() {
        let params = params(PluginKind::PcmSynth);
        for (host, param) in params.list() {
            assert!(
                !param.smoothed.is_smoothing(),
                "{} would smooth, so a host's write would ramp",
                host.id
            );
        }
    }

    #[test]
    fn the_binding_and_the_editor_size_survive_a_save_and_reopen() {
        for kind in PluginKind::ALL.into_iter().skip(1) {
            let target = if matches!(kind, PluginKind::SnDrum | PluginKind::PcmDrum) {
                Binding::of_part(5).with_key(40)
            } else {
                Binding::of_part(5)
            };
            let saved = params(kind);
            saved.instance.binding.set(Some(target));
            *saved.editor_size.write().unwrap() = (900, 600);
            let fields = saved.serialize_fields();

            let reopened = params(kind);
            assert_eq!(reopened.instance.binding.get(), None, "a new instance");
            assert!(!reopened.instance.was_restored());
            assert!(!reopened.instance.is_settled());
            reopened.deserialize_fields(&fields);
            assert!(reopened.instance.was_restored() && reopened.instance.is_settled());
            assert_eq!(reopened.instance.binding.get(), Some(target), "{kind:?}");
            assert_eq!(*reopened.editor_size.read().unwrap(), (900, 600));
        }
    }

    #[test]
    fn a_state_without_a_binding_keeps_the_new_instances() {
        let params = params(PluginKind::StudioSet);
        params.deserialize_fields(&BTreeMap::new());
        assert_eq!(params.instance.binding.get(), Some(Binding::ANY));
        assert!(
            !params.instance.was_restored() && params.instance.is_settled(),
            "a state without a Binding settles but is no restore"
        );
        let tone = FamilyParams::new(PluginKind::SnSynth);
        tone.deserialize_fields(&BTreeMap::new());
        assert_eq!(tone.instance.binding.get(), None);
    }

    #[test]
    fn a_bound_instance_saved_unbound_reopens_unbound() {
        let saved = FamilyParams::new(PluginKind::SnSynth);
        let fresh = FamilyParams::new(PluginKind::SnSynth);
        fresh.instance.binding.set(Some(Binding::of_part(1)));
        fresh.deserialize_fields(&saved.serialize_fields());
        assert_eq!(fresh.instance.binding.get(), None);
    }

    #[test]
    fn a_state_the_device_side_replaces_values_with_is_no_host_restore() {
        let params = FamilyParams::new(PluginKind::SnSynth);
        let fields = BTreeMap::from([(PERSIST_KEY.to_string(), r#"{"part":4}"#.to_string())]);
        params.instance.set_replacing(true);
        params.deserialize_fields(&fields);
        params.instance.set_replacing(false);
        assert!(!params.instance.was_restored() && !params.instance.is_settled());
        assert_eq!(params.instance.binding.get(), None);
    }
}
