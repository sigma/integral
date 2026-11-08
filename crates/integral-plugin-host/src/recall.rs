//! The `recall` scenario (ADR 0009): a plugin's state, saved and loaded into a
//! fresh instance, brings back every parameter and the Binding, and the
//! instance pushes what the ADR says to the device.
//!
//! The device is the Device service on the chosen link ([`crate::rig`]); the
//! DT1s the service sent are printed as `dt1` lines and checked:
//!
//! - a User tone is selected and pushed in full;
//! - an untouched Preset tone recalls with its selection alone;
//! - an edited Preset tone recalls with its selection and the runs around the
//!   edits, fewer DT1s than a full push;
//! - the Studio Set plugin pushes its blocks and never a Part's tone select
//!   (`00 06`-`00 08`), nor a Studio Set's by number.
//!
//! Each case has its own service, so its session and its tone baselines are
//! new. A first instance adopts the device (and is edited, where the case
//! says); its state is saved and the instance dropped; a second instance loads the state
//! the way a DAW restores a project.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use integral_core::address::Address;
use integral_core::device_spec::{INTEGRA7, tone_read_blocks};
use integral_core::host_params::{Binding, PluginKind};
use integral_core::params::{self, TONE_SELECTION_SIZE, part};
use integral_core::sysex::MAX_DT1_DATA;
use serde_json::json;

use crate::family::{self, Bank, Params};
use crate::host;
use crate::report::Report;
use crate::rig::{Device, Rig, Write, bytes_of_address};

/// The Part the tone plugins are bound to.
const PART: u8 = 0;
/// Silence that ends a push: longer than the service's cold read of a tone's
/// baseline (ADR 0009: about 2 s before the first write).
const QUIET: Duration = Duration::from_millis(2500);
/// The longest a case may take: a whole Studio Set is about 4 s at 40 ms a
/// DT1, a cold baseline a few more.
const LIMIT: Duration = Duration::from_secs(90);
/// The name the fake device gives its tone.
const TONE_NAME: &str = "Warm Pad";
/// The program of the tone the fake device holds.
const PROGRAM: u8 = 3;
/// Blocks the plugin processes after the host edits parameters, which is
/// when it sends the edits.
const PROCESSED: usize = 4;

/// What a case expects to see pushed.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Expect {
    /// The tone selection and nothing else.
    SelectionOnly,
    /// The selection, then the runs around the edits.
    SelectionAndEdits,
    /// The selection, then the tone in full.
    Full,
    /// The Studio Set's blocks, and no Part's tone select.
    StudioSet,
}

struct Case {
    name: &'static str,
    kind: PluginKind,
    bank: Option<Bank>,
    expect: Expect,
    /// How many parameters the first instance edits before saving.
    edits: usize,
}

/// In this order: a full push is what the edited Preset is smaller than.
const CASES: [Case; 4] = [
    Case {
        name: "user tone",
        kind: PluginKind::PcmSynth,
        bank: Some(Bank::User),
        expect: Expect::Full,
        edits: 3,
    },
    Case {
        name: "untouched preset",
        kind: PluginKind::PcmSynth,
        bank: Some(Bank::Preset),
        expect: Expect::SelectionOnly,
        edits: 0,
    },
    Case {
        name: "edited preset",
        kind: PluginKind::PcmSynth,
        bank: Some(Bank::Preset),
        expect: Expect::SelectionAndEdits,
        edits: 3,
    },
    Case {
        name: "studio set",
        kind: PluginKind::StudioSet,
        bank: None,
        expect: Expect::StudioSet,
        edits: 3,
    },
];

/// Runs the cases of the plugin named `only`, or all; `Ok(true)` when every
/// assertion held.
pub fn run(
    bundle: &Path,
    device: &Device,
    only: Option<&str>,
    report: &Arc<Report>,
) -> Result<bool> {
    let mut ok = true;
    let mut ran = 0;
    let mut full: Option<usize> = None;
    for case in &CASES {
        if only.is_some_and(|name| name != case.kind.name()) {
            continue;
        }
        ran += 1;
        let (held, dt1s) = run_case(bundle, device, case, full, report)
            .with_context(|| format!("case {:?}", case.name))?;
        ok &= held;
        if case.expect == Expect::Full {
            full = Some(dt1s);
        }
    }
    if ran == 0 {
        ok &= report.check(
            "a recall case exists for the plugin",
            false,
            json!({ "plugin": only }),
        );
    }
    Ok(ok)
}

