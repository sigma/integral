//! The clients the service knows: who is connected, what they subscribed to,
//! and how a message reaches them.
//!
//! Sans-IO like the rest of the service core: a client is an [`Outbox`] the
//! socket layer (or a test) gives to [`Input::Connect`](crate::Input), and
//! everything the service says is a [`Message`] handed to it.

use std::collections::BTreeMap;

use integral_core::host_params::Binding;

use crate::protocol::{BulkKind, ClientId, Message, ParamId};
use crate::session::SessionId;

/// Where the service puts what it says to one client.
///
/// Called on the service thread, so it must not block: a socket's outbox
/// queues the message for a writer thread of its own. When it returns
/// `false` (the client is gone, or too far behind to be served) the service
/// drops the client, which then reconnects and re-reads.
pub trait Outbox: Send {
    /// Take `message` for the client. Returns whether it was accepted.
    fn deliver(&mut self, message: &Message) -> bool;
}

/// One connected client.
struct Client {
    outbox: Box<dyn Outbox>,
    /// Has sent a good `Hello`.
    greeted: bool,
    subscriptions: Vec<Binding>,
    raw: bool,
}

/// Every connected client.
#[derive(Default)]
pub(crate) struct Clients {
    clients: BTreeMap<ClientId, Client>,
}

impl Clients {
    /// A client connected; it must say `Hello` before anything else.
    pub(crate) fn connect(&mut self, id: ClientId, outbox: Box<dyn Outbox>) {
        self.clients.insert(
            id,
            Client {
                outbox,
                greeted: false,
                subscriptions: Vec::new(),
                raw: false,
            },
        );
    }

    /// A client went away.
    pub(crate) fn disconnect(&mut self, id: ClientId) {
        self.clients.remove(&id);
    }

    /// Whether `id` has said `Hello`.
    pub(crate) fn is_greeted(&self, id: ClientId) -> bool {
        self.clients.get(&id).is_some_and(|c| c.greeted)
    }

    /// Mark `id` as greeted.
    pub(crate) fn greet(&mut self, id: ClientId) {
        if let Some(client) = self.clients.get_mut(&id) {
            client.greeted = true;
        }
    }

    /// Add a subscription for `id`, unless it has that scope already.
    pub(crate) fn subscribe(&mut self, id: ClientId, binding: Binding) {
        if let Some(client) = self.clients.get_mut(&id)
            && !client.subscriptions.contains(&binding)
        {
            client.subscriptions.push(binding);
        }
    }

    /// Remove a subscription of `id`.
    pub(crate) fn unsubscribe(&mut self, id: ClientId, binding: &Binding) {
        if let Some(client) = self.clients.get_mut(&id) {
            client.subscriptions.retain(|s| s != binding);
        }
    }

    /// Turn raw SysEx delivery to `id` on or off.
    pub(crate) fn set_raw(&mut self, id: ClientId, on: bool) {
        if let Some(client) = self.clients.get_mut(&id) {
            client.raw = on;
        }
    }

    /// Send `message` to `id`; drop the client if it cannot take it.
    pub(crate) fn send(&mut self, id: ClientId, message: &Message) {
        let Some(client) = self.clients.get_mut(&id) else {
            return;
        };
        if !client.outbox.deliver(message) {
            self.clients.remove(&id);
        }
    }

    /// Send `message` to every greeted client `wants`, dropping any that
    /// cannot take it.
    fn broadcast(&mut self, message: &Message, wants: impl Fn(&Client) -> bool) {
        self.clients.retain(|_, client| {
            !(client.greeted && wants(client)) || client.outbox.deliver(message)
        });
    }

    /// Tell every client the link came up.
    pub(crate) fn link_up(&mut self, session: SessionId) {
        self.broadcast(&Message::LinkUp { session }, |_| true);
    }

    /// Tell every client the link went down.
    pub(crate) fn link_down(&mut self) {
        self.broadcast(&Message::LinkDown, |_| true);
    }

    /// Tell the clients subscribed to `binding` that `param` changed.
    pub(crate) fn changed(
        &mut self,
        writer: ClientId,
        param: ParamId,
        binding: Binding,
        value: i32,
    ) {
        let message = Message::Changed {
            writer,
            param,
            binding,
            value,
        };
        self.broadcast(&message, |c| {
            c.subscriptions.iter().any(|s| s.matches(&binding))
        });
    }

    /// Announce a bulk change. A Studio Set reaches every subscriber, a tone
    /// the ones subscribed to its scope.
    pub(crate) fn bulk(&mut self, writer: ClientId, kind: BulkKind, binding: Binding) {
        let message = Message::Bulk {
            writer,
            kind,
            binding,
        };
        self.broadcast(&message, |c| match kind {
            BulkKind::StudioSet | BulkKind::Chorus | BulkKind::Reverb => {
                !c.subscriptions.is_empty()
            }
            BulkKind::Tone | BulkKind::Mfx => c.subscriptions.iter().any(|s| s.matches(&binding)),
        });
    }

    /// Pass a SysEx message from the device to the raw clients.
    pub(crate) fn sysex(&mut self, bytes: &[u8]) {
        if !self.clients.values().any(|c| c.raw) {
            return;
        }
        let message = Message::Sysex {
            bytes: bytes.to_vec(),
        };
        self.broadcast(&message, |c| c.raw);
    }
}
