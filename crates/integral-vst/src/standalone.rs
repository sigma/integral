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

/// Query the default audio output device's sample rate via cpal.
/// Returns None if no device is available or the config can't be read.
fn detect_sample_rate() -> Option<u32> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    let device = host.default_output_device()?;
    let config = device.default_output_config().ok()?;
    Some(config.sample_rate().0)
}

fn main() {
    // If the user didn't explicitly pass --sample-rate, detect it from the
    // system audio output to avoid CoreAudio buffer size mismatches.
    let user_args: Vec<String> = std::env::args().collect();
    let has_sample_rate = user_args.iter().any(|a| a.starts_with("--sample-rate"));

    if has_sample_rate {
        nih_plug::wrapper::standalone::nih_export_standalone::<Integral>();
    } else if let Some(rate) = detect_sample_rate() {
        eprintln!("[integral] Detected system sample rate: {rate} Hz");
        let mut args = user_args;
        args.push("--sample-rate".to_string());
        args.push(rate.to_string());
        nih_plug::wrapper::standalone::nih_export_standalone_with_args::<Integral, _>(args);
    } else {
        // No audio device — fall back to default (nih-plug will use dummy backend)
        nih_plug::wrapper::standalone::nih_export_standalone::<Integral>();
    }
}
