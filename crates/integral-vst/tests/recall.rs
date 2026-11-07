//! Recall from the DAW project (ADR 0009): instances the host restored push
//! their state to a real Device service on a fake link.
//!
//! The host's restore is played by setting what `deserialize_fields` sets
//! (the Binding, the saved tone, the restored mark) and the host's values
//! (`ParamBank::host_changed`, what a parameter's callback does) before the
//! connection starts. The device is a [`Memory`] that answers reads, and the
//! DT1s the service sends are what the fake link carried.

#![cfg(unix)]

use std::io;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use integral_core::device_spec::INTEGRA7;
use integral_core::host_params::{Binding, PluginKind};
use integral_core::params::{self, part};
use integral_core::sysex;
use integral_vst::family::link::Connection;
use integral_vst::family::recall::full_runs;
use integral_vst::family::tone::SavedTone;
use integral_vst::family::{Instance, Notice, Status};

mod common;

use common::{Memory, TempDir, TestService, addr, wait_for};

const LINK_DEVICE_ID: u8 = 0x10;

/// PCM Synth, Preset bank: a tone the service reads back as a baseline.
fn preset() -> SavedTone {
    SavedTone::new(87, 64, 3, "Warm Pad")
}

/// PCM Synth, User bank: a slot that may hold anything by now.
fn user() -> SavedTone {
    SavedTone::new(87, 0, 5, "My Pad")
}

/// A relay between the plugin and the service that a test can cut, to drop
/// the plugin's socket while the service (and its session) lives on.
struct Proxy {
    path: PathBuf,
    open: Arc<Mutex<Vec<UnixStream>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Proxy {
    fn start(path: PathBuf, upstream: PathBuf) -> Self {
        let listener = UnixListener::bind(&path).expect("bind the proxy");
        let open = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let (open, stop) = (Arc::clone(&open), Arc::clone(&stop));
            std::thread::spawn(move || {
                while let Ok((client, _)) = listener.accept() {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(server) = UnixStream::connect(&upstream) else {
                        continue;
                    };
                    let mut open = open.lock().unwrap();
                    for (from, to) in [(&client, &server), (&server, &client)] {
                        let (mut from, mut to) =
                            (from.try_clone().unwrap(), to.try_clone().unwrap());
                        std::thread::spawn(move || {
                            let _ = io::copy(&mut from, &mut to);
                            let _ = to.shutdown(std::net::Shutdown::Both);
                        });
                    }
                    open.push(client);
                    open.push(server);
                }
            })
        };
        Self {
            path,
            open,
            stop,
            thread: Some(thread),
        }
    }

