//! `part show` and `part set`: a Part's settings (KBD, PITCH, OFFSET, MIDI,
//! VOICE, SCALE) by the names, ranges and displays of core's
//! [`part_settings`](integral_core::part_settings) registry.
//!
//! Both read the Part as a mixer load does ([`settings_blocks`]); `set`
//! then writes through [`DeviceState::set_part_setting`], so a key or
//! velocity range never crosses.

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use integral_core::device::DeviceState;
use integral_core::host_params::HostFormat;
use integral_core::mixer_load::MixerBlock;
use integral_core::param_registry::ParamFormat;
use integral_core::part_settings::{PartSetting, PartSettingGroup, PartSettings};
use integral_core::state::MixerState;

use super::pack::send_next;
use crate::device::IntegraDevice;
use crate::midi::Endpoint;

/// The mixer-load blocks that hold Part `part`'s (0-based) settings: its
/// Part dump (KBD, PITCH, OFFSET), the rest of its Part block (MIDI, SCALE,
/// Velocity Curve) and Studio Set Common's Voice Reserves.
pub fn settings_blocks(part: u8) -> [MixerBlock; 3] {
    [
        MixerBlock::Part(part),
        MixerBlock::PartSettings(part),
        MixerBlock::VoiceReserve,
    ]
}

/// Read Part `part`'s (0-based) settings into `state`.
fn read_settings(dev: &mut IntegraDevice, state: &mut MixerState, part: u8) -> Result<()> {
    for block in settings_blocks(part) {
        let (address, size) = block
            .request()
            .context("a Part settings block has a request")?;
        let data = dev
            .request_data(&address, &size)
            .with_context(|| format!("could not read {}", block.label()))?;
        block.apply(state, &data);
    }
    Ok(())
}

/// The field whose CLI name is `name` (`key-lower`).
///
/// # Errors
///
/// When no field has that name; the message lists every name.
pub fn parse_field(name: &str) -> Result<PartSetting> {
    PartSetting::from_name(name).with_context(|| {
        let names: Vec<_> = PartSetting::ALL.iter().map(|f| f.name()).collect();
        format!(
            "unknown Part setting `{name}`; one of: {}",
            names.join(", ")
        )
    })
}

/// The display text of `field`'s raw `value` (`C4`, `+12`, `ON`, `TONE`).
pub fn display(field: PartSetting, value: u8) -> String {
    field.def().spec.format.format(i32::from(value))
}

/// The raw value of `field` that `text` reads as, in the display form
/// [`display`] prints (case and spaces aside; a signed number may drop its
/// `+`).
///
/// # Errors
///
/// When `text` names no value in the field's range; the message gives the
/// range, or every choice of a named one.
pub fn parse_value(field: PartSetting, text: &str) -> Result<u8> {
    let spec = field.def().spec;
    HostFormat::Field(spec.format.clone())
        .parse(text, spec.min, spec.max)
        .and_then(|raw| u8::try_from(raw).ok())
        .with_context(|| {
            let takes = match spec.format {
                ParamFormat::Named(names) => format!("one of {}", names.join(", ")),
                _ => format!(
                    "{} to {}",
                    display(field, spec.min as u8),
                    display(field, spec.max as u8)
                ),
            };
            format!("{} takes {takes}, not `{text}`", field.name())
        })
}

/// `part show`'s text: every field by group, as `name  value`.
pub fn render_show(part_number: u8, settings: &PartSettings) -> String {
    let width = PartSetting::ALL
        .iter()
        .map(|f| f.name().len())
        .max()
        .unwrap_or(0);
    let mut out = format!("Part {part_number}\n");
    for group in PartSettingGroup::ALL {
        out.push_str(&format!("\n{}\n", group.label()));
        for field in PartSetting::ALL
            .into_iter()
            .filter(|f| f.def().group == group)
        {
            let value = display(field, settings.get(field));
            out.push_str(&format!("  {:width$}  {value}\n", field.name()));
        }
    }
    out
}

/// What `part set` did.
#[derive(Debug, PartialEq, Eq)]
pub struct SetOutcome {
    /// The field set.
    pub field: PartSetting,
    /// The raw value asked for.
    pub requested: u8,
    /// The raw value stored: a range's other end may hold it back.
    pub stored: u8,
}

