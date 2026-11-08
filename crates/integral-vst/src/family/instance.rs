//! What one plugin instance shares between the plugin, its worker thread and
//! (later) its editor.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};

use integral_core::factory_catalog::FactoryTone;
use integral_core::host_params::{Binding, PluginKind};
use integral_core::params::tone_type_from_bank_msb;
use integral_core::tone_banks::default_tone;
use integral_device_service::protocol::{Bound, ClientId};
use thiserror::Error;

use super::bank::ParamBank;
use super::binding::BindingCell;
use super::binding_params::BindingParams;
use super::host::{self, HostTask};
use super::notice::{Notice, NoticeCell};
use super::status::StatusCell;
use super::tone::ToneCell;

/// A Binding the plugin cannot have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{} cannot be bound to part {:?}, key {:?}", kind.name(), binding.part, binding.key)]
pub struct BindRefused {
    kind: PluginKind,
    binding: Binding,
}

/// No request for the default tone.
const DEFAULT_TONE_IDLE: u8 = 0;
/// The editor asked; the worker has not taken the request.
const DEFAULT_TONE_REQUESTED: u8 = 1;
/// The worker selected the tone and waits for the Part to hold it.
const DEFAULT_TONE_SENT: u8 = 2;

/// Why a request to load the plugin's default tone was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DefaultToneRefused {
    /// The instance is not bound to a Part.
    #[error("the instance is not bound to a Part")]
    Unbound,
    /// The Part holds a tone of the plugin's type, or the instance has not
    /// found what it holds: there is no mismatch to repair.
    #[error("the Part is not known to hold another tone type")]
    NoMismatch,
    /// The default tone was asked for already and is being loaded.
    #[error("the default tone is being loaded")]
    Pending,
}

/// Whether the Part an instance is bound to holds a tone of the plugin's
/// type (ADR 0007). Anything but [`Fits`](Self::Fits) leaves the instance
/// inert: it writes nothing and takes no change from the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ToneFit {
    /// Not known: the instance has not asked the device yet, or the Part's
    /// tone may have just changed (a bulk event, a new connection or a new
    /// Binding). The instance waits for the answer. Where every Tone and Kit
    /// plugin starts.
    Unknown = 0,
    /// The Part holds a tone of the plugin's type (always, for the Studio Set
    /// plugin, which acts on any).
    Fits = 1,
    /// The Part holds another type: [`Notice::TypeMismatch`] is reported and
    /// the instance stays inert until the Part holds the plugin's type
    /// again. Its automation lanes stay.
    Mismatch = 2,
}

impl ToneFit {
    /// The fit of a plugin of `kind` on a Part whose tone is selected with
    /// Bank Select MSB `msb`. A bank that is no tone bank at all is another
    /// type.
    pub fn of(kind: PluginKind, msb: u8) -> Self {
        match kind.tone_type() {
            None => Self::Fits,
            Some(wanted) if tone_type_from_bank_msb(msb) == Some(wanted) => Self::Fits,
            Some(_) => Self::Mismatch,
        }
    }

    fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Fits,
            2 => Self::Mismatch,
            _ => Self::Unknown,
        }
    }
}

