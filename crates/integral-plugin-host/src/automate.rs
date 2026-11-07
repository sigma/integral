//! The `automate` scenario: host automation played back with
//! `set_parameter_at` reaches the device as the writes it should, within the
//! service's budget of 25 DT1 per second.
//!
//! A Studio Set instance (it is always bound) takes the device's values; a
//! ramp then moves one parameter from one end of its range to the other, a
//! step a block as a host plays a lane, and the audio blocks are processed in
//! real time. The DT1s the service sent are printed as `dt1` lines.

use std::path::Path;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow};
use integral_core::device::THROTTLE_MS;
use integral_core::host_params::{Binding, PluginKind};
use serde_json::json;
use vst3_host::AudioBuffers;

use crate::family::{self, Params};
use crate::host::{self, BLOCK_SIZE, SAMPLE_RATE};
use crate::report::Report;
use crate::rig::{Device, Rig, Write, bytes_of_address};

/// The budget the service holds the device to: DT1 per second, over any
/// second. Derived from the core's throttle (one message per 40 ms).
const BUDGET: usize = (1000.0 / THROTTLE_MS) as usize;
/// The longest the run may take.
const LIMIT: Duration = Duration::from_secs(60);

/// Scenario inputs.
pub struct Options {
    /// The host parameter the ramp moves.
    pub param: String,
    /// How long the ramp lasts.
    pub seconds: f64,
}

/// The most DT1s in any window of one second.
fn busiest_second(writes: &[Write]) -> usize {
    writes
        .iter()
        .map(|w| {
            writes
                .iter()
                .filter(|v| (w.at_ms..w.at_ms + 1000).contains(&v.at_ms))
                .count()
        })
        .max()
        .unwrap_or(0)
}

/// Runs the scenario; `Ok(true)` when every assertion held.
pub fn run(
    bundle: &Path,
    device: &Device,
    options: &Options,
    report: &Arc<Report>,
) -> Result<bool> {
    let kind = PluginKind::StudioSet;
    let mut rig = Rig::start(device, family::studio_set_touched(), report)?;
    rig.bring_up()?;
    let mut plugin = host::load(bundle, Some(kind.name()))?;
    let params = Params::of(kind, &plugin)?;
    family::settle(&mut plugin)?;
    let Some(index) = params.find(&options.param) else {
        return Ok(report.check(
            "the plugin declares the host parameter",
            false,
            json!({ "wanted": options.param }),
        ));
    };
    let target = params.host(index);
    let at = target.device_address(&Binding::ANY)?;
    rig.wait_quiet((1, 0), Duration::from_secs(1), LIMIT)?;
    rig.clear();

    let start = params.values(&plugin)?[index];
    let end = params.farthest(index, start);
    let blocks = (options.seconds * SAMPLE_RATE / BLOCK_SIZE as f64).ceil() as usize;
    report.emit(
        "ramp",
        json!({ "parameter": target.name, "address": bytes_of_address(&at),
                "from": start, "to": end, "blocks": blocks }),
    );

    let id = plugin
        .get_parameters()
        .map_err(|e| anyhow!("listing parameters: {e}"))?[index]
        .id;
    let mut buffers = AudioBuffers::new(2, 2, BLOCK_SIZE, SAMPLE_RATE);
    for block in 1..=blocks {
        let value = params.normalized(index, start)
            + (params.normalized(index, end) - params.normalized(index, start))
                * (block as f64 / blocks as f64);
        plugin
            .set_parameter_at(id, value.clamp(0.0, 1.0), 0)
            .map_err(|e| anyhow!("automating: {e}"))?;
        plugin
            .process_audio(&mut buffers)
            .map_err(|e| anyhow!("processing: {e}"))?;
        sleep(family::block_time());
    }

    let writes = rig.wait_quiet((0, 2), Duration::from_secs(1), LIMIT)?;
    rig.report_writes(report, "automation", &writes);

    let values: Vec<Option<i32>> = writes
        .iter()
        .map(|w| (w.address == at).then(|| target.decode(&w.data)).flatten())
        .collect();
    let mut ok = report.check(
        "the ramp reaches the link as writes of the parameter alone",
        !writes.is_empty() && values.iter().all(Option::is_some),
        json!({ "dt1s": writes.len(), "address": bytes_of_address(&at) }),
    );
    // The service sends a ramp's values at its pace, skipping the ones a later
    // value replaced: at least most of what the pace allows, or all the values
    // there are.
    let steps = (end - start).unsigned_abs() as usize;
    let paced = (options.seconds * BUDGET as f64) as usize;
    let at_least = blocks.min(steps).min(paced) * 3 / 4;
    ok &= report.check(
        "the ramp makes about as many writes as the pace allows",
        writes.len() >= at_least,
        json!({ "dt1s": writes.len(), "at_least": at_least,
                "blocks": blocks, "steps": steps, "paced": paced }),
    );
    let seen: Vec<i32> = values.iter().flatten().copied().collect();
    let toward_end = |a: i32, b: i32| if end > start { a <= b } else { a >= b };
    ok &= report.check(
        "the written values move monotonically toward the end of the ramp",
        seen.windows(2).all(|w| toward_end(w[0], w[1])),
        json!({ "first": seen.first(), "last": seen.last() }),
    );
    ok &= report.check(
        "the last write is the end of the ramp",
        seen.last() == Some(&end),
        json!({ "last": seen.last(), "end": end }),
    );
    let busiest = busiest_second(&writes);
    ok &= report.check(
        "no second carries more than 25 DT1",
        busiest <= BUDGET,
        json!({ "busiest_second": busiest, "budget": BUDGET }),
    );
    Ok(ok)
}

#[cfg(test)]
mod tests {
    use integral_core::address::Address;

    use super::*;

    fn at(ms: &[u64]) -> Vec<Write> {
        ms.iter()
            .map(|&at_ms| Write {
                at_ms,
                address: Address::new(0, 0, 0, 0),
                data: vec![0],
            })
            .collect()
    }

    #[test]
    fn the_budget_is_twenty_five_a_second() {
        assert_eq!(BUDGET, 25);
    }

    #[test]
    fn the_busiest_second_is_found_in_any_window() {
        assert_eq!(busiest_second(&at(&[])), 0);
        assert_eq!(busiest_second(&at(&[0, 500, 999, 1000, 2500])), 3);
        assert_eq!(busiest_second(&at(&[0, 1000, 2000])), 1);
    }
}
