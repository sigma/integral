//! The service against a fake link that records what it would send.

use std::time::{Duration, Instant};

use std::sync::{Arc, Mutex};

use integral_core::address::{Address, DataSize};
use integral_core::device_spec::tone_read_blocks;
use integral_core::host_params::PluginKind;
use integral_core::host_params::{Binding, ParamIndex};
use integral_core::mixer_load::MixerBlock;
use integral_core::params::{self, part};
use integral_core::sysex;
use integral_device_service::protocol::{
    BulkKind, ClientId, ExpansionLoadResult, Message, PROTOCOL_VERSION, ParamId, Reject, Request,
    ToneSelect,
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
fn a_clients_note_goes_out_ahead_of_its_queued_sysex() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);

    for part in 0..3 {
        ask(
            &mut service,
            a,
            Request::Raw {
                bytes: level_dt1(part, 50 + part),
            },
        );
    }
    service.pump(0.0); // the first DT1 leaves
    // Two messages in one request, as a page may send a chord.
    let chord = [NOTE_ON.as_slice(), &[0x91, 64, 90]].concat();
    service.handle(
        Input::Client {
            client: a,
            request: Request::Note { bytes: chord },
        },
        1.0,
    );
    service.pump(1.0); // inside the throttle window: only the notes
    let mut now = 1.0;
    while service.device().has_pending() {
        now += 1.0;
        service.pump(now);
    }

    assert_eq!(
        link.sent(),
        [
            level_dt1(0, 50),
            NOTE_ON.to_vec(),
            vec![0x91, 64, 90],
            level_dt1(1, 51),
            level_dt1(2, 52),
        ]
    );
    assert!(
        inbox
            .take()
            .iter()
            .all(|m| !matches!(m, Message::Rejected { .. }))
    );
}

#[test]
fn a_note_request_that_is_not_whole_channel_voice_messages_is_refused() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    for bad in [
        vec![0x90, 60],
        vec![0xF0, 0x41, 0xF7],
        vec![0xF8],
        vec![0x90, 60, 100, 61],
    ] {
        ask(&mut service, a, Request::Note { bytes: bad.clone() });
        assert!(
            matches!(
                inbox.take()[..],
                [Message::Rejected {
                    reason: Reject::Invalid { .. },
                    param: None
                }]
            ),
            "{bad:02X?}"
        );
    }
    service.pump(10.0);
    assert!(link.sent().is_empty());
}

#[test]
fn a_note_request_before_hello_is_refused() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let inbox = Inbox::default();
    let a = ClientId(1);
    service.handle(
        Input::Connect {
            client: a,
            outbox: Box::new(inbox.clone()),
        },
        0.0,
    );
    ask(
        &mut service,
        a,
        Request::Note {
            bytes: NOTE_ON.to_vec(),
        },
    );
    service.pump(1.0);
    assert!(link.sent().is_empty());
    assert!(matches!(
        inbox.take()[..],
        [Message::Rejected {
            reason: Reject::NotGreeted,
            ..
        }]
    ));
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
            binding: Binding::of_part(3),
            selected: true,
            tone: Some(ToneSelect {
                msb: 95,
                lsb: 0,
                pc: 7
            }),
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
            binding: Binding::ANY,
            selected: true,
            tone: None,
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

/// A raw RQ1 of `len` bytes at the Level of Part `part_index`.
fn level_rq1(part_index: u8, len: u32) -> Request {
    let size = DataSize::from_value(len).unwrap();
    raw(sysex::build_rq1(
        DEVICE_ID,
        &params::part_address(part_index, part::LEVEL),
        &size,
    ))
}

#[test]
fn a_read_reply_older_than_a_write_is_not_a_device_change() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (reader, reader_inbox) = client(&mut service, 1);
    let (writer, inbox) = client(&mut service, 2);
    // The session knows the value A.
    service.handle(Input::Link(LinkEvent::Midi(level_dt1(1, 50))), 0.0);
    inbox.take();

    // The read leaves, then the write B; the device answers in order.
    ask(&mut service, reader, level_rq1(1, 1));
    service.pump(10.0);
    ask(&mut service, writer, write_level(1, 70));
    service.pump(60.0);
    assert_eq!(sysex_sent(&link).len(), 2);
    inbox.take();
    reader_inbox.take();
    service.handle(Input::Link(LinkEvent::Midi(level_dt1(1, 50))), 70.0);
    assert!(level_changes(inbox.take()).is_empty(), "A is not news");
    assert!(level_changes(reader_inbox.take()).is_empty());

    // A typed read afterwards finds B.
    ask(
        &mut service,
        writer,
        Request::Read {
            param: level(1),
            binding: Binding::ANY,
        },
    );
    service.pump(110.0);
    service.handle(Input::Link(LinkEvent::Midi(level_dt1(1, 70))), 120.0);
    assert!(matches!(
        inbox.take()[..],
        [Message::Value { value: 70, .. }]
    ));
    assert!(level_changes(reader_inbox.take()).is_empty());
}

#[test]
fn a_typed_read_overtaken_by_a_write_is_answered_with_the_written_value() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (reader, reader_inbox) = client(&mut service, 1);
    let (writer, _inbox) = client(&mut service, 2);
    service.handle(Input::Link(LinkEvent::Midi(level_dt1(1, 50))), 0.0);
    let read = Request::Read {
        param: level(1),
        binding: Binding::ANY,
    };
    ask(&mut service, reader, read);
    service.pump(10.0);
    ask(&mut service, writer, write_level(1, 70));
    service.pump(60.0);
    reader_inbox.take();
    service.handle(Input::Link(LinkEvent::Midi(level_dt1(1, 50))), 70.0);
    let heard = reader_inbox.take();
    assert!(matches!(heard[..], [Message::Value { value: 70, .. }]));
    service.pump(5000.0);
    assert!(reader_inbox.take().is_empty(), "no timeout either");
}

#[test]
fn a_raw_client_still_gets_the_reply_older_than_a_write() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (reader, reader_inbox) = client(&mut service, 1);
    ask(&mut service, reader, Request::ReceiveRaw { on: true });
    let (writer, _inbox) = client(&mut service, 2);
    ask(&mut service, reader, level_rq1(1, 1));
    service.pump(10.0);
    ask(&mut service, writer, write_level(1, 70));
    service.pump(60.0);
    reader_inbox.take();
    let reply = level_dt1(1, 50);
    service.handle(Input::Link(LinkEvent::Midi(reply.clone())), 70.0);
    assert!(
        reader_inbox
            .take()
            .contains(&Message::Sysex { bytes: reply })
    );
}

#[test]
fn a_reply_with_no_write_in_between_is_still_a_device_change() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (reader, inbox) = client(&mut service, 1);
    service.handle(Input::Link(LinkEvent::Midi(level_dt1(1, 50))), 0.0);
    ask(&mut service, reader, level_rq1(1, 1));
    service.pump(10.0);
    // Someone turned the knob on the front panel.
    service.handle(Input::Link(LinkEvent::Midi(level_dt1(1, 51))), 20.0);
    assert_eq!(
        level_changes(inbox.take()),
        [(ClientId::DEVICE, level(1), 51)]
    );
}

#[test]
fn a_reply_covering_a_written_byte_and_others_is_taken_for_the_others() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (reader, inbox) = client(&mut service, 1);
    let pan = param_at(&params::part_address(1, part::PAN));
    // The session knows the Level and the Pan.
    service.handle(Input::Link(LinkEvent::Midi(level_dt1(1, 50))), 0.0);
    let pan_dt1 = part_dt1(1, part::PAN, 64);
    service.handle(Input::Link(LinkEvent::Midi(pan_dt1)), 0.0);

    // One read of both, then a write of the Level.
    ask(&mut service, reader, level_rq1(1, 2));
    service.pump(10.0);
    ask(&mut service, reader, write_level(1, 70));
    service.pump(60.0);
    inbox.take();
    let both = sysex::build_dt1(DEVICE_ID, &params::part_address(1, part::LEVEL), &[50, 65]);
    service.handle(Input::Link(LinkEvent::Midi(both)), 70.0);
    assert_eq!(level_changes(inbox.take()), [(ClientId::DEVICE, pan, 65)]);
}