/// The Binding and status of an instance, and whether it has settled.
#[derive(Debug)]
pub struct Instance {
    kind: PluginKind,
    /// Where the instance is bound; `None` until it takes a target.
    pub binding: BindingCell,
    /// The Binding as host parameters: a Part, and a key on a Kit plugin.
    pub binding_params: BindingParams,
    /// How the connection is doing.
    pub status: StatusCell,
    /// The host parameters and what is known of their values.
    pub bank: ParamBank,
    /// The instance has learned what the device holds and may write host
    /// changes to it: see [`arm`](Self::arm).
    armed: AtomicBool,
    /// The bank's values are being replaced by the device's side, so a state
    /// restore it causes is no restore from the host (see
    /// [`replacing`](Self::replacing)).
    replacing: AtomicBool,
    /// What else the editor should show.
    pub notices: NoticeCell,
    /// Whether the bound Part holds the plugin's tone type ([`ToneFit`]).
    fit: AtomicU8,
    /// A recall of the saved tone is owed or under way: the push selects it, so
    /// the default tone is not offered meanwhile.
    recall_owed: AtomicBool,
    /// Where a request for the plugin's default tone has got to
    /// ([`DEFAULT_TONE_IDLE`] and the following).
    default_tone: AtomicU8,
    /// A request for the default tone was dropped unserved since the editor
    /// last heard.
    default_tone_dropped: AtomicBool,
    /// The tone selection saved with the project: what the instance last
    /// learned the device holds, and what the recall selects (ADR 0009). A
    /// Studio Set instance has none.
    pub tone: ToneCell,
    /// The host has finished restoring this instance's state: set by the
    /// first `process()` or by the editor opening, never before.
    settled: AtomicBool,
    /// How many times the host gave this instance a saved state (a project
    /// load, and again for a preset load, a revert or a hot swap): counted by
    /// [`mark_restored`](Self::mark_restored) and never reset.
    restores: AtomicU64,
    /// The Device service socket the instance's connection uses.
    socket: OnceLock<PathBuf>,
    /// How the worker has the host's main thread run a task: nih-plug's
    /// event loop, set once ([`set_poster`](Self::set_poster)).
    poster: OnceLock<Poster>,
    /// Posting is on: off from `deactivate()` to the next `initialize()`.
    posting: AtomicBool,
    /// The editor's client (`ClientId` + 1; 0 when it has none).
    editor_client: AtomicU64,
}

impl Instance {
    /// A new instance of `kind`. The Studio Set plugin starts bound, to its
    /// one target; the others wait to take one.
    pub fn new(kind: PluginKind) -> Self {
        let binding = (kind == PluginKind::StudioSet).then_some(Binding::ANY);
        // The Studio Set plugin acts on any tone.
        let fit = if kind.tone_type().is_none() {
            ToneFit::Fits
        } else {
            ToneFit::Unknown
        };
        Self {
            kind,
            binding: BindingCell::new(binding),
            binding_params: BindingParams::new(kind),
            status: StatusCell::default(),
            bank: ParamBank::new(kind),
            armed: AtomicBool::new(false),
            replacing: AtomicBool::new(false),
            notices: NoticeCell::default(),
            fit: AtomicU8::new(fit as u8),
            recall_owed: AtomicBool::new(false),
            default_tone: AtomicU8::new(DEFAULT_TONE_IDLE),
            default_tone_dropped: AtomicBool::new(false),
            tone: ToneCell::default(),
            settled: AtomicBool::new(false),
            restores: AtomicU64::new(0),
            socket: OnceLock::new(),
            poster: OnceLock::new(),
            posting: AtomicBool::new(true),
            editor_client: AtomicU64::new(0),
        }
    }

    /// Note the Device service socket the instance connects to. The first
    /// one stays: it is the single source for the connection and the
    /// editor's link.
    pub fn set_socket(&self, path: PathBuf) {
        let _ = self.socket.set(path);
    }

    /// The Device service socket the instance connects to, once its
    /// connection has started.
    pub fn socket(&self) -> Option<&Path> {
        self.socket.get().map(PathBuf::as_path)
    }

    /// Publish the ID of the editor's own client of the service (`None`: it
    /// has none, because the editor is closed or its connection is down).
    /// Wait-free.
    pub fn set_editor_client(&self, client: Option<ClientId>) {
        let stored = client.map_or(0, |id| id.0.saturating_add(1));
        self.editor_client.store(stored, Ordering::Release);
    }