    /// Drop every connection through the proxy.
    fn sever(&self) {
        for stream in self.open.lock().unwrap().drain(..) {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.sever();
        let _ = UnixStream::connect(&self.path);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

/// An instance of the project on a service, with its device.
struct Rig {
    // Dropped in this order: the instance's side first.
    _connection: Connection,
    proxy: Proxy,
    instance: Arc<Instance>,
    binding: Binding,
    memory: Memory,
    service: TestService,
    _dir: TempDir,
}

impl Rig {
    /// An instance of `kind` bound to `binding` that holds `tone`. With
    /// `restored` the host gave it that state; else it is new, and takes the
    /// Binding itself.
    fn new(kind: PluginKind, binding: Binding, tone: Option<SavedTone>, restored: bool) -> Self {
        let dir = TempDir::new();
        let service = TestService::start(&dir.socket());
        let memory = Memory::answer(&service.link);
        let proxy = Proxy::start(dir.0.join("p.sock"), dir.socket());
        let instance = Arc::new(Instance::new(kind));
        if restored {
            instance.binding.set(Some(binding));
            instance.tone.set(tone);
            instance.mark_restored();
        } else {
            instance.settle();
        }
        let connection = Connection::start(proxy.path.clone(), Arc::clone(&instance));
        wait_for("the service", || {
            instance.status.get() == Status::DeviceOffline
        });
        Self {
            _connection: connection,
            proxy,
            instance,
            binding,
            memory,
            service,
            _dir: dir,
        }
    }

    /// The device comes up: a new session.
    fn up(&self) {
        self.service.link.bring_up(Some(LINK_DEVICE_ID));
    }

    /// Make the device hold, for every field the recall would send, what the
    /// host holds: the baseline of a preset nobody edited. The slots of the
    /// type the host has go last, so they win an address they share.
    fn device_holds_the_hosts_values(&self) {
        let bank = &self.instance.bank;
        for active_pass in [false, true] {
            for i in 0..bank.len() {
                if bank.placed(i, &self.binding) && bank.host_active(i) == active_pass {
                    let host = &bank.list()[i].0;
                    let at = host.device_address(&self.binding).unwrap();
                    self.memory.put(at, &host.encode(bank.value(i)));
                }
            }
        }
    }

    /// The parameters the recall sends, with their addresses.
    fn live_params(&self) -> Vec<(usize, u32)> {
        let bank = &self.instance.bank;
        let mut live: Vec<_> = (0..bank.len())
            .filter(|i| bank.placed(*i, &self.binding) && bank.host_active(*i))
            .map(|i| {
                let at = bank.list()[i].0.device_address(&self.binding).unwrap();
                (i, at.value())
            })
            .collect();
        live.sort_by_key(|(_, at)| *at);
        live
    }

    /// Set the host's value of parameter `i` to one other than it holds.
    fn host_edits(&self, i: usize) {
        let bank = &self.instance.bank;
        let host = &bank.list()[i].0;
        let now = bank.value(i);
        bank.host_changed(i, if now == host.max { host.min } else { host.max });
    }

    /// The DT1s the link carried, as (address, data).
    fn dt1s(&self) -> Vec<(u32, Vec<u8>)> {
        self.service
            .link
            .sent()
            .iter()
            .filter_map(|m| sysex::parse_dt1(m).ok())
            .map(|d| (d.address.value(), d.data))
            .collect()
    }

    /// Wait until the instance has pushed (it reported `Recalling` and then
    /// stopped) and the link has carried `count` DT1s, then check that no more
    /// follow.
    fn dt1s_after_recall(&self, count: usize) -> Vec<(u32, Vec<u8>)> {
        // A whole Studio Set takes about five seconds at 40 ms a DT1.
        let deadline = Instant::now() + Duration::from_secs(30);
        while self.dt1s().len() < count || !self.instance.notices.get().is_empty() {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {count} DT1s"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(Duration::from_millis(250));
        let dt1s = self.dt1s();
        assert_eq!(dt1s.len(), count, "{dt1s:?}");
        self.only_dt1_and_rq1();
        dt1s
    }

    /// Wait until the instance has pushed and the link has stopped carrying
    /// DT1s; how many it carried.
    fn wait_for_recall(&self) -> usize {
        wait_for("the recall", || {
            self.dt1s().len() >= 2 && self.instance.notices.get().is_empty()
        });
        let mut count = self.dt1s().len();
        loop {
            std::thread::sleep(Duration::from_millis(250));
            let now = self.dt1s().len();
            if now == count {
                return count;
            }
            count = now;
        }
    }

    /// The link carried nothing but Roland DT1s and RQ1s, and no DT1 chose a
    /// Studio Set: the recall never loads one by number.
    fn only_dt1_and_rq1(&self) {
        let studio_set_selects = [
            params::SETUP_STUDIO_SET_BS_MSB.value(),
            params::SETUP_STUDIO_SET_BS_LSB.value(),
            params::SETUP_STUDIO_SET_PC.value(),
        ];
        for message in self.service.link.sent() {
            assert_eq!(message[0], sysex::SYSEX_START, "{message:?}");
            assert_eq!(message[1], sysex::ROLAND_ID, "{message:?}");
            assert!(matches!(message[6], 0x11 | 0x12), "{message:?}");
            if let Ok(dt1) = sysex::parse_dt1(&message) {
                assert!(
                    !studio_set_selects.contains(&dt1.address.value()),
                    "a Studio Set was loaded by number"
                );
            }
        }
    }

    /// An instance of the same project's service, as another plugin on the
    /// track would be.
    fn attach(
        &self,
        kind: PluginKind,
        binding: Binding,
        tone: Option<SavedTone>,
    ) -> (Connection, Arc<Instance>) {
        let instance = Arc::new(Instance::new(kind));
        instance.binding.set(Some(binding));
        instance.tone.set(tone);
        instance.mark_restored();
        let connection = Connection::start(self.proxy.path.clone(), Arc::clone(&instance));
        (connection, instance)
    }

    /// Part 0's tone-select DT1.
    fn select_at(&self) -> u32 {
        params::part_address(self.binding.part.unwrap_or(0), part::TONE_BANK_MSB).value()
    }
}

/// The bytes a list of DT1s sets, by address.
fn bytes_of(dt1s: &[(u32, Vec<u8>)]) -> std::collections::BTreeMap<u32, u8> {
    let mut out = std::collections::BTreeMap::new();
    for (at, data) in dt1s {
        for (k, b) in data.iter().enumerate() {
            assert!(out.insert(at + k as u32, *b).is_none(), "{at:#x} twice");
        }
    }
    out
}

const PART0: Binding = Binding::of_part(0);

/// The addresses of a Part's tone blocks the service reads back.
fn tone_blocks(kind: PluginKind) -> Vec<(u32, u32)> {
    let tone_type = kind.tone_type().expect("a tone plugin");
    integral_core::device_spec::tone_read_blocks(tone_type, 0)
        .into_iter()
        .map(|(a, s)| (a.value(), s.value()))
        .collect()
}

#[test]
fn an_untouched_preset_recalls_in_one_dt1_the_selection() {
    let rig = Rig::new(PluginKind::PcmSynth, PART0, Some(preset()), true);
    rig.device_holds_the_hosts_values();
    rig.up();

    let dt1s = rig.dt1s_after_recall(1);
    assert_eq!(dt1s, [(rig.select_at(), vec![87, 64, 3])]);
}

#[test]
fn a_preset_with_edits_in_a_block_recalls_the_selection_and_one_dt1_over_the_baselines_bytes() {
    let rig = Rig::new(PluginKind::PcmSynth, PART0, Some(preset()), true);
    rig.device_holds_the_hosts_values();
    let live = rig.live_params();
    let bank = &rig.instance.bank;
    let width = |k: usize| bank.list()[live[k].0].0.width as u32;
    let blocks = tone_blocks(PluginKind::PcmSynth);
    let block_of = |at: u32| blocks.iter().position(|(s, n)| (*s..*s + *n).contains(&at));
    // Two edits with other fields and gaps between them, in one block and
    // close enough for one DT1.
    let (first, last) = (0..live.len())
        .flat_map(|a| (a + 3..live.len()).map(move |b| (a, b)))
        .find(|(a, b)| {
            block_of(live[*a].1).is_some()
                && block_of(live[*a].1) == block_of(live[*b].1 + width(*b) - 1)
                && live[*b].1 + width(*b) - live[*a].1 <= sysex::MAX_DT1_DATA as u32
        })
        .expect("two fields in a block");
    for k in [first, last] {
        rig.host_edits(live[k].0);
    }
    rig.up();

    let dt1s = rig.dt1s_after_recall(2);
    assert_eq!(
        dt1s[0],
        (rig.select_at(), vec![87, 64, 3]),
        "the selection first"
    );
    let (start, data) = &dt1s[1];
    assert_eq!(*start, live[first].1);
    assert_eq!(
        data.len() as u32,
        live[last].1 + width(last) - live[first].1,
        "from the first change to the last"
    );
    for (k, b) in data.iter().enumerate() {
        let at = start + k as u32;
        let edited = [first, last]
            .into_iter()
            .find(|f| (live[*f].1..live[*f].1 + width(*f)).contains(&at));
        let wanted = match edited {
            Some(f) => {
                let i = live[f].0;
                bank.list()[i].0.encode(bank.value(i))[(at - live[f].1) as usize]
            }
            // Between the changes: what the device holds.
            None => rig.memory.get(addr(at)),
        };
        assert_eq!(*b, wanted, "{at:#x}");
    }
}

#[test]
fn a_preset_edited_all_over_recalls_about_a_dt1_per_block() {
    let rig = Rig::new(PluginKind::PcmSynth, PART0, Some(preset()), true);
    rig.device_holds_the_hosts_values();
    let live = rig.live_params();
    for (n, (i, _)) in live.iter().enumerate() {
        if n % 2 == 0 {
            rig.host_edits(*i);
        }
    }
    let blocks = tone_blocks(PluginKind::PcmSynth);
    let full = full_runs(&rig.instance.bank, &PART0, None);
    rig.up();

    // Never more than the blocks' own DT1s (a block longer than one message
    // takes two), and fewer than the unfilled full runs.
    let most: usize = blocks
        .iter()
        .map(|(_, n)| (*n as usize).div_ceil(sysex::MAX_DT1_DATA))
        .sum();
    let count = rig.wait_for_recall();
    assert!(
        count >= 2 && count <= 1 + most,
        "{count} DT1s for {most} at most"
    );
    assert!(count - 1 < full.len(), "{} of {}", count - 1, full.len());
    let dt1s = rig.dt1s();
    assert_eq!(dt1s[0].0, rig.select_at());
    // Every field is in the DT1s with the host's value.
    let sent = bytes_of(&dt1s[1..]);
    let bank = &rig.instance.bank;
    for (i, at) in &live {
        let host = &bank.list()[*i].0;
        for (k, b) in host.encode(bank.value(*i)).iter().enumerate() {
            let edited_or_equal = sent.get(&(at + k as u32));
            // An unchanged field may be left out: the device holds it.
            if let Some(b2) = edited_or_equal {
                assert_eq!(b2, b, "{}", host.id);
            } else {
                assert_eq!(
                    rig.memory.get(addr(at + k as u32)),
                    *b,
                    "{} left out",
                    host.id
                );
            }
        }
    }
}

#[test]
fn a_user_tone_is_selected_and_pushed_in_full() {
    let rig = Rig::new(PluginKind::PcmSynth, PART0, Some(user()), true);
    // The slot holds exactly what the project has: still the full push.
    rig.device_holds_the_hosts_values();
    let full = full_runs(&rig.instance.bank, &PART0, None);
    assert!(full.len() > 1);
    rig.up();

    let dt1s = rig.dt1s_after_recall(1 + full.len());
    assert_eq!(dt1s[0], (rig.select_at(), vec![87, 0, 5]));
    let wanted: Vec<_> = full
        .iter()
        .map(|r| (r.address.value(), r.data.clone()))
        .collect();
    assert_eq!(dt1s[1..], wanted[..]);
    // Every host value is in it, once.
    let sent = bytes_of(&dt1s[1..]);
    let bank = &rig.instance.bank;
    for (i, at) in rig.live_params() {
        let host = &bank.list()[i].0;
        for (k, b) in host.encode(bank.value(i)).iter().enumerate() {
            assert_eq!(sent.get(&(at + k as u32)), Some(b), "{}", host.id);
        }
    }
}

#[test]
fn the_studio_set_pushes_its_blocks_in_full_and_never_a_parts_tone_select() {
    let rig = Rig::new(PluginKind::StudioSet, Binding::ANY, None, true);
    let full = full_runs(&rig.instance.bank, &Binding::ANY, None);
    assert!(full.len() > 5);
    // Edit some fields: the push sends the host's values.
    let live = rig.live_params();
    rig.host_edits(live[3].0);
    rig.host_edits(live[live.len() - 2].0);
    rig.up();

    let dt1s = rig.dt1s_after_recall(full.len());
    let sent = bytes_of(&dt1s);
    for p in 0..INTEGRA7.part_count {
        let select = params::part_address(p, part::TONE_BANK_MSB).value();
        for k in 0..params::TONE_SELECTION_SIZE.value() {
            assert!(
                !sent.contains_key(&(select + k)),
                "part {p}: select byte {k}"
            );
        }
    }
    let bank = &rig.instance.bank;
    for (i, at) in &live {
        let host = &bank.list()[*i].0;
        for (k, b) in host.encode(bank.value(*i)).iter().enumerate() {
            assert_eq!(sent.get(&(at + k as u32)), Some(b), "{}", host.id);
        }
    }
}

#[test]
fn a_new_instance_sends_nothing_and_takes_the_tone_the_device_holds() {
    let rig = Rig::new(PluginKind::PcmSynth, PART0, None, false);
    // The device's Part 0: a PCM Synth preset, named.
    rig.memory.put(addr(rig.select_at()), &[87, 64, 9]);
    rig.memory.put(
        params::tone_name_address(0, integral_core::params::ToneType::PcmSynth),
        b"Soft Strings",
    );
    rig.up();

    wait_for("a Part and the first read", || {
        rig.instance.binding.get() == Some(PART0) && rig.instance.is_armed()
    });
    wait_for("the tone to be learned", || {
        rig.instance.tone.get().is_some()
    });
    assert_eq!(
        rig.instance.tone.get(),
        Some(SavedTone::new(87, 64, 9, "Soft Strings"))
    );
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(rig.dt1s(), [], "nothing is pushed");
    assert!(rig.instance.notices.get().is_empty());
    rig.only_dt1_and_rq1();
}

#[test]
fn a_restored_instance_without_a_saved_tone_adopts_from_the_start() {
    let rig = Rig::new(PluginKind::PcmSynth, PART0, None, true);
    rig.memory.put(addr(rig.select_at()), &[87, 64, 9]);
    rig.up();

    wait_for("the tone to be learned", || {
        rig.instance.tone.get().is_some()
    });
    wait_for("armed", || rig.instance.is_armed());
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(rig.dt1s(), [], "there was no tone to select");
}

#[test]
fn a_new_session_pushes_again_and_a_socket_drop_within_one_does_not() {
    let rig = Rig::new(PluginKind::PcmSynth, PART0, Some(preset()), true);
    rig.device_holds_the_hosts_values();
    rig.up();
    rig.dt1s_after_recall(1);

    // The socket drops; the service and its session carry on.
    rig.service.link.clear();
    rig.proxy.sever();
    wait_for("the instance to notice", || !rig.instance.is_armed());
    wait_for("the instance to be back", || {
        rig.instance.is_armed() && rig.instance.status.get() == Status::Connected
    });
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(rig.dt1s(), [], "the same session has had its push");
    assert!(rig.instance.notices.get().is_empty());

    // The device goes away and comes back: another session.
    rig.service.link.bring_down();
    wait_for("the instance to notice", || !rig.instance.is_armed());
    rig.service.link.clear();
    rig.up();
    let dt1s = rig.dt1s_after_recall(1);
    assert_eq!(dt1s, [(rig.select_at(), vec![87, 64, 3])]);
}

#[test]
fn an_instance_that_adopted_the_device_pushes_what_it_holds_after_a_power_cycle() {
    let rig = Rig::new(PluginKind::PcmSynth, PART0, None, false);
    rig.memory.put(addr(rig.select_at()), &[87, 64, 9]);
    rig.device_holds_the_hosts_values();
    rig.up();
    wait_for("the tone to be learned", || {
        rig.instance.tone.get().is_some()
    });
    wait_for("armed", || rig.instance.is_armed());
    std::thread::sleep(Duration::from_millis(200));

    // The user edits in the DAW, and the device is switched off and on.
    let live = rig.live_params();
    rig.host_edits(live[10].0);
    rig.service.link.bring_down();
    wait_for("the instance to notice", || !rig.instance.is_armed());
    rig.service.link.clear();
    rig.up();

    let dt1s = rig.dt1s_after_recall(2);
    assert_eq!(dt1s[0], (rig.select_at(), vec![87, 64, 9]));
    let bank = &rig.instance.bank;
    let host = &bank.list()[live[10].0].0;
    let (start, data) = &dt1s[1];
    let at = live[10].1;
    assert!(*start <= at && at < start + data.len() as u32);
    assert_eq!(
        data[(at - start) as usize..][..host.width],
        host.encode(bank.value(live[10].0))[..]
    );
}

#[test]
fn a_state_the_host_restores_in_the_middle_of_a_session_is_pushed() {
    let rig = Rig::new(PluginKind::PcmSynth, PART0, Some(preset()), true);
    rig.device_holds_the_hosts_values();
    rig.up();
    rig.dt1s_after_recall(1);

    // The user loads a DAW preset: another tone, saved as a User tone.
    rig.service.link.clear();
    rig.instance.tone.set(Some(user()));
    rig.instance.mark_restored();
    let full = full_runs(&rig.instance.bank, &PART0, None);
    let dt1s = rig.dt1s_after_recall(1 + full.len());
    assert_eq!(dt1s[0], (rig.select_at(), vec![87, 0, 5]));
}

#[test]
fn recalling_is_reported_until_the_push_is_out() {
    let rig = Rig::new(PluginKind::PcmSynth, PART0, Some(user()), true);
    let full = full_runs(&rig.instance.bank, &PART0, None);
    let seen = Arc::new(AtomicBool::new(false));
    let watching = Arc::new(AtomicBool::new(true));
    let watcher = {
        let (instance, seen, watching) = (
            Arc::clone(&rig.instance),
            Arc::clone(&seen),
            Arc::clone(&watching),
        );
        std::thread::spawn(move || {
            while watching.load(Ordering::SeqCst) {
                if instance.notices.get().contains(&Notice::Recalling) {
                    seen.store(true, Ordering::SeqCst);
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        })
    };
    assert!(rig.instance.notices.get().is_empty(), "not before");
    rig.up();
    wait_for("the notice", || seen.load(Ordering::SeqCst));
    rig.dt1s_after_recall(1 + full.len());
    assert!(rig.instance.notices.get().is_empty(), "cleared");
    assert!(rig.instance.is_armed(), "host changes go out again");
    watching.store(false, Ordering::SeqCst);
    watcher.join().unwrap();
}

#[test]
fn a_key_plugins_fields_survive_the_whole_kits_selection_that_follows() {
    let kit = SavedTone::new(86, 64, 0, "Jazz Kit");
    let key = Binding::of_part(0).with_key(INTEGRA7.pcm_drum_keys.first);
    let rig = Rig::new(PluginKind::PcmDrum, key, Some(kit.clone()), true);
    // The device's Part 0 holds the saved kit.
    rig.memory.put(addr(rig.select_at()), &[86, 64, 0]);
    let live = rig.live_params();
    assert!(!live.is_empty(), "the key's fields");
    rig.host_edits(live[0].0);
    let edited = live[0].1;
    rig.up();

    // The key plugin pushes its fields alone: no selection.
    wait_for("the key's fields", || {
        rig.dt1s()
            .iter()
            .any(|(a, d)| *a <= edited && edited < a + d.len() as u32)
    });
    wait_for("the notice to clear", || {
        rig.instance.notices.get().is_empty()
    });
    assert!(rig.dt1s().iter().all(|(a, _)| *a != rig.select_at()));

    // The plugin of the whole kit comes up and selects it: every key is
    // reloaded on the device.
    let (_whole, whole_instance) = rig.attach(PluginKind::PcmDrum, PART0, Some(kit));
    wait_for("the selection", || {
        rig.dt1s().iter().any(|(a, _)| *a == rig.select_at())
    });
    wait_for("the whole kit to be out", || {
        whole_instance.notices.get().is_empty() && whole_instance.is_armed()
    });
    // The key's fields went again, after the selection.
    wait_for("the key's fields again", || {
        let sent = rig.dt1s();
        let last_select = sent
            .iter()
            .rposition(|(a, _)| *a == rig.select_at())
            .unwrap();
        sent[last_select..]
            .iter()
            .any(|(a, d)| *a <= edited && edited < a + d.len() as u32)
    });
    rig.only_dt1_and_rq1();
}

#[test]
fn a_restored_instance_on_a_part_of_another_type_recalls_its_tone_and_fits_after() {
    use integral_vst::family::instance::ToneFit;
    let rig = Rig::new(PluginKind::PcmSynth, PART0, Some(user()), true);
    // The Part holds an SN Synth tone: the plugin is inert until the recall's
    // selection puts its own back.
    rig.memory.put(addr(rig.select_at()), &[95, 0, 1]);
    let full = full_runs(&rig.instance.bank, &PART0, None);
    rig.up();

    wait_for("the recall to start", || {
        rig.instance.notices.get().contains(&Notice::Recalling)
    });
    assert!(
        rig.instance.default_tone().is_none(),
        "no default tone is offered while the saved one is owed"
    );
    let dt1s = rig.dt1s_after_recall(1 + full.len());
    assert_eq!(dt1s[0], (rig.select_at(), vec![87, 0, 5]));
    assert_eq!(rig.instance.fit(), ToneFit::Fits);
    assert!(rig.instance.is_armed(), "armed once the push is out");
    assert!(
        rig.instance.notices.get().is_empty(),
        "no mismatch is reported any more"
    );
    assert!(rig.instance.default_tone().is_some());
}

/// The first parameter of `instance` that acts on `binding`, set to another
/// value in the host; where it is on the device.
fn edit_first_field(instance: &Instance, binding: &Binding) -> u32 {
    let bank = &instance.bank;
    let i = (0..bank.len())
        .find(|i| bank.placed(*i, binding) && bank.host_active(*i))
        .expect("a field");
    let host = &bank.list()[i].0;
    let now = bank.value(i);
    bank.host_changed(i, if now == host.max { host.min } else { host.max });
    host.device_address(binding).unwrap().value()
}

#[test]
fn key_plugins_fields_survive_a_kit_selected_again_and_again() {
    use integral_device_service::Client;
    use integral_device_service::protocol::Request;
    let kit = SavedTone::new(86, 64, 0, "Jazz Kit");
    let first = INTEGRA7.pcm_drum_keys.first;
    let keys: Vec<Binding> = (0..3)
        .map(|k| Binding::of_part(0).with_key(first + k))
        .collect();
    let rig = Rig::new(PluginKind::PcmDrum, keys[0], Some(kit.clone()), true);
    rig.memory.put(addr(rig.select_at()), &[86, 64, 0]);
    let mut edited = vec![edit_first_field(&rig.instance, &keys[0])];
    let mut others = Vec::new();
    for key in &keys[1..] {
        let (connection, instance) = rig.attach(PluginKind::PcmDrum, *key, Some(kit.clone()));
        edited.push(edit_first_field(&instance, key));
        others.push((connection, instance));
    }
    let (_whole, whole) = rig.attach(PluginKind::PcmDrum, PART0, Some(kit));
    rig.up();

    let carried = |sent: &[(u32, Vec<u8>)], at: u32| {
        sent.iter()
            .any(|(a, d)| *a <= at && at < a + d.len() as u32)
    };
    let settled = || {
        rig.instance.notices.get().is_empty()
            && whole.notices.get().is_empty()
            && others.iter().all(|(_, i)| i.notices.get().is_empty())
    };
    wait_for("everything pushed", || {
        let sent = rig.dt1s();
        edited.iter().all(|at| carried(&sent, *at)) && settled()
    });

    // A Program Change reaches the Part three times: each reloads the kit.
    let (pc, _events) = Client::connect(rig.proxy.path.clone());
    for round in 0..3 {
        std::thread::sleep(Duration::from_millis(300));
        rig.service.link.clear();
        pc.send(Request::ChangeTone {
            part: 0,
            msb: 86,
            lsb: 64,
            pc: 0,
        });
        wait_for("the selection", || {
            rig.dt1s().iter().any(|(a, _)| *a == rig.select_at())
        });
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let sent = rig.dt1s();
            let select = sent
                .iter()
                .rposition(|(a, _)| *a == rig.select_at())
                .unwrap();
            if edited.iter().all(|at| carried(&sent[select..], *at)) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "round {round}: a key's fields are gone"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        wait_for("settled", settled);
    }
}

#[test]
fn a_key_plugin_on_a_part_of_another_type_does_not_hide_the_default_tone() {
    let kit = SavedTone::new(86, 64, 0, "Jazz Kit");
    let key = Binding::of_part(0).with_key(INTEGRA7.pcm_drum_keys.first);
    let rig = Rig::new(PluginKind::PcmDrum, key, Some(kit), true);
    // The Part holds a PCM Synth tone, and nothing selects the kit.
    rig.memory.put(addr(rig.select_at()), &[87, 64, 3]);
    rig.up();

    wait_for("the mismatch", || {
        rig.instance.notices.get().contains(&Notice::TypeMismatch)
    });
    std::thread::sleep(Duration::from_millis(300));
    assert!(rig.instance.default_tone().is_some(), "offered");
    assert!(!rig.instance.notices.get().contains(&Notice::Recalling));
    assert_eq!(rig.dt1s(), [], "a key's plugin selects nothing");
}
