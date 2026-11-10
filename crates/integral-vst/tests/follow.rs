//! Host parameters follow the device: instances against a real Device
//! service on a fake link.
//!
//! The host is played from two sides. Its changes are `ParamBank::host_changed`
//! (what a parameter's callback does) followed by a `process()`'s
//! [`Outbound::poll`]; the editor's context is a [`Recorder`] that logs what a
//! host would hear (begin, perform, end, one bulk replace) and applies the
//! value the way the wrapper does. nih-plug gives a plugin no other way to
//! set a parameter, so a test cannot watch a real `IComponentHandler`
//! without opening the editor window.

#![cfg(unix)]

use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use integral_core::host_params::{Binding, HostParam, PluginKind};
use integral_core::params::{self, ToneType, part};
use integral_core::sysex;
use integral_core::tone_banks::default_tone;
use integral_device_service::protocol::{ClientId, ParamId, Request};
use integral_device_service::{Client, ClientEvent, Events};
use integral_vst::family::host::{self, HostEdits, HostTask, Replace};
use integral_vst::family::link::Connection;
use integral_vst::family::out::Outbound;
use integral_vst::family::{Instance, Notice, Status, ToneFit};
use nih_plug::prelude::IntParam;

mod common;

use common::{Memory, TempDir, TestService, WAIT, is_rq1, wait_for};

/// What the host would hear.
#[derive(Debug, Clone, PartialEq)]
enum Edit {
    Begin(String),
    Perform(String, i32),
    End(String),
    Replace(Vec<(String, i32)>),
}

/// The editor's context, for a test: logs the edits and applies values like
/// the wrapper does.
struct Recorder {
    instance: Weak<Instance>,
    log: Mutex<Vec<(Instant, Edit)>>,
    /// The host does not take a bulk replace (the audio thread is not
    /// picking states up).
    refuse: std::sync::atomic::AtomicBool,
}

impl Recorder {
    fn id_of(&self, param: &IntParam) -> (Arc<Instance>, usize) {
        let instance = self.instance.upgrade().expect("the instance");
        let i = instance
            .bank
            .list()
            .iter()
            .position(|(_, p)| std::ptr::eq(p, param))
            .expect("one of the instance's parameters");
        (instance, i)
    }

    fn push(&self, edit: Edit) {
        self.log.lock().unwrap().push((Instant::now(), edit));
    }

    fn log(&self) -> Vec<(Instant, Edit)> {
        self.log.lock().unwrap().clone()
    }

    fn edits(&self) -> Vec<Edit> {
        self.log().into_iter().map(|(_, e)| e).collect()
    }

    fn clear(&self) {
        self.log.lock().unwrap().clear();
    }
}

impl HostEdits for Recorder {
    fn begin(&self, param: &IntParam) {
        let (instance, i) = self.id_of(param);
        self.push(Edit::Begin(instance.bank.list()[i].0.id.clone()));
    }

    fn perform(&self, param: &IntParam, value: i32) {
        let (instance, i) = self.id_of(param);
        instance.bank.host_changed(i, value);
        self.push(Edit::Perform(instance.bank.list()[i].0.id.clone(), value));
    }

    fn end(&self, param: &IntParam) {
        let (instance, i) = self.id_of(param);
        self.push(Edit::End(instance.bank.list()[i].0.id.clone()));
    }

    fn replace(&self, changes: &[Replace<'_>]) -> bool {
        if self.refuse.load(std::sync::atomic::Ordering::SeqCst) {
            self.push(Edit::Replace(Vec::new()));
            return false;
        }
        let instance = self.instance.upgrade().expect("the instance");
        let mut seen = Vec::new();
        for change in changes {
            let i = instance
                .bank
                .position(&change.host.id)
                .expect("in the list");
            instance.bank.host_changed(i, change.value);
            seen.push((change.host.id.clone(), change.value));
        }
        self.push(Edit::Replace(seen));
        true
    }
}

/// An armed PCM Synth instance bound to Part 0 on a service, with a device
/// that answers reads.
struct Rig {
    // Dropped in this order: the instance's side first.
    outbound: Outbound,
    _connection: Connection,
    instance: Arc<Instance>,
    recorder: Arc<Recorder>,
    tasks: Option<Arc<Mutex<std::collections::VecDeque<HostTask>>>>,
    /// How many read requests the first read took.
    first_reads: usize,
    memory: Memory,
    service: TestService,
    dir: TempDir,
}

const BINDING: Binding = Binding::of_part(0);
const LINK_DEVICE_ID: u8 = 0x10;

/// How a [`Rig`] is set up.
#[derive(Clone, Copy)]
struct Setup {
    /// The host can be told of the device's changes (an editor is open).
    editor: bool,
    /// Host tasks wait in a queue for the test to run them, as they wait for
    /// the host's main thread; else they run on the worker.
    ring: bool,
    /// The host restored a state into the instance.
    restored: bool,
    /// The type of tone the bound Part holds on the device.
    holds: ToneType,
}

const OPEN: Setup = Setup {
    editor: true,
    ring: false,
    restored: false,
    holds: ToneType::PcmSynth,
};

/// Where Part `part` has its tone select on the device.
fn select_address(part: u8) -> integral_core::address::Address {
    params::part_address(part, part::TONE_BANK_MSB)
}

/// Put a tone of `holds` in `memory` as Part `part`'s.
fn put_tone(memory: &Memory, part: u8, holds: ToneType) {
    let tone = default_tone(holds).expect("a default tone");
    memory.put(select_address(part), &[tone.msb, tone.lsb, tone.pc]);
}

impl Rig {
    fn new() -> Self {
        Self::with(OPEN)
    }