#[test]
fn a_read_that_is_never_answered_hides_no_later_change() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    service.handle(Input::Link(LinkEvent::Midi(level_dt1(1, 50))), 0.0);
    ask(&mut service, a, level_rq1(1, 1));
    service.pump(10.0);
    ask(&mut service, a, write_level(1, 70));
    service.pump(60.0);
    inbox.take();
    // The reply is lost; much later the front panel moves the knob.
    service.handle(Input::Link(LinkEvent::Midi(level_dt1(1, 50))), 10_000.0);
    assert_eq!(
        level_changes(inbox.take()),
        [(ClientId::DEVICE, level(1), 50)]
    );
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
            binding: Binding::of_part(3),
            selected: false,
            tone: None,
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
            binding: Binding::of_part(0),
            selected: false,
            tone: None,
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
fn a_typed_write_to_a_slot_of_another_effect_type_than_the_known_one_is_dropped() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    let index = ParamIndex::new();
    let (two, slot_at) = mfx_slot(2, 1);
    let def = index.get(two);
    let type_at = def
        .effect_type_address(&Binding::of_part(0))
        .unwrap()
        .unwrap();
    let write = Request::Write {
        param: ParamId::of(&index, two),
        binding: Binding::of_part(0),
        value: 7,
    };

    // The effect is type 3 as far as the session knows.
    ask(
        &mut service,
        a,
        Request::Raw {
            bytes: sysex::build_dt1(DEVICE_ID, &type_at, &[3]),
        },
    );
    service.pump(0.0);
    link.clear();
    ask(&mut service, a, write.clone());
    service.pump(40.0);
    service.pump(80.0);
    assert_eq!(link.sent(), Vec::<Vec<u8>>::new(), "type 2 is not active");

    // Type 2 becomes the effect's: the write goes.
    ask(
        &mut service,
        a,
        Request::Raw {
            bytes: sysex::build_dt1(DEVICE_ID, &type_at, &[2]),
        },
    );
    service.pump(120.0);
    link.clear();
    ask(&mut service, a, write);
    service.pump(160.0);
    assert_eq!(
        link.sent(),
        [sysex::build_dt1(DEVICE_ID, &slot_at, &def.encode(7))]
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
            binding: Binding::of_part(2),
            selected: true,
            tone: None,
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

// ---------------------------------------------------------------------------
// Pacing and cache rules
// ---------------------------------------------------------------------------

/// A raw DT1 of one byte at `offset` in Part `part_index`'s block.
fn part_dt1(part_index: u8, offset: [u8; 3], value: u8) -> Vec<u8> {
    sysex::build_dt1(
        DEVICE_ID,
        &params::part_address(part_index, offset),
        &[value],
    )
}

fn raw(bytes: Vec<u8>) -> Request {
    Request::Raw { bytes }
}

/// Everything the link has been sent that is SysEx.
fn sysex_sent(link: &FakeLink) -> Vec<Vec<u8>> {
    link.sent().into_iter().filter(|m| is_sysex(m)).collect()
}

#[test]
fn clients_with_twenty_writes_each_are_served_in_turn() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _a_inbox) = client(&mut service, 1);
    let (b, _b_inbox) = client(&mut service, 2);
    let mut from_a = Vec::new();
    let mut from_b = Vec::new();
    for p in 0..10 {
        for (offset, value) in [(part::LEVEL, 10 + p), (part::PAN, 30 + p)] {
            from_a.push(part_dt1(p, offset, value));
        }
        for (offset, value) in [(part::CHORUS_SEND, 50 + p), (part::REVERB_SEND, 70 + p)] {
            from_b.push(part_dt1(p, offset, value));
        }
    }
    // All of a's, then all of b's.
    for bytes in &from_a {
        ask(&mut service, a, raw(bytes.clone()));
    }
    for bytes in &from_b {
        ask(&mut service, b, raw(bytes.clone()));
    }
    run_until(&mut service, 0.0, 2000.0);

    let expected: Vec<_> = from_a
        .iter()
        .zip(&from_b)
        .flat_map(|(x, y)| [x.clone(), y.clone()])
        .collect();
    assert_eq!(sysex_sent(&link), expected);
}

#[test]
fn a_write_queued_after_a_push_lands_after_all_of_it() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, a_inbox) = client(&mut service, 1);
    let (b, _b_inbox) = client(&mut service, 2);
    let pushed: Vec<_> = (0..3).map(|p| part_dt1(p, part::LEVEL, 10 + p)).collect();
    ask(
        &mut service,
        a,
        Request::Push {
            requests: pushed.iter().cloned().map(raw).collect(),
        },
    );
    // Covers the push's write to Part 1's level, and is queued after it.
    ask(&mut service, a, write_level(0, 99));
    for p in 3..6 {
        ask(&mut service, b, raw(part_dt1(p, part::LEVEL, 40 + p)));
    }
    run_until(&mut service, 0.0, 2000.0);

    let sent = sysex_sent(&link);
    let position = |bytes: &Vec<u8>| sent.iter().position(|m| m == bytes).expect("sent");
    let mut a_order: Vec<_> = pushed.iter().map(position).collect();
    a_order.push(position(&level_dt1(0, 99)));
    assert!(
        a_order.windows(2).all(|w| w[0] < w[1]),
        "the push, then the write: {a_order:?}"
    );
    assert_eq!(sent.len(), 3 + 1 + 3, "the push's write was not replaced");
    let heard = a_inbox.take();
    assert_eq!(
        heard.iter().filter(|m| **m == Message::Pushed).count(),
        1,
        "told once that the push went out"
    );
}

#[test]
fn a_push_is_refused_whole_if_any_request_in_it_is_not_valid() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    ask(
        &mut service,
        a,
        Request::Push {
            requests: vec![
                write_level(0, 10),
                Request::Read {
                    param: level(0),
                    binding: Binding::ANY,
                },
            ],
        },
    );
    ask(
        &mut service,
        a,
        Request::Push {
            requests: vec![write_level(0, 10), write_level(1, 500)],
        },
    );
    let refusals: Vec<_> = inbox
        .take()
        .into_iter()
        .filter(|m| {
            matches!(
                m,
                Message::Rejected {
                    reason: Reject::Invalid { .. },
                    ..
                }
            )
        })
        .collect();
    assert_eq!(refusals.len(), 2);
    run_until(&mut service, 0.0, 500.0);
    assert!(link.sent().is_empty());
}

#[test]
fn a_fresh_session_sends_a_write_equal_to_the_default() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    let index = ParamIndex::new();
    let handle = param_at(&params::part_address(2, part::LEVEL))
        .handle(&index)
        .expect("a parameter");
    let default = index.get(handle).default;
    ask(&mut service, a, write_level(2, default));
    run_until(&mut service, 0.0, 200.0);
    assert_eq!(link.sent(), [level_dt1(2, default as u8)]);
}

#[test]
fn a_repeated_write_in_one_session_is_dropped() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, a_inbox) = client(&mut service, 1);
    let (b, _b_inbox) = client(&mut service, 2);
    ask(&mut service, a, write_level(2, 70));
    run_until(&mut service, 0.0, 200.0);
    assert_eq!(link.sent().len(), 1);
    a_inbox.take();

    // The same value again, from the same client or another.
    ask(&mut service, a, write_level(2, 70));
    ask(&mut service, b, write_level(2, 70));
    run_until(&mut service, 300.0, 1500.0);
    assert_eq!(link.sent().len(), 1, "nothing new went out");
    assert!(
        level_changes(a_inbox.take()).is_empty(),
        "nor was announced"
    );

    // A different value goes, and so does the old one after it.
    ask(&mut service, a, write_level(2, 71));
    run_until(&mut service, 700.0, 800.0);
    ask(&mut service, a, write_level(2, 70));
    run_until(&mut service, 900.0, 1000.0);
    assert_eq!(link.sent().len(), 3);
}

#[test]
fn a_value_the_device_reported_is_known_and_a_write_of_it_is_dropped() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    service.handle(Input::Link(LinkEvent::Midi(level_dt1(4, 33))), 1.0);
    ask(&mut service, a, write_level(4, 33));
    ask(&mut service, a, write_level(5, 33));
    run_until(&mut service, 2.0, 300.0);
    assert_eq!(link.sent(), [level_dt1(5, 33)], "only the unknown field");
}

#[test]
fn a_push_sends_what_the_service_believes_the_device_has() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    ask(&mut service, a, write_level(2, 70));
    run_until(&mut service, 0.0, 100.0);
    ask(
        &mut service,
        a,
        Request::Push {
            requests: vec![write_level(2, 70)],
        },
    );
    run_until(&mut service, 200.0, 400.0);
    assert_eq!(link.sent(), [level_dt1(2, 70), level_dt1(2, 70)]);
}