    /// The ID of the editor's own client of the service, while it is
    /// connected.
    ///
    /// The editor talks to the service through a second client (see
    /// `webview::service_link`), so a change the service reports with that
    /// client as its `writer` is the instance's own: treat writes from either
    /// this instance's client or its editor's client as its own, not as
    /// another writer's. Wait-free.
    pub fn editor_client(&self) -> Option<ClientId> {
        match self.editor_client.load(Ordering::Acquire) {
            0 => None,
            stored => Some(ClientId(stored - 1)),
        }
    }

    /// The plugin kind.
    pub fn kind(&self) -> PluginKind {
        self.kind
    }

    /// Bind the instance to `binding`, the editor's choice. The service is
    /// told within a worker tick, and the Binding is saved with the project.
    ///
    /// # Errors
    ///
    /// [`BindRefused`] if the plugin cannot have `binding`
    /// ([`PluginKind::accepts`]); the Binding is left as it was.
    pub fn bind(&self, binding: Binding) -> Result<(), BindRefused> {
        if !self.kind.accepts(&binding) {
            return Err(BindRefused {
                kind: self.kind,
                binding,
            });
        }
        self.binding.set(Some(binding));
        Ok(())
    }

    /// Reconcile the Binding with its host parameters, once per worker tick.
    /// A host change of a parameter binds the instance, as the editor's bind
    /// does (the tick that follows announces it to the service, which checks
    /// the duplicate, and the follower reads the new target). Otherwise a
    /// Binding the parameters do not hold yet (the editor's, a claim's, a
    /// restore's) is set on them as a gesture, on the host's main thread.
    ///
    /// A host change before the host has had its say ([`settle`](Self::settle))
    /// is the project being restored or initialised, not an edit: the Binding
    /// stands.
    pub fn follow_binding_params(&self) {
        self.follow_binding_params_at(std::time::Instant::now());
    }

    /// [`follow_binding_params`](Self::follow_binding_params) at the time `now`.
    pub fn follow_binding_params_at(&self, now: std::time::Instant) {
        let params = &self.binding_params;
        if !self.is_settled() {
            params.discard_host_edit();
            return;
        }
        let held_back = params.rebinding();
        let mut rebound = false;
        if let Some(edit) = params.take_host_edit(self.binding.get())
            && self.binding.get() != Some(edit)
            // The host's ranges are the plugin's: a refusal is not possible.
            && self.bind(edit).is_ok()
        {
            // Host edits made for the old target stay dirty until the read
            // of the new one re-arms the instance: the host wins there.
            self.disarm();
            rebound = true;
        }
        if params.bound() && held_back && !rebound {
            // Nothing rebinds (the same target again, a refusal), so no read
            // will re-arm: have the next poll write what was held back.
            self.bank.nudge();
        }
        // An edit that came in meanwhile is bound by the next tick, not undone.
        if params.edited() || self.bank.edits().is_none() {
            return;
        }
        if let Some(binding) = self.binding.get()
            && !params.differing(&binding).is_empty()
            && params.try_queue_sync(now)
            && !self.post(HostTask::SyncBinding)
        {
            params.sync_done();
        }
    }

    /// What the service should know this instance as: its plugin and, once
    /// it has one, its Binding.
    pub fn bound(&self) -> Option<Bound> {
        self.binding.get().map(|binding| Bound {
            plugin: self.kind,
            binding,
        })
    }

    /// Note that the host has had its say about this instance's state. It
    /// does not take a target (which a restore would replace) before this.
    /// Wait-free.
    ///
    /// Called when a state is restored, from the first `process()`, and when
    /// the editor window is created (`Editor::spawn`). Not from
    /// `Plugin::editor()`: nih-plug calls that while it constructs the
    /// wrapper, before any `setState`.
    pub fn settle(&self) {
        self.settled.store(true, Ordering::Release);
    }

    /// Whether [`settle`](Self::settle) has been called. This only says the
    /// host has had its say, not that it gave a state: the recall (#458) keys
    /// on [`was_restored`](Self::was_restored), never on this.
    pub fn is_settled(&self) -> bool {
        self.settled.load(Ordering::Acquire)
    }

