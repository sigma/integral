//! The Binding as host parameters: a Part on every Tone and Kit plugin, and a
//! key on a Kit plugin, automatable from the host's device view.
//!
//! They are apart from the [`ParamBank`](super::bank::ParamBank): they live
//! at no device address, and a change of theirs rebinds the instance instead
//! of writing the device.
//!
//! - **A host change rebinds.** A parameter's callback runs on whichever
//!   thread the host changed it on (the audio thread for automation) and
//!   only stores atomics: it keeps the host's value and flags an edit. The
//!   instance's worker takes the edit within a tick and binds
//!   ([`Instance::follow_binding_params`](super::instance::Instance::follow_binding_params)),
//!   the way the editor's bind does.
//! - **The Binding sets the parameters.** The same tick finds the parameters
//!   behind the Binding and has the host's main thread set them as a gesture
//!   ([`HostTask::SyncBinding`](super::host::HostTask)). The values it is
//!   about to set are noted as expected ([`expect`](BindingParams::expect)),
//!   so the callback they cause (at once, or a process cycle later when the
//!   host passes them back) is no host edit and does not rebind. That is how
//!   the two do not loop.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use integral_core::host_params::{Binding, BindingField, BindingParam, PluginKind, binding_of};
use nih_plug::prelude::*;

/// How long a queued sync is waited for before it is queued again: the host
/// may have dropped it, and a duplicate is harmless (it re-checks what
/// differs).
pub const SYNC_RETRY: Duration = Duration::from_secs(1);

/// `pending` when the instance has set nothing the host has yet to pass back.
const NOT_PENDING: i64 = i64::MIN;

/// What the parameters' callbacks and the worker share.
struct Shared {
    /// The value the host holds for each parameter.
    held: Vec<AtomicI32>,
    /// The last value the instance set, until the host passes it back, and
    /// the one before it (the host may pass them back late and in order).
    pending: Vec<AtomicI64>,
    before: Vec<AtomicI64>,
    /// The host changed this value, and the instance did not set it.
    edited: Vec<AtomicBool>,
    /// Some value is edited and the worker has not bound it yet: the old
    /// Binding must not take the host's edits meanwhile
    /// ([`Instance::is_armed`](super::instance::Instance::is_armed)).
    rebinding: AtomicBool,
    /// When a [`HostTask::SyncBinding`](super::host::HostTask) was queued, until
    /// it runs. The host's queue drops tasks silently when full, so this
    /// expires ([`SYNC_RETRY`]).
    sync_queued: Mutex<Option<Instant>>,
}

impl Shared {
    /// The host's value of parameter `i` is now `value`. Wait-free.
    fn changed(&self, i: usize, value: i32) {
        self.held[i].store(value, Ordering::Release);
        let value64 = i64::from(value);
        if self.pending[i].load(Ordering::Acquire) != NOT_PENDING {
            if value64 == self.pending[i].load(Ordering::Acquire) {
                self.pending[i].store(NOT_PENDING, Ordering::Release);
                self.before[i].store(NOT_PENDING, Ordering::Release);
                return;
            }
            if value64 == self.before[i].load(Ordering::Acquire) {
                return;
            }
            self.pending[i].store(NOT_PENDING, Ordering::Release);
            self.before[i].store(NOT_PENDING, Ordering::Release);
        }
        self.edited[i].store(true, Ordering::Release);
        self.rebinding.store(true, Ordering::Release);
    }
}

/// An instance's Binding parameters, none for the Studio Set plugin.
pub struct BindingParams {
    list: Vec<(BindingParam, IntParam)>,
    shared: Arc<Shared>,
}

impl std::fmt::Debug for BindingParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BindingParams")
            .field("len", &self.list.len())
            .finish_non_exhaustive()
    }
}