#[test]
fn a_new_session_makes_every_field_unknown_again() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    ask(&mut service, a, write_level(2, 70));
    run_until(&mut service, 0.0, 100.0);

    // Dropped and back: the device may have been power-cycled.
    service.handle(Input::Link(LinkEvent::Down), 200.0);
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 300.0);
    ask(&mut service, a, write_level(2, 70));
    run_until(&mut service, 300.0, 500.0);
    assert_eq!(link.sent().len(), 2);

    // And a second Up with no Down between, like a new device ID.
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 600.0);
    ask(&mut service, a, write_level(2, 70));
    run_until(&mut service, 600.0, 800.0);
    assert_eq!(link.sent().len(), 3);
}

#[test]
fn what_was_queued_before_a_drop_is_not_sent_after_it() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    for p in 0..4 {
        ask(&mut service, a, write_level(p, 60 + i32::from(p)));
    }
    service.pump(0.0); // the first leaves, the second waits in the device's queue
    assert_eq!(link.sent().len(), 1);
    assert!(service.device().has_pending());

    service.handle(Input::Link(LinkEvent::Down), 10.0);
    assert!(!service.device().has_pending(), "the device queue is gone");
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 20.0);
    run_until(&mut service, 20.0, 1000.0);
    assert_eq!(link.sent().len(), 1, "none of the rest was sent");
}

#[test]
fn two_hundred_updates_of_one_field_in_a_second_stay_within_the_device_rate() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    let mut last = 0;
    for i in 0..200u32 {
        last = 1 + (i % 100);
        ask(&mut service, a, write_level(0, last as i32));
        service.pump(f64::from(i) * 5.0);
    }
    assert!(
        link.sent().len() <= 25,
        "{} DT1s in the first second",
        link.sent().len()
    );
    run_until(&mut service, 1000.0, 1200.0);
    assert_eq!(link.sent().last(), Some(&level_dt1(0, last as u8)));
}

/// The blocks the service asks for to read a tone back.
fn tone_requests(part_index: u8, tone_type: params::ToneType) -> Vec<(Address, DataSize)> {
    tone_read_blocks(tone_type, part_index)
}

/// What a test plays the device in: answers the RQ1s for `blocks` the moment
/// they leave, each with one DT1 of `fill` bytes, and counts what it saw.
struct Device {
    blocks: Vec<(Address, DataSize)>,
    fill: u8,
    seen: usize,
    asked: Vec<Address>,
}

impl Device {
    fn new(blocks: Vec<(Address, DataSize)>, fill: u8) -> Self {
        Self {
            blocks,
            fill,
            seen: 0,
            asked: Vec::new(),
        }
    }

    /// Pump at `now` and answer what left. Returns the wait the service asked.
    fn step(&mut self, service: &mut Service, link: &FakeLink, now: f64) -> Option<Duration> {
        let mut wait = service.pump(now);
        let sent = link.sent();
        let fresh: Vec<_> = sent[self.seen..].to_vec();
        self.seen = sent.len();
        let mut answered = false;
        for message in fresh {
            if message.len() < 15 || message[6] != 0x11 {
                continue;
            }
            let address = Address([message[7], message[8], message[9], message[10]]);
            let size = DataSize([message[11], message[12], message[13], message[14]]);
            let Some((_, size)) = self
                .blocks
                .iter()
                .find(|(a, s)| *a == address && *s == size)
                .copied()
            else {
                continue;
            };
            self.asked.push(address);
            let data = vec![self.fill; size.value() as usize];
            let reply = sysex::build_dt1(DEVICE_ID, &address, &data);
            service.handle(Input::Link(LinkEvent::Midi(reply)), now);
            answered = true;
        }
        if answered {
            // As the runtime does after an input.
            wait = service.pump(now);
        }
        wait
    }

    /// Run from `from` to `to` the way the runtime does.
    fn run(&mut self, service: &mut Service, link: &FakeLink, from: f64, to: f64) {
        let mut now = from;
        while now <= to {
            let Some(wait) = self.step(service, link, now) else {
                return;
            };
            now += wait.as_secs_f64() * 1000.0;
        }
    }
}

fn change_tone(part: u8, msb: u8, lsb: u8, pc: u8) -> Request {
    Request::ChangeTone { part, msb, lsb, pc }
}

fn baseline_asked(part: u8) -> Request {
    Request::Baseline {
        binding: Binding::of_part(part),
    }
}

fn baselines_heard(messages: Vec<Message>) -> Vec<(Option<u8>, bool, usize)> {
    messages
        .into_iter()
        .filter_map(|m| match m {
            Message::Baseline {
                binding,
                tone,
                runs,
            } => Some((
                binding.part,
                tone.is_some(),
                runs.iter().map(|r| r.bytes.len()).sum(),
            )),
            _ => None,
        })
        .collect()
}

/// A one-byte parameter of the PCM Synth plugin in `block`, for Part `part_index`:
/// its ID, its address and its lowest value.
fn pcm_param_in(
    block: &(Address, DataSize),
    part_index: u8,
    skip: usize,
) -> (ParamId, Address, i32) {
    let index = ParamIndex::new();
    let binding = Binding::of_part(part_index);
    let (from, to) = (block.0.value(), block.0.value() + block.1.value());
    let list = PluginKind::PcmSynth.host_params();
    list.iter()
        .enumerate()
        .filter(|(_, p)| p.width == 1 && p.effect.is_none())
        .filter_map(|(i, p)| {
            let at = p.device_address(&binding).ok()?;
            (from..to).contains(&at.value()).then(|| {
                let handle = integral_core::host_params::ParamHandle {
                    plugin: PluginKind::PcmSynth,
                    index: i as u32,
                };
                (ParamId::of(&index, handle), at, p.min)
            })
        })
        .nth(skip)
        .expect("a parameter in the block")
}

#[test]
fn a_preset_tone_is_read_back_block_by_block_behind_its_select_and_kept_as_the_baseline() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    let blocks = tone_requests(2, params::ToneType::PcmSynth);
    assert_eq!(blocks.len(), 8, "Common, MFX, PMT, 4 Partials, Common 2");
    let mut device = Device::new(blocks.clone(), 9);
    ask(&mut service, a, change_tone(2, 87, 64, 5));
    ask(&mut service, a, baseline_asked(2));
    device.run(&mut service, &link, 0.0, 3000.0);

    let sent = sysex_sent(&link);
    let select = sysex::build_dt1(
        DEVICE_ID,
        &params::part_address(2, part::TONE_BANK_MSB),
        &[87, 64, 5],
    );
    assert_eq!(sent[0], select);
    assert_eq!(device.asked, blocks.iter().map(|b| b.0).collect::<Vec<_>>());
    let baseline = service.baseline(2).expect("whole");
    assert_eq!(
        (baseline.tone().msb, baseline.tone().lsb, baseline.tone().pc),
        (87, 64, 5)
    );
    let (address, size) = &blocks[3];
    assert_eq!(
        baseline.get(address, size.value() as usize),
        Some(vec![9; size.value() as usize])
    );
    let total: usize = blocks.iter().map(|(_, s)| s.value() as usize).sum();
    assert_eq!(baselines_heard(inbox.take()), [(Some(2), true, total)]);

    // Asked again it is answered at once, and another Part has none.
    ask(&mut service, a, baseline_asked(2));
    ask(&mut service, a, baseline_asked(3));
    service.pump(3100.0);
    assert_eq!(
        baselines_heard(inbox.take()),
        [(Some(2), true, total), (Some(3), false, 0)]
    );
}

#[test]
fn a_preset_read_once_is_not_read_again_by_any_part() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    let mut device = Device::new(tone_requests(2, params::ToneType::PcmSynth), 9);
    ask(&mut service, a, change_tone(2, 87, 64, 5));
    device.run(&mut service, &link, 0.0, 3000.0);
    assert_eq!(device.asked.len(), 8);

    // The same preset on Part 4: the read before stands for it.
    let mut again = Device::new(tone_requests(3, params::ToneType::PcmSynth), 1);
    again.seen = link.sent().len();
    ask(&mut service, a, change_tone(3, 87, 64, 5));
    again.run(&mut service, &link, 3000.0, 4000.0);
    assert!(again.asked.is_empty(), "nothing read");
    let baseline = service.baseline(3).expect("at once");
    let (address, size) = &tone_requests(3, params::ToneType::PcmSynth)[0];
    assert_eq!(
        baseline.get(address, size.value() as usize),
        Some(vec![9; size.value() as usize]),
        "Part-relative: the bytes are Part 3's"
    );
}

#[test]
fn a_user_tone_keeps_no_baseline() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    ask(&mut service, a, change_tone(2, 87, 0, 5));
    ask(&mut service, a, baseline_asked(2));
    run_until(&mut service, 0.0, 500.0);
    // The select and the name and category reads; no tone read-back.
    assert_eq!(sysex_sent(&link).len(), 3);
    assert!(service.baseline(2).is_none());
    assert_eq!(baselines_heard(inbox.take()), [(Some(2), false, 0)]);
}

