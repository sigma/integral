//! The host parameters of one instance, and what is known about their values.
//!
//! The list is the kind's [`HostParam`]s, each beside the nih-plug
//! [`IntParam`] that declares it. nih-plug gives a plugin no setter for a
//! parameter, so the bank cannot be told what the host did except through the
//! parameter's callback, which runs on whichever thread changed it (the audio
//! thread for automation) after the value is stored. The callback keeps a
//! **mirror** of the host's values in atomics ([`host_changed`](ParamBank::host_changed)),
//! which is also what a test plays the host through. Beside it, per parameter:
//!
//! - **`seen`**: the value the instance last saw the host hold. The audio
//!   thread compares the host's value with it ([`poll`](ParamBank::poll)); a
//!   difference is a host change. Two threads write it: the audio thread in
//!   `poll`, and the worker in [`settle`](ParamBank::settle) and
//!   [`arm`](ParamBank::arm), when the host is known to hold what the device
//!   gave. A lost race costs at most one redundant write, which the service
//!   drops as unchanged.
//! - **`pending`** and the one before it: the last values the device's side
//!   gave the parameter (a gesture, or a bulk update) that the audio thread
//!   may not have seen yet, because the host passes a value back in a later
//!   audio callback. A difference that is one of them is the echo of the
//!   device's own value and is not written back; any other value is the
//!   host's own edit, even in the middle of a gesture, and is.
//! - **`device_type`**: for the parameter that holds an effect's type, the type
//!   the **device** has, as the instance last learned it (from a change, a read
//!   or its own write). The host's value of that parameter can be stale (the
//!   editor was closed when the device changed it), so a slot of another type
//!   must not be written on its account.
//! - **`dirty`**: a host edit made while writing was off (a reconnect, a read
//!   under way): written when it is back on. The host wins over the device.
//!
//! Everything but the replacement queue is an atomic, so the audio thread, the
//! worker and the main thread share the bank without locks.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use integral_core::address::Address;
use integral_core::host_params::{Anchor, Binding, HostParam, ParamHandle, PluginKind};
use integral_core::params::ToneType;
use nih_plug::prelude::*;

use super::host::HostEdits;

/// `pending` when the device's side has nothing in flight for a parameter.
const NOT_PENDING: i64 = i64::MIN;

/// `device_type` before the instance has learned the effect's type: no type
/// matches, so no slot is written.
const UNKNOWN_TYPE: i32 = -1;

/// The host's values, as the parameters' callbacks report them.
struct Mirror {
    host: Vec<AtomicI32>,
    /// Some value changed since [`ParamBank::poll`] last looked.
    changed: AtomicBool,
}

impl Mirror {
    fn set(&self, i: usize, value: i32) {
        self.host[i].store(value, Ordering::Release);
        self.changed.store(true, Ordering::Release);
    }
}

/// An instance's host parameters.
pub struct ParamBank {
    kind: PluginKind,
    /// Each host parameter and the nih-plug integer that declares it.
    list: Vec<(HostParam, IntParam)>,
    by_id: HashMap<String, usize>,
    /// For an effect type's parameter, the position of the parameter that
    /// holds the effect's type.
    type_slot: Vec<Option<usize>>,
    mirror: Arc<Mirror>,
    seen: Vec<AtomicI32>,
    pending: Vec<AtomicI64>,
    pending_before: Vec<AtomicI64>,
    /// Whether a parameter holds some effect's type.
    is_holder: Vec<bool>,
    device_type: Vec<AtomicI32>,
    dirty: Vec<AtomicBool>,
    /// Host edits made while writing was off are kept (see `dirty`): off until
    /// the first [`arm`](Self::arm), since what the host holds before then is
    /// a restored state, not an edit (recall is the push's business).
    tracking: AtomicBool,
    /// The values to replace, for the next [`HostTask::Replace`](super::host::HostTask).
    replace: Mutex<Vec<(usize, i32)>>,
    /// A replace was handed back untaken (see `requeue_replace`).
    retry: AtomicBool,
    /// What changes the host's view from the device's side, if anything can
    /// (see [`host`](super::host)).
    edits: RwLock<Option<Arc<dyn HostEdits>>>,
    /// The edits are an editor's context, taken while the instance had none
    /// ([`set_editor_edits`](Self::set_editor_edits)).
    editor_edits: AtomicBool,
    /// Who holds the host's gesture on each parameter (see [`Gesturer`]).
    /// Read and written on the host's main thread, where gestures are made.
    gesture: Vec<AtomicU8>,
}