impl BindingParams {
    /// The Binding parameters of `kind`, at their defaults.
    pub fn new(kind: PluginKind) -> Self {
        let specs = kind.binding_params();
        let shared = Arc::new(Shared {
            held: specs.iter().map(|s| AtomicI32::new(s.default)).collect(),
            pending: specs.iter().map(|_| AtomicI64::new(NOT_PENDING)).collect(),
            before: specs.iter().map(|_| AtomicI64::new(NOT_PENDING)).collect(),
            edited: specs.iter().map(|_| AtomicBool::new(false)).collect(),
            rebinding: AtomicBool::new(false),
            sync_queued: Mutex::new(None),
        });
        let list = specs
            .into_iter()
            .enumerate()
            .map(|(i, spec)| {
                let (to_text, from_text) = (spec, spec);
                let callback = Arc::clone(&shared);
                let param = IntParam::new(
                    spec.name,
                    spec.default,
                    IntRange::Linear {
                        min: spec.min,
                        max: spec.max,
                    },
                )
                .with_value_to_string(Arc::new(move |raw| to_text.format(raw)))
                .with_string_to_value(Arc::new(move |text| from_text.parse(text)))
                .with_callback(Arc::new(move |value| callback.changed(i, value)));
                (spec, param)
            })
            .collect();
        Self { list, shared }
    }

    /// The parameters with their specs, in list order.
    pub fn list(&self) -> &[(BindingParam, IntParam)] {
        &self.list
    }

    /// The value the host holds for parameter `i`.
    pub fn value(&self, i: usize) -> i32 {
        self.shared.held[i].load(Ordering::Acquire)
    }

    /// Note that the instance is about to set parameter `i` to `value`, so the
    /// callback that follows is not taken for the host's edit.
    pub fn expect(&self, i: usize, value: i32) {
        let last = self.shared.pending[i].swap(i64::from(value), Ordering::AcqRel);
        self.shared.before[i].store(last, Ordering::Release);
    }

    /// Note that the host set parameter `i` to `value`: what the parameter's
    /// callback does. Tests call it to play the host.
    pub fn host_changed(&self, i: usize, value: i32) {
        self.shared.changed(i, value);
    }

    /// Whether the host changed a value since the last
    /// [`take_host_edit`](Self::take_host_edit).
    pub fn edited(&self) -> bool {
        self.shared.edited.iter().any(|e| e.load(Ordering::Acquire))
    }

    /// Whether a host change waits to be bound: until it is, the old Binding
    /// must write nothing.
    pub fn rebinding(&self) -> bool {
        self.shared.rebinding.load(Ordering::Acquire)
    }

    /// The Binding after the host's changes since the last call, if there
    /// were any: `current` with the halves the host changed replaced, and
    /// the others left as they are (the values held for those may be stale,
    /// as the instance's own are only passed back later).
    pub fn take_host_edit(&self, current: Option<Binding>) -> Option<Binding> {
        let edited: Vec<bool> = self
            .shared
            .edited
            .iter()
            .map(|e| e.swap(false, Ordering::AcqRel))
            .collect();
        if !edited.contains(&true) {
            return None;
        }
        let raw: Vec<i32> = (0..self.list.len()).map(|i| self.value(i)).collect();
        let specs: Vec<BindingParam> = self.list.iter().map(|(spec, _)| *spec).collect();
        let held = binding_of(&specs, &raw);
        let mut binding = current.unwrap_or(held);
        for ((spec, _), _) in self.list.iter().zip(&edited).filter(|(_, e)| **e) {
            match spec.field {
                BindingField::Part => binding.part = held.part,
                BindingField::Key(_) => binding.key = held.key,
            }
        }
        Some(binding)
    }

    /// The worker has bound the host's change (or found nothing to bind): the
    /// old Binding's writes were held back until now.
    /// Returns whether the hold is over: an edit landing meanwhile keeps it.
    pub fn bound(&self) -> bool {
        // Clear first, then look: an edit that lands after the look raises
        // the flag itself, and one before it is seen here.
        self.shared.rebinding.store(false, Ordering::Release);
        if self.edited() {
            self.shared.rebinding.store(true, Ordering::Release);
            return false;
        }
        true
    }