#[test]
fn a_raw_preset_select_gets_a_baseline_too() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    let select = params::part_address(4, part::TONE_BANK_MSB);
    let bytes = sysex::build_dt1(DEVICE_ID, &select, &[95, 64, 3]);
    ask(&mut service, a, raw(bytes));
    let blocks = tone_requests(4, params::ToneType::SnSynth);
    let mut device = Device::new(blocks.clone(), 1);
    device.run(&mut service, &link, 0.0, 3000.0);
    assert_eq!(device.asked.len(), blocks.len());
    assert_eq!(service.baseline(4).map(|b| b.tone().msb), Some(95));
}

#[test]
fn a_reply_for_the_tone_before_is_neither_the_new_tones_baseline_nor_cached() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _a_inbox) = client(&mut service, 1);
    let (b, _b_inbox) = client(&mut service, 2);
    let blocks = tone_requests(1, params::ToneType::PcmSynth);
    let (param, address, min) = pcm_param_in(&blocks[0], 1, 0);
    ask(&mut service, a, change_tone(1, 87, 64, 0));
    // Run until the first block's request left, and leave it unanswered.
    let mut nobody = Device::new(Vec::new(), 0);
    nobody.run(&mut service, &link, 0.0, 500.0);
    let rq1 = sysex::build_rq1(DEVICE_ID, &blocks[0].0, &blocks[0].1);
    assert!(link.sent().contains(&rq1));
    // Another client selects another tone on the Part meanwhile.
    ask(&mut service, b, change_tone(1, 87, 64, 1));
    nobody.run(&mut service, &link, 500.0, 900.0);

    // The first tone's answer comes late: the service does not cache it.
    let stale = vec![min as u8; blocks[0].1.value() as usize];
    let reply = sysex::build_dt1(DEVICE_ID, &blocks[0].0, &stale);
    service.handle(Input::Link(LinkEvent::Midi(reply)), 900.0);
    assert!(service.baseline(1).is_none());
    ask(
        &mut service,
        b,
        Request::Write {
            param,
            binding: Binding::of_part(1),
            value: min,
        },
    );
    let mut device = Device::new(blocks.clone(), 1);
    device.seen = link.sent().len();
    device.run(&mut service, &link, 900.0, 4000.0);
    let baseline = service.baseline(1).expect("the second tone's");
    assert_eq!(baseline.tone().pc, 1);
    assert_eq!(
        baseline.get(&address, 1),
        Some(vec![1]),
        "from the second tone's own answer"
    );
}

#[test]
fn a_tone_the_device_never_reads_back_ends_with_no_baseline() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    ask(&mut service, a, change_tone(0, 87, 64, 0));
    ask(&mut service, a, baseline_asked(0));
    run_until(&mut service, 0.0, 1000.0);
    assert!(baselines_heard(inbox.take()).is_empty(), "still waiting");
    // Each block gets two seconds from the moment it left.
    run_until(&mut service, 1000.0, 30_000.0);
    assert!(service.baseline(0).is_none());
    assert_eq!(baselines_heard(inbox.take()), [(Some(0), false, 0)]);
}

#[test]
fn a_new_select_drops_the_baseline_and_a_studio_set_select_drops_them_all() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    let mut now = 0.0;
    for part in [0, 1] {
        let blocks = tone_requests(part, params::ToneType::PcmSynth);
        let mut device = Device::new(blocks, 1);
        device.seen = link.sent().len();
        ask(&mut service, a, change_tone(part, 87, 64, part));
        device.run(&mut service, &link, now, now + 1500.0);
        now += 1500.0;
        assert!(service.baseline(part).is_some());
    }
    ask(&mut service, a, change_tone(0, 87, 0, 0));
    run_until(&mut service, now, now + 500.0);
    assert!(service.baseline(0).is_none(), "now a User tone");
    assert!(service.baseline(1).is_some());
    ask(&mut service, a, Request::SwitchStudioSet { pc: 3 });
    run_until(&mut service, now + 600.0, now + 1000.0);
    assert!(service.baseline(1).is_none());
}

#[test]
fn a_late_reply_after_a_studio_set_switch_does_not_make_a_field_known() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _a_inbox) = client(&mut service, 1);
    let (b, _b_inbox) = client(&mut service, 2);
    let blocks = tone_requests(0, params::ToneType::PcmSynth);
    let (param, _, min) = pcm_param_in(&blocks[0], 0, 0);
    ask(&mut service, a, change_tone(0, 87, 64, 0));
    let mut nobody = Device::new(Vec::new(), 0);
    nobody.run(&mut service, &link, 0.0, 500.0);
    ask(&mut service, b, Request::SwitchStudioSet { pc: 3 });
    nobody.run(&mut service, &link, 500.0, 700.0);
    assert!(service.baseline(0).is_none());

    let late = vec![min as u8; blocks[0].1.value() as usize];
    let reply = sysex::build_dt1(DEVICE_ID, &blocks[0].0, &late);
    service.handle(Input::Link(LinkEvent::Midi(reply)), 700.0);

    // A write of that very value is not dropped as unchanged.
    let before = link.sent().len();
    ask(
        &mut service,
        b,
        Request::Write {
            param,
            binding: Binding::of_part(0),
            value: min,
        },
    );
    // (The service reads every Part's select after the switch first.)
    run_until(&mut service, 700.0, 2500.0);
    assert!(
        sysex_sent(&link)[before.min(sysex_sent(&link).len())..]
            .iter()
            .any(|m| m[6] == 0x12),
        "the write went out"
    );
}

#[test]
fn a_push_to_a_part_it_did_not_select_is_sent_whatever_the_cache_holds() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    ask(&mut service, a, write_level(1, 50));
    run_until(&mut service, 0.0, 100.0);
    ask(
        &mut service,
        a,
        Request::Push {
            requests: vec![change_tone(2, 87, 0, 1), write_level(1, 50)],
        },
    );
    run_until(&mut service, 200.0, 800.0);
    let levels = sysex_sent(&link)
        .into_iter()
        .filter(|m| *m == level_dt1(1, 50))
        .count();
    assert_eq!(levels, 2);
}

#[test]
fn five_quick_selects_on_one_part_cost_one() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    for pc in 0..5 {
        ask(&mut service, a, change_tone(0, 87, 0, pc));
    }
    run_until(&mut service, 0.0, 1000.0);
    let select = params::part_address(0, part::TONE_BANK_MSB);
    let selects: Vec<_> = sysex_sent(&link)
        .into_iter()
        .filter(|m| m[6] == 0x12 && m[7..11] == *select.as_bytes())
        .collect();
    assert_eq!(selects.len(), 1);
    assert_eq!(&selects[0][11..14], &[87, 0, 4], "the last");
}

#[test]
fn a_fader_stays_responsive_while_sixteen_parts_recall_a_preset() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let mut all_blocks = Vec::new();
    let mut clients = Vec::new();
    for p in 0..16u8 {
        let (id, _inbox) = client(&mut service, 10 + u64::from(p));
        all_blocks.extend(tone_requests(p, params::ToneType::PcmSynth));
        clients.push((id, p));
    }
    let (drag, _drag_inbox) = client(&mut service, 1);
    let mut device = Device::new(all_blocks, 1);
    for (id, p) in &clients {
        ask(
            &mut service,
            *id,
            Request::Push {
                requests: vec![change_tone(*p, 87, 64, *p)],
            },
        );
    }
    device.run(&mut service, &link, 0.0, 1000.0);
    let started = 1000.0;
    ask(&mut service, drag, write_level(0, 99));
    let wanted = level_dt1(0, 99);
    let mut now = started;
    let mut at = None;
    while now < started + 3000.0 && at.is_none() {
        let wait = device.step(&mut service, &link, now).expect("busy");
        if link.sent().contains(&wanted) {
            at = Some(now - started);
        }
        now += wait.as_secs_f64() * 1000.0;
    }
    let at = at.expect("the fader's DT1 went out");
    assert!(at <= 160.0, "first DT1 after {at} ms");
}

// --- Program Changes on the note port ---------------------------------------