/// Set `part`'s (0-based) `field` to raw `requested` in `state`, which
/// holds the Part as read, queueing what to send.
///
/// # Errors
///
/// A value out of the field's range, or a Part past the device's.
pub fn set_in(
    state: &mut DeviceState,
    part: u8,
    field: PartSetting,
    requested: u8,
) -> Result<SetOutcome> {
    let stored = state.set_part_setting(part, field, requested)?;
    Ok(SetOutcome {
        field,
        requested,
        stored,
    })
}

/// The line `part set` prints, and a note when the value was held back.
pub fn render_set(part_number: u8, outcome: &SetOutcome) -> (String, Option<String>) {
    let field = outcome.field;
    let line = format!(
        "Part {part_number} {}: {}",
        field.name(),
        display(field, outcome.stored)
    );
    let note = (outcome.stored != outcome.requested).then(|| {
        format!(
            "{} held at {}: a range's Lower stays at or under its Upper",
            field.name(),
            display(field, outcome.stored)
        )
    });
    (line, note)
}

/// The 0-based index of Part `part_number` (1-based).
fn part_index(part_number: u8) -> Result<u8> {
    part_number
        .checked_sub(1)
        .context("Parts are numbered from 1")
}

/// `part show <n>`: read Part `part_number` (1-based) and print its
/// settings.
pub fn show(
    endpoint: Endpoint<'_>,
    device_id: Option<u8>,
    timeout: Duration,
    part_number: u8,
) -> Result<()> {
    let part = part_index(part_number)?;
    let mut dev = IntegraDevice::connect_detecting(endpoint, device_id, timeout)?.quiet();
    let mut state = MixerState::default();
    read_settings(&mut dev, &mut state, part)?;
    print!(
        "{}",
        render_show(part_number, &state.parts[usize::from(part)].settings)
    );
    Ok(())
}

/// `part set <n> <field> <value>`: read Part `part_number` (1-based), set
/// one field and send it.
pub fn set(
    endpoint: Endpoint<'_>,
    device_id: Option<u8>,
    timeout: Duration,
    part_number: u8,
    name: &str,
    text: &str,
) -> Result<()> {
    let part = part_index(part_number)?;
    // Refuse a bad name or value before opening the connection.
    let field = parse_field(name)?;
    let requested = parse_value(field, text)?;
    let mut dev = IntegraDevice::connect_detecting(endpoint, device_id, timeout)?.quiet();
    let mut state = DeviceState::new(dev.device_id());
    read_settings(&mut dev, state.state_mut(), part)?;
    let outcome = set_in(&mut state, part, field, requested)?;
    send_queued(&mut dev, &mut state)?;
    let (line, note) = render_set(part_number, &outcome);
    if let Some(note) = note {
        eprintln!("{note}");
    }
    println!("{line}");
    Ok(())
}

