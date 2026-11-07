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
mod ax;
#[cfg(target_os = "macos")]
mod cgwindow;
#[cfg(target_os = "macos")]
mod drag;
#[cfg(target_os = "macos")]
mod driver;
#[cfg(target_os = "macos")]
mod gesture;
#[cfg(target_os = "macos")]
mod host;
#[cfg(target_os = "macos")]
mod pointer;
#[cfg(target_os = "macos")]
mod report;
#[cfg(target_os = "macos")]
mod screenshot;

#[cfg(target_os = "macos")]
mod cli {
    use std::path::PathBuf;
    use std::process::ExitCode;
    use std::sync::Arc;

    use clap::{Parser, Subcommand};

    use crate::{drag, driver, report::Report};

    #[derive(Parser)]
    #[command(about = "Dev-only VST3 test host for the Integral plugin")]
    struct Cli {
        /// The `.vst3` bundle to load (`just bundle-plugin` makes it).
        #[arg(long)]
        bundle: PathBuf,
        /// Where screenshots (window-only) are written.
        #[arg(long)]
        out_dir: PathBuf,
        #[command(subcommand)]
        scenario: Scenario,
    }

    #[derive(Subcommand)]
    enum Scenario {
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
        let (name, outcome) = match cli.scenario {
            Scenario::Drag { param, control, to } => (
                "drag",
                drag::run(
                    &cli.bundle,
                    &drag::Options {
                        param,
                        control,
                        to,
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