/// Tell the service what the device holds in the receive settings: Part `n`
/// on channel `channel_of(n)`, Program Change on except where `off` says,
/// and the Studio Set on channel `studio`.
fn rx_read(service: &mut Service, channel_of: impl Fn(u8) -> u8, off: &[u8], studio: u8, now: f64) {
    for p in 0..16 {
        let mut data = vec![0u8; 58];
        data[0] = channel_of(p);
        data[57] = u8::from(!off.contains(&p));
        let dt1 = sysex::build_dt1(
            DEVICE_ID,
            &params::part_address(p, part::RECEIVE_CHANNEL),
            &data,
        );
        service.handle(Input::Link(LinkEvent::Midi(dt1)), now);
    }
    let dt1 = sysex::build_dt1(
        DEVICE_ID,
        &params::SYSTEM_STUDIO_SET_CONTROL_CHANNEL,
        &[studio],
    );
    service.handle(Input::Link(LinkEvent::Midi(dt1)), now);
}

fn tone_bulks(messages: Vec<Message>) -> Vec<Option<u8>> {
    messages
        .into_iter()
        .filter_map(|m| match m {
            Message::Bulk {
                writer: ClientId::DEVICE,
                kind: BulkKind::Tone,
                binding,
                ..
            } => Some(binding.part),
            _ => None,
        })
        .collect()
}

fn tone_field(part_index: u8) -> (Address, Vec<u8>) {
    let index = ParamIndex::new();
    let handle = index
        .find(PluginKind::PcmSynth, "pcms/common2/0x10")
        .expect("a PCM Synth parameter");
    let def = index.get(handle);
    let at = def.device_address(&Binding::of_part(part_index)).unwrap();
    (at, def.encode(1))
}

#[test]
fn a_program_change_on_the_note_port_reaches_the_parts_on_its_channel_that_listen() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (_a, inbox) = client(&mut service, 1);
    // Parts 3 and 7 are on channel 4 (index 3); Part 7 ignores Program Change.
    rx_read(
        &mut service,
        |p| if p == 3 || p == 7 { 3 } else { p },
        &[7],
        15,
        1.0,
    );
    for p in [3, 4] {
        let (at, bytes) = tone_field(p);
        let dt1 = sysex::build_dt1(DEVICE_ID, &at, &bytes);
        service.handle(Input::Link(LinkEvent::Midi(dt1)), 2.0);
    }
    inbox.take();

    service.handle(Input::Note(vec![0xC3, 5]), 3.0);
    assert_eq!(tone_bulks(inbox.take()), [Some(3)]);
    run_until(&mut service, 3.0, 200.0);
    assert_eq!(
        link.sent()[0],
        [0xC3, 5],
        "the message still reaches the device"
    );
    let (dump_at, dump_size) = MixerBlock::Part(3).request().unwrap();
    assert!(
        link.sent()
            .contains(&sysex::build_rq1(DEVICE_ID, &dump_at, &dump_size)),
        "the Part's dump is read again"
    );

    // What the device then says of Part 4's tone differs and is news; of
    // Part 3's, it is a first sighting again and quiet.
    for p in [3, 4] {
        let (at, mut bytes) = tone_field(p);
        bytes[0] ^= 1;
        let dt1 = sysex::build_dt1(DEVICE_ID, &at, &bytes);
        service.handle(Input::Link(LinkEvent::Midi(dt1)), 300.0);
    }
    let news: Vec<_> = level_changes(inbox.take())
        .into_iter()
        .filter(|c| c.1.id == "pcms/common2/0x10")
        .collect();
    assert_eq!(news.len(), 1);
}

#[test]
fn bank_select_and_other_messages_on_the_note_port_change_nothing() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (_a, inbox) = client(&mut service, 1);
    rx_read(&mut service, |p| p, &[], 15, 1.0);
    inbox.take();
    for message in [
        [0xB5, 0, 87],
        [0xB5, 32, 64],
        [0xB5, 7, 100],
        [0x95, 60, 100],
    ] {
        service.handle(Input::Note(message.to_vec()), 3.0);
        assert!(inbox.take().is_empty(), "{message:02X?}");
    }
    service.handle(Input::Note(vec![0xC5, 1]), 4.0);
    assert_eq!(tone_bulks(inbox.take()), [Some(5)]);
}

#[test]
fn a_program_change_on_the_studio_set_control_channel_is_a_studio_set_change() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (_a, inbox) = client(&mut service, 1);
    rx_read(
        &mut service,
        |_| 0,
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        9,
        1.0,
    );
    inbox.take();
    service.handle(Input::Note(vec![0xC9, 2]), 3.0);
    assert_eq!(
        inbox.take(),
        [Message::Bulk {
            writer: ClientId::DEVICE,
            kind: BulkKind::StudioSet,
            binding: Binding::ANY,
            selected: true,
            tone: None,
        }]
    );
}

#[test]
fn a_program_change_waits_for_the_receive_settings_and_then_reaches_only_the_parts_on_it() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (_a, inbox) = client(&mut service, 1);
    service.handle(Input::Note(vec![0xC2, 1]), 1.0);
    assert!(
        tone_bulks(inbox.take()).is_empty(),
        "not before they are read"
    );
    run_until(&mut service, 1.0, 1000.0);
    let part_reads = link
        .sent()
        .into_iter()
        .filter(|m| m.len() > 6 && m[6] == 0x11)
        .count();
    assert!(
        part_reads >= 17,
        "16 Parts and the Studio Set: {part_reads}"
    );

    rx_read(&mut service, |p| p % 4, &[], 15, 1100.0);
    assert_eq!(
        tone_bulks(inbox.take()),
        [Some(2), Some(6), Some(10), Some(14)]
    );
}

#[test]
fn a_program_change_whose_settings_are_never_answered_reaches_every_part_after_a_while() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (_a, inbox) = client(&mut service, 1);
    service.handle(Input::Note(vec![0xC2, 1]), 1.0);
    run_until(&mut service, 1.0, 4000.0);
    assert!(tone_bulks(inbox.take()).is_empty());
    run_until(&mut service, 4000.0, 7000.0);
    assert_eq!(tone_bulks(inbox.take()).len(), 16);
}

#[test]
fn a_program_change_that_lands_on_a_preset_gets_a_baseline_once_the_tone_is_read() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    rx_read(&mut service, |p| p, &[], 15, 1.0);
    // Part 3 holds a baselined tone to begin with.
    let first = tone_requests(3, params::ToneType::PcmSynth);
    ask(&mut service, a, change_tone(3, 87, 64, 0));
    Device::new(first, 1).run(&mut service, &link, 1.0, 1500.0);
    assert!(service.baseline(3).is_some());
    link.clear();

    service.handle(Input::Note(vec![0xC3, 9]), 2000.0);
    assert!(service.baseline(3).is_none(), "gone at once");
    run_until(&mut service, 2000.0, 2500.0);
    let (dump_at, dump_size) = MixerBlock::Part(3).request().unwrap();
    assert!(
        link.sent()
            .contains(&sysex::build_rq1(DEVICE_ID, &dump_at, &dump_size))
    );

    // The dump says the device is now on an SN Synth preset.
    let mut dump = vec![0u8; dump_size.value() as usize];
    dump[0x00] = 3;
    dump[0x06..0x09].copy_from_slice(&[95, 65, 9]);
    let reply = sysex::build_dt1(DEVICE_ID, &dump_at, &dump);
    service.handle(Input::Link(LinkEvent::Midi(reply)), 2500.0);
    let blocks = tone_requests(3, params::ToneType::SnSynth);
    let mut device = Device::new(blocks, 4);
    device.seen = link.sent().len();
    device.run(&mut service, &link, 2500.0, 5000.0);
    let baseline = service.baseline(3).expect("read");
    assert_eq!(
        (baseline.tone().msb, baseline.tone().lsb, baseline.tone().pc),
        (95, 65, 9)
    );
}

#[test]
fn the_late_dump_of_a_tone_a_select_replaced_is_neither_applied_nor_cached() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    rx_read(&mut service, |p| p, &[], 15, 1.0);
    service.handle(Input::Note(vec![0xC3, 9]), 2.0);
    run_until(&mut service, 2.0, 200.0);
    let (dump_at, dump_size) = MixerBlock::Part(3).request().unwrap();
    assert!(
        link.sent()
            .contains(&sysex::build_rq1(DEVICE_ID, &dump_at, &dump_size))
    );
    // A select replaces the tone while the dump is on its way.
    ask(&mut service, a, change_tone(3, 87, 0, 2));
    run_until(&mut service, 200.0, 400.0);

    let mut dump = vec![0u8; dump_size.value() as usize];
    dump[0x06..0x09].copy_from_slice(&[95, 65, 9]);
    dump[usize::from(part::LEVEL[2])] = 77;
    let reply = sysex::build_dt1(DEVICE_ID, &dump_at, &dump);
    service.handle(Input::Link(LinkEvent::Midi(reply)), 400.0);
    assert_eq!(service.device().state().parts[3].tone_pc, 2);

    // Its level is not known: a write of that value goes out.
    let before = link.sent().len();
    ask(&mut service, a, write_level(3, 77));
    run_until(&mut service, 400.0, 600.0);
    assert!(link.sent().len() > before);
}

