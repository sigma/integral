//! nih-plug hashes each parameter ID string to 31 bits and never checks for a
//! collision (ADR 0008). This runs every plugin kind's full host parameter
//! list through nih-plug's own VST3 wrapper, on the family's real plugin
//! types, and reads back the IDs the host would see, so the hash is
//! nih-plug's, not a copy.

use crate::{
    PcmDrumPlugin, PcmSynthPlugin, SnAcousticPlugin, SnDrumPlugin, SnSynthPlugin, StudioSetPlugin,
};
use integral_core::host_params::PluginKind;
use nih_plug::prelude::Vst3Plugin;
use nih_plug::wrapper::vst3::Wrapper;
use nih_plug::wrapper::vst3::vst3_sys::vst::{IEditController, ParameterInfo};

/// The (ID, title) of each parameter the VST3 wrapper reports for `P`.
fn reported<P: Vst3Plugin>() -> Vec<(u32, String)> {
    let wrapper = Wrapper::<P>::new();
    // SAFETY: the calls only read the wrapper, and `info` is a plain struct
    // the wrapper fills in.
    unsafe {
        (0..wrapper.get_parameter_count())
            .map(|index| {
                let mut info: ParameterInfo = std::mem::zeroed();
                assert_eq!(wrapper.get_parameter_info(index, &mut info), 0);
                let title = info.title.iter().take_while(|c| **c != 0);
                let title = String::from_utf16_lossy(&title.map(|c| *c as u16).collect::<Vec<_>>());
                (info.id, title)
            })
            .collect()
    }
}

fn assert_no_collision<P: Vst3Plugin>(kind: PluginKind) {
    let reported = reported::<P>();
    // The Binding's parameters are in the same space as the device's.
    let expected = kind.host_params().len() + kind.binding_params().len();
    assert_eq!(reported.len(), expected, "{kind:?}");
    let mut seen = std::collections::HashMap::new();
    for (id, title) in &reported {
        if let Some(other) = seen.insert(*id, title) {
            panic!("{kind:?}: {other:?} and {title:?} both hash to {id}");
        }
    }
}

#[test]
fn no_two_parameters_of_a_plugin_hash_alike() {
    assert_no_collision::<StudioSetPlugin>(PluginKind::StudioSet);
    assert_no_collision::<SnSynthPlugin>(PluginKind::SnSynth);
    assert_no_collision::<SnAcousticPlugin>(PluginKind::SnAcoustic);
    assert_no_collision::<PcmSynthPlugin>(PluginKind::PcmSynth);
    assert_no_collision::<SnDrumPlugin>(PluginKind::SnDrum);
    assert_no_collision::<PcmDrumPlugin>(PluginKind::PcmDrum);
}

/// The ID the host sees for the parameter titled `title` of `P`.
fn id_of<P: Vst3Plugin>(title: &str) -> u32 {
    let mut found = reported::<P>().into_iter().filter(|(_, t)| t == title);
    let (id, _) = found.next().expect("a parameter of that title");
    assert!(found.next().is_none(), "{title} is unique");
    id
}

#[test]
fn the_wrappers_binding_parameters_reach_the_right_values() {
    use crate::family::params::LAST_INSTANCE;
    use nih_plug::wrapper::vst3::vst3_sys::base::kResultOk;
    use nih_plug::wrapper::vst3::vst3_sys::vst::IEditController;
    let wrapper = Wrapper::<PcmDrumPlugin>::new();
    let instance = LAST_INSTANCE
        .with(|last| last.borrow().as_ref().and_then(std::sync::Weak::upgrade))
        .expect("the wrapper's instance");
    let params = instance.binding_params.list();
    assert_eq!(
        (params[0].0.id, params[1].0.id),
        (
            integral_core::host_params::BINDING_PART_ID,
            integral_core::host_params::BINDING_KEY_ID
        )
    );
    // The host sets each parameter by the ID it knows it by: each lands in
    // its own slot and, once the host has settled, rebinds.
    let (part, key) = (params[0].0, params[1].0);
    instance.settle();
    let set = |title: &str, raw: i32, spec: &integral_core::host_params::BindingParam| {
        let normalized = f64::from(raw - spec.min) / f64::from(spec.max - spec.min);
        // SAFETY: a plain call on a wrapper nothing else uses.
        unsafe {
            assert_eq!(
                wrapper.set_param_normalized(id_of::<PcmDrumPlugin>(title), normalized),
                kResultOk
            )
        };
    };
    set(part.name, 9, &part);
    assert_eq!(
        (
            instance.binding_params.value(0),
            instance.binding_params.value(1)
        ),
        (9, 0)
    );
    set(key.name, 3, &key);
    assert_eq!(
        (
            instance.binding_params.value(0),
            instance.binding_params.value(1)
        ),
        (9, 3)
    );
    instance.follow_binding_params();
    let first = integral_core::device_spec::INTEGRA7.pcm_drum_keys.first;
    assert_eq!(
        instance.binding.get(),
        Some(integral_core::host_params::Binding::of_part(8).with_key(first + 2))
    );
}