/// Who has a gesture open on a host parameter. The host sees one gesture per
/// parameter: begin, performs, end. The device's side (another client's
/// edit, followed by the instance) and the editor (this instance's own
/// Control drag) can both want one, so whoever has it is kept here, and the
/// other waits or gives way ([`host`](super::host)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesturer {
    /// The follower, giving the host the device's changes.
    Follower,
    /// The editor, relaying a Control drag.
    Editor,
}

impl Gesturer {
    const NONE: u8 = 0;

    fn code(gesturer: Option<Self>) -> u8 {
        match gesturer {
            None => Self::NONE,
            Some(Self::Follower) => 1,
            Some(Self::Editor) => 2,
        }
    }

    fn decode(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Follower),
            2 => Some(Self::Editor),
            _ => None,
        }
    }
}

impl std::fmt::Debug for ParamBank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParamBank")
            .field("kind", &self.kind)
            .field("len", &self.list.len())
            .finish_non_exhaustive()
    }
}

/// The nih-plug parameter number `i` for `host`: an integer over its raw
/// range, with the list's text both ways, no smoothing, and a callback that
/// keeps `mirror` current.
fn int_param(host: &HostParam, i: usize, mirror: &Arc<Mirror>) -> IntParam {
    let mirror = Arc::clone(mirror);
    let range = IntRange::Linear {
        min: host.min,
        max: host.max,
    };
    let (to_text, from_text) = (host.format.clone(), host.format.clone());
    let (min, max) = (host.min, host.max);
    IntParam::new(host.name.clone(), host.default, range)
        .with_value_to_string(Arc::new(move |raw| to_text.format(raw)))
        .with_string_to_value(Arc::new(move |text| from_text.parse(text, min, max)))
        .with_callback(Arc::new(move |value| mirror.set(i, value)))
}

impl ParamBank {
    /// The bank of `kind`: every parameter at its default, which is what the
    /// host holds until it restores a state.
    pub fn new(kind: PluginKind) -> Self {
        let hosts = kind.host_params();
        let mirror = Arc::new(Mirror {
            host: hosts.iter().map(|h| AtomicI32::new(h.default)).collect(),
            changed: AtomicBool::new(false),
        });
        let list: Vec<(HostParam, IntParam)> = hosts
            .into_iter()
            .enumerate()
            .map(|(i, host)| {
                let param = int_param(&host, i, &mirror);
                (host, param)
            })
            .collect();
        let by_id = list
            .iter()
            .enumerate()
            .map(|(i, (host, _))| (host.id.clone(), i))
            .collect();
        let holders: HashMap<Address, usize> = list
            .iter()
            .enumerate()
            .filter(|(_, (host, _))| host.effect.is_none())
            .map(|(i, (host, _))| (host.address, i))
            .collect();
        let type_slot: Vec<Option<usize>> = list
            .iter()
            .map(|(host, _)| {
                host.effect
                    .and_then(|effect| holders.get(&effect.type_address).copied())
            })
            .collect();
        let mut is_holder = vec![false; list.len()];
        for holder in type_slot.iter().flatten() {
            is_holder[*holder] = true;
        }
        let seen = list
            .iter()
            .map(|(h, _)| AtomicI32::new(h.default))
            .collect();
        let pending = list.iter().map(|_| AtomicI64::new(NOT_PENDING)).collect();
        let n = list.len();
        Self {
            kind,
            list,
            by_id,
            type_slot,
            mirror,
            seen,
            pending,
            pending_before: (0..n).map(|_| AtomicI64::new(NOT_PENDING)).collect(),
            is_holder,
            device_type: (0..n).map(|_| AtomicI32::new(UNKNOWN_TYPE)).collect(),
            dirty: (0..n).map(|_| AtomicBool::new(false)).collect(),
            tracking: AtomicBool::new(false),
            replace: Mutex::new(Vec::new()),
            retry: AtomicBool::new(false),
            edits: RwLock::new(None),
            editor_edits: AtomicBool::new(false),
            gesture: (0..n).map(|_| AtomicU8::new(Gesturer::NONE)).collect(),
        }
    }

