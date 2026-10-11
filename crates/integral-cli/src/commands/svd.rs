//! SVD file commands: validate.

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use integral_core::mfx;
use integral_core::params::tone_type;
use integral_core::sn_synth;
use integral_core::svd::{ChunkType, SvdFile};
use integral_core::svd_convert::svd_to_sysex;
use integral_core::svd_specs::SNS_TONE_SPEC;

use crate::midi;

/// Validate SVD decode against the device by comparing SysEx reads.
pub fn svd_validate(
    port_pattern: &str,
    timeout: Duration,
    device_id: u8,
    path: &Path,
    part: u8,
    index: usize,
) -> Result<()> {
    if !(1..=16).contains(&part) {
        bail!("part must be 1-16, got {part}");
    }
    let part_index = part - 1;

    // Parse SVD and decode the entry.
    let data = std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let svd =
        SvdFile::parse(&data).with_context(|| format!("failed to parse {}", path.display()))?;
    let sns_chunk = svd
        .chunks
        .iter()
        .find(|c| c.chunk_type == ChunkType::SnSynthTone)
        .context("no SN Synth Tone chunk")?;
    if index < 1 || index > sns_chunk.entries.len() {
        bail!(
            "index {} out of range (1-{})",
            index,
            sns_chunk.entries.len()
        );
    }
    let sections = svd_to_sysex(&sns_chunk.entries[index - 1], &SNS_TONE_SPEC)
        .context("failed to decode SVD entry")?;

    // Sections are now: 0=Common, 1=MFX, 2=Partial1, 3=Partial2, 4=Partial3.
    let svd_common = &sections[0];
    let svd_mfx = &sections[1];

    let svd_name: String = svd_common[..12]
        .iter()
        .map(|&b| {
            if (32..=127).contains(&b) {
                b as char
            } else {
                ' '
            }
        })
        .collect::<String>()
        .trim_end()
        .to_string();
    println!("SVD entry {index}: \"{svd_name}\"");

    // Open MIDI and read from the device.
    let (_conn_in, mut conn_out, rx) = midi::open_midi(port_pattern)?;

    // Read each SN-S section from the device.
    let common_addr = sn_synth::sns_common_address(part_index);
    let common_size = sn_synth::SNS_COMMON_BLOCK_SIZE;
    println!("Reading SN-S Common from Part {} ...", part);
    let dev_common = midi::request_data(
        &mut conn_out,
        &rx,
        device_id,
        &common_addr,
        &common_size,
        timeout,
        true,
    )
    .context("failed to read Common")?;

    let dev_name: String = dev_common[..12]
        .iter()
        .map(|&b| {
            if (32..=127).contains(&b) {
                b as char
            } else {
                ' '
            }
        })
        .collect::<String>()
        .trim_end()
        .to_string();
    println!("Device Part {part}: \"{dev_name}\"");

    if svd_name != dev_name {
        bail!(
            "Tone name mismatch: SVD=\"{svd_name}\" vs Device=\"{dev_name}\". Make sure the correct tone is loaded on Part {part}."
        );
    }

    // Compare Common.
    let mut mismatches = 0;
    print!("Common ({} bytes): ", svd_common.len());
    let common_ok = compare_sysex("Common", svd_common, &dev_common, &mut mismatches);
    println!("{}", if common_ok { "OK" } else { "MISMATCH" });

    let mfx_base = mfx::tone_mfx_block_address(part_index, tone_type::SN_SYNTH);
    println!("Reading MFX ...");
    let dev_mfx = midi::request_data(
        &mut conn_out,
        &rx,
        device_id,
        &mfx_base,
        &mfx::MFX_BLOCK_SIZE,
        timeout,
        true,
    )
    .context("failed to read MFX")?;
    print!("MFX ({} bytes): ", svd_mfx.len());
    let mfx_ok = compare_sysex("MFX", svd_mfx, &dev_mfx, &mut mismatches);
    println!("{}", if mfx_ok { "OK" } else { "MISMATCH" });

    // Read Partials 1-3.
    for pi in 0..3u8 {
        let partial_addr = sn_synth::sns_partial_address(part_index, pi);
        let partial_size = sn_synth::SNS_PARTIAL_BLOCK_SIZE;
        println!("Reading Partial {} ...", pi + 1);
        let dev_partial = midi::request_data(
            &mut conn_out,
            &rx,
            device_id,
            &partial_addr,
            &partial_size,
            timeout,
            true,
        )
        .context(format!("failed to read Partial {}", pi + 1))?;
        let svd_partial = &sections[(pi + 2) as usize];
        print!("Partial {} ({} bytes): ", pi + 1, svd_partial.len());
        let p_ok = compare_sysex(
            &format!("Partial {}", pi + 1),
            svd_partial,
            &dev_partial,
            &mut mismatches,
        );
        println!("{}", if p_ok { "OK" } else { "MISMATCH" });
    }

    if mismatches == 0 {
        println!("\nValidation PASSED: all sections match.");
    } else {
        println!("\nValidation FAILED: {mismatches} byte(s) differ.");
    }

    Ok(())
}

/// Compare two SysEx byte vectors and report differences.
fn compare_sysex(label: &str, svd: &[u8], device: &[u8], mismatches: &mut usize) -> bool {
    let len = svd.len().min(device.len());
    let mut ok = true;
    if svd.len() != device.len() {
        eprintln!(
            "  {label}: length mismatch: SVD={} vs Device={}",
            svd.len(),
            device.len()
        );
        ok = false;
        *mismatches += svd.len().abs_diff(device.len());
    }
    for i in 0..len {
        if svd[i] != device[i] {
            if ok {
                ok = false;
            }
            eprintln!(
                "  {label}[0x{i:02X}]: SVD=0x{:02X} Device=0x{:02X}",
                svd[i], device[i]
            );
            *mismatches += 1;
        }
    }
    ok
}
