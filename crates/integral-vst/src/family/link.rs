//! An instance's connection to the Device service: a client, a worker
//! thread that reads its events, and the [`Tracker`] that decides what they
//! mean.
//!
//! The audio thread is not involved: it only calls [`Instance::settle`]. The
//! worker polls every [`TICK`], so a Binding set elsewhere (a restored
//! state, a picker) is announced to the service within one tick.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use integral_core::host_params::PluginKind;
use integral_device_service::protocol::{Bound, Message, Reject};
use integral_device_service::{Client, ClientEvent, Events};

use super::instance::Instance;
use super::status::Status;

/// How long the worker waits for an event before it looks at the instance.
const TICK: Duration = Duration::from_millis(50);

/// How long a refusal still counts as the answer to a `Bind`: the service
/// does not acknowledge one, so after this, or once a `Claim` has been
/// answered (proving the service knows the requests), a refusal is not it.
const BIND_ACK_WITHIN: Duration = Duration::from_secs(2);

/// What the worker should ask of the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Tell the service what this instance is bound to.
    Bind(Option<Bound>),
    /// Take the first free target of the plugin from the service.
    Claim(PluginKind),
}

/// The rules for an instance's connection, without any I/O: events and
/// ticks in, [`Action`]s out, [`Status`] and Binding updated on the
/// [`Instance`].
///
/// A service older than `Bind` and `Claim` answers them
/// [`Reject::Malformed`], and would otherwise leave the instance waiting for
/// a target for ever. So a malformed refusal that names no parameter, and
/// arrives while a `Claim` is unanswered or within [`BIND_ACK_WITHIN`] of a
/// `Bind`, makes the status [`Status::ServiceIncompatible`] until the next
/// connection. (A newer service's refusal of a kind we do not know is
/// [`Reject::Unknown`], which says nothing about the service being old.)
#[derive(Debug)]
pub struct Tracker {
    instance: Arc<Instance>,
    /// Welcomed by a service, and not disconnected since.
    connected: bool,
    /// What the service was last told, if anything. Forgotten on every
    /// connection and after a lost event, and whenever a claim is answered
    /// (the service has bound the client to its answer, which may not be
    /// what the instance holds).
    announced: Option<Bound>,
    /// Asked for a target on this connection already: once is enough, and
    /// an answer of "none free" stays an answer until it reconnects.
    asked: bool,
    /// A `Claim` has not been answered.
    claim_pending: bool,
    /// When the last `Bind` was sent on this connection, until it is known
    /// to have been understood.
    bind_sent: Option<Instant>,
    /// The service refused `Bind` or `Claim` as malformed.
    incompatible: bool,
}

impl Tracker {
    /// A tracker for `instance`, which starts as [`Status::ServiceNotRunning`].
    pub fn new(instance: Arc<Instance>) -> Self {
        Self {
            instance,
            connected: false,
            announced: None,
            asked: false,
            claim_pending: false,
            bind_sent: None,
            incompatible: false,
        }
    }

    /// Take the next event, if there was one, and say what to do about it
    /// and about the instance's own changes since the last step.
    pub fn step(&mut self, event: Option<&ClientEvent>) -> Vec<Action> {
        self.step_at(event, Instant::now())
    }

    /// [`step`](Self::step) at the time `now`.
    pub fn step_at(&mut self, event: Option<&ClientEvent>, now: Instant) -> Vec<Action> {
        if let Some(event) = event {
            self.on_event(event, now);
        }
        let mut actions = Vec::new();

        let wanted = self.instance.bound();
        if self.announced != wanted {
            self.announced = wanted;
            self.bind_sent = Some(now);
            actions.push(Action::Bind(wanted));
        }

        // A new instance takes a target only once the host has had its say
        // (a restore would replace the target it took).
        if self.connected
            && !self.incompatible
            && !self.asked
            && self.instance.is_settled()
            && self.instance.binding.get().is_none()
        {
            self.asked = true;
            self.claim_pending = true;
            actions.push(Action::Claim(self.instance.kind()));
        }
        actions
    }

