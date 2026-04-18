//! VST3 plugin for the Integral Integra-7 control surface.
//!
//! This crate integrates with nih-plug to expose the Integral core
//! engine as a VST3/CLAP plugin for DAW integration. It provides
//! MIDI SysEx pass-through with no audio I/O.

use nih_plug::prelude::*;
use std::sync::Arc;

/// The main plugin struct for the Integral control surface.
struct Integral {
    params: Arc<IntegralParams>,
}

/// Plugin parameters (currently empty — will be populated as features land).
#[derive(Params)]
struct IntegralParams {}

impl Default for Integral {
    fn default() -> Self {
        Self {
            params: Arc::new(IntegralParams {}),
        }
    }
}

impl Plugin for Integral {
    const NAME: &'static str = "Integral";
    const VENDOR: &'static str = "sigma";
    const URL: &'static str = "https://github.com/sigma/integral";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[];
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn process(
        &mut self,
        _buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Forward incoming MIDI SysEx — will be wired to DeviceState later
        while let Some(event) = context.next_event() {
            context.send_event(event);
        }
        ProcessStatus::Normal
    }
}

impl ClapPlugin for Integral {
    const CLAP_ID: &'static str = "com.sigma.integral";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Integra-7 Control Surface");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] =
        &[ClapFeature::Utility, ClapFeature::NoteEffect];
}

impl Vst3Plugin for Integral {
    const VST3_CLASS_ID: [u8; 16] = *b"IntgrI7CtrlSurf\0";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Tools];
}

nih_export_clap!(Integral);
nih_export_vst3!(Integral);