/// The bytes a list of DT1s sets, by address.
fn bytes_of(writes: &[Write]) -> BTreeMap<u32, u8> {
    let mut bytes = BTreeMap::new();
    for w in writes {
        for (k, b) in w.data.iter().enumerate() {
            bytes.insert(w.address.value() + k as u32, *b);
        }
    }
    bytes
}

/// How many DT1s the blocks of the tone take at most, a block longer than
/// one message taking several.
fn most_dt1s(kind: PluginKind) -> usize {
    let tone_type = kind.tone_type().expect("a tone plugin");
    tone_read_blocks(tone_type, PART)
        .iter()
        .map(|(_, size)| (size.value() as usize).div_ceil(MAX_DT1_DATA))
        .sum()
}

/// Runs one case; whether its assertions held, and how many DT1s the push
/// took.
fn run_case(
    bundle: &Path,
    device: &Device,
    case: &Case,
    full: Option<usize>,
    report: &Report,
) -> Result<(bool, usize)> {
    let touched = match case.bank {
        Some(_) => family::tone_blocks(case.kind, PART)?,
        None => family::studio_set_touched(),
    };
    let mut rig = Rig::start(device, touched, report)?;
    let class = host::classes(bundle)?
        .into_iter()
        .find(|c| c.name == case.kind.name())
        .ok_or_else(|| anyhow!("the bundle has no {} class", case.kind.name()))?;
    let binding = if case.kind == PluginKind::StudioSet {
        Binding::ANY
    } else {
        Binding::of_part(PART)
    };
    if let Some(memory) = rig.memory() {
        family::hold_defaults(memory, case.kind, &binding);
        if let Some(bank) = case.bank {
            family::hold_tone(memory, case.kind, PART, bank, PROGRAM, TONE_NAME)?;
        }
    }
    rig.bring_up()?;
    // The precondition: what the Part holds is what the case is about. On a
    // real device that is up to the user; the expectation is what it holds.
    let selection = match case.bank {
        Some(bank) => {
            let held = rig.held(
                family::select_address(PART),
                TONE_SELECTION_SIZE.value() as usize,
            )?;
            let selection = [held[0], held[1], held[2]];
            family::require_tone(case.kind, bank, selection)?;
            Some(selection)
        }
        None => None,
    };

    // The first instance is new: it takes the Binding and the device's
    // values, sends nothing.
    let mut first = host::load_class(bundle, &class.class_id)?;
    let params = Params::of(case.kind, &first)?;
    family::settle(&mut first)?;
    rig.wait_quiet((1, 0), Duration::from_secs(1), LIMIT)
        .context("the first instance adopting the device")?;
    let adopted = rig.writes();
    let mut ok = report.check(
        "a new instance sends nothing",
        adopted.is_empty(),
        json!({ "case": case.name, "dt1s": adopted.len() }),
    );

    let before = params.values(&first)?;
    let mut edited = Vec::new();
    for index in params.editable(&binding, case.edits) {
        let value = params.other_than(index, before[index]);
        params.set(&mut first, index, value)?;
        edited.push((index, value));
    }
    ok &= report.check(
        "the case has parameters to edit",
        edited.len() == case.edits,
        json!({ "case": case.name, "wanted": case.edits, "found": edited.len() }),
    );
    // The plugin sends the host's changes from `process()`.
    family::process_blocks(&mut first, PROCESSED)?;
    if !edited.is_empty() {
        rig.wait_quiet((1, edited.len()), Duration::from_secs(1), LIMIT)
            .context("the edits reaching the device")?;
    }
    let saved = first.save_state().map_err(|e| anyhow!("saving: {e}"))?;
    let wanted = params.values(&first)?;
    drop(first);
    rig.wait_quiet((1, edited.len()), Duration::from_secs(1), LIMIT)?;
    rig.clear();
    report.emit(
        "saved",
        json!({ "case": case.name, "bytes": saved.len(), "edited": edited.len() }),
    );

    // The project is opened: a fresh instance restores the state.
    let mut second = host::load_class(bundle, &class.class_id)?;
    second
        .load_state(&saved)
        .map_err(|e| anyhow!("restoring: {e}"))?;
    family::settle(&mut second)?;
    let writes = rig.wait_quiet((0, 1), QUIET, LIMIT)?;
    rig.report_writes(report, case.name, &writes);

    let restored = params.values(&second)?;
    ok &= report.check(
        "every parameter comes back",
        restored == wanted,
        json!({
            "case": case.name,
            "parameters": wanted.len(),
            "different": restored.iter().zip(&wanted).filter(|(a, b)| a != b).count(),
        }),
    );
    let resaved = second.save_state().map_err(|e| anyhow!("saving: {e}"))?;
    ok &= report.check(
        "the Binding and the other saved fields come back",
        family::state_fields(&resaved)? == family::state_fields(&saved)?,
        json!({ "case": case.name }),
    );
    let push = Push {
        case,
        params: &params,
        binding: &binding,
        selection,
        edited: &edited,
        wanted: &wanted,
        full,
    };
    ok &= push.check(&writes, report);
    Ok((ok, writes.len()))
}

