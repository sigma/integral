//! Standalone binary for the Integral Integra-7 control surface.
//!
//! Runs the same editor as the VST3 plugins (here the Studio Set plugin), but
//! outside a DAW, with audio through nih-plug's backends. It holds no MIDI
//! port: like the plugins, it reaches the device through the Device service
//! (ADR 0006). nih-plug's `--midi-input` and `--midi-output` would open ports
//! of their own, so they are refused.
//!
//! Use `--help` for all options (provided by nih-plug's standalone wrapper).

use integral_vst::StudioSetPlugin;

/// Query the default audio output device's sample rate via cpal.
/// Returns None if no device is available or the config can't be read.
fn detect_sample_rate() -> Option<u32> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    let device = host.default_output_device()?;
    let config = device.default_output_config().ok()?;
    Some(config.sample_rate().0)
}

/// The flags nih-plug's standalone would open MIDI ports with.
const MIDI_FLAGS: [&str; 2] = ["--midi-input", "--midi-output"];

/// The first of `args` that asks for a MIDI port, as `--flag` or
/// `--flag=value`.
fn midi_flag(args: &[String]) -> Option<&str> {
    args.iter().find_map(|arg| {
        MIDI_FLAGS.into_iter().find(|flag| {
            arg.strip_prefix(flag)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('='))
        })
    })
}

fn main() {
    // If the user didn't explicitly pass --sample-rate, detect it from the
    // system audio output to avoid CoreAudio buffer size mismatches.
    let user_args: Vec<String> = std::env::args().collect();
    if let Some(flag) = midi_flag(&user_args[1..]) {
        eprintln!(
            "[integral] {flag} is not supported: the standalone holds no MIDI port, the Device service does."
        );
        std::process::exit(2);
    }
    let has_sample_rate = user_args.iter().any(|a| a.starts_with("--sample-rate"));

    if has_sample_rate {
        nih_plug::wrapper::standalone::nih_export_standalone::<StudioSetPlugin>();
    } else if let Some(rate) = detect_sample_rate() {
        eprintln!("[integral] Detected system sample rate: {rate} Hz");
        let mut args = user_args;
        args.push("--sample-rate".to_string());
        args.push(rate.to_string());
        nih_plug::wrapper::standalone::nih_export_standalone_with_args::<StudioSetPlugin, _>(args);
    } else {
        // No audio device — fall back to default (nih-plug will use dummy backend)
        nih_plug::wrapper::standalone::nih_export_standalone::<StudioSetPlugin>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(args: &[&str]) -> Vec<String> {
        args.iter().map(|a| a.to_string()).collect()
    }

    #[test]
    fn a_flag_that_opens_a_midi_port_is_found() {
        assert_eq!(
            midi_flag(&args(&["--midi-input", "Integra-7"])),
            Some("--midi-input")
        );
        assert_eq!(
            midi_flag(&args(&["--sample-rate", "48000", "--midi-output=x"])),
            Some("--midi-output")
        );
    }

    #[test]
    fn other_flags_pass() {
        assert_eq!(midi_flag(&args(&["--sample-rate", "48000"])), None);
        assert_eq!(midi_flag(&args(&["--midi-inputs"])), None);
    }
}
