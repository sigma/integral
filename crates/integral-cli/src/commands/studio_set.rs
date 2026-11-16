//! `studio-set` commands: show and set the current Studio Set's settings
//! (Tone Control 1–4 Source, the tempo, each MIDI channel's Phase Lock).
//!
//! The fields, their names, ranges and displays live in
//! `integral_core::studio_set_settings`; the reads are the mixer load's
//! `MixerBlock`s, and a write goes through `DeviceState`. This module parses
//! arguments, talks MIDI and prints.

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use integral_core::device::DeviceState;
use integral_core::device_spec::MIDI_CHANNELS;
use integral_core::host_params::HostFormat;
use integral_core::mfx_params::control_source_name;
use integral_core::mixer_load::MixerBlock;
use integral_core::param_registry::ParamFormat;
use integral_core::state::{MixerState, StudioSetSettings};
use integral_core::studio_set_settings::StudioSetField;

use crate::device::IntegraDevice;
use crate::midi::Endpoint;

/// The mixer-load blocks that hold the Studio Set settings: the Tone
/// Control sources with the tempo, then each MIDI channel's Phase Lock.
fn blocks() -> impl Iterator<Item = MixerBlock> {
    std::iter::once(MixerBlock::StudioSetControl)
        .chain((0..MIDI_CHANNELS).map(MixerBlock::PhaseLock))
}

/// Read the Studio Set settings, one RQ1 per block.
fn read_settings(dev: &mut IntegraDevice) -> Result<StudioSetSettings> {
    let mut state = MixerState::default();
    for block in blocks() {
        let (address, size) = block
            .request()
            .with_context(|| format!("{} has no read", block.label()))?;
        let data = dev
            .request_data(&address, &size)
            .with_context(|| format!("could not read {}", block.label()))?;
        block.apply(&mut state, &data);
    }
    Ok(state.studio_set_settings)
}

/// The `studio-set show` table: one `<name>  <value>` line per field, in
/// registry order, values aligned.
pub fn render_show(settings: &StudioSetSettings) -> String {
    let rows: Vec<(String, String)> = StudioSetField::all()
        .map(|field| (field.name(), display(field, field.value(settings))))
        .collect();
    let width = rows.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
    rows.iter()
        .map(|(name, value)| format!("{name:width$}  {value}\n"))
        .collect()
}

/// The display form of `field`'s raw value `raw` (`96`, `CC16`, `ON`).
fn display(field: StudioSetField, raw: i32) -> String {
    field.spec().format.format(raw)
}

/// Whether `format` names `raw`: a Control Source list skips a value (the
/// unused 32), which it shows as the bare number.
fn names_value(format: &ParamFormat, raw: i32) -> bool {
    match format {
        ParamFormat::ControlSource(kind) => {
            u8::try_from(raw).is_ok_and(|v| control_source_name(*kind, v).is_some())
        }
        _ => true,
    }
}

/// Parse a `studio-set set` argument pair: a registry field name (exact,
/// as `studio-set show` prints it) and a value in its display form (`96`,
/// `CC16`, `BEND`, `ON`; the value's case and spaces are ignored), into the
/// field and its raw value.
pub fn parse_assignment(name: &str, value: &str) -> Result<(StudioSetField, i32)> {
    let field = StudioSetField::from_name(name).with_context(|| {
        let names: Vec<String> = StudioSetField::all().map(StudioSetField::name).collect();
        format!(
            "unknown Studio Set field `{name}` (expected one of {})",
            names.join(", ")
        )
    })?;
    let spec = field.spec();
    let raw = HostFormat::Field(spec.format.clone())
        .parse(value, spec.min, spec.max)
        .filter(|&raw| names_value(&spec.format, raw))
        .with_context(|| {
            format!(
                "invalid value `{value}` for {name} ({}): expected {} to {}",
                field.label(),
                display(field, spec.min),
                display(field, spec.max)
            )
        })?;
    Ok((field, raw))
}

/// `studio-set show`: read and print every Studio Set setting.
pub fn show(endpoint: Endpoint<'_>, device_id: Option<u8>, timeout: Duration) -> Result<()> {
    let mut dev = IntegraDevice::connect_detecting(endpoint, device_id, timeout)?.quiet();
    let settings = read_settings(&mut dev)?;
    print!("{}", render_show(&settings));
    Ok(())
}

