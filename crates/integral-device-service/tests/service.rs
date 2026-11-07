//! The service against a fake link that records what it would send.

use std::time::{Duration, Instant};

use std::sync::{Arc, Mutex};

use integral_core::address::{Address, DataSize};
use integral_core::host_params::{Binding, ParamIndex};
use integral_core::params::{self, part};
use integral_core::sysex;
use integral_device_service::protocol::{
    BulkKind, ClientId, Message, PROTOCOL_VERSION, ParamId, Reject, Request,
};
use integral_device_service::{
    Config, FakeLink, Input, LinkEvent, Outbox, SequentialSessionIds, Service, SessionId, spawn,
};

const DEVICE_ID: u8 = 0x10;
const NOTE_ON: [u8; 3] = [0x90, 60, 100];

fn config() -> Config {
    Config {
        device_id: Some(DEVICE_ID),
        session_ids: Box::new(SequentialSessionIds(0)),
    }
}

fn service() -> (Service, FakeLink) {
    let link = FakeLink::new();
    (Service::new(Box::new(link.clone()), config()), link)
}

fn is_sysex(message: &[u8]) -> bool {
    message.first() == Some(&0xF0)
}

#[test]
fn a_note_queued_behind_ten_dt1s_goes_out_before_the_second() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    for part in 0..10 {
        service.device_mut().set_part_level(part, 100 + part);
    }

    service.pump(0.0); // the first DT1 leaves
    service.handle(Input::Note(NOTE_ON.to_vec()), 1.0);
    service.pump(1.0); // inside the throttle window: only the note
    let mut now = 1.0;
    while service.device().has_pending() {
        now += 1.0;
        service.pump(now);
    }

    let sent = link.sent();
    assert_eq!(sent.len(), 11);
    assert!(is_sysex(&sent[0]));
    assert_eq!(sent[1], NOTE_ON);
    assert!(sent[2..].iter().all(|m| is_sysex(m)));
}

#[test]
fn sysex_on_the_note_port_never_reaches_the_link() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let reply = sysex::build_dt1(DEVICE_ID, &params::part_address(0, part::LEVEL), &[42]);
    service.handle(Input::Note(reply), 1.0);
    service.handle(Input::Note(sysex::identity_request().to_vec()), 2.0);
    service.handle(Input::Note(vec![0xF7]), 3.0);
    service.pump(10.0);
    service.pump(100.0);
    assert!(link.sent().is_empty());
}

#[test]
fn realtime_messages_on_the_note_port_reach_the_link() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    for realtime in [0xF8, 0xFA, 0xFB, 0xFC, 0xFE] {
        service.handle(Input::Note(vec![realtime]), 1.0);
    }
    service.pump(1.0);
    let sent = link.sent();
    assert_eq!(
        sent[sent.len() - 5..],
        [[0xF8], [0xFA], [0xFB], [0xFC], [0xFE]]
    );
}

#[test]
fn notes_are_dropped_while_the_link_is_down() {
    let (mut service, link) = service();
    service.handle(Input::Note(NOTE_ON.to_vec()), 0.0);
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 1.0);
    service.handle(Input::Note(NOTE_ON.to_vec()), 2.0);
    service.handle(Input::Link(LinkEvent::Down), 3.0);
    service.pump(4.0);
    assert!(link.sent().is_empty());
}

#[test]
fn sysex_stays_queued_while_the_link_is_down() {
    let (mut service, link) = service();
    service.device_mut().set_part_level(0, 77);
    assert_eq!(service.pump(0.0), None);
    assert!(link.sent().is_empty());
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 1.0);
    service.pump(1.0);
    assert_eq!(link.sent().len(), 1);
}

#[test]
fn the_session_id_changes_on_every_link_up() {
    let (mut service, _link) = service();
    assert_eq!(service.session(), None);
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let first = service.session().expect("up");
    service.handle(Input::Link(LinkEvent::Down), 1.0);
    assert_eq!(service.session(), None);
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 2.0);
    let second = service.session().expect("up again");
    assert_ne!(first, second);
}