    /// The plugin kind.
    pub fn kind(&self) -> PluginKind {
        self.kind
    }

    /// The host parameters with their nih-plug counterparts, in list order.
    pub fn list(&self) -> &[(HostParam, IntParam)] {
        &self.list
    }

    /// How many parameters there are.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// The position of the parameter with ID `id`.
    pub fn position(&self, id: &str) -> Option<usize> {
        self.by_id.get(id).copied()
    }

    /// The value the host holds for parameter `i`.
    pub fn value(&self, i: usize) -> i32 {
        self.mirror.host[i].load(Ordering::Acquire)
    }

    /// Note that the host set parameter `i` to `value`: what the parameter's
    /// callback does. Tests call it to play the host (or the wrapper
    /// applying a value the instance handed it).
    pub fn host_changed(&self, i: usize, value: i32) {
        self.mirror.set(i, value);
    }

    /// The position of the parameter that holds the type of the effect
    /// parameter `i` belongs to, or `None` for any other parameter.
    pub fn type_slot(&self, i: usize) -> Option<usize> {
        self.type_slot[i]
    }

    /// Whether parameter `i` holds some effect's type.
    pub fn is_holder(&self, i: usize) -> bool {
        self.is_holder[i]
    }

    /// The effect type the device has, as held by parameter `holder`, if the
    /// instance has learned it.
    pub fn device_type(&self, holder: usize) -> Option<i32> {
        Some(self.device_type[holder].load(Ordering::Acquire)).filter(|t| *t != UNKNOWN_TYPE)
    }

    /// Note the effect type the device has, as held by parameter `holder`.
    pub fn set_device_type(&self, holder: usize, value: i32) {
        self.device_type[holder].store(value, Ordering::Release);
    }

    /// Whether parameter `i` does something now: always, unless it is a slot
    /// of an effect type other than the one the **device** has (the host's
    /// value of the type parameter may be stale). An effect whose type the
    /// instance has not learned is inert.
    pub fn is_active(&self, i: usize) -> bool {
        match (self.list[i].0.effect, self.type_slot[i]) {
            (None, _) => true,
            (Some(effect), Some(holder)) => {
                self.device_type(holder) == Some(i32::from(effect.type_id))
            }
            (Some(_), None) => false,
        }
    }

    /// Whether parameter `i` is on the device for `binding`: the plugin can
    /// have that Binding and the parameter lives where it points (a key's
    /// parameter needs a key, and a Kit plugin bound to a key leaves the
    /// kit-wide ones inert).
    pub fn placed(&self, i: usize, binding: &Binding) -> bool {
        let kit = matches!(
            self.kind.tone_type(),
            Some(ToneType::SnDrum | ToneType::PcmDrum)
        );
        self.kind.accepts(binding)
            && match self.list[i].0.anchor {
                Anchor::Absolute => true,
                Anchor::Part => binding.part.is_some() && !(kit && binding.key.is_some()),
                Anchor::Key(_) => binding.key.is_some(),
            }
    }

    /// Forget the effect types the device has: they are out of date until the
    /// next read or change tells them (and meanwhile no effect slot is
    /// written, so a write cannot land on a type just loaded).
    pub fn forget_types(&self) {
        for t in &self.device_type {
            t.store(UNKNOWN_TYPE, Ordering::Release);
        }
    }

    /// Whether parameter `i` is not an effect slot, or the effect has its type
    /// as far as the host's own value of the type parameter says: the slots a
    /// recall sends, whatever the device held before.
    pub fn host_active(&self, i: usize) -> bool {
        match (self.list[i].0.effect, self.type_slot[i]) {
            (None, _) => true,
            (Some(effect), Some(holder)) => self.value(holder) == i32::from(effect.type_id),
            (Some(_), None) => false,
        }
    }

    /// Whether parameter `i` acts on the device for `binding`: it is
    /// [`placed`](Self::placed) and, for an effect slot, its type is the
    /// active one.
    pub fn live(&self, i: usize, binding: &Binding) -> bool {
        self.placed(i, binding) && self.is_active(i)
    }

