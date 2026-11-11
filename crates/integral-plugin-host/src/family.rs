//! A loaded plugin of the family seen through the lists core declares for
//! it: its parameters in raw values, its state, and the tones a fake device
//! can hold for it.

use std::collections::{BTreeMap, HashMap};
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use integral_core::address::{Address, DataSize};
use integral_core::device_spec::{INTEGRA7, tone_read_blocks};
use integral_core::host_params::{Anchor, Binding, HostParam, PluginKind};
use integral_core::params::{
    self, TONE_NAME_SIZE, TONE_SELECTION_SIZE, ToneType, part, tone_name_address,
    tone_type_from_bank_msb,
};
use integral_core::tone_banks::{PRESET_LABEL, TONE_BANK_GROUPS, USER_LABEL, find_bank};
use integral_device_service::FakeMemory;
use vst3_host::{AudioBuffers, Plugin};

use crate::rig::Touched;

use crate::host::{BLOCK_SIZE, SAMPLE_RATE};

/// The names of every parameter the plugin of `kind` declares, in the order
/// its `param_map` registers them: the device-address parameters
/// ([`PluginKind::host_params`]), then the Binding parameters
/// ([`PluginKind::binding_params`]), which `FamilyParams::param_map` chains
/// after the device's.
pub fn declared_names(kind: PluginKind) -> Vec<String> {
    kind.host_params()
        .iter()
        .map(|h| h.name.clone())
        .chain(kind.binding_params().iter().map(|b| b.name.to_string()))
        .collect()
}

/// A plugin's parameters: core's list, and the IDs the host knows them by.
pub struct Params {
    list: Vec<HostParam>,
    ids: Vec<u32>,
}

impl Params {
    /// The parameters of the loaded `plugin` of `kind`.
    ///
    /// # Errors
    ///
    /// The plugin does not declare core's list for `kind`, name for name.
    pub fn of(kind: PluginKind, plugin: &Plugin) -> Result<Self> {
        let names = declared_names(kind);
        let declared = plugin
            .get_parameters()
            .map_err(|e| anyhow!("listing parameters: {e}"))?;
        if declared.len() != names.len() || declared.iter().zip(&names).any(|(d, n)| d.name != *n) {
            bail!("the plugin does not declare {}'s parameters", kind.name());
        }
        let list = kind.host_params();
        // The Binding parameters come last, so the device's keep their indices.
        let ids = declared.iter().take(list.len()).map(|p| p.id).collect();
        Ok(Self { list, ids })
    }

    /// The core parameter at `index`.
    pub fn host(&self, index: usize) -> &HostParam {
        &self.list[index]
    }

    /// The index of the parameter called `name`.
    pub fn find(&self, name: &str) -> Option<usize> {
        self.list.iter().position(|h| h.name == name)
    }

    /// The raw value a normalized `value` of parameter `index` stands for.
    pub fn raw(&self, index: usize, value: f64) -> i32 {
        let h = &self.list[index];
        (f64::from(h.min) + value * f64::from(h.max - h.min)).round() as i32
    }

    /// The normalized value of the raw `value` of parameter `index`.
    pub fn normalized(&self, index: usize, value: i32) -> f64 {
        let h = &self.list[index];
        f64::from(value - h.min) / f64::from(h.max - h.min)
    }

    /// Every device parameter's raw value now (the Binding's are not in core's
    /// list).
    ///
    /// # Errors
    ///
    /// The plugin would not list its parameters.
    pub fn values(&self, plugin: &Plugin) -> Result<Vec<i32>> {
        let declared = plugin
            .get_parameters()
            .map_err(|e| anyhow!("listing parameters: {e}"))?;
        Ok(declared
            .iter()
            .take(self.list.len())
            .enumerate()
            .map(|(i, p)| self.raw(i, p.value))
            .collect())
    }