#[test]
fn what_the_device_sends_updates_the_cache() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let dt1 = sysex::build_dt1(DEVICE_ID, &params::part_address(2, part::LEVEL), &[33]);
    service.handle(Input::Link(LinkEvent::Midi(dt1)), 1.0);
    assert_eq!(service.device().state().parts[2].level, 33);
}

#[test]
fn run_reaches_the_one_state_from_another_thread() {
    let link = FakeLink::new();
    let service = Service::new(Box::new(link.clone()), config());
    let running = spawn(service, None).expect("start");
    link.bring_up(None);
    let handle = running.handle();
    handle.run(|service| service.device_mut().set_part_level(0, 55));
    wait_for(|| !link.sent().is_empty());
    let expected = sysex::build_dt1(DEVICE_ID, &params::part_address(0, part::LEVEL), &[55]);
    assert_eq!(link.sent(), vec![expected]);
    running.stop();
}

#[test]
fn the_session_id_changes_on_reconnect_while_running() {
    use std::sync::{Arc, Mutex};

    use integral_device_service::{Event, Listener};

    struct Ups(Arc<Mutex<Vec<SessionId>>>);
    impl Listener for Ups {
        fn event(&mut self, event: Event<'_>) {
            if let Event::LinkUp(id) = event {
                self.0.lock().unwrap().push(id);
            }
        }
    }

    let link = FakeLink::new();
    let ups = Arc::new(Mutex::new(Vec::new()));
    let service = Service::new(Box::new(link.clone()), config())
        .with_listener(Box::new(Ups(Arc::clone(&ups))));
    let running = spawn(service, None).expect("start");
    link.bring_up(None);
    link.bring_down();
    link.bring_up(None);
    wait_for(|| ups.lock().unwrap().len() == 2);
    let ids = ups.lock().unwrap().clone();
    assert_ne!(ids[0], ids[1]);
    running.stop();
}

fn wait_for(condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Pump the way the runtime does: when the service says to, with no input.
fn run_until(service: &mut Service, from: f64, to: f64) {
    let mut now = from;
    while now <= to {
        let Some(wait) = service.pump(now) else {
            return;
        };
        now += wait.as_secs_f64() * 1000.0;
    }
}

#[test]
fn a_lost_tracked_read_is_sent_again_without_other_input() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    service.device_mut().load_sns_tone(0);
    let blocks = {
        run_until(&mut service, 0.0, 1_000.0);
        link.sent().len()
    };
    assert!(blocks > 1);
    link.clear();
    // No reply ever comes: the first request is sent again once it times out.
    run_until(&mut service, 1_000.0, 5_000.0);
    assert!(!link.sent().is_empty(), "no retry was driven");
}

#[test]
fn an_expansion_load_is_driven_to_its_timeout() {
    use integral_core::expansion_slots::ExpansionSlots;

    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let current = ExpansionSlots::from_values(&[0x03, 0x05, 0x07, 0x08]).unwrap();
    service.device_mut().set_expansion_slots(current);
    service
        .device_mut()
        .start_expansion_load(current.with(0, None))
        .unwrap();
    link.clear();
    // The load request goes out; no ack ever comes, so the service must wake
    // by itself to time the load out and release the queue.
    run_until(&mut service, 100.0, 120_000.0);
    assert_eq!(link.sent().len(), 1, "only the load request");
    assert_eq!(service.device().expansion_slots(), None, "timed out");
    assert!(service.pump(130_000.0).is_none(), "idle once the load ends");
}

#[test]
fn the_device_id_comes_from_the_link_and_a_fixed_id_wins() {
    let link = FakeLink::new();
    let discovering = Config {
        device_id: None,
        session_ids: Box::new(SequentialSessionIds(0)),
    };
    let mut service = Service::new(Box::new(link.clone()), discovering);
    service.handle(
        Input::Link(LinkEvent::Up {
            device_id: Some(0x12),
        }),
        0.0,
    );
    assert_eq!(service.device().device_id(), 0x12);

    let mut fixed = Service::new(Box::new(FakeLink::new()), config());
    fixed.handle(
        Input::Link(LinkEvent::Up {
            device_id: Some(0x12),
        }),
        0.0,
    );
    assert_eq!(fixed.device().device_id(), DEVICE_ID);
}