    /// The binding a write of parameter `i` carries for an instance bound to
    /// `binding`: a Studio Set ID holds its Part and has none.
    pub fn wire_binding(&self, i: usize, binding: &Binding) -> Binding {
        match self.list[i].0.anchor {
            Anchor::Absolute => Binding::ANY,
            _ => *binding,
        }
    }

    /// The audio thread's look at the parameters: for each whose host value
    /// differs from the last seen, note it and, if it is a host change to a
    /// parameter that [`live`](Self::live) for `binding`, hand it to `write`
    /// (handle, binding, value). `write` returns whether it took it; if not,
    /// the change stays to be found by the next call.
    ///
    /// A value that is one the device's side gave (see the module docs) is
    /// not the host's and is written nowhere. With `armed` false the host's
    /// edits are written nowhere yet: once the first [`arm`](Self::arm) has
    /// passed they are kept and written when writing is back on. A written
    /// effect type becomes the device's.
    ///
    /// Wait-free: atomics only, no allocation.
    pub fn poll(
        &self,
        binding: Option<Binding>,
        armed: bool,
        mut write: impl FnMut(ParamHandle, Binding, i32) -> bool,
    ) {
        if !self.mirror.changed.swap(false, Ordering::AcqRel) {
            return;
        }
        let mut again = false;
        for i in 0..self.list.len() {
            let value = self.value(i);
            let before = self.seen[i].load(Ordering::Relaxed);
            if value == before {
                continue;
            }
            let expected = self.pending[i].load(Ordering::Acquire);
            if expected != NOT_PENDING {
                let value64 = i64::from(value);
                if value64 == expected {
                    self.pending[i].store(NOT_PENDING, Ordering::Release);
                    self.pending_before[i].store(NOT_PENDING, Ordering::Release);
                    self.seen[i].store(value, Ordering::Relaxed);
                    continue;
                }
                if value64 == self.pending_before[i].load(Ordering::Acquire) {
                    // The host passing back the value before the last.
                    self.seen[i].store(value, Ordering::Relaxed);
                    continue;
                }
                // Neither: the host's own edit, in the middle of a gesture.
                self.pending[i].store(NOT_PENDING, Ordering::Release);
                self.pending_before[i].store(NOT_PENDING, Ordering::Release);
            }
            let live = binding.filter(|b| self.live(i, b));
            match live {
                Some(binding) if armed => {
                    let handle = ParamHandle {
                        plugin: self.kind,
                        // A list is far shorter than `u32::MAX`.
                        index: i as u32,
                    };
                    if write(handle, self.wire_binding(i, &binding), value) {
                        self.seen[i].store(value, Ordering::Relaxed);
                        if self.is_holder[i] {
                            self.set_device_type(i, value);
                        }
                    } else {
                        again = true;
                    }
                }
                _ if !armed
                    && self.tracking.load(Ordering::Acquire)
                    && binding.is_some_and(|b| {
                        self.placed(i, &b) && (self.is_active(i) || self.host_active(i))
                    }) =>
                {
                    // Not recorded as seen: found again once writing is on.
                    // A slot of the type the host just gave the effect counts:
                    // the type is written first, in list order.
                    self.dirty[i].store(true, Ordering::Release);
                }
                _ => self.seen[i].store(value, Ordering::Relaxed),
            }
        }
        if again {
            self.mirror.changed.store(true, Ordering::Release);
        }
    }

    /// Note that the device's side is about to give parameter `i` `value`, so
    /// that the audio thread does not take it for the host's.
    pub fn expect(&self, i: usize, value: i32) {
        let last = self.pending[i].swap(i64::from(value), Ordering::AcqRel);
        self.pending_before[i].store(last, Ordering::Release);
    }

    /// The device's side is done with parameter `i`: whatever the host holds
    /// now is what the instance has seen.
    pub fn settle(&self, i: usize) {
        self.seen[i].store(self.value(i), Ordering::Relaxed);
        self.pending[i].store(NOT_PENDING, Ordering::Release);
        self.pending_before[i].store(NOT_PENDING, Ordering::Release);
    }

