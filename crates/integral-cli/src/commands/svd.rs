//! SVD file commands: validate, import.

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use integral_core::address::DataSize;
use integral_core::mfx;
use integral_core::sn_synth;
use integral_core::svd::{ChunkType, SvdFile};
use integral_core::svd_convert::{sns_to_dt1s, svd_to_sysex};
use integral_core::svd_specs::SNS_TONE_SPEC;

use crate::midi;

/// Import patches from an SVD file to the device.
pub fn svd_import(
    port_pattern: &str,
    device_id: u8,
    path: &Path,
    part: u8,
    index: Option<usize>,
    dry_run: bool,
) -> Result<()> {
    if !(1..=16).contains(&part) {
        bail!("part must be 1-16, got {part}");
    }
    let part_index = part - 1;

    let data = std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let svd =
        SvdFile::parse(&data).with_context(|| format!("failed to parse {}", path.display()))?;

    // Find the SN Synth chunk (only supported type for now).
    let sns_chunk = svd
        .chunks
        .iter()
        .find(|c| c.chunk_type == ChunkType::SnSynthTone)
        .with_context(|| "no SN Synth Tone (SHPa) chunk in this SVD file")?;

    if sns_chunk.entries.is_empty() {
        println!("No SN Synth patches to import.");
        return Ok(());
    }

    // Determine which entries to import.
    let entries: Vec<(usize, &Vec<u8>)> = match index {
        Some(i) => {
            if i < 1 || i > sns_chunk.entries.len() {
                bail!("index {} out of range (1-{})", i, sns_chunk.entries.len());
            }
            vec![(i - 1, &sns_chunk.entries[i - 1])]
        }
        None => sns_chunk.entries.iter().enumerate().collect(),
    };

    println!(
        "Importing {} SN-S patch(es) to Part {} temporary area{}",
        entries.len(),
        part,
        if dry_run { " (dry run)" } else { "" }
    );

    // Open MIDI only if not dry-run.
    let mut conn = if dry_run {
        None
    } else {
        let (_conn_in, conn_out, _rx) = midi::open_midi(port_pattern)?;
        Some((_conn_in, conn_out, _rx))
    };

    for (i, entry) in &entries {
        let sections = svd_to_sysex(entry, &SNS_TONE_SPEC)
            .with_context(|| format!("failed to decode entry {}", i + 1))?;

        // Extract tone name for display.
        let name: String = sections[0][..12]
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

        let dt1s = sns_to_dt1s(device_id, part_index, &sections);

        if dry_run {
            println!(
                "  {:>3}: {} ({} DT1 messages, {} bytes total)",
                i + 1,
                name,
                dt1s.len(),
                dt1s.iter().map(|m| m.len()).sum::<usize>()
            );
        } else {
            print!("  {:>3}: {} ... ", i + 1, name);
            let conn_out = &mut conn.as_mut().unwrap().1;
            for dt1 in &dt1s {
                conn_out.send(dt1).context("failed to send DT1")?;
                std::thread::sleep(Duration::from_millis(20));
            }
            println!("OK");

            // Small extra delay between patches.
            if entries.len() > 1 {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }

    if !dry_run {
        println!("Done. Patch is now in Part {}'s temporary area.", part);
        println!("Use the device to save it to user memory if desired.");
    }

    Ok(())
}

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

    // Read MFX in two chunks (header + params) since 273 bytes may exceed
    // the device's single-response limit.
    let mfx_base = sn_synth::sns_common_address(part_index).offset([0x00, 0x00, 0x02, 0x00]);
    let mfx_hdr_size = mfx::MFX_HEADER_SIZE;
    println!("Reading MFX header ...");
    let mut dev_mfx = midi::request_data(
        &mut conn_out,
        &rx,
        device_id,
        &mfx_base,
        &mfx_hdr_size,
        timeout,
    )
    .context("failed to read MFX header")?;
    // Read params: 32 x 4 bytes starting at offset 0x11
    let mfx_params_addr = mfx_base.offset([0x00, 0x00, 0x00, 0x11]);
    let mfx_params_size = DataSize::new(0x00, 0x00, 0x01, 0x00);
    println!("Reading MFX params ...");
    let dev_mfx_params = midi::request_data(
        &mut conn_out,
        &rx,
        device_id,
        &mfx_params_addr,
        &mfx_params_size,
        timeout,
    )
    .context("failed to read MFX params")?;
    dev_mfx.extend_from_slice(&dev_mfx_params);
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
