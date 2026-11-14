//! `integral-device-service`: the Device service and nothing else. It reads
//! the link from `service.toml` (by default [`config_path_in`] the home
//! directory), takes flags over it, and runs until it is killed.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use integral_device_service::{LinkConfig, ServiceArgs, config_path_in};

/// Arguments of `integral-device-service`.
#[derive(Parser)]
#[command(
    name = "integral-device-service",
    version,
    about = "The Integral Device service"
)]
struct Cli {
    /// The configuration file to read the link from. Defaults to
    /// `~/Library/Application Support/Integral/service.toml`; with no file
    /// the link is wireless.
    #[arg(long)]
    config: Option<PathBuf>,
    #[command(flatten)]
    service: ServiceArgs,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    // The binary's edge: the one place the home directory is looked up.
    let Some(path) = cli
        .config
        .or_else(|| std::env::home_dir().map(|home| config_path_in(&home)))
    else {
        eprintln!("integral-device-service: no home directory; name the file with --config");
        return ExitCode::FAILURE;
    };
    let result = match LinkConfig::load(&path) {
        Ok(file) => {
            if path.is_file() {
                eprintln!("Link configuration: {}", path.display());
            } else {
                eprintln!("No {}: the wireless link", path.display());
            }
            cli.service.run(file).map_err(|e| e.to_string())
        }
        Err(e) => Err(e.to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("integral-device-service: {e}");
            ExitCode::FAILURE
        }
    }
}