    fn on_event(&mut self, event: &ClientEvent, now: Instant) {
        let status = &self.instance.status;
        match event {
            ClientEvent::Connected { session, .. } => {
                self.connected = true;
                self.incompatible = false;
                self.asked = false;
                self.claim_pending = false;
                self.bind_sent = None;
                self.announced = None;
                status.set(if session.is_some() {
                    Status::Connected
                } else {
                    Status::DeviceOffline
                });
            }
            ClientEvent::Disconnected => {
                self.connected = false;
                self.asked = false;
                self.claim_pending = false;
                status.set(Status::ServiceNotRunning);
            }
            ClientEvent::Incompatible { .. } => {
                self.connected = false;
                status.set(Status::ServiceIncompatible);
            }
            ClientEvent::Overflowed => {
                // Events were dropped: say what we are again, and if the
                // answer to a claim may have been among them, claim again.
                self.announced = None;
                if self.claim_pending {
                    self.claim_pending = false;
                    self.asked = false;
                }
            }
            ClientEvent::Message(Message::LinkUp { .. }) if !self.incompatible => {
                status.set(Status::Connected);
            }
            ClientEvent::Message(Message::LinkDown) if !self.incompatible => {
                status.set(Status::DeviceOffline);
            }
            ClientEvent::Message(Message::Claimed { plugin, binding })
                if *plugin == self.instance.kind() =>
            {
                self.claim_pending = false;
                // The service knows the requests.
                self.bind_sent = None;
                self.announced = None;
                if let Some(binding) = binding
                    && self.instance.binding.get().is_none()
                {
                    self.instance.binding.set(Some(*binding));
                }
            }
            ClientEvent::Message(Message::Rejected {
                reason: Reject::Malformed { .. },
                param: None,
            }) if self.claim_pending
                || self
                    .bind_sent
                    .is_some_and(|sent| now.saturating_duration_since(sent) < BIND_ACK_WITHIN) =>
            {
                self.incompatible = true;
                self.claim_pending = false;
                self.bind_sent = None;
                status.set(Status::ServiceIncompatible);
            }
            _ => {}
        }
    }
}