/// Send everything `state` queued, as its throttle allows.
fn send_queued(dev: &mut IntegraDevice, state: &mut DeviceState) -> Result<()> {
    let start = Instant::now();
    while state.has_pending() {
        send_next(dev, state, start)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use integral_core::sysex;

    #[test]
    fn a_value_parses_from_its_display_form() {
        let parse = |name, text| parse_value(parse_field(name).unwrap(), text).unwrap();
        assert_eq!(parse("key-lower", "C4"), 60);
        assert_eq!(parse("key-upper", "g9"), 127);
        assert_eq!(parse("key-lower", "C-1"), 0);
        assert_eq!(parse("coarse-tune", "+12"), 76);
        assert_eq!(parse("coarse-tune", "12"), 76);
        assert_eq!(parse("coarse-tune", "-48"), 16);
        assert_eq!(parse("octave", "0"), 64);
        assert_eq!(parse("rx-hold", "ON"), 1);
        assert_eq!(parse("rx-hold", "off"), 0);
        assert_eq!(parse("bend-range", "12"), 12);
        assert_eq!(parse("bend-range", "TONE"), 25);
        assert_eq!(parse("portamento-time", "tone"), 128);
        assert_eq!(parse("voice-reserve", "FULL"), 64);
        assert_eq!(parse("scale-type", "just-maj"), 2);
        assert_eq!(parse("scale-key", "F#"), 6);
        assert_eq!(parse("velocity-lower", "30"), 30);
        assert_eq!(parse("mono-poly", "MONO"), 0);
    }

    #[test]
    fn a_value_outside_the_range_is_refused_with_the_range() {
        let field = parse_field("octave").unwrap();
        let err = parse_value(field, "+4").unwrap_err().to_string();
        assert_eq!(err, "octave takes -3 to +3, not `+4`");
        let field = parse_field("velocity-lower").unwrap();
        assert!(parse_value(field, "0").is_err());
        let field = parse_field("key-lower").unwrap();
        assert!(parse_value(field, "H2").is_err());
        assert!(parse_value(field, "").is_err());
        let field = parse_field("scale-type").unwrap();
        let err = parse_value(field, "just").unwrap_err().to_string();
        assert!(
            err.starts_with("scale-type takes one of CUSTOM, EQUAL, JUST-MAJ"),
            "{err}"
        );
    }

    #[test]
    fn an_unknown_field_is_refused_with_the_names() {
        let err = parse_field("level").unwrap_err().to_string();
        assert!(err.starts_with("unknown Part setting `level`; one of: key-lower, key-upper"));
        assert!(err.contains("rx-hold"), "{err}");
        assert!(err.ends_with("scale-b"), "{err}");
        assert!(parse_field("Key-Lower").is_err(), "names are kebab-case");
    }

    #[test]
    fn show_lists_every_field_by_group() {
        let mut settings = PartSettings::default();
        settings.set(PartSetting::KeyRangeLower, 60);
        settings.set(PartSetting::CoarseTune, 76);
        let text = render_show(16, &settings);
        assert!(text.starts_with("Part 16\n\nKBD\n  key-lower "), "{text}");
        assert!(text.contains("\nPITCH\n"), "{text}");
        assert!(text.contains("\nSCALE\n"), "{text}");
        let rows: Vec<_> = text.lines().filter(|l| l.starts_with("  ")).collect();
        assert_eq!(rows.len(), PartSetting::ALL.len());
        let row = |name: &str| {
            rows.iter()
                .find(|l| l.split_whitespace().next() == Some(name))
                .map(|l| l.split_whitespace().nth(1).unwrap().to_string())
        };
        assert_eq!(row("key-lower").as_deref(), Some("C4"));
        assert_eq!(row("coarse-tune").as_deref(), Some("+12"));
        assert_eq!(row("bend-range").as_deref(), Some("TONE"));
        assert_eq!(row("rx-hold").as_deref(), Some("ON"));
    }

    #[test]
    fn set_sends_the_parsed_value_to_the_part() {
        let mut state = DeviceState::new(0x10);
        let outcome = set_in(&mut state, 15, PartSetting::CoarseTune, 76).unwrap();
        assert_eq!(outcome.stored, 76);
        let at = PartSetting::CoarseTune.address(15).unwrap();
        assert_eq!(state.drain(0.0), Some(sysex::build_dt1(0x10, &at, &[76])));
        assert_eq!(
            render_set(16, &outcome),
            ("Part 16 coarse-tune: +12".to_string(), None)
        );
    }

    #[test]
    fn set_holds_a_lower_at_its_upper() {
        let mut state = DeviceState::new(0x10);
        state.state_mut().parts[15]
            .settings
            .set(PartSetting::KeyRangeUpper, 48);
        let outcome = set_in(&mut state, 15, PartSetting::KeyRangeLower, 60).unwrap();
        assert_eq!((outcome.requested, outcome.stored), (60, 48));
        let at = PartSetting::KeyRangeLower.address(15).unwrap();
        assert_eq!(state.drain(0.0), Some(sysex::build_dt1(0x10, &at, &[48])));
        let (line, note) = render_set(16, &outcome);
        assert_eq!(line, "Part 16 key-lower: C3");
        assert!(note.unwrap().starts_with("key-lower held at C3"));
    }

    #[test]
    fn the_blocks_read_cover_every_field() {
        let mut state = MixerState::default();
        for block in settings_blocks(15) {
            let (_, size) = block.request().unwrap();
            block.apply(&mut state, &vec![1; size.value() as usize]);
        }
        let settings = &state.parts[15].settings;
        for field in PartSetting::ALL {
            let expected = if field == PartSetting::PortamentoTime {
                0x11
            } else {
                1
            };
            assert_eq!(settings.get(field), expected, "{field:?} is not read");
        }
    }
}