    /// Note that the host restored a saved state into this instance, which
    /// also settles it. Each restore counts: a state restored in the middle
    /// of a device session is owed a push again.
    pub fn mark_restored(&self) {
        self.restores.fetch_add(1, Ordering::AcqRel);
        self.settle();
    }

    /// Whether a saved state was restored into this instance: the recall
    /// (ADR 0009) pushes only then, or once the instance has adopted the
    /// device's state, and a fresh instance adopts the device. A state without
    /// a valid Binding (one the plugin cannot have, or missing) does not
    /// count.
    pub fn was_restored(&self) -> bool {
        self.restores() > 0
    }

    /// How many saved states the host restored into this instance.
    pub fn restores(&self) -> u64 {
        self.restores.load(Ordering::Acquire)
    }

    /// Let host changes go to the device: the instance has read what the
    /// device holds into its parameters, and what the host held before is
    /// not a change to send (except what it edited while writing was off,
    /// see [`ParamBank::arm`]).
    ///
    /// Only an instance whose Part holds the plugin's type ([`ToneFit::Fits`])
    /// can be armed: elsewhere this does nothing.
    pub fn arm(&self) {
        if self.fit() != ToneFit::Fits {
            return;
        }
        self.bank.arm();
        self.armed.store(true, Ordering::Release);
    }

    /// Say whether the bound Part holds the plugin's tone type. Anything but
    /// [`ToneFit::Fits`] stops host changes going to the device (as
    /// [`disarm`](Self::disarm)), and [`ToneFit::Mismatch`] reports
    /// [`Notice::TypeMismatch`]. The follower sets it; moving to
    /// [`ToneFit::Fits`] does not arm the instance, which the read that
    /// follows does. Wait-free.
    pub fn set_fit(&self, fit: ToneFit) {
        self.fit.store(fit as u8, Ordering::Release);
        self.notices
            .set(Notice::TypeMismatch, fit == ToneFit::Mismatch);
        if fit != ToneFit::Fits {
            self.disarm();
        }
        if fit == ToneFit::Mismatch {
            // What the host edited before is for a tone that is gone: the
            // device's values win when the type fits again.
            self.bank.forget_edits();
        }
        if fit == ToneFit::Fits {
            self.clear_default_tone();
        }
    }

    /// The instance's own request selected a tone with Bank Select MSB `msb`
    /// on its Part (the recall's `ChangeTone`, the editor's default tone):
    /// the Part's type is that, with no need to ask the device. Does not arm.
    pub fn selected(&self, msb: u8) {
        self.set_fit(ToneFit::of(self.kind, msb));
    }

    /// Whether the bound Part holds the plugin's tone type. Wait-free.
    pub fn fit(&self) -> ToneFit {
        ToneFit::from_u8(self.fit.load(Ordering::Acquire))
    }

    /// Whether the instance is known to be on a Part of another tone type,
    /// and so writes nothing. Wait-free.
    pub fn is_inert(&self) -> bool {
        self.fit() == ToneFit::Mismatch
    }