const IDENTITY_REPLY_13: [u8; 15] = [
    0xF0, 0x7E, 0x13, 0x06, 0x02, 0x41, 0x64, 0x02, 0, 0, 0, 0, 0, 0, 0xF7,
];

fn discovering() -> (Service, FakeLink) {
    let link = FakeLink::new();
    let config = Config {
        device_id: None,
        session_ids: Box::new(SequentialSessionIds(0)),
    };
    (Service::new(Box::new(link.clone()), config), link)
}

#[test]
fn the_identity_request_leaves_first_and_holds_the_rest() {
    let (mut service, link) = discovering();
    service.device_mut().set_part_level(0, 77); // queued before the link
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    service.pump(0.0);
    assert_eq!(link.sent(), vec![sysex::identity_request().to_vec()]);

    // Waiting for the reply: notes go, SysEx does not.
    service.handle(Input::Note(NOTE_ON.to_vec()), 100.0);
    service.pump(100.0);
    service.pump(900.0);
    assert_eq!(link.sent().len(), 2);
    assert_eq!(link.sent()[1], NOTE_ON);
}

#[test]
fn the_reply_releases_the_queue_no_sooner_than_the_throttle() {
    let (mut service, link) = discovering();
    service.device_mut().set_part_level(0, 77);
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    service.pump(0.0);
    let same_id = [
        0xF0, 0x7E, 0x10, 0x06, 0x02, 0x41, 0x64, 0x02, 0, 0, 0, 0, 0, 0, 0xF7,
    ];
    service.handle(Input::Link(LinkEvent::Midi(same_id.to_vec())), 5.0);
    service.pump(10.0);
    assert_eq!(link.sent().len(), 1, "the request's send time counts");
    service.pump(40.0);
    assert_eq!(link.sent().len(), 2);
    assert_eq!(service.device().device_id(), 0x10);
}

#[test]
fn without_a_reply_the_queue_goes_after_a_second() {
    let (mut service, link) = discovering();
    service.device_mut().set_part_level(0, 77);
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    run_until(&mut service, 0.0, 999.0);
    assert_eq!(link.sent().len(), 1);
    run_until(&mut service, 1_000.0, 1_100.0);
    assert_eq!(link.sent().len(), 2, "the default ID stands");
}

#[test]
fn a_different_id_starts_a_new_session() {
    use std::sync::{Arc, Mutex};

    use integral_device_service::{Event, Listener};

    struct Ups(Arc<Mutex<Vec<SessionId>>>);
    impl Listener for Ups {
        fn event(&mut self, event: Event<'_>) {
            if let Event::LinkUp(id) = event {
                self.0.lock().unwrap().push(id);
            }
        }
    }

    let (service, link) = discovering();
    let ups = Arc::new(Mutex::new(Vec::new()));
    let mut service = service.with_listener(Box::new(Ups(Arc::clone(&ups))));
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    service.pump(0.0);
    let first = service.session().unwrap();
    service.handle(
        Input::Link(LinkEvent::Midi(IDENTITY_REPLY_13.to_vec())),
        5.0,
    );
    assert_eq!(service.device().device_id(), 0x13);
    assert_ne!(service.session().unwrap(), first);
    assert_eq!(ups.lock().unwrap().len(), 2, "LinkUp again");
    // And the state is usable at once with the new ID.
    service.device_mut().set_part_level(0, 1);
    service.pump(50.0);
    let sent = link.sent();
    assert_eq!(sent[1][2], 0x13);
}