    /// Sets parameter `index` to the raw `value`, as a host does.
    ///
    /// # Errors
    ///
    /// The plugin refused the value.
    pub fn set(&self, plugin: &mut Plugin, index: usize, value: i32) -> Result<()> {
        plugin
            .set_parameter(self.ids[index], self.normalized(index, value))
            .map_err(|e| anyhow!("setting {}: {e}", self.list[index].name))
    }

    /// Each parameter's address under `binding`, with how many parameters
    /// of the list lie at each address.
    fn addressed(&self, binding: &Binding) -> (Vec<Option<u32>>, HashMap<u32, usize>) {
        let addresses: Vec<Option<u32>> = self
            .list
            .iter()
            .map(|h| h.device_address(binding).ok().map(|a| a.value()))
            .collect();
        let mut counts: HashMap<u32, usize> = HashMap::new();
        for at in addresses.iter().flatten() {
            *counts.entry(*at).or_default() += 1;
        }
        (addresses, counts)
    }

    /// Parameters of `binding` that a scenario can change and find again on
    /// the wire: ones with an address of their own (no other parameter shares
    /// it) that do not depend on an effect's type, `count` of them spread
    /// over the list.
    pub fn editable(&self, binding: &Binding, count: usize) -> Vec<usize> {
        let (addresses, counts) = self.addressed(binding);
        let candidates: Vec<usize> = (0..self.list.len())
            .filter(|&i| {
                self.list[i].effect.is_none()
                    && self.list[i].max > self.list[i].min
                    && addresses[i].is_some_and(|at| counts[&at] == 1)
            })
            .collect();
        if candidates.is_empty() || count == 0 {
            return Vec::new();
        }
        let step = (candidates.len() / count).max(1);
        candidates.into_iter().step_by(step).take(count).collect()
    }

    /// A Kit plugin's two sides, as positions a scenario can change and find
    /// again on the wire: the first kit-wide parameter and the first of one
    /// key's, each not an effect's slot, with an address of its own under
    /// `binding` — which needs a key, or only the kit-wide side has an
    /// address — and at least three values, so an edit can differ from both
    /// the host's value and the device's. `None` for a list without a key
    /// side (any but a Kit plugin's), or when a side has no such parameter.
    pub fn kit_sides(&self, binding: &Binding) -> Option<(usize, usize)> {
        let (addresses, counts) = self.addressed(binding);
        let side = |key: bool| {
            (0..self.list.len()).find(|&i| {
                matches!(self.list[i].anchor, Anchor::Key(_)) == key
                    && self.list[i].effect.is_none()
                    && self.list[i].max - self.list[i].min >= 2
                    && addresses[i].is_some_and(|at| counts[&at] == 1)
            })
        };
        Some((side(false)?, side(true)?))
    }

    /// The end of the range of parameter `index` that is farthest from
    /// `current`: the longest ramp.
    pub fn farthest(&self, index: usize, current: i32) -> i32 {
        let h = &self.list[index];
        if current - h.min > h.max - current {
            h.min
        } else {
            h.max
        }
    }

    /// A raw value other than `current`: the far end of the range.
    pub fn other_than(&self, index: usize, current: i32) -> i32 {
        let h = &self.list[index];
        if current == h.max { h.min } else { h.max }
    }
}

/// The fields a plugin saved beside its parameters, as nih-plug saves them
/// (a JSON document with a `fields` map). They hold the Binding and the tone
/// selection, among others.
///
/// `vst3-host` frames the plugin's component state in a header of its own
/// (a magic, a version and a length) and follows it with the controller's, so
/// this reads the one JSON value that starts at the first `{`.
///
/// # Errors
///
/// `state` holds no such document.
pub fn state_fields(state: &[u8]) -> Result<BTreeMap<String, String>> {
    let start = state
        .iter()
        .position(|b| *b == b'{')
        .context("the saved state holds no JSON document")?;
    let document: serde_json::Value = serde_json::Deserializer::from_slice(&state[start..])
        .into_iter()
        .next()
        .context("the saved state holds no JSON document")?
        .context("the saved state's document is not JSON")?;
    serde_json::from_value(document["fields"].clone()).context("the saved state has no fields")
}