    /// The tone the plugin loads onto a Part that holds another type (see
    /// [`default_tone`]); `None` for the Studio Set plugin.
    ///
    /// Not offered while a recall of the project's saved tone is owed or
    /// under way ([`set_recall_owed`](Self::set_recall_owed)): that selection
    /// is what puts the Part right.
    pub fn default_tone(&self) -> Option<&'static FactoryTone> {
        if self.recall_owed.load(Ordering::Acquire) {
            return None;
        }
        default_tone(self.kind.tone_type()?)
    }

    /// Say whether a recall of the saved tone is owed or under way. The
    /// worker sets it.
    pub fn set_recall_owed(&self, owed: bool) {
        self.recall_owed.store(owed, Ordering::Release);
    }

    /// Ask for the plugin's default tone to be loaded onto the bound Part,
    /// which the worker does (the editor has no connection of the instance's
    /// own). Until the Part holds the tone, or the request is dropped, further
    /// requests are refused.
    ///
    /// # Errors
    ///
    /// [`DefaultToneRefused`] if the instance is not bound or is not known to
    /// be on another tone type, or a request is pending.
    pub fn request_default_tone(&self) -> Result<(), DefaultToneRefused> {
        if self.binding.get().is_none() {
            return Err(DefaultToneRefused::Unbound);
        }
        if !self.is_inert() || self.default_tone().is_none() {
            return Err(DefaultToneRefused::NoMismatch);
        }
        self.default_tone
            .compare_exchange(
                DEFAULT_TONE_IDLE,
                DEFAULT_TONE_REQUESTED,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(drop)
            .map_err(|_| DefaultToneRefused::Pending)
    }

    /// Whether the default tone was requested and not yet taken; taking it
    /// is the worker's, which then selects the tone or
    /// [drops](Self::drop_default_tone) the request.
    pub fn take_default_tone_request(&self) -> bool {
        self.default_tone
            .compare_exchange(
                DEFAULT_TONE_REQUESTED,
                DEFAULT_TONE_SENT,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    /// Whether a request for the default tone is pending: made, and the Part
    /// does not hold the tone yet. The editor disables its button meanwhile.
    pub fn default_tone_pending(&self) -> bool {
        self.default_tone.load(Ordering::Acquire) != DEFAULT_TONE_IDLE
    }

    /// The worker could not serve the request (no connection, no longer on
    /// another type): it is over, and the editor is told
    /// ([`take_default_tone_dropped`](Self::take_default_tone_dropped)).
    pub fn drop_default_tone(&self) {
        self.default_tone_dropped.store(true, Ordering::Release);
        self.clear_default_tone();
    }

    /// The request is over without the editor needing to hear of it: the Part
    /// holds the plugin's type, or what it was about is gone (a connection, a
    /// Binding).
    pub fn clear_default_tone(&self) {
        self.default_tone
            .store(DEFAULT_TONE_IDLE, Ordering::Release);
    }

    /// Whether a request was dropped since the last call.
    pub fn take_default_tone_dropped(&self) -> bool {
        self.default_tone_dropped.swap(false, Ordering::AcqRel)
    }

    /// Stop host changes going to the device, until the next
    /// [`arm`](Self::arm): its values are out of date (a reconnect, a bulk
    /// change) and the instance is reading them again. Wait-free.
    pub fn disarm(&self) {
        self.armed.store(false, Ordering::Release);
    }

    /// Whether host changes go to the device: armed, on a Part of the
    /// plugin's type, and with no host change of the Binding waiting to be
    /// bound (it would write to the old target). Wait-free.
    pub fn is_armed(&self) -> bool {
        self.armed.load(Ordering::Acquire)
            && self.fit() == ToneFit::Fits
            && !self.binding_params.rebinding()
    }

    /// Mark that the values are being replaced from the device's side (or
    /// that it is done). The wrapper restores such a state through
    /// `deserialize_fields`, which must not take it for a restore by the
    /// host.
    pub fn set_replacing(&self, replacing: bool) {
        self.replacing.store(replacing, Ordering::Release);
    }

    /// Whether the values are being replaced from the device's side.
    pub fn replacing(&self) -> bool {
        self.replacing.load(Ordering::Acquire)
    }

    /// Have tasks run by `poster`, which hands them to the host's main thread
    /// ([`host::run`]). The first poster stays: a second call (the host
    /// initializes the plugin again for every bulk update) changes nothing.
    /// Returns whether this one was taken.
    pub fn set_poster(&self, poster: Poster) -> bool {
        self.poster.set(poster).is_ok()
    }

    /// Whether a poster was set.
    pub fn has_poster(&self) -> bool {
        self.poster.get().is_some()
    }

    /// Stop (or start again) posting: the host deactivated the plugin, which
    /// it may be about to destroy, and the worker must not post to a main thread
    /// that is tearing it down. Refused tasks are retried by the follower
    /// (gestures end, replaces repeat) once posting is back.
    pub fn set_posting(&self, on: bool) {
        self.posting.store(on, Ordering::Release);
    }

    /// Have `task` carried out on the host's main thread, in the order posted.
    /// With no poster (a test, or a plugin outside a host) it is carried out
    /// here; with one it never is, because the poster only queues, so a task
    /// cannot run inline on a thread that is waiting for the one that posts it
    /// (a bulk update waits for the audio thread). Returns whether it was
    /// taken.
    pub fn post(&self, task: HostTask) -> bool {
        if !self.posting.load(Ordering::Acquire) {
            return false;
        }
        match self.poster.get() {
            Some(poster) => {
                (poster.0)(task);
                true
            }
            None => {
                host::run(self, task);
                true
            }
        }
    }
}

/// Queues a [`HostTask`] for the host's main thread.
pub struct Poster(Arc<dyn Fn(HostTask) + Send + Sync>);

impl Poster {
    /// A poster that calls `queue` for each task.
    pub fn new(queue: impl Fn(HostTask) + Send + Sync + 'static) -> Self {
        Self(Arc::new(queue))
    }
}

impl std::fmt::Debug for Poster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Poster")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fit_follows_the_banks_tone_type() {
        let sn_synth = default_tone(integral_core::params::ToneType::SnSynth).unwrap();
        let pcm_synth = default_tone(integral_core::params::ToneType::PcmSynth).unwrap();
        assert_eq!(
            ToneFit::of(PluginKind::SnSynth, sn_synth.msb),
            ToneFit::Fits
        );
        assert_eq!(
            ToneFit::of(PluginKind::SnSynth, pcm_synth.msb),
            ToneFit::Mismatch
        );
        // Not a tone bank at all.
        assert_eq!(ToneFit::of(PluginKind::SnSynth, 0), ToneFit::Mismatch);
        assert_eq!(ToneFit::of(PluginKind::StudioSet, 0), ToneFit::Fits);
    }

    #[test]
    fn a_mismatch_reports_its_notice_and_stops_writing_and_a_fit_clears_it() {
        let instance = Instance::new(PluginKind::PcmSynth);
        assert_eq!(
            instance.fit(),
            ToneFit::Unknown,
            "a Tone plugin starts unsure"
        );
        instance.set_fit(ToneFit::Fits);
        instance.arm();
        assert!(instance.is_armed());
        assert_eq!(instance.notices.get(), []);

        instance.set_fit(ToneFit::Mismatch);
        assert!(instance.is_inert());
        assert!(!instance.is_armed());
        assert_eq!(instance.notices.get(), [Notice::TypeMismatch]);

        // Fitting again clears the notice but arms nothing: the read does.
        instance.set_fit(ToneFit::Fits);
        assert_eq!(instance.notices.get(), []);
        assert!(!instance.is_armed());

        // Unsure again (a bulk event) stops writing without a notice.
        instance.arm();
        instance.set_fit(ToneFit::Unknown);
        assert!(!instance.is_armed());
        assert_eq!(instance.notices.get(), []);
    }

    #[test]
    fn only_an_instance_on_a_part_of_its_type_can_be_armed() {
        let instance = Instance::new(PluginKind::PcmSynth);
        instance.arm();
        assert!(!instance.is_armed(), "unsure");
        instance.set_fit(ToneFit::Mismatch);
        instance.arm();
        assert!(!instance.is_armed(), "another type");
        instance.set_fit(ToneFit::Fits);
        assert!(!instance.is_armed(), "arming is the read's");
        instance.arm();
        assert!(instance.is_armed());
        // The fit going is what disarms, whoever armed.
        instance.set_fit(ToneFit::Unknown);
        assert!(!instance.is_armed());
    }

    #[test]
    fn an_edit_made_while_unsure_is_dropped_by_a_mismatch_and_not_written_when_it_fits_again() {
        let instance = Instance::new(PluginKind::PcmSynth);
        instance.bind(Binding::of_part(0)).unwrap();
        let writes = std::cell::RefCell::new(Vec::new());
        let poll = |instance: &Instance| {
            instance.bank.poll(
                instance.binding.get().filter(|_| !instance.is_inert()),
                instance.is_armed(),
                |handle, _, value| {
                    writes.borrow_mut().push((handle.index, value));
                    true
                },
            );
        };
        // Writing has been on once, so edits made while it is off are kept.
        instance.set_fit(ToneFit::Fits);
        instance.arm();
        instance.set_fit(ToneFit::Unknown);

        let i = instance
            .bank
            .position("pcms/partial1/0x0F")
            .expect("a parameter");
        let host = instance.bank.list()[i].0.clone();
        let away = if host.default == host.max {
            host.min
        } else {
            host.max
        };
        instance.bank.host_changed(i, away);
        poll(&instance);
        assert!(instance.bank.is_dirty(i), "kept for when it is back on");

        instance.set_fit(ToneFit::Mismatch);
        assert!(!instance.bank.is_dirty(i), "it was for a tone that is gone");
        instance.set_fit(ToneFit::Fits);
        instance.arm();
        poll(&instance);
        assert_eq!(*writes.borrow(), [], "nothing stale is written");
    }

    #[test]
    fn the_studio_set_plugin_always_fits() {
        assert_eq!(Instance::new(PluginKind::StudioSet).fit(), ToneFit::Fits);
    }

    #[test]
    fn the_default_tone_is_asked_for_only_while_bound_and_inert_and_taken_once() {
        let instance = Instance::new(PluginKind::SnDrum);
        assert_eq!(
            instance.request_default_tone(),
            Err(DefaultToneRefused::Unbound)
        );
        instance.bind(Binding::of_part(2)).unwrap();
        assert_eq!(
            instance.request_default_tone(),
            Err(DefaultToneRefused::NoMismatch)
        );
        instance.set_fit(ToneFit::Mismatch);
        assert_eq!(instance.request_default_tone(), Ok(()));
        assert!(instance.take_default_tone_request());
        assert!(!instance.take_default_tone_request());
        let tone = instance.default_tone().expect("a default tone");
        assert_eq!(
            tone_type_from_bank_msb(tone.msb),
            PluginKind::SnDrum.tone_type()
        );
        assert!(
            Instance::new(PluginKind::StudioSet)
                .default_tone()
                .is_none()
        );
    }

    #[test]
    fn binding_takes_a_target_the_kind_accepts() {
        let instance = Instance::new(PluginKind::PcmDrum);
        assert_eq!(instance.binding.get(), None);
        instance.bind(Binding::of_part(3)).unwrap();
        assert_eq!(instance.binding.get(), Some(Binding::of_part(3)));
        instance.bind(Binding::of_part(4).with_key(36)).unwrap();
        assert_eq!(
            instance.binding.get(),
            Some(Binding::of_part(4).with_key(36))
        );
        assert_eq!(
            instance.bound().map(|b| b.binding),
            Some(Binding::of_part(4).with_key(36))
        );
    }

    #[test]
    fn binding_refuses_what_the_kind_cannot_have_and_keeps_the_old() {
        let instance = Instance::new(PluginKind::SnSynth);
        instance.bind(Binding::of_part(1)).unwrap();
        for refused in [
            Binding::ANY,
            Binding::of_part(16),
            Binding::of_part(2).with_key(36),
        ] {
            assert!(instance.bind(refused).is_err(), "{refused:?}");
            assert_eq!(instance.binding.get(), Some(Binding::of_part(1)));
        }
        let studio_set = Instance::new(PluginKind::StudioSet);
        assert!(studio_set.bind(Binding::of_part(0)).is_err());
        assert!(studio_set.bind(Binding::ANY).is_ok());
    }
}