    /// Whether the host edited parameter `i` while writing was off.
    pub fn is_dirty(&self, i: usize) -> bool {
        self.dirty[i].load(Ordering::Acquire)
    }

    /// Writing is on: take the host's values as seen, except those it edited
    /// meanwhile, which the next [`poll`](Self::poll) writes (the host wins),
    /// and those the device's side has in flight. From now on edits made while
    /// writing is off are kept.
    pub fn arm(&self) {
        for i in 0..self.list.len() {
            if self.dirty[i].swap(false, Ordering::AcqRel) {
                continue;
            }
            if self.pending[i].load(Ordering::Acquire) == NOT_PENDING {
                self.seen[i].store(self.value(i), Ordering::Relaxed);
            }
        }
        self.tracking.store(true, Ordering::Release);
        self.mirror.changed.store(true, Ordering::Release);
    }

    /// Forget the host's edits made while writing was off: they were taken as
    /// seen, and are not written when it is back on.
    pub fn forget_edits(&self) {
        for i in 0..self.list.len() {
            if self.dirty[i].swap(false, Ordering::AcqRel) {
                self.seen[i].store(self.value(i), Ordering::Relaxed);
            }
        }
    }

    /// Queue `changes` (position, value) for the next replace task.
    pub fn queue_replace(&self, changes: impl IntoIterator<Item = (usize, i32)>) {
        self.replace.lock().expect("replace lock").extend(changes);
    }

    /// Put replacements the host did not take back in front of the queue (the
    /// newer ones win) and note that the worker should try again.
    pub fn requeue_replace(&self, changes: Vec<(usize, i32)>) {
        let mut queue = self.replace.lock().expect("replace lock");
        queue.splice(0..0, changes);
        self.retry.store(true, Ordering::Release);
    }

    /// Whether a replace was handed back since the last call.
    pub fn take_retry(&self) -> bool {
        self.retry.swap(false, Ordering::AcqRel)
    }

    /// Who has the host's gesture on parameter `i`.
    pub fn gesturer(&self, i: usize) -> Option<Gesturer> {
        Gesturer::decode(self.gesture[i].load(Ordering::Acquire))
    }

    /// Note who has the host's gesture on parameter `i`.
    pub fn set_gesturer(&self, i: usize, gesturer: Option<Gesturer>) {
        self.gesture[i].store(Gesturer::code(gesturer), Ordering::Release);
    }

    /// Take the queued replacements.
    pub fn take_replace(&self) -> Vec<(usize, i32)> {
        std::mem::take(&mut *self.replace.lock().expect("replace lock"))
    }

    /// What changes the host's view from the device's side, while there is
    /// something to: an open editor's context, or a test's recorder.
    pub fn edits(&self) -> Option<Arc<dyn HostEdits>> {
        self.edits.read().expect("edits lock").clone()
    }

    /// Set (or, with `None`, withdraw) what changes the host's view.
    pub fn set_edits(&self, edits: Option<Arc<dyn HostEdits>>) {
        let mut slot = self.edits.write().expect("edits lock");
        *slot = edits;
        self.editor_edits.store(false, Ordering::Release);
    }

    /// Set what changes the host's view unless something does already; an
    /// editor's context ([`set_editor_edits`](Self::set_editor_edits)) gives
    /// way. Returns whether `edits` was taken.
    pub fn set_edits_once(&self, edits: Arc<dyn HostEdits>) -> bool {
        let mut slot = self.edits.write().expect("edits lock");
        if slot.is_some() && !self.editor_edits.load(Ordering::Acquire) {
            return false;
        }
        *slot = Some(edits);
        self.editor_edits.store(false, Ordering::Release);
        true
    }

    /// An editor's context, for the host's view while the instance has none
    /// (the host opened the editor before activating the plugin). Returns
    /// whether it was taken. [`set_edits_once`](Self::set_edits_once) replaces
    /// it later; [`withdraw_editor_edits`](Self::withdraw_editor_edits) takes
    /// it away.
    pub fn set_editor_edits(&self, edits: Arc<dyn HostEdits>) -> bool {
        let mut slot = self.edits.write().expect("edits lock");
        if slot.is_some() {
            return false;
        }
        *slot = Some(edits);
        self.editor_edits.store(true, Ordering::Release);
        true
    }

