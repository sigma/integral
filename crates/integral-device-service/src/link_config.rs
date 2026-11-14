//! Which link the service opens: the `service.toml` file, the flags that
//! override it, and the [`DeviceLink`] they name.
//!
//! The file sits at [`config_path_in`] the user's home directory. With no
//! file the link is [`LinkKind::Wireless`]. Nothing here finds the home
//! directory: the binary does, and passes the path in.

use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use clap::{Args, ValueEnum};
use integral_core::device_spec::INTEGRA7;
use serde::Deserialize;
use thiserror::Error;

use crate::{DeviceLink, Log, MidirLink, PortMatch, WirelessLink};

/// The name of the service's configuration file.
pub const CONFIG_FILE_NAME: &str = "service.toml";

/// The directory of [`CONFIG_FILE_NAME`] under a home directory, as macOS
/// lays it out.
const CONFIG_DIR: [&str; 3] = ["Library", "Application Support", "Integral"];

/// The configuration file under the home directory `home`:
/// `~/Library/Application Support/Integral/service.toml`.
pub fn config_path_in(home: &Path) -> PathBuf {
    let mut path = home.to_path_buf();
    path.extend(CONFIG_DIR);
    path.push(CONFIG_FILE_NAME);
    path
}

/// How the service reaches the INTEGRA-7.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum LinkKind {
    /// The wireless LAN session (also shared as the MIDI port
    /// "INTEGRA-7 (Wireless)").
    #[default]
    Wireless,
    /// An RTP-MIDI session; `port` names it (a substring).
    Rtp,
    /// A USB port; `port` defaults to "INTEGRA-7".
    Usb,
}

/// The link the service opens, as `service.toml` holds it. The default is
/// [`LinkKind::Wireless`], with the device discovered on the LAN.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LinkConfig {
    /// The kind of link.
    pub link: LinkKind,
    /// Wireless: the device's IP address. Discovered on the LAN if absent.
    pub host: Option<IpAddr>,
    /// Rtp, usb: the MIDI port pair to open.
    pub port: Option<String>,
    /// The SysEx device ID to use instead of the one the device gives in its
    /// Identity Reply: hex in a string, as `--device-id` takes it (`"10"`).
    #[serde(deserialize_with = "hex_device_id")]
    pub device_id: Option<u8>,
}

/// Deserialize `device_id` with [`parse_device_id`], so the file and the
/// flag read the same digits the same way.
fn hex_device_id<'de, D: serde::Deserializer<'de>>(de: D) -> Result<Option<u8>, D::Error> {
    let text = String::deserialize(de)?;
    parse_device_id(&text)
        .map(Some)
        .map_err(serde::de::Error::custom)
}

/// The flags that override [`LinkConfig`]'s fields, as the service's
/// command line takes them.
#[derive(Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct LinkOverrides {
    /// The link to the INTEGRA-7. Defaults to the configuration file's link
    /// when one is read, else wireless. Naming another kind than the file's
    /// drops the file's `host` and `port`.
    #[arg(long, value_enum)]
    pub link: Option<LinkKind>,
    /// Wireless: device IP address. Discovered on the LAN if omitted.
    #[arg(long)]
    pub host: Option<IpAddr>,
    /// Rtp, usb: the MIDI port pair to open.
    #[arg(long)]
    pub port: Option<String>,
    /// SysEx device ID (hex, e.g. 10), instead of the one the device gives
    /// in its Identity Reply.
    #[arg(long, value_parser = parse_device_id)]
    pub device_id: Option<u8>,
}

/// Parse a hex device ID (`10` is 0x10), as the `--device-id` flags take it.
///
/// # Errors
///
/// The message for clap, when `s` is not one hex byte.
pub fn parse_device_id(s: &str) -> Result<u8, String> {
    u8::from_str_radix(s, 16).map_err(|e| format!("invalid hex device ID: {e}"))
}

/// Why `service.toml` could not be read.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The file exists but could not be read.
    #[error("read {}: {source}", path.display())]
    Read {
        /// The file.
        path: PathBuf,
        /// The OS error.
        source: io::Error,
    },
    /// The file is not a valid configuration.
    #[error("parse {}: {source}", path.display())]
    Parse {
        /// The file.
        path: PathBuf,
        /// What is wrong with it.
        source: Box<toml::de::Error>,
    },
}

/// Why a [`LinkConfig`] names no link.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LinkConfigError {
    /// A `port` on the wireless link.
    #[error("port applies to the rtp and usb links")]
    PortOnWireless,
    /// A `host` on a MIDI port link.
    #[error("host applies to the wireless link")]
    HostOnPort,
    /// An rtp link with no port.
    #[error("the rtp link needs a port: the name of the RTP-MIDI session")]
    RtpWithoutPort,
}

impl LinkConfig {
    /// Parse the text of a `service.toml`.
    ///
    /// # Errors
    ///
    /// The TOML error, for a malformed file, an unknown key or link, or a
    /// value of the wrong type.
    pub fn parse(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }

    /// Read the `service.toml` at `path`. A missing file is the default
    /// config, the wireless link.
    ///
    /// # Errors
    ///
    /// [`ConfigError`], when the file exists but cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text).map_err(|source| ConfigError::Parse {
                path: path.to_path_buf(),
                source: Box::new(source),
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(ConfigError::Read {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    /// This config with each flag given in `flags` in place of its field. A
    /// `link` other than this one's drops `host` and `port`, which belong to
    /// the file's link.
    #[must_use]
    pub fn with_overrides(mut self, flags: &LinkOverrides) -> Self {
        if let Some(link) = flags.link
            && link != self.link
        {
            self.link = link;
            self.host = None;
            self.port = None;
        }
        if let Some(host) = flags.host {
            self.host = Some(host);
        }
        if let Some(port) = &flags.port {
            self.port = Some(port.clone());
        }
        if let Some(device_id) = flags.device_id {
            self.device_id = Some(device_id);
        }
        self
    }

    /// The link this config names, logging to `log`.
    ///
    /// # Errors
    ///
    /// [`LinkConfigError`], for a field the link does not take or a missing
    /// one it needs.
    pub fn open(&self, log: Log) -> Result<Box<dyn DeviceLink>, LinkConfigError> {
        match self.link {
            LinkKind::Wireless => {
                if self.port.is_some() {
                    return Err(LinkConfigError::PortOnWireless);
                }
                Ok(Box::new(
                    WirelessLink::new(self.host, log).with_shared_port(true),
                ))
            }
            LinkKind::Rtp | LinkKind::Usb if self.host.is_some() => {
                Err(LinkConfigError::HostOnPort)
            }
            LinkKind::Rtp => {
                let name = self.port.clone().ok_or(LinkConfigError::RtpWithoutPort)?;
                Ok(Box::new(MidirLink::new(PortMatch::Contains(name), log)))
            }
            LinkKind::Usb => {
                let name = self
                    .port
                    .clone()
                    .unwrap_or_else(|| INTEGRA7.name.to_string());
                Ok(Box::new(MidirLink::new(PortMatch::Exact(name), log)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    #[test]
    fn config_path_is_under_application_support() {
        assert_eq!(
            config_path_in(Path::new("/Users/someone")),
            PathBuf::from("/Users/someone/Library/Application Support/Integral/service.toml")
        );
    }

    #[test]
    fn parses_a_wireless_link() {
        let config = LinkConfig::parse("link = \"wireless\"\nhost = \"192.168.1.20\"\n").unwrap();
        assert_eq!(
            config,
            LinkConfig {
                link: LinkKind::Wireless,
                host: Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20))),
                ..LinkConfig::default()
            }
        );
    }

    #[test]
    fn parses_an_rtp_link() {
        let config =
            LinkConfig::parse("link = \"rtp\"\nport = \"Network Integra\"\ndevice_id = \"11\"\n")
                .unwrap();
        assert_eq!(
            config,
            LinkConfig {
                link: LinkKind::Rtp,
                port: Some("Network Integra".into()),
                device_id: Some(0x11),
                ..LinkConfig::default()
            }
        );
    }

    #[test]
    fn parses_a_usb_link() {
        let config = LinkConfig::parse("link = \"usb\"\n").unwrap();
        assert_eq!(
            config,
            LinkConfig {
                link: LinkKind::Usb,
                ..LinkConfig::default()
            }
        );
    }

    #[test]
    fn an_empty_file_is_wireless() {
        assert_eq!(LinkConfig::parse("").unwrap().link, LinkKind::Wireless);
    }

    #[test]
    fn refuses_an_unknown_link_or_key() {
        assert!(LinkConfig::parse("link = \"bluetooth\"\n").is_err());
        assert!(LinkConfig::parse("lnik = \"usb\"\n").is_err());
        assert!(LinkConfig::parse("device_id = \"300\"\n").is_err());
        assert!(
            LinkConfig::parse("device_id = 16\n").is_err(),
            "an integer would read differently from the flag's hex"
        );
    }

    #[test]
    fn another_link_drops_the_files_host_and_port() {
        let file = LinkConfig {
            link: LinkKind::Rtp,
            port: Some("Network Integra".into()),
            device_id: Some(0x10),
            ..LinkConfig::default()
        };
        let flags = LinkOverrides {
            link: Some(LinkKind::Wireless),
            ..LinkOverrides::default()
        };
        assert_eq!(
            file.with_overrides(&flags),
            LinkConfig {
                link: LinkKind::Wireless,
                device_id: Some(0x10),
                ..LinkConfig::default()
            }
        );
    }

    #[test]
    fn the_same_link_keeps_the_files_port() {
        let file = LinkConfig {
            link: LinkKind::Usb,
            port: Some("INTEGRA-7 USB".into()),
            ..LinkConfig::default()
        };
        let flags = LinkOverrides {
            link: Some(LinkKind::Usb),
            ..LinkOverrides::default()
        };
        assert_eq!(file.clone().with_overrides(&flags), file);
    }

    #[test]
    fn refuses_fields_the_link_does_not_take() {
        let log = crate::stderr_log();
        let wireless_port = LinkConfig {
            port: Some("x".into()),
            ..LinkConfig::default()
        };
        assert_eq!(
            wireless_port.open(log.clone()).err(),
            Some(LinkConfigError::PortOnWireless)
        );
        let usb_host = LinkConfig {
            link: LinkKind::Usb,
            host: Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            ..LinkConfig::default()
        };
        assert_eq!(
            usb_host.open(log.clone()).err(),
            Some(LinkConfigError::HostOnPort)
        );
        let rtp_bare = LinkConfig {
            link: LinkKind::Rtp,
            ..LinkConfig::default()
        };
        assert_eq!(
            rtp_bare.open(log).err(),
            Some(LinkConfigError::RtpWithoutPort)
        );
    }
}