/// Which of a tone type's own banks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bank {
    /// The factory tones, which never change.
    Preset,
    /// The user's slots, which can hold anything.
    User,
}

impl Bank {
    pub fn label(self) -> &'static str {
        match self {
            Self::Preset => PRESET_LABEL,
            Self::User => USER_LABEL,
        }
    }
}

/// The Bank Select MSB and LSB of the first `which` bank of `kind`'s type
/// that no Expansion provides.
///
/// # Errors
///
/// `kind` has no tone type, or the type has no such bank.
pub fn bank(kind: PluginKind, which: Bank) -> Result<(u8, u8)> {
    let wanted = kind.tone_type().context("a Studio Set has no tone bank")?;
    let label = which.label();
    TONE_BANK_GROUPS
        .iter()
        .flat_map(|group| group.banks)
        .find(|b| {
            b.label == label
                && b.expansion.is_none()
                && tone_type_from_bank_msb(b.msb) == Some(wanted)
        })
        .and_then(|b| Some((b.msb, *b.lsbs.first()?)))
        .with_context(|| format!("{} has no {label} bank", kind.name()))
}

/// Makes the fake device hold tone `program` of `bank_of` on `part_index`, named
/// `name`, as its Part's selection and Temporary Tone name say.
///
/// # Errors
///
/// As [`bank`].
pub fn hold_tone(
    memory: &FakeMemory,
    kind: PluginKind,
    part_index: u8,
    bank_of: Bank,
    program: u8,
    name: &str,
) -> Result<[u8; 3]> {
    let (msb, lsb) = bank(kind, bank_of)?;
    let selection = [msb, lsb, program];
    memory.put(select_address(part_index), &selection);
    let tone_type: ToneType = kind.tone_type().context("a tone plugin")?;
    let mut padded = name.as_bytes().to_vec();
    padded.resize(TONE_NAME_SIZE.value() as usize, b' ');
    memory.put(tone_name_address(part_index, tone_type), &padded);
    // Selecting it again loads this tone, as the device does.
    memory.store_tone(selection, part_index);
    Ok(selection)
}

/// The address of `part_index`'s tone select (Bank Select MSB, LSB, PC).
pub fn select_address(part_index: u8) -> Address {
    params::part_address(part_index, part::TONE_BANK_MSB)
}

/// What a scenario changes when it works with `kind`'s tone on
/// `part_index`: the Part's tone select, then the tone itself.
///
/// # Errors
///
/// `kind` has no tone type.
pub fn tone_blocks(kind: PluginKind, part_index: u8) -> Result<Touched> {
    let tone_type = kind.tone_type().context("a tone plugin")?;
    let mut blocks = vec![(select_address(part_index), TONE_SELECTION_SIZE)];
    blocks.extend(tone_read_blocks(tone_type, part_index));
    Ok(Touched::whole(blocks))
}

/// What a scenario changes when it works with the Studio Set: its blocks that
/// hold parameters, written back only where the Studio Set plugin has them.
pub fn studio_set_touched() -> Touched {
    Touched {
        blocks: params::studio_set_blocks(INTEGRA7.part_count)
            .into_iter()
            .filter_map(|b| Some((b.base, DataSize::from_value(b.size)?)))
            .collect(),
        only: Some(studio_set_param_runs()),
    }
}

/// Checks that the Part's tone, as the device selected it (`selection` is
/// Bank Select MSB, LSB and PC), is of `kind`'s type, from any bank.
///
/// # Errors
///
/// It is not: the scenario's precondition does not hold.
pub fn require_type(kind: PluginKind, selection: [u8; 3]) -> Result<()> {
    if tone_type_from_bank_msb(selection[0]) == kind.tone_type() {
        return Ok(());
    }
    bail!(
        "Part 1 must hold a {} tone, and holds {selection:?}",
        kind.name()
    )
}

