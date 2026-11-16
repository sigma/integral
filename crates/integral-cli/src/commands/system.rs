//! `system show` and `system set`: System Common, by the field names of
//! core's System registry ([`integral_core::system`]).
//!
//! The names, ranges and parsing all come from the registry; this module
//! only does the I/O and prints.

use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use integral_core::address::SYSTEM_COMMON;
use integral_core::device::DeviceState;
use integral_core::system::{
    self, SYSTEM_COMMON_SIZE, SystemCommon, SystemField, VOLATILE_NOTICE, system_spec,
};

use crate::device::IntegraDevice;
use crate::midi::Endpoint;

/// `system show`: read System Common and print every field.
pub fn show(endpoint: Endpoint<'_>, device_id: Option<u8>, timeout: Duration) -> Result<()> {
    let mut dev = IntegraDevice::connect_detecting(endpoint, device_id, timeout)?.quiet();
    let data = dev.request_data(&SYSTEM_COMMON, &SYSTEM_COMMON_SIZE)?;
    let common = system::parse_system_common(&data)?;
    print!("{}", render_show(&common));
    Ok(())
}

/// Every field's name, in block order.
fn field_names() -> impl Iterator<Item = &'static str> + Clone {
    SystemField::ALL.iter().map(|&f| system_spec(f).name)
}

/// One line per field, in block order: its name, then its display value,
/// the values aligned.
pub fn render_show(common: &SystemCommon) -> String {
    let width = field_names().map(str::len).max().unwrap_or(0);
    field_names()
        .zip(SystemField::ALL)
        .map(|(name, &field)| format!("{name:<width$}  {}\n", common.display(field)))
        .collect()
}

/// The field `name` names and the raw value `value` (its display form)
/// stands for, both per the registry.
///
/// # Errors
/// No field has that name, or nothing in the field's range shows as
/// `value`.
pub fn parse_set(name: &str, value: &str) -> Result<(SystemField, i32)> {
    let field = SystemField::from_name(name).ok_or_else(|| {
        let names: Vec<_> = field_names().collect();
        anyhow!("unknown System field: {name}. Use: {}", names.join(", "))
    })?;
    let spec = system_spec(field);
    let raw = spec.parse(value).ok_or_else(|| {
        anyhow!(
            "{name} takes {} to {}, not {value:?}",
            spec.format.format(spec.min),
            spec.format.format(spec.max)
        )
    })?;
    Ok((field, raw))
}

/// `system set`: write one field, then say what was set and that it lasts
/// until power-off. The field and value are checked before any port opens.
pub fn set(
    endpoint: Endpoint<'_>,
    device_id: Option<u8>,
    timeout: Duration,
    name: &str,
    value: &str,
) -> Result<()> {
    let (field, raw) = parse_set(name, value)?;
    let mut dev = IntegraDevice::connect_detecting(endpoint, device_id, timeout)?.quiet();
    let mut state = DeviceState::new(dev.device_id());
    state.set_system_field(field, raw)?;
    // One DT1, and the throttle starts open: it drains at once.
    let dt1 = state.drain(0.0).context("no DT1 queued")?;
    dev.conn_out_mut()
        .send(&dt1)
        .context("failed to send DT1")?;
    let spec = system_spec(field);
    println!("{} = {}", spec.name, spec.format.format(raw));
    println!("{VOLATILE_NOTICE}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_value_in_its_display_form() {
        assert_eq!(
            parse_set("master-key-shift", "-3").unwrap(),
            (SystemField::MasterKeyShift, 61)
        );
        assert_eq!(
            parse_set("scale-tune-switch", "ON").unwrap(),
            (SystemField::ScaleTuneSwitch, 1)
        );
        assert_eq!(
            parse_set("control-source", "studio set").unwrap(),
            (SystemField::ControlSource, 1)
        );
        assert_eq!(
            parse_set("system-tempo", "120").unwrap(),
            (SystemField::SystemTempo, 120)
        );
    }

    #[test]
    fn master_tune_takes_hz() {
        let (field, raw) = parse_set("master-tune", "442.0").unwrap();
        assert_eq!(field, SystemField::MasterTune);
        assert_eq!(raw, system::master_tune_from_hz(442.0));
        assert_eq!(
            parse_set("master-tune", "440").unwrap().1,
            system::MASTER_TUNE_CENTER
        );
    }

    #[test]
    fn an_unknown_field_lists_the_names() {
        let err = parse_set("master-volume", "1").unwrap_err().to_string();
        assert!(err.contains("unknown System field: master-volume"), "{err}");
        for &field in SystemField::ALL {
            assert!(err.contains(system_spec(field).name), "{err}");
        }
    }

    #[test]
    fn a_value_out_of_range_names_the_range() {
        let err = parse_set("master-key-shift", "+25")
            .unwrap_err()
            .to_string();
        assert_eq!(err, "master-key-shift takes -24 to +24, not \"+25\"");
        assert!(parse_set("scale-tune-switch", "maybe").is_err());
        // The unused Control Source 32 names no source, though it is in range.
        assert!(parse_set("system-control-1-source", "32").is_err());
        assert!(parse_set("master-tune", "500").is_err());
    }

    #[test]
    fn show_prints_every_field_by_name() {
        let mut data = [0u8; system::SYSTEM_COMMON_LEN];
        // Master Tune 0 cent (1024, nibblized), Key Shift 0, tempo 120.
        data[0x00..0x04].copy_from_slice(&[0, 4, 0, 0]);
        data[0x04] = 64;
        data[0x26..0x28].copy_from_slice(&[7, 8]);
        let shown = render_show(&system::parse_system_common(&data).unwrap());
        let lines: Vec<_> = shown.lines().collect();
        assert_eq!(lines.len(), SystemField::ALL.len());
        for (line, &field) in lines.iter().zip(SystemField::ALL) {
            assert!(line.starts_with(system_spec(field).name), "{line}");
        }
        let value = |name: &str| {
            let line = lines.iter().find(|l| l.starts_with(name)).unwrap();
            line[name.len()..].trim().to_string()
        };
        assert_eq!(value("master-tune"), "440.00 Hz");
        assert_eq!(value("master-key-shift"), "0");
        assert_eq!(value("system-tempo"), "120");
        // The values start in one column.
        let columns: Vec<_> = (lines.iter().zip(SystemField::ALL))
            .map(|(line, &f)| line.len() - line[system_spec(f).name.len()..].trim_start().len())
            .collect();
        assert!(columns.iter().all(|&c| c == columns[0]), "{shown}");
    }
}