    /// Forget the host's changes: they were a state being restored, whose
    /// saved Binding is the one that stands.
    pub fn discard_host_edit(&self) {
        for e in &self.shared.edited {
            e.store(false, Ordering::Release);
        }
        self.shared.rebinding.store(false, Ordering::Release);
    }

    /// Claim the right to queue a sync at `now`: false if one was queued
    /// less than [`SYNC_RETRY`] ago and has not run.
    pub fn try_queue_sync(&self, now: Instant) -> bool {
        let mut queued = self.shared.sync_queued.lock().expect("sync lock");
        if queued.is_some_and(|at| now.saturating_duration_since(at) < SYNC_RETRY) {
            return false;
        }
        *queued = Some(now);
        true
    }

    /// The queued sync ran, or could not be queued.
    pub fn sync_done(&self) {
        *self.shared.sync_queued.lock().expect("sync lock") = None;
    }

    /// The parameters that do not hold `binding` yet, with the value each
    /// should take. A value the instance set and the host has yet to pass
    /// back counts as held.
    pub fn differing(&self, binding: &Binding) -> Vec<(usize, i32)> {
        self.list
            .iter()
            .enumerate()
            .filter_map(|(i, (spec, _))| {
                let wanted = spec.raw_of(binding);
                let pending = self.shared.pending[i].load(Ordering::Acquire);
                let held = if pending == NOT_PENDING {
                    i64::from(self.value(i))
                } else {
                    pending
                };
                (held != i64::from(wanted)).then_some((i, wanted))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::super::host::{HostEdits, Replace};
    use super::super::instance::Instance;
    use super::super::params::FamilyParams;
    use super::*;
    use integral_core::host_params::{BINDING_KEY_ID, BINDING_PART_ID};
    use std::collections::BTreeMap;
    use std::sync::{Mutex, Weak};

    #[derive(Debug, Clone, PartialEq)]
    enum Edit {
        Begin(&'static str),
        Perform(&'static str, i32),
        End(&'static str),
    }

    /// The host: records the gestures on the Binding's parameters and, if it
    /// `echoes`, passes each performed value back through the parameter's
    /// callback as a host not processing audio does.
    struct Host {
        instance: Weak<Instance>,
        echoes: bool,
        edits: Mutex<Vec<Edit>>,
    }

    impl Host {
        fn at(&self, param: &IntParam) -> usize {
            let instance = self.instance.upgrade().expect("the instance");
            let list = instance.binding_params.list();
            list.iter()
                .position(|(_, p)| std::ptr::eq(p, param))
                .expect("a Binding parameter")
        }
        fn id(&self, param: &IntParam) -> &'static str {
            let instance = self.instance.upgrade().expect("the instance");
            instance.binding_params.list()[self.at(param)].0.id
        }
        fn edits(&self) -> Vec<Edit> {
            self.edits.lock().unwrap().clone()
        }
    }

    impl HostEdits for Host {
        fn begin(&self, param: &IntParam) {
            let edit = Edit::Begin(self.id(param));
            self.edits.lock().unwrap().push(edit);
        }
        fn perform(&self, param: &IntParam, value: i32) {
            let edit = Edit::Perform(self.id(param), value);
            self.edits.lock().unwrap().push(edit);
            if self.echoes {
                let instance = self.instance.upgrade().expect("the instance");
                instance.binding_params.host_changed(self.at(param), value);
            }
        }
        fn end(&self, param: &IntParam) {
            let edit = Edit::End(self.id(param));
            self.edits.lock().unwrap().push(edit);
        }
        fn replace(&self, _: &[Replace<'_>]) -> bool {
            true
        }
    }

    const PART: &str = BINDING_PART_ID;
    const KEY: &str = BINDING_KEY_ID;

    /// A settled instance of `kind` with a host.
    fn settled(kind: PluginKind, echoes: bool) -> (Arc<Instance>, Arc<Host>) {
        let instance = Arc::new(Instance::new(kind));
        let host = Arc::new(Host {
            instance: Arc::downgrade(&instance),
            echoes,
            edits: Mutex::new(Vec::new()),
        });
        instance.bank.set_edits(Some(host.clone()));
        instance.settle();
        (instance, host)
    }

    fn gesture(id: &'static str, value: i32) -> [Edit; 3] {
        [Edit::Begin(id), Edit::Perform(id, value), Edit::End(id)]
    }

    #[test]
    fn a_host_change_of_part_or_key_rebinds() {
        let (instance, host) = settled(PluginKind::PcmDrum, true);
        let keys = integral_core::device_spec::INTEGRA7.pcm_drum_keys;

        instance.binding_params.host_changed(0, 5);
        instance.follow_binding_params();
        assert_eq!(instance.binding.get(), Some(Binding::of_part(4)));

        instance.binding_params.host_changed(1, 3);
        instance.follow_binding_params();
        let key = keys.first + 2;
        assert_eq!(
            instance.binding.get(),
            Some(Binding::of_part(4).with_key(key))
        );

        // Back to the whole kit.
        instance.binding_params.host_changed(1, 0);
        instance.follow_binding_params();
        assert_eq!(instance.binding.get(), Some(Binding::of_part(4)));

        // The host's own changes are not set on it again.
        instance.follow_binding_params();
        assert_eq!(host.edits(), []);
    }

    #[test]
    fn an_editor_bind_moves_the_parameters_as_a_gesture_without_rebinding() {
        let (instance, host) = settled(PluginKind::PcmDrum, true);
        let keys = integral_core::device_spec::INTEGRA7.pcm_drum_keys;
        instance
            .bind(Binding::of_part(6).with_key(keys.first + 1))
            .unwrap();
        instance.follow_binding_params();

        let mut expected = gesture(PART, 7).to_vec();
        expected.extend(gesture(KEY, 2));
        assert_eq!(host.edits(), expected);
        assert_eq!(instance.binding_params.value(0), 7);

        // Its own change is no host edit: nothing is set again or rebound.
        instance.follow_binding_params();
        instance.follow_binding_params();
        assert_eq!(host.edits(), expected);
        assert_eq!(
            instance.binding.get(),
            Some(Binding::of_part(6).with_key(keys.first + 1))
        );
    }

    #[test]
    fn a_value_the_host_passes_back_late_is_not_a_host_edit() {
        let (instance, host) = settled(PluginKind::SnSynth, false);
        instance.bind(Binding::of_part(1)).unwrap();
        instance.follow_binding_params();
        instance.bind(Binding::of_part(2)).unwrap();
        instance.follow_binding_params();
        let mut expected = gesture(PART, 2).to_vec();
        expected.extend(gesture(PART, 3));
        assert_eq!(host.edits(), expected);

        // Neither a repeat: the host has not shown a value yet.
        instance.follow_binding_params();
        assert_eq!(host.edits().len(), 6);

        // The host passes the values back, in order.
        instance.binding_params.host_changed(0, 2);
        instance.binding_params.host_changed(0, 3);
        instance.follow_binding_params();
        assert_eq!(instance.binding.get(), Some(Binding::of_part(2)));
        assert_eq!(host.edits().len(), 6);

        // And its own edit afterwards still counts.
        instance.binding_params.host_changed(0, 9);
        instance.follow_binding_params();
        assert_eq!(instance.binding.get(), Some(Binding::of_part(8)));
    }

    #[test]
    fn a_change_before_the_host_has_settled_is_not_an_edit() {
        let instance = Instance::new(PluginKind::SnSynth);
        instance.binding_params.host_changed(0, 4);
        instance.follow_binding_params();
        assert_eq!(instance.binding.get(), None);
        instance.settle();
        instance.follow_binding_params();
        assert_eq!(instance.binding.get(), None);
    }

    #[test]
    fn the_studio_set_plugin_has_no_binding_parameters() {
        let (instance, host) = settled(PluginKind::StudioSet, true);
        assert!(instance.binding_params.list().is_empty());
        instance.follow_binding_params();
        assert_eq!(
            (instance.binding.get(), host.edits()),
            (Some(Binding::ANY), vec![])
        );
        let params = FamilyParams::new(PluginKind::StudioSet);
        assert!(
            params
                .param_map()
                .iter()
                .all(|(id, ..)| !id.starts_with("binding/"))
        );
    }

    #[test]
    fn a_restored_project_keeps_the_parameters_and_the_binding_equal() {
        let keys = integral_core::device_spec::INTEGRA7.sn_drum_keys;
        let target = Binding::of_part(4).with_key(keys.first + 5);
        let saved = FamilyParams::new(PluginKind::SnDrum);
        saved.instance.binding.set(Some(target));
        let fields: BTreeMap<_, _> = saved.serialize_fields();

        // The parameters the project saved disagree with its Binding (it was
        // saved before they existed, or the user edited them in the saved
        // state): the Binding stands and the parameters follow it.
        let reopened = FamilyParams::new(PluginKind::SnDrum);
        let instance = &reopened.instance;
        let host = Arc::new(Host {
            instance: Arc::downgrade(instance),
            echoes: true,
            edits: Mutex::new(Vec::new()),
        });
        instance.bank.set_edits(Some(host.clone()));
        // What the wrapper restores before the fields.
        instance.binding_params.host_changed(0, 11);
        reopened.deserialize_fields(&fields);
        instance.follow_binding_params();
        assert_eq!(instance.binding.get(), Some(target));
        assert!(instance.was_restored());
        assert_eq!(instance.binding_params.value(0), 5);
        assert_eq!(instance.binding_params.value(1), 6);
        instance.follow_binding_params();
        assert_eq!(instance.binding.get(), Some(target));

        // Parameters that already agree are left alone.
        let agreeing = FamilyParams::new(PluginKind::SnDrum);
        let host = Arc::new(Host {
            instance: Arc::downgrade(&agreeing.instance),
            echoes: true,
            edits: Mutex::new(Vec::new()),
        });
        agreeing.instance.bank.set_edits(Some(host.clone()));
        agreeing.instance.binding_params.host_changed(0, 5);
        agreeing.instance.binding_params.host_changed(1, 6);
        agreeing.deserialize_fields(&fields);
        agreeing.instance.follow_binding_params();
        assert_eq!(agreeing.instance.binding.get(), Some(target));
        assert_eq!(host.edits(), []);
    }

    #[test]
    fn a_host_change_of_the_other_half_keeps_the_binding_the_editor_just_set() {
        let (instance, host) = settled(PluginKind::PcmDrum, false);
        let keys = integral_core::device_spec::INTEGRA7.pcm_drum_keys;
        instance
            .bind(Binding::of_part(5).with_key(keys.first))
            .unwrap();
        instance.follow_binding_params();
        // The host has not passed the Part back: it still holds the default.
        assert_eq!(instance.binding_params.value(0), 1);
        assert_eq!(host.edits().len(), 6);

        instance.binding_params.host_changed(1, 4);
        instance.follow_binding_params();
        assert_eq!(
            instance.binding.get(),
            Some(Binding::of_part(5).with_key(keys.first + 3))
        );
    }

    #[test]
    fn a_host_change_of_part_holds_back_the_old_targets_writes_and_the_new_one_gets_them() {
        let (instance, _host) = settled(PluginKind::PcmSynth, true);
        instance.bind(Binding::of_part(2)).unwrap();
        instance.follow_binding_params();
        instance.set_fit(super::super::instance::ToneFit::Fits);
        instance.arm();
        let binding = instance.binding.get().unwrap();
        let i = (0..instance.bank.len())
            .find(|i| instance.bank.live(*i, &binding))
            .expect("a live parameter");
        let (min, max) = {
            let host = &instance.bank.list()[i].0;
            (host.min, host.max)
        };
        let value = if instance.bank.value(i) == max {
            min
        } else {
            max
        };
        let poll = |instance: &Instance| {
            let mut written = Vec::new();
            instance.bank.poll(
                instance.binding.get().filter(|_| !instance.is_inert()),
                instance.is_armed(),
                |_, binding, v| {
                    written.push((binding, v));
                    true
                },
            );
            written
        };

        // The clip changes Part and the parameter at the same point.
        instance.binding_params.host_changed(0, 5);
        instance.bank.host_changed(i, value);
        assert!(!instance.is_armed(), "the old Part must not be written");
        assert_eq!(poll(&instance), []);

        // The tick binds Part 5 and disarms; the read arms again, and the
        // host's edit goes to the new Part.
        instance.follow_binding_params();
        assert!(!instance.is_armed() && !instance.binding_params.rebinding());
        instance.arm();
        assert_eq!(
            poll(&instance),
            [(Binding::of_part(4), value)],
            "the kept edit is written to the new Part"
        );
    }

    #[test]
    fn a_sync_is_queued_once_until_it_runs() {
        let (instance, host) = settled(PluginKind::SnSynth, true);
        let posted = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = Arc::clone(&posted);
        instance.set_poster(super::super::instance::Poster::new(move |_| {
            count.fetch_add(1, Ordering::SeqCst);
        }));
        instance.bind(Binding::of_part(1)).unwrap();
        for _ in 0..5 {
            instance.follow_binding_params();
        }
        assert_eq!(posted.load(Ordering::SeqCst), 1);
        // The main thread runs it; a later change queues another.
        super::super::host::run(&instance, super::super::host::HostTask::SyncBinding);
        assert_eq!(host.edits().len(), 3);
        instance.bind(Binding::of_part(2)).unwrap();
        instance.follow_binding_params();
        assert_eq!(posted.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_sync_the_host_dropped_is_queued_again_after_a_second() {
        let (instance, _host) = settled(PluginKind::SnSynth, true);
        let posted = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = Arc::clone(&posted);
        // A poster whose tasks never run: the host's queue was full.
        instance.set_poster(super::super::instance::Poster::new(move |_| {
            count.fetch_add(1, Ordering::SeqCst);
        }));
        instance.bind(Binding::of_part(1)).unwrap();
        let t0 = Instant::now();
        instance.follow_binding_params_at(t0);
        instance.follow_binding_params_at(t0 + SYNC_RETRY / 2);
        assert_eq!(posted.load(Ordering::SeqCst), 1);
        instance.follow_binding_params_at(t0 + SYNC_RETRY);
        assert_eq!(posted.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn edits_held_back_for_a_rebind_that_never_comes_are_written_after_all() {
        let (instance, _host) = settled(PluginKind::PcmSynth, true);
        instance.bind(Binding::of_part(2)).unwrap();
        instance.follow_binding_params();
        instance.set_fit(super::super::instance::ToneFit::Fits);
        instance.arm();
        let binding = instance.binding.get().unwrap();
        let i = (0..instance.bank.len())
            .find(|i| instance.bank.live(*i, &binding))
            .expect("a live parameter");
        let host = &instance.bank.list()[i].0;
        let value = if instance.bank.value(i) == host.max {
            host.min
        } else {
            host.max
        };
        let poll = || {
            let mut written = Vec::new();
            instance.bank.poll(
                instance.binding.get(),
                instance.is_armed(),
                |_, binding, v| {
                    written.push((binding, v));
                    true
                },
            );
            written
        };

        // Part 3, then back to 2, within one tick: the Binding is unchanged.
        instance.binding_params.host_changed(0, 4);
        instance.binding_params.host_changed(0, 3);
        instance.bank.host_changed(i, value);
        assert_eq!(poll(), []);
        instance.follow_binding_params();
        assert_eq!(instance.binding.get(), Some(Binding::of_part(2)));
        assert!(instance.is_armed());
        assert_eq!(poll(), [(Binding::of_part(2), value)]);
    }

    #[test]
    fn an_edit_landing_as_the_hold_ends_keeps_it() {
        let instance = Instance::new(PluginKind::SnSynth);
        instance.binding_params.host_changed(0, 4);
        assert!(!instance.binding_params.bound());
        assert!(instance.binding_params.rebinding());
        instance.binding_params.discard_host_edit();
        assert!(instance.binding_params.bound());
        assert!(!instance.binding_params.rebinding());
    }
}