    /// The editor closes: withdraw its context, if that is what the instance
    /// has.
    pub fn withdraw_editor_edits(&self) {
        let mut slot = self.edits.write().expect("edits lock");
        if self.editor_edits.swap(false, Ordering::AcqRel) {
            *slot = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use integral_core::host_params::{EffectKind, PluginKind};

    fn bank(kind: PluginKind) -> ParamBank {
        ParamBank::new(kind)
    }

    fn position(bank: &ParamBank, id: &str) -> usize {
        bank.position(id).unwrap_or_else(|| panic!("no {id}"))
    }

    /// A changed host value, polled with the given binding and armed flag.
    fn written(
        bank: &ParamBank,
        binding: Option<Binding>,
        armed: bool,
    ) -> Vec<(usize, Binding, i32)> {
        let mut out = Vec::new();
        bank.poll(binding, armed, |handle, binding, value| {
            out.push((handle.index as usize, binding, value));
            true
        });
        out
    }

    fn some_in_range(host: &HostParam) -> i32 {
        if host.default == host.max {
            host.min
        } else {
            host.max
        }
    }

    #[test]
    fn every_effect_slot_has_a_type_parameter_before_it() {
        for kind in PluginKind::ALL {
            let bank = bank(kind);
            for (i, (host, _)) in bank.list().iter().enumerate() {
                match (host.effect, bank.type_slot(i)) {
                    (None, None) => {}
                    (Some(effect), Some(holder)) => {
                        let holder_host = &bank.list()[holder].0;
                        assert!(holder < i, "{kind:?} {}: its type is after it", host.id);
                        assert_eq!(holder_host.address, effect.type_address, "{}", host.id);
                        assert!(holder_host.effect.is_none(), "{}", host.id);
                    }
                    other => panic!("{kind:?} {}: {other:?}", host.id),
                }
            }
        }
    }

    #[test]
    fn a_host_change_is_written_once_with_the_handle_and_binding() {
        let bank = bank(PluginKind::PcmSynth);
        let binding = Binding::of_part(3);
        let i = position(&bank, "pcms/partial1/0x0F");
        let value = some_in_range(&bank.list()[i].0);
        bank.host_changed(i, value);
        assert_eq!(written(&bank, Some(binding), true), [(i, binding, value)]);
        assert_eq!(written(&bank, Some(binding), true), [], "seen now");
    }

    #[test]
    fn unbound_unarmed_and_unaccepted_instances_write_nothing() {
        let bank = bank(PluginKind::PcmSynth);
        let i = position(&bank, "pcms/partial1/0x0F");
        let value = some_in_range(&bank.list()[i].0);
        bank.host_changed(i, value);
        assert_eq!(written(&bank, None, true), [], "unbound");

        bank.host_changed(i, bank.list()[i].0.default);
        assert_eq!(
            written(&bank, Some(Binding::of_part(0)), false),
            [],
            "unarmed"
        );

        bank.host_changed(i, value);
        let drum_key = Binding::of_part(0).with_key(36);
        assert_eq!(written(&bank, Some(drum_key), true), [], "a key on a Tone");
        bank.host_changed(i, bank.list()[i].0.default);
        assert_eq!(written(&bank, Some(Binding::ANY), true), [], "no Part");
        bank.host_changed(i, value);
        assert_eq!(
            written(&bank, Some(Binding::of_part(16)), true),
            [],
            "Part 16"
        );
    }

    #[test]
    fn the_studio_set_plugin_writes_without_a_binding_for_its_parts_ids() {
        let bank = bank(PluginKind::StudioSet);
        let i = position(&bank, "part03/0x09");
        let value = some_in_range(&bank.list()[i].0);
        bank.host_changed(i, value);
        assert_eq!(
            written(&bank, Some(Binding::ANY), true),
            [(i, Binding::ANY, value)]
        );
    }

    #[test]
    fn a_kit_plugin_bound_to_a_key_leaves_the_kit_wide_parameters_inert_and_a_kit_the_keys() {
        let bank = bank(PluginKind::PcmDrum);
        let kit_wide = position(&bank, "pcmd/common/0x0C");
        let key_wide = position(&bank, "pcmd/key/0x0E");
        let (kit, key) = (Binding::of_part(1), Binding::of_part(1).with_key(40));
        assert!(bank.live(kit_wide, &kit) && !bank.live(key_wide, &kit));
        assert!(!bank.live(kit_wide, &key) && bank.live(key_wide, &key));
    }

    #[test]
    fn an_inactive_effect_types_parameters_write_nothing() {
        let bank = bank(PluginKind::PcmSynth);
        let binding = Binding::of_part(0);
        let slot_of = |type_id: u8| {
            bank.list()
                .iter()
                .position(|(h, _)| {
                    h.effect
                        .is_some_and(|e| e.kind == EffectKind::Mfx && e.type_id == type_id)
                        && h.min < h.max
                })
                .expect("a slot of that type")
        };
        let (first, second) = (slot_of(1), slot_of(2));
        let holder = bank.type_slot(first).expect("a holder");
        assert_eq!(bank.type_slot(second), Some(holder));
        // The effect starts as type 0 (THRU), which has no parameters: both
        // are inert.
        assert_eq!(bank.value(holder), 0);
        bank.host_changed(first, some_in_range(&bank.list()[first].0));
        bank.host_changed(second, some_in_range(&bank.list()[second].0));
        assert_eq!(written(&bank, Some(binding), true), [], "THRU");

        // The effect becomes type 1: its slot writes, type 2's does not.
        bank.host_changed(holder, 1);
        assert_eq!(written(&bank, Some(binding), true), [(holder, binding, 1)]);
        let value = bank.list()[first].0.max;
        bank.host_changed(first, value);
        bank.host_changed(second, bank.list()[second].0.max);
        assert_eq!(
            written(&bank, Some(binding), true),
            [(first, binding, value)],
            "the active type's only"
        );

        // The type and its slot changed in one look: in list order, the new
        // type counts for the slot.
        bank.host_changed(holder, 2);
        let value = bank.list()[second].0.min;
        bank.host_changed(second, value);
        assert_eq!(
            written(&bank, Some(binding), true),
            [(holder, binding, 2), (second, binding, value)]
        );
    }

    #[test]
    fn what_the_device_gave_is_not_written_back_even_when_seen_late() {
        let bank = bank(PluginKind::PcmSynth);
        let binding = Binding::of_part(0);
        let i = position(&bank, "pcms/partial1/0x0F");
        let host = &bank.list()[i].0;
        let (a, b) = (host.max, host.min);
        // The device gives `a`, then `b` before the host passes `a` back.
        bank.expect(i, a);
        bank.expect(i, b);
        bank.host_changed(i, a);
        assert_eq!(written(&bank, Some(binding), true), [], "the intermediate");
        bank.host_changed(i, b);
        assert_eq!(written(&bank, Some(binding), true), [], "the last");
        // Pending is cleared: the host's own next change is written.
        bank.host_changed(i, a);
        assert_eq!(written(&bank, Some(binding), true), [(i, binding, a)]);
    }

    #[test]
    fn a_write_the_ring_refused_is_found_again() {
        let bank = bank(PluginKind::PcmSynth);
        let binding = Binding::of_part(0);
        let i = position(&bank, "pcms/partial1/0x0F");
        let value = some_in_range(&bank.list()[i].0);
        bank.host_changed(i, value);
        bank.poll(Some(binding), true, |_, _, _| false);
        assert_eq!(written(&bank, Some(binding), true), [(i, binding, value)]);
    }

    #[test]
    fn settling_takes_the_hosts_value_as_seen() {
        let bank = bank(PluginKind::PcmSynth);
        let i = position(&bank, "pcms/partial1/0x0F");
        let value = some_in_range(&bank.list()[i].0);
        bank.host_changed(i, value);
        bank.expect(i, 0);
        bank.arm();
        assert_eq!(written(&bank, Some(Binding::of_part(0)), true), []);
    }

    #[test]
    fn a_host_edit_made_while_writing_was_off_is_written_when_it_is_back_on() {
        let bank = bank(PluginKind::PcmSynth);
        let binding = Binding::of_part(0);
        let i = position(&bank, "pcms/partial1/0x0F");
        let value = some_in_range(&bank.list()[i].0);

        // Before the first arm the host's values are a restored state: not
        // edits, not kept.
        bank.host_changed(i, value);
        assert_eq!(written(&bank, Some(binding), false), []);
        assert!(!bank.is_dirty(i));
        bank.arm();
        assert_eq!(
            written(&bank, Some(binding), true),
            [],
            "restored, not sent"
        );

        // Writing off (a lost link): the edit is kept, not seen.
        let back = bank.list()[i].0.default;
        bank.host_changed(i, back);
        assert_eq!(written(&bank, Some(binding), false), []);
        assert!(bank.is_dirty(i));
        // Arming again does not take it for what the device holds.
        bank.arm();
        assert!(!bank.is_dirty(i));
        assert_eq!(written(&bank, Some(binding), true), [(i, binding, back)]);
    }

    #[test]
    fn a_host_value_that_is_not_the_one_the_device_gave_is_the_hosts_even_mid_gesture() {
        let bank = bank(PluginKind::PcmSynth);
        let binding = Binding::of_part(0);
        let i = position(&bank, "pcms/partial1/0x0F");
        let host = &bank.list()[i].0;
        let (a, edit) = (host.max, host.max - 1);
        bank.expect(i, a);
        bank.host_changed(i, edit);
        assert_eq!(written(&bank, Some(binding), true), [(i, binding, edit)]);
    }

    #[test]
    fn an_edit_to_the_slot_of_a_type_the_host_just_chose_is_kept_with_the_type() {
        let bank = bank(PluginKind::PcmSynth);
        let binding = Binding::of_part(0);
        let slot = bank
            .list()
            .iter()
            .position(|(h, _)| h.effect.is_some_and(|e| e.type_id == 1) && h.min < h.max)
            .expect("a slot of type 1");
        let holder = bank.type_slot(slot).expect("a holder");
        bank.set_device_type(holder, 0);
        bank.arm();

        // Writing off: the host switches the effect to type 1 and edits its slot.
        bank.host_changed(holder, 1);
        bank.host_changed(slot, bank.list()[slot].0.max.max(bank.value(slot) + 1));
        let value = bank.value(slot);
        assert_eq!(written(&bank, Some(binding), false), []);
        bank.arm();
        assert_eq!(
            written(&bank, Some(binding), true),
            [(holder, binding, 1), (slot, binding, value)],
            "the type first, then its slot"
        );
    }

    #[test]
    fn forgotten_effect_types_leave_every_slot_inert_until_told() {
        let bank = bank(PluginKind::PcmSynth);
        let slot = bank
            .list()
            .iter()
            .position(|(h, _)| h.effect.is_some_and(|e| e.type_id == 1))
            .expect("a slot");
        let holder = bank.type_slot(slot).expect("a holder");
        bank.set_device_type(holder, 1);
        assert!(bank.is_active(slot));
        bank.forget_types();
        assert_eq!(bank.device_type(holder), None);
        assert!(!bank.is_active(slot));
    }

    #[test]
    fn the_first_edits_stay() {
        struct Nothing;
        impl HostEdits for Nothing {
            fn begin(&self, _: &IntParam) {}
            fn perform(&self, _: &IntParam, _: i32) {}
            fn end(&self, _: &IntParam) {}
            fn replace(&self, _: &[super::super::host::Replace<'_>]) -> bool {
                true
            }
        }
        let bank = bank(PluginKind::PcmSynth);
        let first: Arc<dyn HostEdits> = Arc::new(Nothing);
        assert!(bank.set_edits_once(Arc::clone(&first)));
        assert!(!bank.set_edits_once(Arc::new(Nothing)));
        assert!(Arc::ptr_eq(&bank.edits().unwrap(), &first));
    }

    #[test]
    fn replacements_handed_back_go_first_and_ask_for_a_retry() {
        let bank = bank(PluginKind::PcmSynth);
        bank.queue_replace([(3, 9)]);
        bank.requeue_replace(vec![(3, 1), (4, 2)]);
        assert!(bank.take_retry() && !bank.take_retry());
        // The newer value for 3 comes last, so it wins.
        assert_eq!(bank.take_replace(), [(3, 1), (4, 2), (3, 9)]);
    }
}