#[test]
fn a_link_down_during_an_expansion_load_waits_for_the_load() {
    use integral_core::expansion_slots::ExpansionSlots;

    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let current = ExpansionSlots::from_values(&[0x03, 0x05, 0x07, 0x08]).unwrap();
    service.device_mut().set_expansion_slots(current);
    service
        .device_mut()
        .start_expansion_load(current.with(0, None))
        .unwrap();
    service.pump(100.0); // the load request leaves
    let session = service.session();
    assert!(session.is_some());

    service.handle(Input::Link(LinkEvent::Down), 5_000.0);
    assert_eq!(service.session(), session, "no session change mid-load");
    service.pump(6_000.0);
    assert_eq!(service.session(), session);

    // The load times out; only then does the Down apply.
    run_until(&mut service, 6_000.0, 120_000.0);
    assert_eq!(service.session(), None);
    assert!(!link.sent().is_empty());
}

#[test]
fn a_link_that_bounces_during_a_load_keeps_its_session() {
    use integral_core::expansion_slots::ExpansionSlots;

    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let current = ExpansionSlots::from_values(&[0x03, 0x05, 0x07, 0x08]).unwrap();
    service.device_mut().set_expansion_slots(current);
    service
        .device_mut()
        .start_expansion_load(current.with(0, None))
        .unwrap();
    let session = service.session();
    service.handle(Input::Link(LinkEvent::Down), 1_000.0);
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 2_000.0);
    assert_eq!(service.session(), session);
}

#[test]
fn a_note_after_a_probe_goes_out_without_waiting_for_the_gap() {
    use std::time::Duration;

    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    service.device_mut().set_part_level(0, 77); // SysEx pending
    link.hold_sysex(Duration::from_millis(30)); // the link just probed

    service.handle(Input::Note(NOTE_ON.to_vec()), 1.0);
    let wait = service.pump(1.0).expect("a deadline");
    assert_eq!(
        link.sent(),
        vec![NOTE_ON.to_vec()],
        "the note, not the SysEx"
    );
    assert!(wait >= Duration::from_millis(1) && wait <= Duration::from_millis(30));

    // When the gap is over, the SysEx follows.
    link.hold_sysex(Duration::ZERO);
    service.pump(31.0);
    assert_eq!(link.sent().len(), 2);
    assert!(is_sysex(&link.sent()[1]));
}

/// A client that keeps what the service says to it.
#[derive(Clone, Default)]
struct Inbox(Arc<Mutex<Vec<Message>>>);

impl Outbox for Inbox {
    fn deliver(&mut self, message: &Message) -> bool {
        self.0.lock().unwrap().push(message.clone());
        true
    }
}

impl Inbox {
    fn take(&self) -> Vec<Message> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

fn client(service: &mut Service, id: u64) -> (ClientId, Inbox) {
    let (id, inbox) = (ClientId(id), Inbox::default());
    service.handle(
        Input::Connect {
            client: id,
            outbox: Box::new(inbox.clone()),
        },
        0.0,
    );
    ask(
        service,
        id,
        Request::Hello {
            version: PROTOCOL_VERSION,
            subscriptions: vec![Binding::ANY],
            raw: false,
            bound: None,
        },
    );
    inbox.take();
    (id, inbox)
}

fn ask(service: &mut Service, client: ClientId, request: Request) {
    service.handle(Input::Client { client, request }, 0.0);
}

/// The wire name of the parameter at `address`.
fn param_at(address: &Address) -> ParamId {
    let index = ParamIndex::new();
    let hit = index.locate(address)[0];
    ParamId::of(&index, hit.handle)
}

fn level(part_index: u8) -> ParamId {
    param_at(&params::part_address(part_index, part::LEVEL))
}

fn write_level(part_index: u8, value: i32) -> Request {
    Request::Write {
        param: level(part_index),
        binding: Binding::ANY,
        value,
    }
}

fn level_dt1(part_index: u8, value: u8) -> Vec<u8> {
    sysex::build_dt1(
        DEVICE_ID,
        &params::part_address(part_index, part::LEVEL),
        &[value],
    )
}

/// Changes in what a client was told: (writer, part, value).
fn level_changes(messages: Vec<Message>) -> Vec<(ClientId, ParamId, i32)> {
    messages
        .into_iter()
        .filter_map(|m| match m {
            Message::Changed {
                writer,
                param,
                value,
                ..
            } => Some((writer, param, value)),
            _ => None,
        })
        .collect()
}

#[test]
fn each_clients_writes_keep_their_writer_to_the_moment_they_are_queued() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, a_inbox) = client(&mut service, 1);
    let (b, b_inbox) = client(&mut service, 2);

    ask(&mut service, a, write_level(0, 10));
    ask(&mut service, b, write_level(1, 20));
    ask(&mut service, a, write_level(2, 30));
    // Queued with their clients; nothing is applied or announced yet.
    assert!(!service.device().has_pending());
    assert!(a_inbox.take().is_empty());

    // One item enters the device's queue at a time, and each is announced
    // when it does, with its own writer.
    let mut heard = Vec::new();
    for now in [0.0, 40.0, 80.0, 120.0] {
        service.pump(now);
        heard.extend(level_changes(b_inbox.take()));
    }
    assert_eq!(
        heard,
        [(a, level(0), 10), (b, level(1), 20), (a, level(2), 30)]
    );
    assert_eq!(link.sent().len(), 3, "paced, one per window");
    assert_eq!(service.device().state().parts[1].level, 20);
}

