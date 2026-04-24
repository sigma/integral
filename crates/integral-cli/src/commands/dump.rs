//! Dump-state commands: read and write parameters from/to the device.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use integral_core::params;
use integral_core::sysex;
use midir::MidiOutput;

use crate::midi;

/// Read a named parameter from the device via RQ1.
pub fn read_param(
    port_pattern: &str,
    timeout: Duration,
    device_id: u8,
    what: &str,
    part: Option<u8>,
) -> Result<()> {
    let (_conn_in, mut conn_out, rx) = midi::open_midi(port_pattern)?;

    match what {
        "studio-set-name" => {
            let data = midi::request_data(
                &mut conn_out,
                &rx,
                device_id,
                &params::STUDIO_SET_NAME,
                &params::STUDIO_SET_NAME_SIZE,
                timeout,
            )?;
            let name = String::from_utf8_lossy(&data);
            println!("Studio Set Name: \"{name}\"");
        }
        "master-level" => {
            let data = midi::request_data(
                &mut conn_out,
                &rx,
                device_id,
                &params::SYSTEM_MASTER_LEVEL,
                &params::SINGLE_BYTE_SIZE,
                timeout,
            )?;
            println!("Master Level: {}", data[0]);
        }
        "part-mixer" => {
            let p = part.context("--part is required for part-mixer")? - 1;
            let addr = params::part_address(p, params::part::RECEIVE_CHANNEL);
            let data =
                midi::request_data(&mut conn_out, &rx, device_id, &addr, &params::PART_MIXER_SIZE, timeout)?;
            println!("Part {} mixer state:", p + 1);
            println!("  Receive Channel: {}", data.first().unwrap_or(&0) + 1);
            println!("  Tone Bank MSB:   {}", data.get(0x06).unwrap_or(&0));
            println!("  Tone Bank LSB:   {}", data.get(0x07).unwrap_or(&0));
            println!("  Tone PC:         {}", data.get(0x08).unwrap_or(&0));
            println!("  Level:           {}", data.get(0x09).unwrap_or(&0));
            println!("  Pan:             {}", data.get(0x0A).unwrap_or(&0));
            println!("  Mute:            {}", data.get(0x25).unwrap_or(&0));
            println!("  Chorus Send:     {}", data.get(0x27).unwrap_or(&0));
            println!("  Reverb Send:     {}", data.get(0x28).unwrap_or(&0));
        }
        "tone-name" => {
            let p = part.context("--part is required for tone-name")? - 1;

            // First read the bank MSB to determine tone type
            let bank_addr = params::part_address(p, params::part::TONE_BANK_MSB);
            let bank_data = midi::request_data(
                &mut conn_out,
                &rx,
                device_id,
                &bank_addr,
                &params::SINGLE_BYTE_SIZE,
                timeout,
            )?;
            let msb = bank_data[0];
            eprintln!("Part {} Bank MSB: {}", p + 1, msb);

            let tt = params::tone_type_from_bank_msb(msb)
                .with_context(|| format!("unknown tone type for bank MSB {msb}"))?;
            let name_addr = params::tone_name_address(p, tt);
            let data = midi::request_data(
                &mut conn_out,
                &rx,
                device_id,
                &name_addr,
                &params::TONE_NAME_SIZE,
                timeout,
            )?;
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