/// Checks that the Part's tone, as the device selected it (`selection` is
/// Bank Select MSB, LSB and PC), is one of `kind`'s type, from `bank_of`.
///
/// # Errors
///
/// It is not: the scenario's precondition does not hold.
pub fn require_tone(kind: PluginKind, bank_of: Bank, selection: [u8; 3]) -> Result<()> {
    let [msb, lsb, _] = selection;
    let found = find_bank(msb, lsb);
    if tone_type_from_bank_msb(msb) == kind.tone_type()
        && found.is_some_and(|b| b.label == bank_of.label())
    {
        return Ok(());
    }
    bail!(
        "Part 1 must hold a {} {} tone, and holds {selection:?}",
        kind.name(),
        bank_of.label()
    )
}

/// Settles a new instance the way a host does once the plugin is on a
/// track: it starts processing and processes a block. A new instance takes a
/// target from the service, or adopts the device, only once the host has had
/// its say about its state, and the first `process()` is where it knows it
/// has. It goes on processing: the plugin sends the host's changes to the
/// device from `process()`.
///
/// # Errors
///
/// The plugin would not process.
pub fn settle(plugin: &mut Plugin) -> Result<()> {
    plugin
        .start_processing()
        .map_err(|e| anyhow!("starting to process: {e}"))?;
    process_blocks(plugin, 1)
}

/// How long a block lasts at the host's sample rate.
pub fn block_time() -> Duration {
    Duration::from_secs_f64(BLOCK_SIZE as f64 / SAMPLE_RATE)
}

/// Processes `count` blocks of silence in real time.
///
/// # Errors
///
/// The plugin is not processing, or failed to.
pub fn process_blocks(plugin: &mut Plugin, count: usize) -> Result<()> {
    let mut block = AudioBuffers::new(2, 2, BLOCK_SIZE, SAMPLE_RATE);
    for _ in 0..count {
        plugin
            .process_audio(&mut block)
            .map_err(|e| anyhow!("processing: {e}"))?;
        sleep(block_time());
    }
    Ok(())
}

/// Binds the plugin of `kind` to `binding` as a host does: by setting the
/// Binding parameters, a change of which rebinds the instance within a
/// worker tick ([`PluginKind::binding_params`]).
///
/// # Errors
///
/// The plugin declares no Binding parameter, or refused the value.
pub fn bind_as_host(plugin: &mut Plugin, kind: PluginKind, binding: &Binding) -> Result<()> {
    let declared = plugin
        .get_parameters()
        .map_err(|e| anyhow!("listing parameters: {e}"))?;
    for param in kind.binding_params() {
        let raw = param.raw_of(binding);
        let Some(found) = declared.iter().find(|d| d.name == param.name) else {
            bail!("the plugin declares no {} parameter", param.name);
        };
        let value = f64::from(raw - param.min) / f64::from(param.max - param.min);
        plugin
            .set_parameter(found.id, value)
            .map_err(|e| anyhow!("setting {}: {e}", param.name))?;
    }
    Ok(())
}

/// Makes the fake device hold every parameter of `kind` at its default for
/// `binding`, as a device that has just been initialized does, so that what a
/// new instance adopts is what the device holds, bit for bit.
pub fn hold_defaults(memory: &FakeMemory, kind: PluginKind, binding: &Binding) {
    for host in kind.host_params() {
        if let Ok(at) = host.device_address(binding) {
            memory.put(at, &host.encode(host.default));
        }
    }
}