#[test]
fn raw_dt1s_to_one_address_inside_the_echo_window_all_land() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    // A window apart, all inside the echo window of the first.
    for (now, value) in [(0.0, 100), (40.0, 50), (80.0, 80)] {
        let bytes = level_dt1(2, value);
        ask(&mut service, a, Request::Raw { bytes });
        service.pump(now);
        assert_eq!(service.device().state().parts[2].level, value, "at {now}");
    }
    let values: Vec<_> = level_changes(inbox.take())
        .into_iter()
        .map(|c| c.2)
        .collect();
    assert_eq!(values, [100, 50, 80]);
    assert_eq!(link.sent().len(), 3);
}

#[test]
fn a_raw_dt1_to_a_tone_select_is_a_tone_change() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    let select = params::part_address(3, part::TONE_BANK_MSB);
    ask(
        &mut service,
        a,
        Request::Raw {
            bytes: sysex::build_dt1(DEVICE_ID, &select, &[95, 0, 7]),
        },
    );
    service.pump(0.0);
    let part = &service.device().state().parts[3];
    assert_eq!((part.tone_bank_msb, part.tone_pc), (95, 7));
    assert_eq!(
        inbox.take(),
        [Message::Bulk {
            writer: a,
            kind: BulkKind::Tone,
            binding: Binding::of_part(3)
        }]
    );
    assert!(!link.sent().is_empty());
}

#[test]
fn a_raw_dt1_to_the_studio_set_select_is_a_studio_set_change() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    let bytes = sysex::build_dt1(DEVICE_ID, &params::SETUP_STUDIO_SET_BS_MSB, &[85, 0, 12]);
    ask(&mut service, a, Request::Raw { bytes });
    service.pump(0.0);
    assert_eq!(service.device().state().studio_set_pc, 12);
    assert_eq!(
        inbox.take(),
        [Message::Bulk {
            writer: a,
            kind: BulkKind::StudioSet,
            binding: Binding::ANY
        }]
    );
}

#[test]
fn a_device_change_is_announced_only_when_the_value_differs() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    ask(&mut service, a, write_level(1, 70));
    service.pump(0.0);
    inbox.take();

    // The same value the service wrote: nothing new.
    service.handle(Input::Link(LinkEvent::Midi(level_dt1(1, 70))), 500.0);
    assert!(level_changes(inbox.take()).is_empty());
    // A different one is the device's change.
    service.handle(Input::Link(LinkEvent::Midi(level_dt1(1, 71))), 600.0);
    assert_eq!(
        level_changes(inbox.take()),
        [(ClientId::DEVICE, level(1), 71)]
    );
    // And the same again is not.
    service.handle(Input::Link(LinkEvent::Midi(level_dt1(1, 71))), 700.0);
    assert!(level_changes(inbox.take()).is_empty());
}

