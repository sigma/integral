//! A fake link for tests: records what the service sends.
//! Behind the `test-util` feature.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use integral_core::address::{Address, DataSize};
use integral_core::device_spec::INTEGRA7;
use integral_core::params::{self, part};
use integral_core::sysex;

use crate::link::{DeviceLink, LinkError, LinkSink};

#[derive(Default)]
struct Shared {
    sent: Vec<Vec<u8>>,
    /// How many messages `clear` has forgotten, so that a message keeps its
    /// number (`cleared` plus its place in `sent`) for a reader that counts.
    cleared: usize,
    sink: Option<LinkSink>,
    gap: std::time::Duration,
}

/// A [`DeviceLink`] that records every message sent. Clones share the
/// recording, so a test keeps one while the service owns the other.
#[derive(Clone, Default)]
pub struct FakeLink(Arc<Mutex<Shared>>);

impl FakeLink {
    /// A link that has sent nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Every message sent so far, in order.
    pub fn sent(&self) -> Vec<Vec<u8>> {
        self.0.lock().expect("fake link lock").sent.clone()
    }

    /// Make the link ask the service to hold SysEx back for `gap`, as a
    /// link does after its own probe.
    pub fn hold_sysex(&self, gap: std::time::Duration) {
        self.0.lock().expect("fake link lock").gap = gap;
    }

    /// Forget what was sent.
    pub fn clear(&self) {
        let mut shared = self.0.lock().expect("fake link lock");
        shared.cleared += shared.sent.len();
        shared.sent.clear();
    }

    /// The messages sent from number `from` on (numbered from 0 since the
    /// link was made, whatever [`clear`](Self::clear) forgot; those are
    /// skipped), and the number to ask from next.
    pub fn sent_from(&self, from: usize) -> (Vec<Vec<u8>>, usize) {
        let shared = self.0.lock().expect("fake link lock");
        let skip = from.saturating_sub(shared.cleared);
        let next = shared.cleared + shared.sent.len();
        (shared.sent.get(skip..).unwrap_or_default().to_vec(), next)
    }

    /// Report the link up, once the service has started it.
    pub fn bring_up(&self, device_id: Option<u8>) {
        self.sink().up(device_id);
    }

    /// Report the link down.
    pub fn bring_down(&self) {
        self.sink().down();
    }

    /// Report a message from the device.
    pub fn device_says(&self, message: &[u8]) {
        self.sink().midi(message);
    }

    fn sink(&self) -> LinkSink {
        self.0
            .lock()
            .expect("fake link lock")
            .sink
            .clone()
            .expect("the service has not started the link")
    }
}

impl DeviceLink for FakeLink {
    fn start(&mut self, events: LinkSink) -> Result<(), LinkError> {
        self.0.lock().expect("fake link lock").sink = Some(events);
        Ok(())
    }

    fn sysex_gap(&self) -> std::time::Duration {
        self.0.lock().expect("fake link lock").gap
    }

    fn send(&mut self, message: &[u8]) -> Result<(), LinkError> {
        self.0
            .lock()
            .expect("fake link lock")
            .sent
            .push(message.to_vec());
        Ok(())
    }
}

/// Whether `message` is a request to read (RQ1).
pub fn is_rq1(message: &[u8]) -> bool {
    message.first() == Some(&sysex::SYSEX_START) && message.get(6) == Some(&sysex::CMD_RQ1)
}

/// How often [`FakeMemory`] looks at what the link carried.
const POLL: Duration = Duration::from_millis(5);

/// The bytes a [`FakeMemory`] reads out per reply, as the device does.
const REPLY_CHUNK: usize = 64;

