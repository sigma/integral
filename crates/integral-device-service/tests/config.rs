//! The service's link configuration: `service.toml` on disk, and the
//! command-line flags over it.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use clap::Parser;
use integral_device_service::{
    CONFIG_FILE_NAME, ConfigError, LinkConfig, LinkKind, ServiceArgs, config_path_in,
};

/// A fresh directory under the target's temporary directory, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "config{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        Self(dir)
    }

    /// The default configuration path, with this directory as the home.
    fn config(&self) -> PathBuf {
        config_path_in(&self.0)
    }

    /// Write `text` as the configuration file.
    fn write(&self, text: &str) -> PathBuf {
        let path = self.config();
        fs::create_dir_all(path.parent().expect("parent")).expect("config dir");
        fs::write(&path, text).expect("write config");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The service's flags, as a binary parses them.
#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    service: ServiceArgs,
}

fn args(flags: &[&str]) -> ServiceArgs {
    Cli::try_parse_from(std::iter::once("service").chain(flags.iter().copied()))
        .expect("flags parse")
        .service
}

#[test]
fn a_missing_file_is_wireless() {
    let home = TempDir::new();
    let config = LinkConfig::load(&home.config()).expect("load");
    assert_eq!(config.link, LinkKind::Wireless);
    assert_eq!(config, LinkConfig::default());
}

#[test]
fn reads_each_link_kind_from_the_file() {
    for (text, link) in [
        ("link = \"wireless\"\n", LinkKind::Wireless),
        (
            "link = \"rtp\"\nport = \"Network Integra\"\n",
            LinkKind::Rtp,
        ),
        ("link = \"usb\"\n", LinkKind::Usb),
    ] {
        let home = TempDir::new();
        let path = home.write(text);
        assert!(path.ends_with(CONFIG_FILE_NAME));
        assert_eq!(LinkConfig::load(&path).expect("load").link, link, "{text}");
    }
}

#[test]
fn a_malformed_file_is_an_error_naming_it() {
    let home = TempDir::new();
    let path = home.write("link = \"carrier pigeon\"\n");
    let err = LinkConfig::load(&path).expect_err("malformed");
    assert!(matches!(err, ConfigError::Parse { .. }));
    assert!(err.to_string().contains(&path.display().to_string()));
}

#[test]
fn a_flag_overrides_the_file() {
    let home = TempDir::new();
    let path = home.write("link = \"rtp\"\nport = \"Network Integra\"\ndevice_id = \"10\"\n");
    let file = LinkConfig::load(&path).expect("load");

    let config = args(&["--port", "Other Session", "--device-id", "11"]).link_config(file.clone());
    assert_eq!(config.link, LinkKind::Rtp);
    assert_eq!(config.port.as_deref(), Some("Other Session"));
    assert_eq!(config.device_id, Some(0x11));

    let config = args(&["--link", "usb"]).link_config(file.clone());
    assert_eq!(config.link, LinkKind::Usb);
    assert_eq!(config.port, None, "the file's port is the rtp link's");
    assert_eq!(config.device_id, Some(0x10));

    assert_eq!(
        args(&[]).link_config(file.clone()),
        file,
        "no flag, the file"
    );
}