#[test]
fn a_read_is_answered_to_the_reader_and_never_announced() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, a_inbox) = client(&mut service, 1);
    let (_b, b_inbox) = client(&mut service, 2);
    ask(
        &mut service,
        a,
        Request::Read {
            param: level(4),
            binding: Binding::ANY,
        },
    );
    service.pump(0.0);
    let size = DataSize::from_value(1).unwrap();
    let rq1 = sysex::build_rq1(DEVICE_ID, &params::part_address(4, part::LEVEL), &size);
    assert_eq!(link.sent(), [rq1]);

    service.handle(Input::Link(LinkEvent::Midi(level_dt1(4, 99))), 10.0);
    assert!(matches!(
        a_inbox.take()[..],
        [Message::Value { value: 99, .. }]
    ));
    assert!(b_inbox.take().is_empty(), "nobody else hears a read");
}

#[test]
fn a_read_the_device_never_answers_times_out() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    ask(
        &mut service,
        a,
        Request::Read {
            param: level(4),
            binding: Binding::ANY,
        },
    );
    let wait = service.pump(0.0).expect("wakes for the timeout");
    assert!(wait.as_millis() > 0);
    service.pump(5000.0);
    assert_eq!(
        inbox.take(),
        [Message::Rejected {
            reason: Reject::Timeout,
            param: Some(level(4))
        }]
    );
}

#[test]
fn a_raw_message_from_a_client_takes_the_service_device_id_and_must_have_a_checksum() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    let size = DataSize::from_value(1).unwrap();
    // An RQ1 built for another device ID.
    let rq1 = sysex::build_rq1(0x17, &params::part_address(4, part::LEVEL), &size);
    ask(&mut service, a, Request::Raw { bytes: rq1 });
    service.pump(0.0);
    assert_eq!(link.sent()[0][2], DEVICE_ID);

    let mut bad = level_dt1(1, 5);
    let checksum = bad.len() - 2;
    bad[checksum] ^= 0x01;
    ask(&mut service, a, Request::Raw { bytes: bad });
    assert!(matches!(
        inbox.take()[..],
        [Message::Rejected {
            reason: Reject::Invalid { .. },
            ..
        }]
    ));
}

#[test]
fn a_multi_byte_dt1_is_a_change_per_parameter_or_one_bulk_past_the_threshold() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    // Part 1's level and pan are neighbours.
    let level_at = params::part_address(0, part::LEVEL);
    let bytes = sysex::build_dt1(DEVICE_ID, &level_at, &[10, 20]);
    ask(&mut service, a, Request::Raw { bytes });
    service.pump(0.0);
    assert_eq!(level_changes(inbox.take()).len(), 2);

    // A PCM Synth Partial names more parameters than the threshold.
    let partial = integral_core::pcm_synth::pcms_partial_address(3, 0);
    let bytes = sysex::build_dt1(DEVICE_ID, &partial, &[1u8; 100]);
    ask(&mut service, a, Request::Raw { bytes });
    service.pump(40.0);
    assert_eq!(
        inbox.take(),
        [Message::Bulk {
            writer: a,
            kind: BulkKind::Tone,
            binding: Binding::of_part(3)
        }]
    );
}

#[test]
fn a_client_not_greeted_is_refused_and_dropped() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let inbox = Inbox::default();
    service.handle(
        Input::Connect {
            client: ClientId(1),
            outbox: Box::new(inbox.clone()),
        },
        0.0,
    );
    ask(&mut service, ClientId(1), write_level(0, 1));
    assert_eq!(
        inbox.take(),
        [Message::Rejected {
            reason: Reject::NotGreeted,
            param: None
        }]
    );
}

#[test]
fn a_fader_dragged_faster_than_the_device_takes_it_sends_the_latest_value() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    let mut last = 0;
    // 200 updates in a second, one every 5 ms.
    for i in 0..200u32 {
        last = 1 + (i % 100);
        ask(&mut service, a, write_level(0, last as i32));
        service.pump(f64::from(i) * 5.0);
    }
    let mut now = 1000.0;
    while now <= 1100.0 {
        service.pump(now);
        now += 5.0;
    }
    let sent = link.sent();
    assert_eq!(
        sent.last(),
        Some(&level_dt1(0, last as u8)),
        "the latest went out"
    );
    assert!(
        sent.len() <= 28,
        "paced to the device's rate, {} sent",
        sent.len()
    );
}