// --- Session boundaries -----------------------------------------------------

#[test]
fn a_session_boundary_rejects_what_clients_were_waiting_for() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    ask(
        &mut service,
        a,
        Request::Read {
            param: level(0),
            binding: Binding::ANY,
        },
    );
    service.pump(0.0); // the read is out
    ask(&mut service, a, write_level(1, 10));
    ask(
        &mut service,
        a,
        Request::Push {
            requests: vec![write_level(2, 10)],
        },
    );
    ask(&mut service, a, change_tone(3, 87, 64, 0));
    service.pump(40.0);
    inbox.take();

    service.handle(Input::Link(LinkEvent::Down), 100.0);
    let heard = inbox.take();
    let refused = heard
        .iter()
        .filter(|m| {
            matches!(
                m,
                Message::Rejected {
                    reason: Reject::LinkDown,
                    ..
                }
            )
        })
        .count();
    assert!(refused >= 3, "the read, the push and the select: {heard:?}");
    assert!(heard.contains(&Message::LinkDown));
}

#[test]
fn a_typed_read_times_out_from_the_moment_it_left_not_from_when_it_was_queued() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    let (b, _b_inbox) = client(&mut service, 2);
    // b keeps the device busy for 3 seconds with raw writes.
    for p in 0..16 {
        for off in [part::LEVEL, part::PAN, part::CHORUS_SEND, part::REVERB_SEND] {
            ask(&mut service, b, raw(part_dt1(p, off, 1)));
        }
    }
    ask(
        &mut service,
        a,
        Request::Read {
            param: level(0),
            binding: Binding::ANY,
        },
    );
    run_until(&mut service, 0.0, 1990.0);
    let timed_out = inbox.take().into_iter().any(|m| {
        matches!(
            m,
            Message::Rejected {
                reason: Reject::Timeout,
                ..
            }
        )
    });
    assert!(!timed_out);
    run_until(&mut service, 1990.0, 6000.0);
    let timed_out = inbox.take().into_iter().any(|m| {
        matches!(
            m,
            Message::Rejected {
                reason: Reject::Timeout,
                ..
            }
        )
    });
    assert!(timed_out, "two seconds after it left");
}

// --- What the preset cache must not keep ------------------------------------

/// Whether selecting the preset `tone` on `part_index` makes the service read
/// it, run from `from` for a second.
fn preset_is_read(
    service: &mut Service,
    link: &FakeLink,
    client: ClientId,
    part_index: u8,
    from: f64,
) -> bool {
    let mut device = Device::new(tone_requests(part_index, params::ToneType::PcmSynth), 1);
    device.seen = link.sent().len();
    ask(service, client, change_tone(part_index, 87, 64, 5));
    device.run(service, link, from, from + 1500.0);
    !device.asked.is_empty()
}

#[test]
fn a_preset_read_while_another_client_wrote_into_it_is_not_kept() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _a_inbox) = client(&mut service, 1);
    let (b, _b_inbox) = client(&mut service, 2);
    let blocks = tone_requests(2, params::ToneType::PcmSynth);
    let mut device = Device::new(blocks.clone(), 9);
    ask(&mut service, a, change_tone(2, 87, 64, 5));
    // Between its blocks, b writes a byte of the tone.
    ask(
        &mut service,
        b,
        raw(sysex::build_dt1(DEVICE_ID, &blocks[1].0, &[3])),
    );
    device.run(&mut service, &link, 0.0, 3000.0);
    assert!(service.baseline(2).is_some(), "it still serves the push");

    // The next Part to select the preset reads it again.
    assert!(preset_is_read(&mut service, &link, a, 3, 3000.0));
}

#[test]
fn a_preset_read_while_a_program_change_may_have_reached_the_part_is_not_kept() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    let blocks = tone_requests(2, params::ToneType::PcmSynth);
    let mut device = Device::new(blocks, 9);
    ask(&mut service, a, change_tone(2, 87, 64, 5));
    device.run(&mut service, &link, 0.0, 250.0);
    assert!(service.baseline(2).is_none(), "still being read");
    // The receive settings are unread, so the Part may be reached.
    service.handle(Input::Note(vec![0xC9, 1]), 250.0);
    device.run(&mut service, &link, 250.0, 3000.0);
    assert!(service.baseline(2).is_some());

    assert!(preset_is_read(&mut service, &link, a, 3, 3000.0));
}

#[test]
fn an_undisturbed_preset_is_kept() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    assert!(preset_is_read(&mut service, &link, a, 2, 0.0));
    assert!(!preset_is_read(&mut service, &link, a, 3, 2000.0));
}

#[test]
fn an_expansion_tone_gets_no_baseline_unless_its_board_is_known_to_be_loaded() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    ask(&mut service, a, change_tone(0, 93, 0, 1)); // SRX-01
    run_until(&mut service, 0.0, 500.0);
    // The select and the name and category reads; nothing is read back.
    assert_eq!(sysex_sent(&link).len(), 3);
    assert!(service.baseline(0).is_none());
}

#[test]
fn the_receive_settings_read_is_not_taken_for_the_part_dump_nor_for_a_stale_one() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    rx_read(&mut service, |p| p, &[], 15, 1.0);
    service.handle(Input::Note(vec![0xC3, 9]), 2.0);
    run_until(&mut service, 2.0, 200.0);
    let (dump_at, dump_size) = MixerBlock::Part(3).request().unwrap();
    let before = link.sent().len();

    // A 58-byte read of the same address says an SN Synth preset: it is
    // the receive settings, not the dump asked for.
    let mut settings = vec![0u8; 58];
    settings[0] = 12;
    settings[57] = 1;
    settings[6..9].copy_from_slice(&[95, 65, 9]);
    let reply = sysex::build_dt1(DEVICE_ID, &dump_at, &settings);
    service.handle(Input::Link(LinkEvent::Midi(reply)), 300.0);
    run_until(&mut service, 300.0, 500.0);
    assert_eq!(link.sent().len(), before, "no baseline was started");

    // ... and it was not eaten as the late reply to a dropped dump.
    ask(&mut service, a, change_tone(3, 87, 0, 2));
    run_until(&mut service, 500.0, 700.0);
    let reply = sysex::build_dt1(DEVICE_ID, &dump_at, &settings);
    service.handle(Input::Link(LinkEvent::Midi(reply)), 700.0);
    inbox.take();
    service.handle(Input::Note(vec![0xC3, 1]), 800.0);
    assert!(
        !tone_bulks(inbox.take()).contains(&Some(3)),
        "Part 4 now receives on channel 13"
    );
    service.handle(Input::Note(vec![0xCC, 1]), 900.0);
    assert!(tone_bulks(inbox.take()).contains(&Some(3)));
    let _ = dump_size;
}

#[test]
fn while_the_studio_set_control_channel_is_unread_a_program_change_clears_the_cache() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    ask(&mut service, a, write_level(1, 50));
    run_until(&mut service, 0.0, 100.0);
    ask(&mut service, a, write_level(1, 50));
    run_until(&mut service, 100.0, 200.0);
    assert_eq!(
        sysex_sent(&link)
            .iter()
            .filter(|m| **m == level_dt1(1, 50))
            .count(),
        1,
        "dropped as unchanged"
    );
    // It may select a Studio Set, which replaces everything.
    service.handle(Input::Note(vec![0xC0, 1]), 300.0);
    ask(&mut service, a, write_level(1, 50));
    run_until(&mut service, 300.0, 1500.0);
    assert_eq!(
        sysex_sent(&link)
            .iter()
            .filter(|m| **m == level_dt1(1, 50))
            .count(),
        2
    );
}

#[test]
fn a_receive_settings_read_does_not_make_the_select_bytes_seen() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, _inbox) = client(&mut service, 1);
    rx_read(&mut service, |p| p, &[], 15, 1.0);
    // Only the Program Change byte of Part 3 is written, by a raw DT1: the
    // bank bytes were never seen, so it goes out as written.
    let pc_at = params::part_address(2, part::TONE_PC);
    let bytes = sysex::build_dt1(DEVICE_ID, &pc_at, &[9]);
    ask(&mut service, a, raw(bytes.clone()));
    run_until(&mut service, 1.0, 300.0);
    assert!(sysex_sent(&link).contains(&bytes));
    let select = params::part_address(2, part::TONE_BANK_MSB);
    let completed = sysex::build_dt1(DEVICE_ID, &select, &[0, 0, 9]);
    assert!(!sysex_sent(&link).contains(&completed));
}

