//! Bridge command: launch the Device service.
//!
//! The service itself lives in `integral-device-service`
//! ([ADR 0006](../../../../docs/adr/0006-device-service-owns-the-link.md)).
//! This module only turns arguments into a link and runs the service until
//! the process is killed, whose exit closes the link and removes the ports.

use std::net::IpAddr;

use anyhow::{Context, Result, bail};
use clap::{Args, ValueEnum};
use integral_core::device_spec::INTEGRA7;
use integral_device_service::{
    Config, DeviceLink, MidirLink, NOTE_PORT_NAME, PortMatch, Service, VirtualNotePort,
    WirelessLink, spawn, stderr_log,
};

/// How the service reaches the INTEGRA-7.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum LinkKind {
    /// The wireless LAN session (also shared as the MIDI port
    /// "INTEGRA-7 (Wireless)").
    Wireless,
    /// An RTP-MIDI session; `--port` names it (a substring).
    Rtp,
    /// A USB port; `--port` defaults to "INTEGRA-7".
    Usb,
}

/// Arguments of the `bridge` command.
#[derive(Args)]
pub struct BridgeArgs {
    /// The link to the INTEGRA-7.
    #[arg(long, value_enum, default_value_t = LinkKind::Wireless)]
    link: LinkKind,
    /// Wireless: device IP address. Discovered on the LAN if omitted.
    #[arg(long)]
    host: Option<IpAddr>,
    /// Rtp, usb: the MIDI port pair to open.
    #[arg(long)]
    port: Option<String>,
    /// SysEx device ID (hex, e.g. 10), instead of the one the device gives
    /// in its Identity Reply.
    #[arg(long, value_parser = crate::parse_device_id)]
    device_id: Option<u8>,
}

/// Build the link the arguments name.
fn link(args: &BridgeArgs) -> Result<Box<dyn DeviceLink>> {
    let log = stderr_log();
    match args.link {
        LinkKind::Wireless => {
            if args.port.is_some() {
                bail!("--port applies to --link rtp and --link usb");
            }
            Ok(Box::new(
                WirelessLink::new(args.host, log).with_shared_port(true),
            ))
        }
        LinkKind::Rtp | LinkKind::Usb if args.host.is_some() => {
            bail!("--host applies to --link wireless")
        }
        LinkKind::Rtp => {
            let name = args
                .port
                .clone()
                .context("--link rtp needs --port <name of the RTP-MIDI session>")?;
            Ok(Box::new(MidirLink::new(PortMatch::Contains(name), log)))
        }
        LinkKind::Usb => {
            let name = args
                .port
                .clone()
                .unwrap_or_else(|| INTEGRA7.name.to_string());
            Ok(Box::new(MidirLink::new(PortMatch::Exact(name), log)))
        }
    }
}

/// Run the Device service until the process is killed.
pub fn bridge(args: &BridgeArgs) -> Result<()> {
    let link = link(args)?;
    let config = Config::new(args.device_id);
    let service = Service::new(link, config);
    let running = spawn(service, Some(Box::new(VirtualNotePort::new())))
        .context("start the Device service")?;
    eprintln!("Device service running; notes go to the MIDI port '{NOTE_PORT_NAME}'");
    running.join();
    Ok(())
}
