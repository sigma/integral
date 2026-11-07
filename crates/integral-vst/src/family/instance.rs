//! What one plugin instance shares between the plugin, its worker thread and
//! (later) its editor.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use integral_core::host_params::{Binding, PluginKind};
use integral_device_service::protocol::{Bound, ClientId};

use super::binding::BindingCell;
use super::status::StatusCell;

/// The Binding and status of an instance, and whether it has settled.
#[derive(Debug)]
pub struct Instance {
    kind: PluginKind,
    /// Where the instance is bound; `None` until it takes a target.
    pub binding: BindingCell,
    /// How the connection is doing.
    pub status: StatusCell,
    /// The host has finished restoring this instance's state: set by the
    /// first `process()` or by the editor opening, never before.
    settled: AtomicBool,
    /// The host gave this instance a saved state: set by
    /// [`restored`](Self::mark_restored) and never cleared.
    restored: AtomicBool,
    /// The Device service socket the instance's connection uses.
    socket: OnceLock<PathBuf>,
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
            settled: AtomicBool::new(false),
            restored: AtomicBool::new(false),
            socket: OnceLock::new(),
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
}