fn bound(plugin: PluginKind, binding: Binding) -> Request {
    Request::Bind {
        bound: Some(integral_device_service::protocol::Bound { plugin, binding }),
    }
}

/// The counts a client was told of its duplicates, in order.
fn shared(messages: Vec<Message>) -> Vec<u32> {
    messages
        .into_iter()
        .filter_map(|m| match m {
            Message::Shared { others } => Some(others),
            _ => None,
        })
        .collect()
}

#[test]
fn clients_on_one_target_hear_of_each_other_and_stop_when_one_leaves_or_moves() {
    let (mut service, _link) = service();
    let (a, a_inbox) = client(&mut service, 1);
    let (b, b_inbox) = client(&mut service, 2);
    let (c, c_inbox) = client(&mut service, 3);
    let part = Binding::of_part(2);

    // A lone instance hears nothing, not even a zero.
    ask(&mut service, a, bound(PluginKind::PcmSynth, part));
    assert_eq!(shared(a_inbox.take()), Vec::<u32>::new());

    ask(&mut service, b, bound(PluginKind::PcmSynth, part));
    assert_eq!(shared(a_inbox.take()), [1]);
    assert_eq!(shared(b_inbox.take()), [1]);

    // A third makes two others for each. Another plugin on the Part is no
    // duplicate: it cannot match the Part's tone type as well.
    let (d, d_inbox) = client(&mut service, 4);
    ask(&mut service, d, bound(PluginKind::SnSynth, part));
    assert_eq!(shared(d_inbox.take()), Vec::<u32>::new());
    assert_eq!(shared(a_inbox.take()), Vec::<u32>::new());
    ask(&mut service, c, bound(PluginKind::PcmSynth, part));
    assert_eq!(shared(a_inbox.take()), [2]);
    assert_eq!(shared(b_inbox.take()), [2]);
    assert_eq!(shared(c_inbox.take()), [2]);

    // Moving away, and leaving, are both a change for those left.
    ask(
        &mut service,
        c,
        bound(PluginKind::SnDrum, Binding::of_part(3)),
    );
    assert_eq!(shared(a_inbox.take()), [1]);
    assert_eq!(shared(b_inbox.take()), [1]);
    assert_eq!(shared(c_inbox.take()), [0]);
    service.handle(Input::Disconnect { client: b }, 1.0);
    assert_eq!(shared(a_inbox.take()), [0]);
}

#[test]
fn a_key_is_not_the_kit_and_the_studio_set_plugins_share_among_themselves() {
    let (mut service, _link) = service();
    let (a, a_inbox) = client(&mut service, 1);
    let (b, b_inbox) = client(&mut service, 2);
    let (c, c_inbox) = client(&mut service, 3);
    let part = Binding::of_part(0);
    ask(&mut service, a, bound(PluginKind::PcmDrum, part));
    ask(
        &mut service,
        b,
        bound(PluginKind::PcmDrum, part.with_key(36)),
    );
    ask(&mut service, c, bound(PluginKind::StudioSet, Binding::ANY));
    assert_eq!(shared(a_inbox.take()), Vec::<u32>::new());
    assert_eq!(shared(b_inbox.take()), Vec::<u32>::new());
    assert_eq!(shared(c_inbox.take()), Vec::<u32>::new());
    ask(
        &mut service,
        b,
        bound(PluginKind::PcmDrum, part.with_key(36)),
    );
    assert_eq!(shared(b_inbox.take()), Vec::<u32>::new());
    let (d, d_inbox) = client(&mut service, 4);
    ask(&mut service, d, bound(PluginKind::StudioSet, Binding::ANY));
    assert_eq!(shared(c_inbox.take()), [1]);
    assert_eq!(shared(d_inbox.take()), [1]);
}

#[test]
fn a_client_that_binds_again_is_told_its_count_again_in_case_it_lost_it() {
    let (mut service, _link) = service();
    let (a, a_inbox) = client(&mut service, 1);
    let (b, _b_inbox) = client(&mut service, 2);
    let target = Binding::of_part(1);
    ask(&mut service, a, bound(PluginKind::SnSynth, target));
    ask(&mut service, b, bound(PluginKind::SnSynth, target));
    assert_eq!(shared(a_inbox.take()), [1]);
    ask(&mut service, a, bound(PluginKind::SnSynth, target));
    assert_eq!(shared(a_inbox.take()), [1]);
}

/// A client whose outbox refuses everything once told to.
#[derive(Clone, Default)]
struct Refusing(Arc<Mutex<(Vec<Message>, bool)>>);

impl Outbox for Refusing {
    fn deliver(&mut self, message: &Message) -> bool {
        let mut inner = self.0.lock().unwrap();
        if inner.1 {
            return false;
        }
        inner.0.push(message.clone());
        true
    }
}

#[test]
fn a_client_dropped_for_not_keeping_up_stops_being_a_duplicate() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, a_inbox) = client(&mut service, 1);
    let (b, outbox) = (ClientId(2), Refusing::default());
    service.handle(
        Input::Connect {
            client: b,
            outbox: Box::new(outbox.clone()),
        },
        0.0,
    );
    ask(
        &mut service,
        b,
        Request::Hello {
            version: PROTOCOL_VERSION,
            subscriptions: vec![Binding::ANY],
            raw: false,
            bound: None,
        },
    );
    let target = Binding::of_part(5);
    ask(&mut service, a, bound(PluginKind::PcmSynth, target));
    ask(&mut service, b, bound(PluginKind::PcmSynth, target));
    assert_eq!(shared(a_inbox.take()), [1]);

    // The service drops b when it cannot deliver a change; a hears of it.
    outbox.0.lock().unwrap().1 = true;
    ask(&mut service, a, write_level(0, 90));
    service.pump(0.0);
    assert_eq!(shared(a_inbox.take()), [0]);
}

#[test]
fn a_tone_selected_through_the_service_is_announced_with_the_tone_and_a_parameter_bulk_is_not_a_selection()
 {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    ask(
        &mut service,
        a,
        Request::ChangeTone {
            part: 4,
            msb: 87,
            lsb: 64,
            pc: 9,
        },
    );
    service.pump(0.0);
    let bulks: Vec<_> = inbox
        .take()
        .into_iter()
        .filter_map(|m| match m {
            Message::Bulk {
                kind: BulkKind::Tone,
                selected,
                tone,
                ..
            } => Some((selected, tone)),
            _ => None,
        })
        .collect();
    assert_eq!(
        bulks,
        [(
            true,
            Some(ToneSelect {
                msb: 87,
                lsb: 64,
                pc: 9
            })
        )]
    );
}

/// The bulks of tone selections `inbox` was told of: (Part, tone).
fn tone_selections(messages: Vec<Message>) -> Vec<(u8, Option<ToneSelect>)> {
    messages
        .into_iter()
        .filter_map(|m| match m {
            Message::Bulk {
                kind: BulkKind::Tone,
                binding,
                selected: true,
                tone,
                ..
            } => Some((binding.part.expect("a Part"), tone)),
            _ => None,
        })
        .collect()
}

#[test]
fn after_a_studio_set_switch_each_part_is_read_and_announced_with_its_tone() {
    let (mut service, link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);

    // The device is the new Studio Set's: its Part 2 holds an SN Synth tone.
    // Nothing is named until the service has read it.
    ask(&mut service, a, Request::SwitchStudioSet { pc: 4 });
    let mut device = Device::new(vec![MixerBlock::Part(2).request().expect("a block")], 95);
    device.run(&mut service, &link, 200.0, 2500.0);
    let told = tone_selections(inbox.take());
    let part_2: Vec<_> = told.iter().filter(|(part, _)| *part == 2).collect();
    assert_eq!(part_2.len(), 1, "{told:?}");
    assert_eq!(part_2[0].1.map(|tone| tone.msb), Some(95), "{told:?}");
}

#[test]
fn a_partial_select_write_after_a_studio_set_switch_goes_out_blind() {
    let (mut service, _link) = service();
    service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
    let (a, inbox) = client(&mut service, 1);
    ask(&mut service, a, change_tone(3, 87, 64, 0));
    run_until(&mut service, 0.0, 200.0);
    ask(&mut service, a, Request::SwitchStudioSet { pc: 4 });
    run_until(&mut service, 200.0, 300.0);
    inbox.take();

    // Only the program number: the bank is whatever the new set has.
    let pc_at = params::part_address(3, part::TONE_PC);
    let bytes = sysex::build_dt1(DEVICE_ID, &pc_at, &[9]);
    ask(&mut service, a, Request::Raw { bytes });
    run_until(&mut service, 300.0, 30000.0);
    // The blind write is announced first, without a tone; the reads that
    // nobody answered fail after it.
    let told = tone_selections(inbox.take());
    assert_eq!(told.first(), Some(&(3, None)), "{told:?}");
    assert!(told.iter().all(|(_, tone)| tone.is_none()), "{told:?}");
}