    fn with(setup: Setup) -> Self {
        let dir = TempDir::new();
        let service = TestService::start(&dir.socket());
        let memory = Memory::answer(&service.link);
        put_tone(&memory, 0, setup.holds);
        let instance = Arc::new(Instance::new(PluginKind::PcmSynth));
        let recorder = Arc::new(Recorder {
            instance: Arc::downgrade(&instance),
            log: Mutex::new(Vec::new()),
            refuse: Default::default(),
        });
        let tasks = setup.ring.then(|| {
            let queue = Arc::new(Mutex::new(std::collections::VecDeque::new()));
            let posted = Arc::clone(&queue);
            instance.set_poster(integral_vst::family::instance::Poster::new(move |task| {
                posted.lock().unwrap().push_back(task);
                true
            }));
            queue
        });
        if setup.editor {
            instance.bank.set_edits(Some(recorder.clone()));
        }
        if setup.restored {
            // A saved tone makes the state one the recall pushes: the
            // project's values stand until it has.
            instance
                .tone
                .set(Some(integral_vst::family::tone::SavedTone::new(
                    87, 0, 0, "",
                )));
            instance.mark_restored();
        }
        instance.settle();
        let connection = Connection::start(dir.socket(), Arc::clone(&instance));
        let outbound = Outbound::new(connection.sender().expect("a sender"));
        wait_for("the service", || {
            instance.status.get() == Status::DeviceOffline
        });
        service.link.bring_up(Some(LINK_DEVICE_ID));
        wait_for("a Part and the first read", || {
            instance.binding.get() == Some(BINDING)
                && if setup.holds == ToneType::PcmSynth {
                    instance.is_armed()
                } else {
                    instance.fit() == ToneFit::Mismatch
                }
        });
        // The blocks of the tone, not the question which tone the Part holds.
        let first_reads = service
            .link
            .sent()
            .iter()
            .filter(|m| is_rq1(m) && m[7..11] != *select_address(0).as_bytes())
            .count();
        // The first read applied the device's values (zeros) as a bulk.
        recorder.clear();
        service.link.clear();
        let rig = Self {
            outbound,
            _connection: connection,
            instance,
            recorder,
            tasks,
            first_reads,
            memory,
            service,
            dir,
        };
        // What the first read posted has been run by now, as the main thread
        // would.
        rig.run_tasks();
        rig.recorder.clear();
        rig
    }

    /// Run the host tasks the worker has posted, as the main thread would.
    fn run_tasks(&self) -> Vec<HostTask> {
        let mut ran = Vec::new();
        if let Some(tasks) = &self.tasks {
            while let Some(task) = tasks.lock().unwrap().pop_front() {
                integral_vst::family::host::run(&self.instance, task);
                ran.push(task);
            }
        }
        ran
    }

    /// Position and definition of the parameter named `name`.
    fn param(&self, name: &str) -> (usize, HostParam) {
        let (i, (host, _)) = self
            .instance
            .bank
            .list()
            .iter()
            .enumerate()
            .find(|(_, (h, _))| h.name == name)
            .unwrap_or_else(|| panic!("no parameter named {name}"));
        (i, host.clone())
    }

    /// The host sets parameter `i`, then the audio thread looks.
    fn host_sets(&mut self, i: usize, value: i32) {
        self.instance.bank.host_changed(i, value);
        self.outbound.poll(&self.instance);
    }

    /// The DT1s the link has carried, as (address value, data).
    fn dt1s(&self) -> Vec<(u32, Vec<u8>)> {
        self.service
            .link
            .sent()
            .iter()
            .filter_map(|m| sysex::parse_dt1(m).ok())
            .map(|d| (d.address.value(), d.data))
            .collect()
    }

    fn dt1_at(&self, host: &HostParam) -> Vec<Vec<u8>> {
        let at = host.device_address(&BINDING).expect("an address").value();
        self.dt1s()
            .into_iter()
            .filter(|(a, _)| *a == at)
            .map(|(_, d)| d)
            .collect()
    }