/// The runs of the Studio Set that hold host parameters of the Studio Set
/// plugin, as a linear start and a length: adjacent parameters are one run,
/// and a Part's tone select (which only a Tone or Kit plugin writes) is in
/// none.
pub fn studio_set_param_runs() -> Vec<(u32, u32)> {
    let selects: Vec<u32> = (0..INTEGRA7.part_count)
        .map(|p| select_address(p).value())
        .collect();
    let mut fields: Vec<(u32, u32)> = PluginKind::StudioSet
        .host_params()
        .iter()
        .filter_map(|h| {
            let at = h.device_address(&Binding::ANY).ok()?.value();
            let len = h.width as u32;
            let on_a_select = selects
                .iter()
                .any(|s| at < s + TONE_SELECTION_SIZE.value() && *s < at + len);
            (!on_a_select).then_some((at, len))
        })
        .collect();
    fields.sort_unstable();
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for (at, len) in fields {
        match runs.last_mut() {
            Some((start, run)) if *start + *run >= at => *run = (*run).max(at + len - *start),
            _ => runs.push((at, len)),
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_studio_sets_runs_hold_no_tone_select_and_no_gap() {
        let runs = studio_set_param_runs();
        assert!(runs.len() > 1);
        let fields: Vec<(u32, u32)> = PluginKind::StudioSet
            .host_params()
            .iter()
            .filter_map(|h| {
                Some((
                    h.device_address(&Binding::ANY).ok()?.value(),
                    h.width as u32,
                ))
            })
            .collect();
        for p in 0..INTEGRA7.part_count {
            let s = select_address(p).value();
            assert!(
                runs.iter().all(|(at, len)| *at + len <= s || s + 3 <= *at),
                "part {p}"
            );
        }
        // Every byte of a run is some parameter's.
        for (at, len) in runs {
            for byte in at..at + len {
                assert!(
                    fields.iter().any(|(f, w)| (*f..f + w).contains(&byte)),
                    "{byte:#x}"
                );
            }
        }
    }

    #[test]
    fn a_kits_list_ends_with_its_binding_parameters() {
        let names = declared_names(PluginKind::PcmDrum);
        let hosts = PluginKind::PcmDrum.host_params().len();
        assert_eq!(&names[hosts..], &["Part", "Key"]);
        assert_eq!(names.len(), hosts + 2);
    }
    #[test]
    fn a_kits_sides_are_one_kit_wide_parameter_and_one_of_a_keys() {
        for kind in [PluginKind::PcmDrum, PluginKind::SnDrum] {
            // `ids` is unused: the test reads the list, it sets nothing.
            let params = Params {
                list: kind.host_params(),
                ids: Vec::new(),
            };
            let first = match kind {
                PluginKind::SnDrum => INTEGRA7.sn_drum_keys.first,
                _ => INTEGRA7.pcm_drum_keys.first,
            };
            let binding = Binding::of_part(0).with_key(first);
            let Some((kit_wide, key)) = params.kit_sides(&binding) else {
                panic!("{} has a kit-wide and a key side", kind.name());
            };
            assert!(
                matches!(params.host(kit_wide).anchor, Anchor::Part),
                "{}: {} is not kit-wide",
                kind.name(),
                params.host(kit_wide).name
            );
            assert!(
                matches!(params.host(key).anchor, Anchor::Key(_)),
                "{}: {} is not a key's",
                kind.name(),
                params.host(key).name
            );
            // Both findable again on the wire: an address of their own, and
            // each with at least three values, as the picking promises.
            let wide = params.host(kit_wide).device_address(&binding).unwrap();
            let one = params.host(key).device_address(&binding).unwrap();
            assert_ne!(wide.value(), one.value(), "{}", kind.name());
            for i in [kit_wide, key] {
                assert!(
                    params.host(i).max - params.host(i).min >= 2,
                    "{}: {} has not three values",
                    kind.name(),
                    params.host(i).name
                );
            }
        }
    }

    #[test]
    fn a_tone_or_studio_set_list_has_no_sides() {
        for kind in [PluginKind::StudioSet, PluginKind::PcmSynth] {
            let params = Params {
                list: kind.host_params(),
                ids: Vec::new(),
            };
            assert_eq!(params.kit_sides(&Binding::of_part(0)), None);
        }
    }

    #[test]
    fn the_studio_sets_list_is_only_its_own() {
        let names = declared_names(PluginKind::StudioSet);
        assert_eq!(
            names,
            PluginKind::StudioSet
                .host_params()
                .iter()
                .map(|h| h.name.clone())
                .collect::<Vec<_>>()
        );
        assert!(PluginKind::StudioSet.binding_params().is_empty());
    }
}
