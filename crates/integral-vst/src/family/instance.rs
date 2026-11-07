//! What one plugin instance shares between the plugin, its worker thread and
//! (later) its editor.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use integral_core::host_params::{Binding, PluginKind};
use integral_device_service::protocol::{Bound, ClientId};

use super::bank::ParamBank;
use super::binding::BindingCell;
use super::host::{self, HostTask};
use super::status::StatusCell;

/// The Binding and status of an instance, and whether it has settled.
#[derive(Debug)]
pub struct Instance {
    kind: PluginKind,
    /// Where the instance is bound; `None` until it takes a target.
    pub binding: BindingCell,
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
    /// The host has finished restoring this instance's state: set by the
    /// first `process()` or by the editor opening, never before.
    settled: AtomicBool,
    /// The host gave this instance a saved state: set by
    /// [`restored`](Self::mark_restored) and never cleared.
    restored: AtomicBool,
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
        Self {
            kind,
            binding: BindingCell::new(binding),
            status: StatusCell::default(),
            bank: ParamBank::new(kind),
            armed: AtomicBool::new(false),
            replacing: AtomicBool::new(false),
            settled: AtomicBool::new(false),
            restored: AtomicBool::new(false),
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
    /// also settles it.
    pub fn mark_restored(&self) {
        self.restored.store(true, Ordering::Release);
        self.settle();
    }

    /// Whether a saved state was restored into this instance: the recall
    /// (ADR 0009) pushes only then, and a fresh instance adopts the device.
    /// A state without a valid Binding (one the plugin cannot have, or
    /// missing) does not count.
    pub fn was_restored(&self) -> bool {
        self.restored.load(Ordering::Acquire)
    }

    /// Let host changes go to the device: the instance has read what the
    /// device holds into its parameters, and what the host held before is
    /// not a change to send (except what it edited while writing was off,
    /// see [`ParamBank::arm`]).
    pub fn arm(&self) {
        self.bank.arm();
        self.armed.store(true, Ordering::Release);
    }

    /// Stop host changes going to the device, until the next
    /// [`arm`](Self::arm): its values are out of date (a reconnect, a bulk
    /// change) and the instance is reading them again. Wait-free.
    pub fn disarm(&self) {
        self.armed.store(false, Ordering::Release);
    }

    /// Whether host changes go to the device. Wait-free.
    pub fn is_armed(&self) -> bool {
        self.armed.load(Ordering::Acquire)
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