#[test]
fn two_clients_alternating_writes_to_one_parameter_merge_and_the_last_wins() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _a_inbox) = client(&mut service, 1);
    let (b, b_inbox) = client(&mut service, 2);
    let mut last = 0;
    // 100 writes each in a second, alternating.
    for i in 0..200u32 {
        let (who, value) = if i % 2 == 0 {
            (a, 1 + i % 100)
        } else {
            (b, 1 + i % 100)
        };
        last = value;
        ask(&mut service, who, write_level(0, value as i32));
        service.pump(f64::from(i) * 5.0);
    }
    let mut now = 1000.0;
    while now <= 1100.0 {
        service.pump(now);
        now += 5.0;
    }
    let sent = link.sent();
    assert_eq!(sent.last(), Some(&level_dt1(0, last as u8)));
    assert!(sent.len() <= 28, "{} sent", sent.len());
    // The last writer is the one the survivor is announced as.
    let heard = level_changes(b_inbox.take());
    assert_eq!(heard.last().map(|c| (c.0, c.2)), Some((b, last as i32)));
}

#[test]
fn a_tone_change_forgets_only_that_parts_tone_area() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    ask(&mut service, a, write_level(7, 100));
    service.pump(0.0);
    ask(
        &mut service,
        a,
        Request::ChangeTone {
            part: 3,
            msb: 95,
            lsb: 0,
            pc: 1,
        },
    );
    service.pump(40.0);
    inbox.take();
    service.handle(Input::Link(LinkEvent::Midi(level_dt1(7, 60))), 100.0);
    assert_eq!(
        level_changes(inbox.take()),
        [(ClientId::DEVICE, level(7), 60)]
    );
}

#[test]
fn the_reads_that_follow_a_raw_tone_select_are_quiet() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    let index = ParamIndex::new();
    let handle = index
        .find(
            integral_core::host_params::PluginKind::PcmSynth,
            "pcms/common2/0x10",
        )
        .expect("a PCM Synth Common 2 parameter");
    let def = index.get(handle);
    let at = def.device_address(&Binding::of_part(4)).unwrap();
    let select = params::part_address(4, part::TONE_BANK_MSB);
    let mut heard = Vec::new();
    let mut now = 0.0;
    for pc in [1u8, 2] {
        let bytes = sysex::build_dt1(DEVICE_ID, &select, &[87, 0, pc]);
        ask(&mut service, a, Request::Raw { bytes });
        // The select waits for the device's queue (the last tone's reads);
        // pump until it is announced.
        loop {
            service.pump(now);
            now += 40.0;
            let batch = inbox.take();
            let selected = batch.iter().any(|m| matches!(m, Message::Bulk { .. }));
            heard.extend(batch);
            if selected {
                break;
            }
            assert!(now < 20_000.0, "the select never went out");
        }
        // The tone's reads come back: a first sighting each time.
        let reply = sysex::build_dt1(DEVICE_ID, &at, &def.encode(i32::from(pc)));
        service.handle(Input::Link(LinkEvent::Midi(reply)), now);
        heard.extend(inbox.take());
    }
    let from_device = level_changes(heard)
        .into_iter()
        .filter(|c| c.0 == ClientId::DEVICE && c.1.id == "pcms/common2/0x10")
        .count();
    assert_eq!(from_device, 0);
}
#[test]
fn a_refused_write_names_its_parameter() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    ask(&mut service, a, write_level(0, 500));
    let heard = inbox.take();
    assert!(matches!(
        &heard[..],
        [Message::Rejected { reason: Reject::Invalid { .. }, param: Some(p) }] if *p == level(0)
    ));
}