/// An instance's client and the worker that serves it. Dropping it stops
/// both (quick, but not for the audio thread).
pub struct Connection {
    // Dropped after the worker has stopped.
    _client: Client,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Connection {
    /// Connect `instance` to the service at `path`. Returns at once; the
    /// client connects, and keeps retrying, in the background.
    pub fn start(path: impl Into<PathBuf>, instance: Arc<Instance>) -> Self {
        let (client, events) = Client::connect(path);
        let stop = Arc::new(AtomicBool::new(false));
        let worker = {
            let (client, stop) = (client.clone(), Arc::clone(&stop));
            // A thread that cannot start leaves the status at "not running".
            thread::Builder::new()
                .name("integral-instance".into())
                .spawn(move || work(&client, &events, instance, &stop))
                .ok()
        };
        Self {
            _client: client,
            stop,
            worker,
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// The worker: feed events to a [`Tracker`] and carry out what it says.
fn work(client: &Client, events: &Events, instance: Arc<Instance>, stop: &AtomicBool) {
    let mut tracker = Tracker::new(instance);
    while !stop.load(Ordering::Acquire) {
        let event = events.wait_event(TICK);
        for action in tracker.step(event.as_ref()) {
            match action {
                Action::Bind(bound) => client.bind(bound),
                Action::Claim(plugin) => client.claim(plugin),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use integral_core::host_params::Binding;
    use integral_device_service::SessionId;
    use integral_device_service::protocol::ClientId;

    fn connected(session: Option<u64>) -> ClientEvent {
        ClientEvent::Connected {
            client: ClientId(1),
            session: session.map(SessionId),
        }
    }

    fn claimed(plugin: PluginKind, part: Option<u8>) -> ClientEvent {
        ClientEvent::Message(Message::Claimed {
            plugin,
            binding: part.map(Binding::of_part),
        })
    }

    fn malformed() -> ClientEvent {
        ClientEvent::Message(Message::Rejected {
            reason: Reject::Malformed {
                message: "unknown variant".into(),
            },
            param: None,
        })
    }

    fn tracker(kind: PluginKind) -> (Tracker, Arc<Instance>) {
        let instance = Arc::new(Instance::new(kind));
        (Tracker::new(Arc::clone(&instance)), instance)
    }

    fn bound(kind: PluginKind, part: u8) -> Bound {
        Bound {
            plugin: kind,
            binding: Binding::of_part(part),
        }
    }

    #[test]
    fn the_status_follows_the_service_and_the_link() {
        let (mut tracker, instance) = tracker(PluginKind::PcmSynth);
        assert_eq!(instance.status.get(), Status::ServiceNotRunning);
        let mut status_after = |event: ClientEvent| {
            tracker.step(Some(&event));
            instance.status.get()
        };
        assert_eq!(status_after(connected(None)), Status::DeviceOffline);
        assert_eq!(
            status_after(ClientEvent::Message(Message::LinkUp {
                session: SessionId(2)
            })),
            Status::Connected
        );
        assert_eq!(
            status_after(ClientEvent::Message(Message::LinkDown)),
            Status::DeviceOffline
        );
        assert_eq!(status_after(connected(Some(3))), Status::Connected);
        assert_eq!(
            status_after(ClientEvent::Disconnected),
            Status::ServiceNotRunning
        );
        assert_eq!(
            status_after(ClientEvent::Incompatible { server: 9 }),
            Status::ServiceIncompatible
        );
    }

    #[test]
    fn a_new_instance_claims_a_target_once_the_host_has_settled() {
        let kind = PluginKind::PcmSynth;
        let (mut tracker, instance) = tracker(kind);
        // Connected but not settled: the host may still restore a Binding.
        assert_eq!(tracker.step(Some(&connected(Some(1)))), []);
        assert_eq!(tracker.step(None), []);
        assert_eq!(instance.binding.get(), None);

        instance.settle();
        assert_eq!(tracker.step(None), [Action::Claim(kind)]);
        // Asked once; the answer binds it, and the service is told again so
        // that the client remembers it across a reconnect.
        assert_eq!(tracker.step(None), []);
        assert_eq!(
            tracker.step(Some(&claimed(kind, Some(4)))),
            [Action::Bind(Some(bound(kind, 4)))]
        );
        assert_eq!(instance.binding.get(), Some(Binding::of_part(4)));
        assert_eq!(tracker.step(None), []);
    }

    #[test]
    fn an_instance_the_host_restored_keeps_its_binding_and_claims_nothing() {
        let kind = PluginKind::SnDrum;
        let (mut tracker, instance) = tracker(kind);
        tracker.step(Some(&connected(Some(1))));
        // The restore lands after `initialize()` and before processing.
        let target = Binding::of_part(7).with_key(36);
        instance.binding.set(Some(target));
        instance.settle();
        assert_eq!(
            tracker.step(None),
            [Action::Bind(Some(Bound {
                plugin: kind,
                binding: target
            }))]
        );
        assert_eq!(tracker.step(None), []);
        assert_eq!(instance.binding.get(), Some(target));
    }

    #[test]
    fn nothing_free_leaves_the_instance_unbound_and_it_claims_again_on_reconnect() {
        let kind = PluginKind::SnSynth;
        let (mut tracker, instance) = tracker(kind);
        instance.settle();
        tracker.step(Some(&connected(Some(1))));
        tracker.step(Some(&claimed(kind, None)));
        assert_eq!(instance.binding.get(), None);
        assert_eq!(tracker.step(None), [], "no polling for a free target");

        tracker.step(Some(&ClientEvent::Disconnected));
        assert_eq!(
            tracker.step(Some(&connected(Some(2)))),
            [Action::Claim(kind)]
        );
    }

    #[test]
    fn an_answer_for_another_plugin_changes_nothing_and_a_late_one_cannot_replace_a_restore() {
        let kind = PluginKind::SnAcoustic;
        let (mut tracker, instance) = tracker(kind);
        instance.settle();
        tracker.step(Some(&connected(Some(1))));
        tracker.step(Some(&claimed(PluginKind::PcmSynth, Some(2))));
        assert_eq!(instance.binding.get(), None);

        // A restore wins over a claim answered after it; the service, which
        // bound the client to its answer, is told what the instance holds.
        instance.binding.set(Some(Binding::of_part(9)));
        assert_eq!(
            tracker.step(Some(&claimed(kind, Some(2)))),
            [Action::Bind(Some(bound(kind, 9)))]
        );
        assert_eq!(instance.binding.get(), Some(Binding::of_part(9)));
    }

    #[test]
    fn the_studio_set_plugin_announces_its_target_and_never_claims() {
        let (mut tracker, instance) = tracker(PluginKind::StudioSet);
        instance.settle();
        let expected = Action::Bind(Some(Bound {
            plugin: PluginKind::StudioSet,
            binding: Binding::ANY,
        }));
        assert_eq!(tracker.step(Some(&connected(Some(1)))), [expected]);
        assert_eq!(tracker.step(None), []);
    }

    #[test]
    fn a_rebound_instance_tells_the_service() {
        let kind = PluginKind::PcmDrum;
        let (mut tracker, instance) = tracker(kind);
        instance.binding.set(Some(Binding::of_part(1)));
        tracker.step(None);
        instance.binding.set(Some(Binding::of_part(1).with_key(40)));
        assert_eq!(
            tracker.step(None),
            [Action::Bind(Some(Bound {
                plugin: kind,
                binding: Binding::of_part(1).with_key(40)
            }))]
        );
    }

    #[test]
    fn a_connection_says_what_the_instance_is_again() {
        let kind = PluginKind::PcmSynth;
        let (mut tracker, instance) = tracker(kind);
        instance.binding.set(Some(Binding::of_part(2)));
        assert_eq!(tracker.step(None).len(), 1);
        assert_eq!(tracker.step(None), []);
        // The Hello carried it, but an old service would have ignored that:
        // the Bind is what it refuses.
        assert_eq!(
            tracker.step(Some(&connected(Some(1)))),
            [Action::Bind(Some(bound(kind, 2)))]
        );
    }

    #[test]
    fn lost_events_make_the_instance_say_what_it_is_and_claim_again() {
        let kind = PluginKind::PcmSynth;
        let (mut tracker, instance) = tracker(kind);
        instance.settle();
        assert_eq!(
            tracker.step(Some(&connected(Some(1)))),
            [Action::Claim(kind)]
        );
        // The answer may have been among the lost events.
        assert_eq!(
            tracker.step(Some(&ClientEvent::Overflowed)),
            [Action::Claim(kind)]
        );
        tracker.step(Some(&claimed(kind, Some(0))));
        assert_eq!(
            tracker.step(Some(&ClientEvent::Overflowed)),
            [Action::Bind(Some(bound(kind, 0)))]
        );
    }

    #[test]
    fn a_service_that_refuses_claim_as_malformed_is_incompatible() {
        let kind = PluginKind::SnSynth;
        let (mut tracker, instance) = tracker(kind);
        instance.settle();
        tracker.step(Some(&connected(Some(1))));
        tracker.step(Some(&malformed()));
        assert_eq!(instance.status.get(), Status::ServiceIncompatible);
        // The link going up and down does not hide it.
        tracker.step(Some(&ClientEvent::Message(Message::LinkUp {
            session: SessionId(2),
        })));
        assert_eq!(instance.status.get(), Status::ServiceIncompatible);
        // A new connection starts afresh.
        tracker.step(Some(&connected(Some(3))));
        assert_eq!(instance.status.get(), Status::Connected);
    }

    #[test]
    fn a_service_that_refuses_bind_as_malformed_is_incompatible() {
        let (mut tracker, instance) = tracker(PluginKind::StudioSet);
        tracker.step(Some(&connected(Some(1))));
        tracker.step(Some(&malformed()));
        assert_eq!(instance.status.get(), Status::ServiceIncompatible);
    }

    #[test]
    fn only_a_malformed_refusal_without_a_parameter_counts() {
        let kind = PluginKind::SnSynth;
        for refusal in [
            // A refusal of a write names its parameter.
            ClientEvent::Message(Message::Rejected {
                reason: Reject::Malformed {
                    message: "x".into(),
                },
                param: Some(integral_device_service::protocol::ParamId {
                    plugin: kind,
                    id: "sns/common/0x0E".into(),
                }),
            }),
            // A kind of refusal only a newer service sends.
            ClientEvent::Message(Message::Rejected {
                reason: Reject::Unknown,
                param: None,
            }),
            ClientEvent::Message(Message::Rejected {
                reason: Reject::LinkDown,
                param: None,
            }),
        ] {
            let (mut tracker, instance) = tracker(kind);
            instance.settle();
            tracker.step(Some(&connected(Some(1))));
            tracker.step(Some(&refusal));
            assert_eq!(instance.status.get(), Status::Connected, "{refusal:?}");
        }
    }

    #[test]
    fn a_refusal_long_after_the_bind_or_after_a_claim_was_answered_is_not_its_answer() {
        let t0 = Instant::now();
        let (mut first, instance) = tracker(PluginKind::StudioSet);
        first.step_at(Some(&connected(Some(1))), t0);
        first.step_at(
            Some(&malformed()),
            t0 + BIND_ACK_WITHIN + Duration::from_millis(1),
        );
        assert_eq!(instance.status.get(), Status::Connected);

        let kind = PluginKind::SnSynth;
        let (mut second, instance) = tracker(kind);
        instance.settle();
        second.step_at(Some(&connected(Some(1))), t0);
        // A claim answered (nothing free) proves the service knows the
        // requests.
        second.step_at(Some(&claimed(kind, None)), t0);
        second.step_at(Some(&malformed()), t0);
        assert_eq!(instance.status.get(), Status::Connected);
    }

    #[test]
    fn a_refusal_with_nothing_outstanding_is_not_ours() {
        let (mut tracker, instance) = tracker(PluginKind::SnSynth);
        tracker.step(Some(&connected(Some(1))));
        tracker.step(Some(&malformed()));
        assert_eq!(instance.status.get(), Status::Connected);
    }
}
