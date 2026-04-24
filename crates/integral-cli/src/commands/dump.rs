//! Dump-state commands: read and write parameters from/to the device.
//!
//! High-level reads (studio-set-name, master-level, part-mixer) go through
//! [`IntegraDevice`](crate::device::IntegraDevice). Lower-level or
//! multi-step reads (tone-name) still use raw MIDI helpers directly.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use integral_core::params;
use integral_core::sysex;
use midir::MidiOutput;

use crate::device::IntegraDevice;
use crate::midi;

/// Read a named parameter from the device via RQ1.
pub fn read_param(
    port_pattern: &str,
    timeout: Duration,
    device_id: u8,
    what: &str,
    part: Option<u8>,
) -> Result<()> {
    match what {
        "studio-set-name" => {
            let mut dev = IntegraDevice::connect(port_pattern, device_id, timeout)?;
            let name = dev.read_studio_set_name()?;
            println!("Studio Set Name: \"{name}\"");
        }
        "master-level" => {
            let mut dev = IntegraDevice::connect(port_pattern, device_id, timeout)?;
            let level = dev.read_master_level()?;
            println!("Master Level: {level}");
        }
        "part-mixer" => {
            let p = part.context("--part is required for part-mixer")? - 1;
            let mut dev = IntegraDevice::connect(port_pattern, device_id, timeout)?;
            let state = dev.read_part_mixer(p)?;
            println!("Part {} mixer state:", p + 1);
            println!("  Receive Channel: {}", state.receive_channel + 1);
            println!("  Tone Bank MSB:   {}", state.tone_bank_msb);
            println!("  Tone Bank LSB:   {}", state.tone_bank_lsb);
            println!("  Tone PC:         {}", state.tone_pc);
            println!("  Level:           {}", state.level);
            println!("  Pan:             {}", state.pan);
            println!("  Mute:            {}", u8::from(state.muted));
            println!("  Chorus Send:     {}", state.chorus_send);
            println!("  Reverb Send:     {}", state.reverb_send);
        }
        "tone-name" => {
            // tone-name requires a multi-step read (bank MSB -> tone type ->
            // name address), so we use the raw helpers via IntegraDevice's
            // request_data method.
            let p = part.context("--part is required for tone-name")? - 1;
            let mut dev = IntegraDevice::connect(port_pattern, device_id, timeout)?;

            let bank_addr = params::part_address(p, params::part::TONE_BANK_MSB);
            let bank_data = dev.request_data(&bank_addr, &params::SINGLE_BYTE_SIZE)?;
            let msb = bank_data[0];
            eprintln!("Part {} Bank MSB: {}", p + 1, msb);

            let tt = params::tone_type_from_bank_msb(msb)
                .with_context(|| format!("unknown tone type for bank MSB {msb}"))?;
            let name_addr = params::tone_name_address(p, tt);
            let data = dev.request_data(&name_addr, &params::TONE_NAME_SIZE)?;
            let name = String::from_utf8_lossy(&data);
            println!("Part {} Tone Name: \"{name}\"", p + 1);
        }
        _ => bail!(
            "unknown read target: {what}. Use: studio-set-name, master-level, part-mixer, tone-name"
        ),
    }

    Ok(())
}

/// Write a named parameter to the device via DT1.
pub fn write_param(
    port_pattern: &str,
    device_id: u8,
    what: &str,
    part: u8,
    value: u8,
) -> Result<()> {
    let midi_out = MidiOutput::new("integral-out").context("failed to create MIDI output")?;
    let out_port = midi::find_port_by_name(&midi_out, port_pattern)
        .with_context(|| format!("no MIDI output port matching '{port_pattern}'"))?;
    let mut conn_out = midi_out
        .connect(&out_port, "integral-out")
        .map_err(|e| anyhow::anyhow!("failed to connect MIDI output: {e}"))?;

    let p = part - 1;
    let (addr, display) = match what {
        "part-level" => (params::part_address(p, params::part::LEVEL), "Level"),
        "part-pan" => (params::part_address(p, params::part::PAN), "Pan"),
        "part-mute" => (params::part_address(p, params::part::MUTE), "Mute"),
        _ => bail!("unknown write target: {what}. Use: part-level, part-pan, part-mute"),
    };

    let dt1 = sysex::build_dt1(device_id, &addr, &[value]);
    eprintln!(
        "Sending DT1: Part {} {} = {} (addr={})",
        part, display, value, addr
    );
    conn_out.send(&dt1).context("failed to send DT1")?;
    println!("OK");
    Ok(())
}
