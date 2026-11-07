//! nih-plug hashes each parameter ID string to 31 bits and never checks for a
//! collision (ADR 0008). This runs every plugin kind's full host parameter
//! list through nih-plug's own VST3 wrapper and reads back the IDs the host
//! would see, so the hash is nih-plug's, not a copy.
//!
//! `Probe<K>` stands in for the six plugin types: #453 builds them, and
//! replaces the probe with the real types.

use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::sync::Arc;

use integral_core::host_params::{HostParam, PluginKind};
use nih_plug::prelude::*;
use nih_plug::wrapper::vst3::Wrapper;
use nih_plug::wrapper::vst3::vst3_sys::vst::{IEditController, ParameterInfo};

/// Names a plugin kind at the type level, for `Probe`.
trait Kind: 'static + Send + Sync {
    const KIND: PluginKind;
}

macro_rules! kinds {
    ($($name:ident),+) => {$(
        struct $name;
        impl Kind for $name {
            const KIND: PluginKind = PluginKind::$name;
        }
    )+};
}
kinds!(StudioSet, SnSynth, SnAcoustic, PcmSynth, SnDrum, PcmDrum);

/// A kind's host parameters as nih-plug integers over the raw range.
struct ProbeParams {
    params: Vec<(HostParam, IntParam)>,
}

// SAFETY: the pointers `param_map` returns point into `self`, which the
// wrapper keeps alive in an `Arc`.
unsafe impl Params for ProbeParams {
    fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
        self.params
            .iter()
            .map(|(host, param)| (host.id.clone(), param.as_ptr(), host.group.clone()))
            .collect()
    }
}

struct Probe<K: Kind> {
    params: Arc<ProbeParams>,
    kind: PhantomData<K>,
}

impl<K: Kind> Default for Probe<K> {
    fn default() -> Self {
        let params = K::KIND
            .host_params()
            .into_iter()
            .map(|host| {
                let range = IntRange::Linear {
                    min: host.min,
                    max: host.max,
                };
                let param = IntParam::new(host.name.clone(), host.default, range);
                (host, param)
            })
            .collect();
        Self {
            params: Arc::new(ProbeParams { params }),
            kind: PhantomData,
        }
    }
}

impl<K: Kind> Plugin for Probe<K> {
    const NAME: &'static str = "Probe";
    const VENDOR: &'static str = "sigma";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = "0";
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn process(
        &mut self,
        _buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        ProcessStatus::Normal
    }
}

impl<K: Kind> Vst3Plugin for Probe<K> {
    const VST3_CLASS_ID: [u8; 16] = *b"IntgrHostProbe\0\0";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Tools];
}

/// The (ID, title) of each parameter the VST3 wrapper reports for `K`.
fn reported<K: Kind>() -> Vec<(u32, String)> {
    let wrapper = Wrapper::<Probe<K>>::new();
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

fn assert_no_collision<K: Kind>() {
    let reported = reported::<K>();
    assert_eq!(reported.len(), K::KIND.host_params().len(), "{:?}", K::KIND);
    let mut seen = std::collections::HashMap::new();
    for (id, title) in &reported {
        if let Some(other) = seen.insert(*id, title) {
            panic!("{:?}: {other:?} and {title:?} both hash to {id}", K::KIND);
        }
    }
}

#[test]
fn no_two_parameters_of_a_plugin_hash_alike() {
    assert_no_collision::<StudioSet>();
    assert_no_collision::<SnSynth>();
    assert_no_collision::<SnAcoustic>();
    assert_no_collision::<PcmSynth>();
    assert_no_collision::<SnDrum>();
    assert_no_collision::<PcmDrum>();
}
