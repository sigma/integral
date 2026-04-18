//! Standalone binary for the Integral Integra-7 control surface.
//!
//! Runs the same Vizia UI as the VST3/CLAP plugin, but outside a DAW
//! with direct MIDI port access via CoreMIDI (macOS), ALSA/JACK (Linux),
//! or WASAPI (Windows).
//!
//! Usage:
//!   integral --midi-input "Integra-7" --midi-output "Integra-7"
//!
//! Use `--help` for all options (provided by nih-plug's standalone wrapper).

use integral_vst::Integral;

fn main() {
    nih_plug::wrapper::standalone::nih_export_standalone::<Integral>();
}