#[test]
fn the_first_sighting_of_a_value_is_silent() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    // A raw RQ1, and the reply for something the session never saw.
    let size = DataSize::from_value(1).unwrap();
    let rq1 = sysex::build_rq1(DEVICE_ID, &params::part_address(4, part::LEVEL), &size);
    ask(&mut service, a, Request::Raw { bytes: rq1 });
    service.pump(0.0);
    service.handle(Input::Link(LinkEvent::Midi(level_dt1(4, 42))), 10.0);
    assert!(level_changes(inbox.take()).is_empty());
    service.handle(Input::Link(LinkEvent::Midi(level_dt1(4, 43))), 20.0);
    assert_eq!(level_changes(inbox.take()).len(), 1);
}

/// The MFX slot `index` of type `type_id` of the SN Synth plugin, for Part 1.
fn mfx_slot(type_id: u8, slot: u8) -> (integral_core::host_params::ParamHandle, Address) {
    let index = ParamIndex::new();
    let handle = index
        .find(
            integral_core::host_params::PluginKind::SnSynth,
            &format!("mfx/{type_id}/{slot}"),
        )
        .expect("an MFX slot");
    let address = index
        .get(handle)
        .device_address(&Binding::of_part(0))
        .unwrap();
    (handle, address)
}

#[test]
fn an_effect_slot_names_the_active_type_when_the_session_knows_it_and_else_one_bulk() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    let index = ParamIndex::new();
    let (two, slot_at) = mfx_slot(2, 1);
    let def = index.get(two);
    let slot = sysex::build_dt1(DEVICE_ID, &slot_at, &def.encode(7));

    // The type is not known: the slot names no one parameter.
    ask(
        &mut service,
        a,
        Request::Raw {
            bytes: slot.clone(),
        },
    );
    service.pump(0.0);
    assert_eq!(
        inbox.take(),
        [Message::Bulk {
            writer: a,
            kind: BulkKind::Mfx,
            binding: Binding::of_part(0)
        }]
    );

    // Say the type, and the same write is the type's parameter.
    let type_at = def
        .effect_type_address(&Binding::of_part(0))
        .unwrap()
        .unwrap();
    let set_type = sysex::build_dt1(DEVICE_ID, &type_at, &[2]);
    ask(&mut service, a, Request::Raw { bytes: set_type });
    service.pump(40.0);
    inbox.take();
    ask(&mut service, a, Request::Raw { bytes: slot });
    service.pump(80.0);
    let heard = level_changes(inbox.take());
    assert_eq!(heard.len(), 1, "{heard:?}");
    assert_eq!(
        (heard[0].0, heard[0].1.id.as_str(), heard[0].2),
        (a, "mfx/2/1", 7)
    );
}

#[test]
fn a_select_written_in_part_passes_through_as_data_while_its_bytes_are_unknown() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    let pc = params::part_address(2, part::TONE_PC);
    // Only the program number, never having seen the bank: sent as written.
    let bytes = sysex::build_dt1(DEVICE_ID, &pc, &[9]);
    ask(
        &mut service,
        a,
        Request::Raw {
            bytes: bytes.clone(),
        },
    );
    service.pump(0.0);
    assert_eq!(link.sent(), [bytes]);
    assert_eq!(
        inbox.take(),
        [Message::Bulk {
            writer: a,
            kind: BulkKind::Tone,
            binding: Binding::of_part(2)
        }]
    );

    // Once the whole select is known, a program number alone completes it.
    let select = params::part_address(2, part::TONE_BANK_MSB);
    let whole = sysex::build_dt1(DEVICE_ID, &select, &[95, 0, 7]);
    ask(&mut service, a, Request::Raw { bytes: whole });
    service.pump(40.0);
    link.clear();
    ask(
        &mut service,
        a,
        Request::Raw {
            bytes: sysex::build_dt1(DEVICE_ID, &pc, &[8]),
        },
    );
    // The tone's own reads leave first; wait for the program number.
    let completed = sysex::build_dt1(DEVICE_ID, &select, &[95, 0, 8]);
    let mut now = 80.0;
    while !link.sent().contains(&completed) {
        assert!(now < 5000.0, "never sent");
        service.pump(now);
        now += 40.0;
    }
}