    /// A second client of the service, as the web UI or the CLI would be.
    fn other_client(&self) -> (Client, Events, ClientId) {
        let (client, events) = Client::connect(self.dir.socket());
        let id = loop {
            match events.wait_event(WAIT) {
                Some(ClientEvent::Connected { client, .. }) => break client,
                Some(_) => {}
                None => panic!("the other client never connected"),
            }
        };
        (client, events, id)
    }
}

fn param_id(host: &HostParam) -> ParamId {
    ParamId {
        plugin: PluginKind::PcmSynth,
        id: host.id.clone(),
    }
}

const CUTOFF: &str = "Partial 1 TVF Cutoff Frequency";

#[test]
fn a_host_change_is_one_write_of_that_field_on_the_bound_part() {
    let mut rig = Rig::new();
    let (i, host) = rig.param(CUTOFF);
    rig.host_sets(i, host.max);
    wait_for("the write", || !rig.dt1_at(&host).is_empty());
    assert_eq!(rig.dt1_at(&host), [host.encode(host.max)]);
    // Only that field went out.
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(rig.dt1s().len(), 1, "{:?}", rig.dt1s());
    assert_eq!(
        rig.recorder.edits(),
        [],
        "the host's own change is no gesture"
    );
}

#[test]
fn another_clients_single_write_is_a_gesture_that_ends_after_a_quiet_300_ms() {
    let mut rig = Rig::new();
    let (i, host) = rig.param(CUTOFF);
    let (other, _events, _id) = rig.other_client();

    other.write(param_id(&host), BINDING, host.max - 1);
    wait_for("the first perform", || rig.recorder.edits().len() >= 2);
    other.write(param_id(&host), BINDING, host.max);
    wait_for("the end", || {
        rig.recorder.edits().last() == Some(&Edit::End(host.id.clone()))
    });
    let log = rig.recorder.log();
    let edits: Vec<_> = log.iter().map(|(_, e)| e.clone()).collect();
    assert_eq!(
        edits,
        [
            Edit::Begin(host.id.clone()),
            Edit::Perform(host.id.clone(), host.max - 1),
            Edit::Perform(host.id.clone(), host.max),
            Edit::End(host.id.clone()),
        ],
        "one gesture around both changes"
    );
    let quiet = log[3].0.duration_since(log[2].0);
    assert!(
        quiet >= Duration::from_millis(280) && quiet < Duration::from_millis(900),
        "ended {quiet:?} after the last change"
    );
    assert_eq!(rig.instance.bank.value(i), host.max);

    // The host's value is the device's now: looking again writes nothing
    // back (the echo), and the other client's write is the only one.
    let before = rig.dt1s().len();
    rig.outbound.poll(&rig.instance);
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(rig.dt1s().len(), before);
    assert_eq!(rig.dt1_at(&host).len(), 2, "the other client's two writes");

    // A change after the gesture is a new one.
    rig.recorder.clear();
    other.write(param_id(&host), BINDING, host.max - 5);
    wait_for("the second gesture", || {
        rig.recorder.edits().last() == Some(&Edit::End(host.id.clone()))
    });
    assert_eq!(rig.recorder.edits().len(), 3);
}

#[test]
fn a_write_to_another_part_is_no_gesture() {
    let rig = Rig::new();
    let (_, host) = rig.param(CUTOFF);
    let (other, _events, _id) = rig.other_client();
    other.write(param_id(&host), Binding::of_part(1), host.max);
    // Wait for the write to have gone through the service.
    wait_for("the write", || !rig.service.link.sent().is_empty());
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(rig.recorder.edits(), []);
}

#[test]
fn the_instances_own_writes_and_its_editors_are_ignored() {
    let mut rig = Rig::new();
    let (i, host) = rig.param(CUTOFF);
    rig.host_sets(i, host.max);
    wait_for("the write", || !rig.dt1_at(&host).is_empty());
    // The service announced it with the instance's client ID.
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(rig.recorder.edits(), [], "its own write");

    let (other, _events, other_id) = rig.other_client();
    rig.instance.set_editor_client(Some(other_id));
    other.write(param_id(&host), BINDING, host.max - 3);
    wait_for("the write", || rig.dt1_at(&host).len() == 2);
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(rig.recorder.edits(), [], "its editor's write");
}

#[test]
fn a_bulk_change_replaces_the_values_silently_and_tells_the_host_once() {
    let rig = Rig::new();
    let (i, cutoff) = rig.param(CUTOFF);
    let (_, resonance) = rig.param("Partial 1 TVF Resonance");
    let at = |h: &HostParam| h.device_address(&BINDING).expect("an address");
    rig.memory.put(at(&cutoff), &cutoff.encode(77));
    rig.memory.put(at(&resonance), &resonance.encode(33));

    // Another client loads a tone on the Part (a User PCM Synth tone).
    let (other, _events, _id) = rig.other_client();
    other.send(Request::ChangeTone {
        part: 0,
        msb: 0x57,
        lsb: 0,
        pc: 0,
    });
    wait_for("the bulk update", || !rig.recorder.edits().is_empty());
    wait_for("writing again", || rig.instance.is_armed());
    let edits = rig.recorder.edits();
    assert_eq!(edits.len(), 1, "one notification, no gestures: {edits:?}");
    let Edit::Replace(changes) = &edits[0] else {
        panic!("not a bulk replace: {edits:?}");
    };
    assert!(changes.contains(&(cutoff.id.clone(), 77)), "{changes:?}");
    assert!(changes.contains(&(resonance.id.clone(), 33)), "{changes:?}");
    assert_eq!(rig.instance.bank.value(i), 77);

    // What the device gave is not written back.
    let before = rig.dt1s().len();
    let mut rig = rig;
    rig.outbound.poll(&rig.instance);
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(rig.dt1s().len(), before);
}

#[test]
fn an_inactive_effect_types_parameters_write_nothing_until_the_type_is_theirs() {
    let mut rig = Rig::new();
    let bank = &rig.instance.bank;
    let slot_of = |type_id: u8| {
        bank.list()
            .iter()
            .position(|(h, _)| h.effect.is_some_and(|e| e.type_id == type_id) && h.min < h.max)
            .expect("a slot of that type")
    };
    // The effect is THRU (type 0) after the first read.
    let slot = slot_of(1);
    let host = bank.list()[slot].0.clone();
    let holder = bank.type_slot(slot).expect("a type parameter");
    let holder_host = bank.list()[holder].0.clone();

    rig.host_sets(slot, host.max);
    std::thread::sleep(Duration::from_millis(250));
    assert_eq!(rig.dt1s(), [], "inactive type: nothing on the link");

    // The effect takes the slot's type; the slot then writes.
    rig.host_sets(holder, 1);
    wait_for("the type", || !rig.dt1_at(&holder_host).is_empty());
    rig.host_sets(slot, host.max - 1);
    wait_for("the slot", || !rig.dt1_at(&host).is_empty());
    assert_eq!(rig.dt1_at(&host), [host.encode(host.max - 1)]);
}

#[test]
fn an_unbound_instance_writes_nothing() {
    let dir = TempDir::new();
    let service = TestService::start(&dir.socket());
    let _memory = Memory::answer(&service.link);
    // Not settled: the host may still restore a Binding, so none is taken.
    let instance = Arc::new(Instance::new(PluginKind::PcmSynth));
    let connection = Connection::start(dir.socket(), Arc::clone(&instance));
    let mut outbound = Outbound::new(connection.sender().expect("a sender"));
    wait_for("the service", || {
        instance.status.get() == Status::DeviceOffline
    });
    service.link.bring_up(Some(LINK_DEVICE_ID));
    wait_for("the link", || instance.status.get() == Status::Connected);
    service.link.clear();
    let i = instance
        .bank
        .list()
        .iter()
        .position(|(h, _)| h.name == CUTOFF)
        .expect("cutoff");
    instance.bank.host_changed(i, instance.bank.list()[i].0.max);
    outbound.poll(&instance);
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(service.link.sent(), Vec::<Vec<u8>>::new());
    assert_eq!(instance.binding.get(), None);
}

#[test]
fn the_first_read_is_a_handful_of_blocks_not_a_message_per_parameter() {
    // About 1,400 parameters, read in a few blocks.
    let rig = Rig::new();
    assert!(
        (3..=20).contains(&rig.first_reads),
        "{} read requests for {} parameters",
        rig.first_reads,
        rig.instance.bank.len()
    );
}

#[test]
fn with_the_editor_closed_only_the_effect_types_are_read_and_the_devices_type_gates_writes() {
    let mut rig = Rig::with(Setup {
        editor: false,
        ..OPEN
    });
    // The MFX header's block and the tone's name (which the instance saves);
    // the Part's tone select is the probe's, which is no read of a block.
    assert_eq!(
        rig.first_reads, 2,
        "only the MFX header's block and the tone's name"
    );
    let bank = &rig.instance.bank;
    let slot_of = |type_id: u8| {
        bank.list()
            .iter()
            .position(|(h, _)| h.effect.is_some_and(|e| e.type_id == type_id) && h.min < h.max)
            .expect("a slot of that type")
    };
    let (first, second) = (slot_of(1), slot_of(2));
    let holder = bank.type_slot(first).expect("a type parameter");
    assert_eq!(bank.device_type(holder), Some(0), "read from the device");

    // Another client switches the effect to type 1. Nobody can tell the host
    // (no editor), but the instance knows what the device has.
    let (other, _events, _id) = rig.other_client();
    other.write(param_id(&bank.list()[holder].0), BINDING, 1);
    wait_for("the device's type", || bank.device_type(holder) == Some(1));
    assert_eq!(rig.recorder.edits(), []);
    assert_eq!(bank.value(holder), 0, "the host's value is stale");

    let (one, two) = (bank.list()[first].0.clone(), bank.list()[second].0.clone());
    let away = |h: &HostParam| if h.default == h.max { h.min } else { h.max };
    rig.host_sets(second, away(&two));
    std::thread::sleep(Duration::from_millis(250));
    assert_eq!(
        rig.dt1_at(&two),
        Vec::<Vec<u8>>::new(),
        "type 2 is not the device's"
    );
    rig.host_sets(first, away(&one));
    wait_for("the slot", || !rig.dt1_at(&one).is_empty());
    assert_eq!(rig.dt1_at(&one), [one.encode(away(&one))]);
}

#[test]
fn gestures_are_posted_for_the_main_thread_and_run_there() {
    let rig = Rig::with(Setup { ring: true, ..OPEN });
    let (_, host) = rig.param(CUTOFF);
    let (other, _events, _id) = rig.other_client();
    other.write(param_id(&host), BINDING, host.max);
    let i = rig.instance.bank.position(&host.id).unwrap() as u32;

    let mut ran = Vec::new();
    let deadline = Instant::now() + WAIT;
    while ran.len() < 3 {
        ran.extend(rig.run_tasks());
        assert!(Instant::now() < deadline, "tasks so far: {ran:?}");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        ran,
        [
            HostTask::Begin(i),
            HostTask::Perform(i, host.max),
            HostTask::End(i)
        ]
    );
    // The recorder was called from here, where the tasks ran.
    assert_eq!(
        rig.recorder.edits(),
        [
            Edit::Begin(host.id.clone()),
            Edit::Perform(host.id.clone(), host.max),
            Edit::End(host.id.clone()),
        ]
    );
}

#[test]
fn a_restored_instance_keeps_its_projects_values_on_its_first_read_and_takes_the_devices_later() {
    let rig = Rig::with(Setup {
        restored: true,
        ..OPEN
    });
    let (i, cutoff) = rig.param(CUTOFF);
    let at = cutoff.device_address(&BINDING).expect("an address");
    // The first read changed nothing in the host (the device holds zeros).
    assert_eq!(rig.recorder.edits(), []);
    assert_eq!(rig.instance.bank.value(i), cutoff.default);

    rig.memory.put(at, &cutoff.encode(77));
    let (other, _events, _id) = rig.other_client();
    other.send(Request::ChangeTone {
        part: 0,
        msb: 0x57,
        lsb: 0,
        pc: 0,
    });
    wait_for("the bulk update", || !rig.recorder.edits().is_empty());
    assert_eq!(rig.instance.bank.value(i), 77);
}

#[test]
fn the_gesture_ends_for_every_parameter_on_its_own_clock() {
    let rig = Rig::new();
    let (_, cutoff) = rig.param(CUTOFF);
    let (_, resonance) = rig.param("Partial 1 TVF Resonance");
    let (other, _events, _id) = rig.other_client();
    other.write(param_id(&cutoff), BINDING, cutoff.max);
    std::thread::sleep(Duration::from_millis(150));
    other.write(param_id(&resonance), BINDING, resonance.max);
    wait_for("both ends", || {
        rig.recorder
            .edits()
            .iter()
            .filter(|e| matches!(e, Edit::End(_)))
            .count()
            == 2
    });
    let ends: Vec<_> = rig
        .recorder
        .edits()
        .into_iter()
        .filter_map(|e| match e {
            Edit::End(id) => Some(id),
            _ => None,
        })
        .collect();
    assert_eq!(ends, [cutoff.id.clone(), resonance.id.clone()]);
}

#[test]
fn a_bulk_forgets_the_effect_types_until_the_read_tells_them_again() {
    let mut rig = Rig::with(Setup { ring: true, ..OPEN });
    let instance = Arc::clone(&rig.instance);
    let bank = &instance.bank;
    let slot = bank
        .list()
        .iter()
        .position(|(h, _)| h.effect.is_some_and(|e| e.type_id == 1) && h.min < h.max)
        .expect("a slot of type 1");
    let holder = bank.type_slot(slot).expect("a type parameter");
    let host = bank.list()[slot].0.clone();
    let holder_host = bank.list()[holder].0.clone();
    // The device's effect is type 1, and the read said so.
    rig.memory.put(
        holder_host.device_address(&BINDING).unwrap(),
        &holder_host.encode(1),
    );
    let (other, _events, _id) = rig.other_client();
    other.send(Request::ChangeTone {
        part: 0,
        msb: 0x57,
        lsb: 0,
        pc: 0,
    });
    wait_for("the type", || bank.device_type(holder) == Some(1));
    wait_for("the read", || rig.instance.is_armed());
    rig.run_tasks();

    // Another tone is loaded: until the read has told the type, the slot is
    // inert, whatever the host holds.
    rig.memory.put(
        holder_host.device_address(&BINDING).unwrap(),
        &holder_host.encode(2),
    );
    other.send(Request::ChangeTone {
        part: 0,
        msb: 0x57,
        lsb: 0,
        pc: 1,
    });
    wait_for("the type to be forgotten", || {
        bank.device_type(holder).is_none()
    });
    rig.host_sets(
        slot,
        if host.default == host.max {
            host.min
        } else {
            host.max
        },
    );
    assert_eq!(
        rig.dt1_at(&host),
        Vec::<Vec<u8>>::new(),
        "no write meanwhile"
    );
    wait_for("the new type", || bank.device_type(holder) == Some(2));
}

#[test]
fn a_replace_the_host_does_not_take_is_tried_again_later_with_nothing_lost() {
    let rig = Rig::new();
    let (i, cutoff) = rig.param(CUTOFF);
    let before = rig.instance.bank.value(i);
    let at = cutoff.device_address(&BINDING).expect("an address");
    rig.memory.put(at, &cutoff.encode(77));
    rig.recorder
        .refuse
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let (other, _events, _id) = rig.other_client();
    other.send(Request::ChangeTone {
        part: 0,
        msb: 0x57,
        lsb: 0,
        pc: 0,
    });
    // Refused (an empty marker), and the main thread was not kept waiting.
    wait_for("a refusal", || {
        rig.recorder.edits().contains(&Edit::Replace(Vec::new()))
    });
    assert_eq!(rig.instance.bank.value(i), before);
    rig.recorder
        .refuse
        .store(false, std::sync::atomic::Ordering::SeqCst);
    wait_for("the retry", || rig.instance.bank.value(i) == 77);
}

#[test]
fn nothing_is_posted_while_the_plugin_is_deactivated_and_gestures_resume_after() {
    let rig = Rig::new();
    let (_, host) = rig.param(CUTOFF);
    let (other, _events, _id) = rig.other_client();
    rig.instance.set_posting(false);
    other.write(param_id(&host), BINDING, host.max);
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(rig.recorder.edits(), []);
    rig.instance.set_posting(true);
    other.write(param_id(&host), BINDING, host.max - 1);
    wait_for("the end", || {
        rig.recorder.edits().last() == Some(&Edit::End(host.id.clone()))
    });
}

#[test]
fn an_editor_gesture_writes_nothing_to_the_device_beyond_what_the_page_wrote() {
    // The page writes the device itself (its raw DT1 is not in this rig); the
    // host's view of the drag must not be written back as a typed write.
    let mut rig = Rig::new();
    let (i, host) = rig.param(CUTOFF);
    let bank = &rig.instance.bank;

    assert!(host::editor_begin(bank, i));
    for value in [host.max - 2, host.max - 1, host.max] {
        host::editor_perform(bank, i, value);
        rig.outbound.poll(&rig.instance);
    }
    host::editor_end(bank, i, Some(host.max));
    rig.outbound.poll(&rig.instance);
    std::thread::sleep(Duration::from_millis(150));

    assert_eq!(rig.dt1s(), [], "no typed write: {:?}", rig.dt1s());
    assert_eq!(
        rig.recorder.edits(),
        [
            Edit::Begin(host.id.clone()),
            Edit::Perform(host.id.clone(), host.max - 2),
            Edit::Perform(host.id.clone(), host.max - 1),
            Edit::Perform(host.id.clone(), host.max),
            Edit::End(host.id.clone()),
        ]
    );

    // The host's own edit afterwards is still written.
    rig.host_sets(i, host.max - 5);
    wait_for("the host's own write", || !rig.dt1_at(&host).is_empty());
}

#[test]
fn a_late_echo_of_a_drags_last_value_is_not_written_either() {
    // The host passes a gesture's value back in a later audio callback: the
    // editor's end comes first.
    let mut rig = Rig::new();
    let (i, host) = rig.param(CUTOFF);
    let bank = &rig.instance.bank;

    assert!(host::editor_begin(bank, i));
    bank.expect(i, host.max);
    // The host has not applied the value yet when the drag ends.
    host::editor_end(bank, i, Some(host.max));
    bank.host_changed(i, host.max);
    rig.outbound.poll(&rig.instance);
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(rig.dt1s(), []);
}

/// Run the host tasks the worker has posted until `done` says enough.
fn run_tasks_until(rig: &Rig, what: &str, done: impl Fn(&[HostTask]) -> bool) -> Vec<HostTask> {
    let mut ran = Vec::new();
    let deadline = Instant::now() + WAIT;
    while !done(&ran) {
        ran.extend(rig.run_tasks());
        assert!(Instant::now() < deadline, "{what}; tasks so far: {ran:?}");
        std::thread::sleep(Duration::from_millis(10));
    }
    ran
}

#[test]
fn a_drag_on_a_parameter_the_follower_has_open_ends_the_followers_gesture_first() {
    let rig = Rig::with(Setup { ring: true, ..OPEN });
    let (i, host) = rig.param(CUTOFF);
    let (other, _events, _id) = rig.other_client();
    other.write(param_id(&host), BINDING, host.max - 1);
    run_tasks_until(&rig, "the follower's gesture to begin", |ran| {
        ran.len() >= 2
    });
    let bank = &rig.instance.bank;

    // The user drags the same Control.
    assert!(host::editor_begin(bank, i));
    host::editor_perform(bank, i, host.max);
    // The follower's later tasks for the parameter wait for nothing: its
    // perform is left out, and so is its end (the drag's is the one).
    other.write(param_id(&host), BINDING, host.max - 3);
    run_tasks_until(&rig, "the follower's end", |ran| {
        ran.contains(&HostTask::End(i as u32))
    });
    host::editor_end(bank, i, Some(host.max));

    let id = || host.id.clone();
    assert_eq!(
        rig.recorder.edits(),
        [
            Edit::Begin(id()),
            Edit::Perform(id(), host.max - 1),
            // The drag takes the parameter over.
            Edit::End(id()),
            Edit::Begin(id()),
            Edit::Perform(id(), host.max),
            Edit::End(id()),
        ]
    );
}

#[test]
fn the_follower_does_not_begin_a_second_gesture_on_a_parameter_being_dragged() {
    let rig = Rig::with(Setup { ring: true, ..OPEN });
    let (i, host) = rig.param(CUTOFF);
    let bank = &rig.instance.bank;
    assert!(host::editor_begin(bank, i));

    let (other, _events, _id) = rig.other_client();
    other.write(param_id(&host), BINDING, host.max - 1);
    run_tasks_until(&rig, "the follower's tasks", |ran| {
        ran.contains(&HostTask::End(i as u32))
    });
    // Still the drag's: nothing of the follower's reached the host.
    assert_eq!(rig.recorder.edits(), [Edit::Begin(host.id.clone())]);
    host::editor_end(bank, i, None);
    assert_eq!(
        rig.recorder.edits(),
        [Edit::Begin(host.id.clone()), Edit::End(host.id.clone())]
    );
}

/// A [`GuiContext`](nih_plug::prelude::GuiContext) that does nothing: an
/// editor's context, for the instance to adopt.
struct EditorContext;

impl nih_plug::prelude::GuiContext for EditorContext {
    fn plugin_api(&self) -> nih_plug::prelude::PluginApi {
        nih_plug::prelude::PluginApi::Standalone
    }
    fn request_resize(&self) -> bool {
        false
    }
    unsafe fn raw_begin_set_parameter(&self, _: nih_plug::prelude::ParamPtr) {}
    unsafe fn raw_set_parameter_normalized(&self, _: nih_plug::prelude::ParamPtr, _: f32) {}
    unsafe fn raw_end_set_parameter(&self, _: nih_plug::prelude::ParamPtr) {}
    fn get_state(&self) -> nih_plug::wrapper::state::PluginState {
        unimplemented!("not read")
    }
    fn set_state(&self, _: nih_plug::wrapper::state::PluginState) {
        unimplemented!("not written")
    }
}

#[test]
fn an_editor_opened_before_activation_gives_the_instance_its_way_to_the_host() {
    let instance = Instance::new(PluginKind::PcmSynth);
    assert!(instance.bank.edits().is_none());
    assert!(host::editor_context(
        &instance.bank,
        Arc::new(EditorContext)
    ));
    assert!(instance.bank.edits().is_some());
    // What the instance has already is kept: the second is not taken.
    assert!(!host::editor_context(
        &instance.bank,
        Arc::new(EditorContext)
    ));
    // The editor withdraws its own when it closes.
    instance.bank.withdraw_editor_edits();
    assert!(instance.bank.edits().is_none());

    // `initialize()` replaces an editor's with the long-lived one, which the
    // editor's closing then leaves alone.
    assert!(host::editor_context(
        &instance.bank,
        Arc::new(EditorContext)
    ));
    assert!(instance.bank.set_edits_once(Arc::new(EditorContext2)));
    instance.bank.withdraw_editor_edits();
    assert!(instance.bank.edits().is_some());
    instance.bank.set_edits(None);

    // A context from `initialize()` is not displaced by an editor's.
    instance.bank.set_edits(Some(Arc::new(EditorContext2)));
    assert!(!host::editor_context(
        &instance.bank,
        Arc::new(EditorContext)
    ));
}

/// Stands in for the long-lived context.
struct EditorContext2;

impl HostEdits for EditorContext2 {
    fn begin(&self, _: &IntParam) {}
    fn perform(&self, _: &IntParam, _: i32) {}
    fn end(&self, _: &IntParam) {}
    fn replace(&self, _: &[Replace<'_>]) -> bool {
        true
    }
}

/// A Part that holds another type of tone: the instance writes nothing and
/// ignores the device, until its default tone is loaded.
/// A value of `host` other than its default and than zero, which is what a
/// device with nothing in its memory holds (so the service does not drop a
/// write of it as unchanged).
fn away(host: &HostParam) -> i32 {
    if host.default == host.max {
        host.max - 1
    } else {
        host.max
    }
}

const ELSEWHERE: Setup = Setup {
    holds: ToneType::SnSynth,
    ..OPEN
};

#[test]
fn a_part_of_another_type_makes_the_instance_inert_and_its_default_tone_resumes_it() {
    let mut rig = Rig::with(ELSEWHERE);
    assert_eq!(rig.instance.notices.get(), [Notice::TypeMismatch]);
    assert!(!rig.instance.is_armed());
    // It read nothing of the tone's blocks (it would only be garbage).
    assert_eq!(rig.first_reads, 0);

    // The host edits a lane, and automation lanes stay: the parameters are
    // there. Nothing is written, now or later.
    let (i, cutoff) = rig.param(CUTOFF);
    assert_eq!(
        rig.instance.bank.len(),
        PluginKind::PcmSynth.host_params().len()
    );
    rig.host_sets(i, away(&cutoff));
    std::thread::sleep(Duration::from_millis(250));
    assert_eq!(rig.dt1s(), [], "nothing written to a tone of another type");

    // Another client changes the Part: the instance does not follow it.
    let (other, _events, _id) = rig.other_client();
    let (j, level) = rig.param("Partial 1 Level");
    other.write(param_id(&level), BINDING, away(&level));
    wait_for("the other client's write", || {
        !rig.dt1_at(&level).is_empty()
    });
    std::thread::sleep(Duration::from_millis(250));
    assert_eq!(rig.recorder.edits(), [], "no gesture on an inert instance");
    assert_ne!(rig.instance.bank.value(j), away(&level));
    assert_eq!(rig.instance.notices.get(), [Notice::TypeMismatch]);

    // The editor's button: the plugin's default tone, the first Preset tone of
    // its type, is selected on the Part.
    let tone = default_tone(ToneType::PcmSynth).expect("a default tone");
    rig.service.link.clear();
    rig.instance
        .request_default_tone()
        .expect("bound, and on another type");
    wait_for("the tone select", || {
        rig.dt1s()
            .contains(&(select_address(0).value(), vec![tone.msb, tone.lsb, tone.pc]))
    });
    wait_for("the instance to resume", || {
        rig.instance.fit() == ToneFit::Fits && rig.instance.is_armed()
    });
    assert_eq!(rig.instance.notices.get(), []);

    // It writes again, and follows the device again.
    rig.service.link.clear();
    rig.host_sets(i, away(&cutoff));
    wait_for("the write", || !rig.dt1_at(&cutoff).is_empty());
    assert_eq!(rig.dt1_at(&cutoff), [cutoff.encode(away(&cutoff))]);
    other.write(param_id(&cutoff), BINDING, cutoff.min);
    wait_for("the gesture", || !rig.recorder.edits().is_empty());
}

#[test]
fn an_instance_resumes_when_the_part_is_given_its_type_by_another_client() {
    let rig = Rig::with(ELSEWHERE);
    let (other, _events, _id) = rig.other_client();
    let tone = default_tone(ToneType::PcmSynth).expect("a default tone");
    other.send(Request::ChangeTone {
        part: 0,
        msb: tone.msb,
        lsb: tone.lsb,
        pc: tone.pc,
    });
    wait_for("the instance to resume", || {
        rig.instance.fit() == ToneFit::Fits && rig.instance.is_armed()
    });
    assert_eq!(rig.instance.notices.get(), []);
    // The service named the tone it selected: the device was not asked.
    let select = select_address(0);
    assert!(
        !rig.service
            .link
            .sent()
            .iter()
            .any(|m| is_rq1(m) && m[7..11] == *select.as_bytes()),
        "no probe"
    );

    // And it is inert again when the Part is given another type.
    let drum = default_tone(ToneType::PcmDrum).expect("a default tone");
    other.send(Request::ChangeTone {
        part: 0,
        msb: drum.msb,
        lsb: drum.lsb,
        pc: drum.pc,
    });
    wait_for("the instance to stop", || {
        rig.instance.fit() == ToneFit::Mismatch && !rig.instance.is_armed()
    });
    assert_eq!(rig.instance.notices.get(), [Notice::TypeMismatch]);
}

#[test]
fn two_instances_on_one_binding_both_work_and_both_warn_until_one_leaves() {
    let mut rig = Rig::new();
    assert_eq!(rig.instance.notices.get(), []);

    let second = Arc::new(Instance::new(PluginKind::PcmSynth));
    second.settle();
    second.bind(BINDING).expect("a Part a Tone plugin has");
    let second_connection = Connection::start(rig.dir.socket(), Arc::clone(&second));
    let mut second_out = Outbound::new(second_connection.sender().expect("a sender"));
    wait_for("the second instance", || {
        second.binding.get() == Some(BINDING) && second.is_armed()
    });
    wait_for("both warnings", || {
        rig.instance.notices.get() == [Notice::Duplicate]
            && second.notices.get() == [Notice::Duplicate]
    });

    // Both write.
    rig.service.link.clear();
    let (i, cutoff) = rig.param(CUTOFF);
    let (j, level) = rig.param("Partial 1 Level");
    rig.host_sets(i, away(&cutoff));
    second.bank.host_changed(j, away(&level));
    second_out.poll(&second);
    wait_for("the first's write", || !rig.dt1_at(&cutoff).is_empty());
    wait_for("the second's write", || !rig.dt1_at(&level).is_empty());

    // The one that leaves takes its warning with it, and frees the other's.
    drop(second_out);
    drop(second_connection);
    wait_for("the warning to go", || {
        rig.instance.notices.get().is_empty()
    });
}
