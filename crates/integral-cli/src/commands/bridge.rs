//! Bridge command: launch the Device service.
//!
//! The service and its launch live in `integral-device-service`
//! ([ADR 0006](../../../../docs/adr/0006-device-service-owns-the-link.md)),
//! which the `integral-device-service` binary runs too. This one reads no
//! `service.toml`: the flags alone name the link, wireless by default.

use anyhow::Result;
use integral_device_service::{LinkConfig, ServiceArgs};

/// Run the Device service until the process is killed.
pub fn bridge(args: &ServiceArgs) -> Result<()> {
    Ok(args.run(LinkConfig::default())?)
}
