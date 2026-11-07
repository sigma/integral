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
    assert_eq!(reported.len(), kind.host_params().len(), "{kind:?}");
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
