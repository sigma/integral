//! The service against a fake link that records what it would send.

use std::time::{Duration, Instant};

use integral_core::params::{self, part};
use integral_core::sysex;
use integral_device_service::{
    Config, FakeLink, Input, LinkEvent, SequentialSessionIds, Service, SessionId, spawn,
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
