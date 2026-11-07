//! Dev-only VST3 test host: loads the Integral plugin bundle, opens its
//! editor and drives it with real input, for the local validation loop that
//! used to need Ableton Live (#442, #443). Not shipped and not run in CI.
//!
//! Output is JSON lines on stdout; the exit status is non-zero when an
//! assertion fails or the run aborts. macOS only.
//!
//! Exit codes: 0 every assertion held; 1 an assertion failed; 2 usage error
//! (bad arguments); 3 the run aborted (a safety guard, a timeout, setup).

#[cfg(target_os = "macos")]
mod automate;
#[cfg(target_os = "macos")]
mod ax;
#[cfg(target_os = "macos")]
mod cgwindow;
#[cfg(target_os = "macos")]
mod drag;
#[cfg(target_os = "macos")]
mod driver;
#[cfg(target_os = "macos")]
mod family;
#[cfg(target_os = "macos")]
mod gesture;
#[cfg(target_os = "macos")]
mod host;
#[cfg(target_os = "macos")]
mod load;
#[cfg(target_os = "macos")]
mod mismatch;
#[cfg(target_os = "macos")]
mod pointer;
#[cfg(target_os = "macos")]
mod recall;
#[cfg(target_os = "macos")]
mod report;
#[cfg(target_os = "macos")]
mod rig;
#[cfg(target_os = "macos")]
mod screenshot;

#[cfg(target_os = "macos")]
mod cli {
    use std::path::PathBuf;
    use std::process::ExitCode;
    use std::sync::Arc;

    use clap::{Parser, Subcommand};

    use crate::rig::Device;
    use crate::{automate, drag, driver, load, mismatch, recall, report::Report};

    #[derive(Parser)]
    #[command(about = "Dev-only VST3 test host for the Integral plugin")]
    struct Cli {
        /// The `.vst3` bundle to load (`just bundle-plugin` makes it).
        #[arg(long)]
        bundle: PathBuf,
        /// The plugin class to load, by name, from a bundle that exports
        /// several (the family's does). Default: the first.
        #[arg(long, global = true)]
        class: Option<String>,
        /// Run `load` or `drag` even if a Device service is listening at the
        /// default socket, to which the plugins would connect. The scenarios
        /// that run a service of their own (`recall`, `automate`,
        /// `binding-mismatch`) listen there themselves and refuse always.
        #[arg(long, global = true)]
        allow_service: bool,
        /// Let the scenarios that run a Device service write to a real
        /// INTEGRA-7 (`--device rtp:...` or `usb`). They save the blocks they
        /// touch first and write them back at the end. Without this flag a
        /// real device is refused.
        #[arg(long, global = true)]
        write_device: bool,
        /// What the scenarios that run a Device service talk to: `fake` (a
        /// fake link in front of a memory that answers reads, the default),
        /// `rtp:<session name>` or `usb[:<port name>]` for the real
        /// INTEGRA-7, to which the plugins' writes then go.
        #[arg(long, global = true, default_value = "fake", value_parser = Device::parse)]
        device: Device,
        /// Where screenshots (window-only) are written.
        #[arg(long)]
        out_dir: PathBuf,
        #[command(subcommand)]
        scenario: Scenario,
    }

    #[derive(Subcommand)]
    enum Scenario {
        /// Load every plugin class of the bundle without opening an editor,
        /// check each against its parameter list, and round-trip its state.
        Load,
        /// Drag a slider and check the gestures on the matching host parameter.
        Drag {
            /// Name of the host parameter the drag must move.
            #[arg(long, default_value = "Level")]
            param: String,
            /// Accessibility label of the control: an exact match is preferred, else a
            /// substring; more than one match aborts. Defaults to the parameter name.
            #[arg(long)]
            control: Option<String>,
            /// Release point as a fraction of the slider's width, 0 to 1
            /// (default: the far side of the parameter's current value).
            #[arg(long, value_parser = fraction)]
            to: Option<f64>,
        },
        /// Save a plugin's state, load it into a fresh instance and check what
        /// comes back and what the instance pushes to the device (ADR 0009).
        /// Runs the cases of `--class`, or all of them.
        Recall,
        /// Play a parameter ramp with `set_parameter_at` and check the writes
        /// it makes, and their rate.
        Automate {
            /// Name of the Studio Set host parameter the ramp moves.
            #[arg(long, default_value = "Part 1 Level")]
            param: String,
            /// How long the ramp lasts, in seconds.
            #[arg(long, default_value_t = 1.0, value_parser = positive)]
            seconds: f64,
        },
        /// Open a project in a plugin whose Part holds another tone type: the
        /// instance writes nothing, and the editor shows the mismatch.
        BindingMismatch {
            /// Check the headless part alone: open no window.
            #[arg(long)]
            no_editor: bool,
        },
    }