mod expansion_load {
    use integral_core::expansion_load::{self, LOAD_DONE_ADDR, LOAD_STARTED_ADDR};
    use integral_core::expansion_slots::{self, ExpansionSlots, SLOT_READ_ADDR};

    use super::*;

    fn current() -> ExpansionSlots {
        ExpansionSlots::from_values(&[0x03, 0x05, 0x07, 0x08]).unwrap()
    }

    fn target() -> ExpansionSlots {
        current().with(0, None)
    }

    fn load(slots: ExpansionSlots) -> Request {
        Request::LoadExpansion {
            slots: slots.values(),
        }
    }

    fn device_says(service: &mut Service, address: &Address, data: &[u8], now: f64) {
        let dt1 = sysex::build_dt1(DEVICE_ID, address, data);
        service.handle(Input::Link(LinkEvent::Midi(dt1)), now);
    }

    fn bulks(messages: &[Message]) -> Vec<(ClientId, BulkKind)> {
        messages
            .iter()
            .filter_map(|m| match m {
                Message::Bulk { writer, kind, .. } => Some((*writer, *kind)),
                _ => None,
            })
            .collect()
    }

    fn loaded(messages: &[Message]) -> bool {
        messages
            .iter()
            .any(|m| matches!(m, Message::ExpansionLoadDone { .. }))
    }

    /// A service up, whose slots are known, and two clients subscribed to
    /// everything.
    fn up() -> (Service, FakeLink, (ClientId, Inbox), (ClientId, Inbox)) {
        let (mut service, link) = service();
        service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
        service.device_mut().set_expansion_slots(current());
        let a = client(&mut service, 1);
        let b = client(&mut service, 2);
        (service, link, a, b)
    }

    #[test]
    fn the_queue_is_held_until_the_load_is_done_and_the_requester_is_told() {
        let (mut service, link, (a, a_inbox), (b, b_inbox)) = up();
        ask(&mut service, a, load(target()));
        ask(&mut service, b, write_level(0, 10));

        service.pump(0.0);
        let request = expansion_load::build_load_request(DEVICE_ID, &target());
        assert_eq!(link.sent(), [request], "the load request first");
        device_says(&mut service, &LOAD_STARTED_ADDR, &[0], 70.0);
        run_until(&mut service, 70.0, 3_000.0);
        let level = level_dt1(0, 10);
        let poll = expansion_slots::build_read_request(DEVICE_ID);
        assert!(
            link.sent()[1..].iter().all(|m| *m == poll),
            "only the load's polls during the load: {:02X?}",
            link.sent()
        );
        assert!(!loaded(&a_inbox.take()));

        device_says(&mut service, &LOAD_DONE_ADDR, &[0], 3_000.0);
        run_until(&mut service, 3_000.0, 3_100.0);
        assert!(link.sent().contains(&level), "the queue runs again");
        assert_eq!(service.device().expansion_slots(), Some(&target()));

        let told = a_inbox.take();
        assert!(
            told.contains(&Message::ExpansionLoadDone {
                result: ExpansionLoadResult::Loaded,
                slots: Some(target().values())
            }),
            "{told:?}"
        );
        assert_eq!(bulks(&told), [(a, BulkKind::StudioSet)]);
        assert_eq!(bulks(&b_inbox.take()), [(a, BulkKind::StudioSet)]);
    }

    #[test]
    fn a_load_the_device_stops_answering_ends_timed_out() {
        let (mut service, _link, (a, a_inbox), (_, b_inbox)) = up();
        ask(&mut service, a, load(target()));
        run_until(&mut service, 0.0, 120_000.0);

        let told = a_inbox.take();
        assert!(
            told.contains(&Message::ExpansionLoadDone {
                result: ExpansionLoadResult::TimedOut,
                slots: None
            }),
            "{told:?}"
        );
        assert_eq!(service.device().expansion_slots(), None);
        assert_eq!(bulks(&b_inbox.take()), [(a, BulkKind::StudioSet)]);
    }

    #[test]
    fn a_load_of_what_the_slots_hold_answers_at_once_and_announces_nothing() {
        let (mut service, link, (a, a_inbox), (_, b_inbox)) = up();
        ask(&mut service, a, load(current()));
        service.pump(0.0);
        assert!(link.sent().is_empty());
        assert_eq!(
            a_inbox.take(),
            [Message::ExpansionLoadDone {
                result: ExpansionLoadResult::Unchanged,
                slots: Some(current().values())
            }]
        );
        assert!(bulks(&b_inbox.take()).is_empty());
    }

    #[test]
    fn unknown_slots_are_read_before_the_load() {
        let (mut service, link) = service();
        service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
        let (a, a_inbox) = client(&mut service, 1);
        ask(&mut service, a, load(target()));

        service.pump(0.0);
        let read = expansion_slots::build_read_request(DEVICE_ID);
        assert_eq!(link.sent(), [read]);
        device_says(&mut service, &SLOT_READ_ADDR, &current().values(), 50.0);
        service.pump(50.0);
        let request = expansion_load::build_load_request(DEVICE_ID, &target());
        assert_eq!(link.sent()[1..], [request]);
        assert_eq!(service.device().expansion_load_target(), Some(&target()));
        assert!(a_inbox.take().is_empty());
    }

    #[test]
    fn a_slot_read_the_device_never_answers_is_refused_with_a_timeout() {
        let (mut service, link) = service();
        service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
        let (a, a_inbox) = client(&mut service, 1);
        ask(&mut service, a, load(target()));
        run_until(&mut service, 0.0, 10_000.0);

        assert_eq!(link.sent().len(), 1, "the slot read only");
        assert_eq!(
            a_inbox.take(),
            [Message::Rejected {
                reason: Reject::Timeout,
                param: None
            }]
        );
        assert_eq!(service.device().expansion_load_target(), None);
    }

    #[test]
    fn a_second_load_while_one_runs_is_refused() {
        let (mut service, _link, (a, a_inbox), (b, b_inbox)) = up();
        ask(&mut service, a, load(target()));
        service.pump(0.0);
        ask(&mut service, b, load(current()));
        service.pump(100.0);
        assert!(
            matches!(
                b_inbox.take()[..],
                [Message::Rejected {
                    reason: Reject::Invalid { .. },
                    ..
                }]
            ),
            "refused"
        );
        assert!(a_inbox.take().is_empty());
        assert_eq!(service.device().expansion_load_target(), Some(&target()));
    }

    #[test]
    fn slots_that_name_no_expansion_are_refused_on_arrival() {
        let (mut service, link, (a, a_inbox), _) = up();
        let bad = Request::LoadExpansion {
            slots: [0x7F, 0, 0, 0],
        };
        ask(&mut service, a, bad);
        service.pump(0.0);
        assert!(link.sent().is_empty());
        assert!(
            matches!(
                a_inbox.take()[..],
                [Message::Rejected {
                    reason: Reject::Invalid { .. },
                    ..
                }]
            ),
            "refused"
        );
    }

    #[test]
    fn a_load_whose_client_left_ends_as_the_devices() {
        let (mut service, _link, (a, _), (_, b_inbox)) = up();
        ask(&mut service, a, load(target()));
        service.pump(0.0);
        service.handle(Input::Disconnect { client: a }, 10.0);
        device_says(&mut service, &LOAD_STARTED_ADDR, &[0], 70.0);
        device_says(&mut service, &LOAD_DONE_ADDR, &[0], 100.0);
        run_until(&mut service, 100.0, 200.0);
        assert_eq!(service.device().expansion_slots(), Some(&target()));
        assert_eq!(
            bulks(&b_inbox.take()),
            [(ClientId::DEVICE, BulkKind::StudioSet)]
        );
    }

    #[test]
    fn a_load_waiting_on_the_slots_is_refused_when_the_link_goes_down() {
        let (mut service, _link) = service();
        service.handle(Input::Link(LinkEvent::Up { device_id: None }), 0.0);
        let (a, a_inbox) = client(&mut service, 1);
        ask(&mut service, a, load(target()));
        service.pump(0.0);
        service.handle(Input::Link(LinkEvent::Down), 10.0);
        let refused = Message::Rejected {
            reason: Reject::LinkDown,
            param: None,
        };
        assert!(a_inbox.take().contains(&refused));
    }
}