/// `studio-set set <field> <value>`: set one Studio Set setting. Nothing
/// is sent if the field or value doesn't parse.
pub fn set(
    endpoint: Endpoint<'_>,
    device_id: Option<u8>,
    timeout: Duration,
    name: &str,
    value: &str,
) -> Result<()> {
    let (field, raw) = parse_assignment(name, value)?;
    let mut dev = IntegraDevice::connect_detecting(endpoint, device_id, timeout)?;
    let mut queue = DeviceState::new(dev.device_id());
    queue
        .set_studio_set_field(field, raw)
        .with_context(|| name.to_string())?;
    let start = Instant::now();
    while queue.has_pending() {
        match queue.drain(start.elapsed().as_secs_f64() * 1000.0) {
            Some(bytes) => dev
                .conn_out_mut()
                .send(&bytes)
                .context("failed to send the DT1")?,
            None => std::thread::sleep(Duration::from_millis(2)),
        }
    }
    println!("{name}: {}", display(field, raw));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use integral_core::studio_set_settings::StudioSetSetting::{self, *};

    fn field(setting: StudioSetSetting, index: u8) -> StudioSetField {
        StudioSetField::new(setting, index).unwrap()
    }

    fn error(name: &str, value: &str) -> String {
        format!("{:#}", parse_assignment(name, value).unwrap_err())
    }

    #[test]
    fn parses_values_in_their_display_form() {
        let ok = |name, value| parse_assignment(name, value).unwrap();
        assert_eq!(ok("tempo", "96"), (field(Tempo, 0), 96));
        assert_eq!(ok("tempo", " 250 "), (field(Tempo, 0), 250));
        assert_eq!(
            ok("tone-control-1-source", "CC16"),
            (field(ToneControlSource, 0), 16)
        );
        assert_eq!(
            ok("tone-control-4-source", "bend"),
            (field(ToneControlSource, 3), 96)
        );
        assert_eq!(
            ok("tone-control-2-source", "OFF"),
            (field(ToneControlSource, 1), 0)
        );
        assert_eq!(ok("phase-lock-3", "ON"), (field(PhaseLock, 2), 1));
        assert_eq!(ok("phase-lock-16", "off"), (field(PhaseLock, 15), 0));
    }

    #[test]
    fn an_unknown_field_is_refused_with_the_names() {
        let e = error("tempi", "96");
        assert!(e.contains("unknown Studio Set field `tempi`"), "{e}");
        assert!(e.contains("tone-control-1-source"), "{e}");
        assert!(e.contains("phase-lock-16"), "{e}");
        assert!(error("phase-lock-17", "ON").contains("unknown Studio Set field"));
        assert!(error("Tempo", "96").contains("unknown Studio Set field"));
    }

    #[test]
    fn a_value_outside_the_field_is_refused_with_its_range() {
        let e = error("tempo", "19");
        assert!(e.contains("expected 20 to 250"), "{e}");
        assert!(error("tempo", "fast").contains("invalid value `fast`"));
        let e = error("phase-lock-3", "1");
        assert!(e.contains("expected OFF to ON"), "{e}");
        let e = error("tone-control-1-source", "CC200");
        assert!(e.contains("Tone Control 1 Source"), "{e}");
        assert!(e.contains("expected OFF to AFT"), "{e}");
        let e = error("tone-control-1-source", "32");
        assert!(e.contains("invalid value `32`"), "the unused 32: {e}");
    }

    #[test]
    fn show_lists_every_field_by_name_and_display_value() {
        let mut settings = StudioSetSettings {
            tone_control_sources: [1, 74, 96, 97],
            tempo: 96,
            ..StudioSetSettings::default()
        };
        settings.phase_lock[2] = true;
        let out = render_show(&settings);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 4 + 1 + 16);
        assert_eq!(lines[0], "tone-control-1-source  CC01");
        assert_eq!(lines[1], "tone-control-2-source  CC74");
        assert_eq!(lines[2], "tone-control-3-source  BEND");
        assert_eq!(lines[3], "tone-control-4-source  AFT");
        assert_eq!(lines[4], "tempo                  96");
        assert_eq!(lines[5], "phase-lock-1           OFF");
        assert_eq!(lines[7], "phase-lock-3           ON");
        assert_eq!(lines[20], "phase-lock-16          OFF");
    }

    #[test]
    fn the_blocks_read_every_field_once() {
        let blocks: Vec<MixerBlock> = blocks().collect();
        assert_eq!(blocks.len(), 1 + usize::from(MIDI_CHANNELS));
        assert!(blocks.iter().all(|b| b.request().is_some()));
    }
}