/// What a case knows about the push it expects.
struct Push<'a> {
    case: &'a Case,
    params: &'a Params,
    binding: &'a Binding,
    selection: Option<[u8; 3]>,
    edited: &'a [(usize, i32)],
    /// Every parameter's raw value when the state was saved.
    wanted: &'a [i32],
    /// How many DT1s a full push of the tone took, if a case showed it.
    full: Option<usize>,
}

impl Push<'_> {
    /// The assertions about what the push sent.
    fn check(&self, writes: &[Write], report: &Report) -> bool {
        let name = self.case.name;
        let select_at = family::select_address(PART);
        let sent = bytes_of(writes);
        let mut ok = true;
        if let Some(selection) = self.selection {
            let first = writes.first();
            ok &= report.check(
                "the push selects the tone first",
                first.is_some_and(|w| w.address == select_at && w.data == selection),
                json!({ "case": name, "wanted": selection, "address": bytes_of_address(&select_at),
                        "first": first.map(|w| (bytes_of_address(&w.address), w.data.clone())) }),
            );
        }
        match self.case.expect {
            Expect::SelectionOnly => {
                ok &= report.check(
                    "an untouched preset recalls with its selection alone",
                    writes.len() == 1,
                    json!({ "case": name, "dt1s": writes.len() }),
                );
            }
            Expect::SelectionAndEdits => {
                let most = most_dt1s(self.case.kind);
                ok &= report.check(
                    "an edited preset recalls with its selection and the runs around the edits",
                    (2..=1 + most).contains(&writes.len()),
                    json!({ "case": name, "dt1s": writes.len(), "at_most": 1 + most }),
                );
                ok &= report.check(
                    "an edited preset takes fewer DT1s than a full push",
                    self.full.is_none_or(|full| writes.len() < full),
                    json!({ "case": name, "dt1s": writes.len(), "full": self.full }),
                );
                // Only the tone's own blocks are written.
                let tone_type = self.case.kind.tone_type().expect("a tone plugin");
                let outside: Vec<String> = writes[1..]
                    .iter()
                    .filter(|w| {
                        !tone_read_blocks(tone_type, PART).iter().any(|(at, size)| {
                            (at.value()..at.value() + size.value()).contains(&w.address.value())
                        })
                    })
                    .map(|w| bytes_of_address(&w.address))
                    .collect();
                ok &= report.check(
                    "the runs lie in the tone's blocks",
                    outside.is_empty(),
                    json!({ "case": name, "outside": outside }),
                );
            }
            Expect::Full => {
                ok &= report.check(
                    "a User tone is pushed in full after its selection",
                    writes.len() > 1,
                    json!({ "case": name, "dt1s": writes.len() }),
                );
                ok &= self.covers(&sent, report);
            }
            Expect::StudioSet => {
                ok &= report.check(
                    "the Studio Set pushes its blocks",
                    writes.len() > 1,
                    json!({ "case": name, "dt1s": writes.len() }),
                );
                ok &= self.covers(&sent, report);
                let selects: Vec<u32> = (0..INTEGRA7.part_count)
                    .flat_map(|p| {
                        let at = params::part_address(p, part::TONE_BANK_MSB).value();
                        (0..TONE_SELECTION_SIZE.value()).map(move |k| at + k)
                    })
                    .chain([
                        params::SETUP_STUDIO_SET_BS_MSB.value(),
                        params::SETUP_STUDIO_SET_BS_LSB.value(),
                        params::SETUP_STUDIO_SET_PC.value(),
                    ])
                    .collect();
                let touched: Vec<String> = selects
                    .iter()
                    .filter(|at| sent.contains_key(at))
                    .filter_map(|at| Address::from_value(*at))
                    .map(|at| bytes_of_address(&at))
                    .collect();
                ok &= report.check(
                    "the Studio Set never sends a Part's tone select or loads a set by number",
                    touched.is_empty(),
                    json!({ "case": name, "touched": touched }),
                );
            }
        }
        // What the first instance edited is what the push carries, wherever
        // the push sends the bytes around it.
        if self.case.expect != Expect::SelectionOnly {
            ok &= self.carries_edits(&sent, report);
        }
        ok
    }

    /// Every byte of every host parameter that has an address is in the
    /// push with the value the state holds. Parameters that share an address
    /// (the slots of an effect, whose meaning depends on its type) share one
    /// byte, which can hold only one of their values: the byte must be one of
    /// them.
    fn covers(&self, sent: &BTreeMap<u32, u8>, report: &Report) -> bool {
        let mut expected: HashMap<u32, Vec<(usize, u8)>> = HashMap::new();
        for (i, raw) in self.wanted.iter().enumerate() {
            let host = self.params.host(i);
            let Ok(at) = host.device_address(self.binding) else {
                continue;
            };
            if !self.is_active(i) {
                continue;
            }
            for (k, byte) in host.encode(*raw).into_iter().enumerate() {
                expected
                    .entry(at.value() + k as u32)
                    .or_default()
                    .push((i, byte));
            }
        }
        let mut wrong: Vec<String> = Vec::new();
        let mut checked = 0;
        for (at, owners) in &expected {
            checked += 1;
            let got = sent.get(at);
            if !owners.iter().any(|(_, byte)| got == Some(byte)) {
                wrong.push(self.params.host(owners[0].0).name.clone());
            }
        }
        wrong.sort();
        wrong.dedup();
        report.check(
            "every active host parameter is pushed with its saved value",
            wrong.is_empty(),
            json!({ "case": self.case.name, "bytes_checked": checked,
                    "parameters_not_pushed": wrong.len(), "first": wrong.iter().take(8).collect::<Vec<_>>() }),
        )
    }

    /// Whether parameter `index` does something now: it belongs to no
    /// effect type, or to the type its effect has in the saved values. The
    /// others are not pushed.
    fn is_active(&self, index: usize) -> bool {
        let host = self.params.host(index);
        let (Some(effect), Some(Ok(type_at))) =
            (host.effect, host.effect_type_address(self.binding))
        else {
            return true;
        };
        (0..self.wanted.len()).any(|j| {
            self.params.host(j).device_address(self.binding) == Ok(type_at)
                && self.wanted[j] == i32::from(effect.type_id)
        })
    }

    fn carries_edits(&self, sent: &BTreeMap<u32, u8>, report: &Report) -> bool {
        let mut ok = true;
        for (index, value) in self.edited {
            let host = self.params.host(*index);
            let Ok(at) = host.device_address(self.binding) else {
                continue;
            };
            let wanted = host.encode(*value);
            let got: Vec<Option<u8>> = (0..wanted.len())
                .map(|k| sent.get(&(at.value() + k as u32)).copied())
                .collect();
            ok &= report.check(
                "an edited value is pushed",
                got.iter().zip(&wanted).all(|(g, w)| *g == Some(*w)),
                json!({ "case": self.case.name, "parameter": host.name,
                        "address": bytes_of_address(&at), "wanted": wanted, "sent": got }),
            );
        }
        ok
    }
}