    impl Scenario {
        /// Whether the scenario runs the Device service itself, so that
        /// nothing else may listen where the plugins look for it.
        fn owns_service(&self) -> bool {
            matches!(
                self,
                Self::Recall | Self::Automate { .. } | Self::BindingMismatch { .. }
            )
        }
    }

    /// A number above 0 (NaN and infinities are rejected).
    fn positive(text: &str) -> Result<f64, String> {
        let value: f64 = text.parse().map_err(|e| format!("{e}"))?;
        if value.is_finite() && value > 0.0 {
            Ok(value)
        } else {
            Err(format!("{text} is not above 0"))
        }
    }

    /// A number from 0 to 1 (NaN and infinities are rejected).
    fn fraction(text: &str) -> Result<f64, String> {
        let value: f64 = text.parse().map_err(|e| format!("{e}"))?;
        if (0.0..=1.0).contains(&value) {
            Ok(value)
        } else {
            Err(format!("{text} is not between 0 and 1"))
        }
    }

    pub fn main() -> ExitCode {
        let cli = Cli::parse();
        let report = Arc::new(Report::new());
        let owns_service = cli.scenario.owns_service();
        if !owns_service && !cli.device.is_fake() {
            eprintln!("--device applies to recall, automate and binding-mismatch");
            return ExitCode::from(2);
        }
        if !cli.device.is_fake() && !cli.write_device {
            eprintln!(
                "--device {:?} would be written to: pass --write-device to allow it",
                cli.device
            );
            return ExitCode::from(2);
        }
        if cli.class.is_some()
            && matches!(
                cli.scenario,
                Scenario::Automate { .. } | Scenario::BindingMismatch { .. }
            )
        {
            eprintln!(
                "--class applies to load, drag and recall: automate and binding-mismatch use the plugin they are about"
            );
            return ExitCode::from(2);
        }
        // A scenario that runs the service cannot share the socket with one.
        let guard = crate::host::refuse_live_service(cli.allow_service && !owns_service);
        let (name, outcome) = match cli.scenario {
            _ if guard.is_err() => ("guard", guard.map(|()| false)),
            Scenario::Load => (
                "load",
                load::run(&cli.bundle, cli.class.as_deref(), &report),
            ),
            Scenario::Drag { param, control, to } => (
                "drag",
                drag::run(
                    &cli.bundle,
                    &drag::Options {
                        param,
                        control,
                        to,
                        class: cli.class,
                        out_dir: cli.out_dir,
                    },
                    &report,
                ),
            ),
            Scenario::Recall => (
                "recall",
                recall::run(&cli.bundle, &cli.device, cli.class.as_deref(), &report),
            ),
            Scenario::Automate { param, seconds } => (
                "automate",
                automate::run(
                    &cli.bundle,
                    &cli.device,
                    &automate::Options { param, seconds },
                    &report,
                ),
            ),
            Scenario::BindingMismatch { no_editor } => (
                "binding-mismatch",
                mismatch::run(
                    &cli.bundle,
                    &cli.device,
                    &mismatch::Options {
                        no_editor,
                        out_dir: cli.out_dir,
                    },
                    &report,
                ),
            ),
        };
        driver::conclude(&report, name, &outcome)
    }

    #[cfg(test)]
    mod tests {
        use super::fraction;

        #[test]
        fn fractions_are_bounded() {
            assert_eq!(fraction("0.25"), Ok(0.25));
            assert_eq!(fraction("0"), Ok(0.0));
            assert_eq!(fraction("1"), Ok(1.0));
            assert!(fraction("1.5").is_err());
            assert!(fraction("-0.1").is_err());
            assert!(fraction("NaN").is_err());
            assert!(fraction("inf").is_err());
            assert!(fraction("x").is_err());
        }
    }
}

#[cfg(target_os = "macos")]
fn main() -> std::process::ExitCode {
    cli::main()
}

#[cfg(not(target_os = "macos"))]
fn main() -> std::process::ExitCode {
    eprintln!("integral-plugin-host runs on macOS only");
    std::process::ExitCode::from(3)
}