/// A device that answers reads: its memory by address, and a thread that
/// replies to every RQ1 the service sends on a [`FakeLink`] with the bytes it
/// holds (0 where nothing was put), keeps what every DT1 writes, and answers
/// an Identity Request as an INTEGRA-7 at the default device ID.
pub struct FakeMemory {
    bytes: Arc<Mutex<BTreeMap<u32, u8>>>,
    tones: Arc<Mutex<Tones>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

/// The tones the device can load, by selection (Bank Select MSB, LSB, PC):
/// the bytes of a Part's Temporary Tone, by distance from its start.
type Tones = BTreeMap<[u8; 3], BTreeMap<u32, u8>>;

/// How far apart Parts' Temporary Tones start, in linear address value.
const PART_SPAN: u32 = 0x20 * 0x4000;

impl FakeMemory {
    /// Start answering reads on `link`.
    pub fn answer(link: &FakeLink) -> Self {
        let bytes = Arc::new(Mutex::new(BTreeMap::new()));
        let tones = Arc::new(Mutex::new(Tones::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let (link, bytes, tones, stop) = (
                link.clone(),
                Arc::clone(&bytes),
                Arc::clone(&tones),
                Arc::clone(&stop),
            );
            std::thread::spawn(move || {
                let mut next = 0;
                while !stop.load(Ordering::Acquire) {
                    let (sent, upto) = link.sent_from(next);
                    for message in &sent {
                        Self::hear(&link, &bytes, &tones, message);
                    }
                    next = upto;
                    std::thread::sleep(POLL);
                }
            })
        };
        Self {
            bytes,
            tones,
            stop,
            thread: Some(thread),
        }
    }

    /// What the device does with one message: keep a write, answer a read
    /// or an Identity Request.
    fn hear(
        link: &FakeLink,
        bytes: &Mutex<BTreeMap<u32, u8>>,
        tones: &Mutex<Tones>,
        message: &[u8],
    ) {
        // A write is kept, as a device would keep it.
        if message.get(6) == Some(&sysex::CMD_DT1)
            && let Ok(dt1) = sysex::parse_dt1(message)
        {
            let mut memory = bytes.lock().expect("memory");
            for (k, byte) in dt1.data.iter().enumerate() {
                memory.insert(dt1.address.value() + k as u32, *byte);
            }
            Self::reload(&mut memory, &tones.lock().expect("tones"), &dt1);
        }
        if message == sysex::identity_request() {
            let identity = sysex::DeviceIdentity {
                device_id: sysex::DEFAULT_DEVICE_ID,
                manufacturer_id: sysex::ROLAND_ID,
                family_code: sysex::INTEGRA7_FAMILY,
                family_number: [0, 0],
                revision: [0; 4],
            };
            link.device_says(&identity.reply());
        }
        // F0 41 id 00 00 64 11 a0..a3 s0..s3 cs F7
        if is_rq1(message) && message.len() > 14 {
            let address = Address([message[7], message[8], message[9], message[10]]);
            let size = DataSize([message[11], message[12], message[13], message[14]]).value();
            let memory = bytes.lock().expect("memory");
            let start = address.value();
            let data: Vec<u8> = (0..size)
                .map(|k| memory.get(&(start + k)).copied().unwrap_or(0))
                .collect();
            drop(memory);
            for (n, chunk) in data.chunks(REPLY_CHUNK).enumerate() {
                let at = Address::from_value(start + (n * REPLY_CHUNK) as u32).expect("an address");
                link.device_says(&sysex::build_dt1(message[2], &at, chunk));
            }
        }
    }

    /// A write that selects a tone on a Part loads it into the Part's
    /// Temporary Tone, as the device does (docs/midi/99-undocumented.md, "6.
    /// Temporary Tone Reloads"): the tone stored for the selection replaces
    /// what the Part held. A selection with no stored tone leaves it be.
    fn reload(memory: &mut BTreeMap<u32, u8>, tones: &Tones, dt1: &sysex::Dt1Message) {
        let end = dt1.address.value() + dt1.data.len() as u32;
        for part_index in 0..INTEGRA7.part_count {
            let select = params::part_address(part_index, part::TONE_BANK_MSB).value();
            if dt1.address.value() >= select + 3 || end <= select {
                continue;
            }
            let held: Vec<u8> = (0..3)
                .map(|k| memory.get(&(select + k)).copied().unwrap_or(0))
                .collect();
            let Some(tone) = tones.get(&[held[0], held[1], held[2]]) else {
                continue;
            };
            let base = params::temporary_tone_base(part_index).value();
            memory.retain(|at, _| !(base..base + PART_SPAN).contains(at));
            memory.extend(tone.iter().map(|(offset, byte)| (base + offset, *byte)));
        }
    }

    /// Store the tone `part_index`'s Temporary Tone holds now as the one the
    /// device loads when `selection` (Bank Select MSB, LSB, PC) is selected.
    pub fn store_tone(&self, selection: [u8; 3], part_index: u8) {
        let base = params::temporary_tone_base(part_index).value();
        let memory = self.bytes.lock().expect("memory");
        let tone = memory
            .range(base..base + PART_SPAN)
            .map(|(at, byte)| (at - base, *byte))
            .collect();
        self.tones.lock().expect("tones").insert(selection, tone);
    }

    /// The byte at `address`; 0 where nothing was put.
    pub fn get(&self, address: Address) -> u8 {
        let memory = self.bytes.lock().expect("memory");
        memory.get(&address.value()).copied().unwrap_or(0)
    }

    /// Put `data` at `address`.
    pub fn put(&self, address: Address, data: &[u8]) {
        let mut memory = self.bytes.lock().expect("memory");
        for (k, byte) in data.iter().enumerate() {
            memory.insert(address.value() + k as u32, *byte);
        }
    }
}

impl Drop for FakeMemory {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;
    use crate::link::LinkEvent;

    /// A client identifies the device before it trusts the link, so the fake
    /// device answers as an INTEGRA-7 does.
    #[test]
    fn answers_an_identity_request_as_an_integra7() {
        let mut link = FakeLink::new();
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        link.start(LinkSink::new(move |event| {
            if let LinkEvent::Midi(message) = event {
                let _ = tx.lock().expect("sender").send(message);
            }
        }))
        .expect("start");
        let _memory = FakeMemory::answer(&link);
        link.send(&sysex::identity_request()).expect("send");
        let reply = rx.recv_timeout(Duration::from_secs(2)).expect("a reply");
        let identity = sysex::parse_identity_reply(&reply).expect("an Identity Reply");
        assert!(identity.is_integra7());
        assert_eq!(identity.device_id, sysex::DEFAULT_DEVICE_ID);
    }
}
