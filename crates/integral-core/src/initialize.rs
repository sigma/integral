//! Undocumented Init of a Temporary area.
//!
//! The INTEGRA-7 resets part of its Temporary area to INIT when it receives
//! an RQ1 at `0F 00 18 TT` whose 4-byte size field carries `7F 7F`, the
//! Part and the Partial. It answers with a DT1 at `0F 00 18 01` when the
//! Init starts and one at `0F 00 18 02` when it finishes, and announces
//! none of the reset values: the host reads the area back. The result
//! equals an unwritten User slot.
//!
//! [`Initialization`] sends one Init and waits for "finished". It does no
//! I/O: the host sends what [`Initialization::next_action`] asks for, feeds
//! DT1s to [`Initialization::on_reply`] and calls
//! [`Initialization::on_timeout`] when the wait runs out.
//!
//! Reference: `docs/midi/99-undocumented.md`, section 7.

use thiserror::Error;

use crate::address::{Address, DataSize};
use crate::device_spec::INTEGRA7;
use crate::params::ToneType;
use crate::sysex;

/// Address of the "Init started" reply.
pub const INIT_STARTED_ADDR: Address = Address::new(0x0F, 0x00, 0x18, 0x01);

/// Address of the "Init finished" reply.
pub const INIT_FINISHED_ADDR: Address = Address::new(0x0F, 0x00, 0x18, 0x02);

/// How long to wait for "Init finished" once the Init is sent (ms). The
/// delay between "started" and "finished" wasn't measured; Studio Set Init,
/// which reloads 16 tones, is the slowest.
pub const INIT_TIMEOUT_MS: f64 = 5_000.0;

/// What to reset. Parts are 0-indexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitTarget {
    /// The Temporary Studio Set, every Part's tone included.
    StudioSet,
    /// The eight Sound Ctrl offsets (Part `00 13`–`00 1A`) of **every**
    /// Part.
    SoundCtrl,
    /// A Part's Studio Set Part block; its tone reloads.
    Part(u8),
    /// The Temporary tone of a Part. It keeps its type.
    Tone(u8),
    /// One partial of a Part's Temporary tone, built by
    /// [`InitTarget::partial`].
    Partial {
        /// The Part.
        part: u8,
        /// The partial, checked against the tone type.
        partial: PartialRef,
    },
}

/// A partial of a tone of a known type: the 0-based partial index for a
/// tone, the MIDI key number for a drum kit. Only [`InitTarget::partial`]
/// builds one, so an index can't be sent to a kit by mistake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartialRef(u8);

impl PartialRef {
    /// The `NN` byte: the partial index, or the key number.
    pub const fn value(self) -> u8 {
        self.0
    }
}

impl InitTarget {
    /// The Partial Init of a Part playing a tone of `tone_type`: `partial`
    /// is the 0-based partial index for SN Synth and PCM Synth tones, the
    /// MIDI key number for SN and PCM Drum Kits (checked on the device,
    /// #288). An SN Acoustic tone has no partials.
    pub fn partial(part: u8, tone_type: ToneType, partial: u8) -> Result<Self, InitError> {
        let exists = match tone_type {
            ToneType::SnSynth => partial < INTEGRA7.sn_synth_partial_count,
            ToneType::PcmSynth => partial < INTEGRA7.pcm_synth_partial_count,
            ToneType::SnAcoustic => return Err(InitError::NoPartials),
            ToneType::SnDrum => INTEGRA7.sn_drum_keys.check(partial).is_ok(),
            ToneType::PcmDrum => INTEGRA7.pcm_drum_keys.check(partial).is_ok(),
        };
        if !exists {
            return Err(InitError::InvalidPartial(partial));
        }
        Ok(Self::Partial {
            part,
            partial: PartialRef(partial),
        })
    }

    /// The `TT`, `PP` and `NN` bytes of the request.
    const fn bytes(self) -> (u8, u8, u8) {
        match self {
            Self::StudioSet => (0x00, 0x00, 0x00),
            Self::SoundCtrl => (0x08, 0x00, 0x00),
            Self::Part(part) => (0x10, part, 0x00),
            Self::Tone(part) => (0x20, part, 0x00),
            Self::Partial { part, partial } => (0x30, part, partial.value()),
        }
    }

    /// The Part it names, if any.
    const fn part(self) -> Option<u8> {
        match self {
            Self::StudioSet | Self::SoundCtrl => None,
            Self::Part(part) | Self::Tone(part) | Self::Partial { part, .. } => Some(part),
        }
    }
}

/// Why an Init can't start, or didn't finish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum InitError {
    /// The Part doesn't exist.
    #[error("no Part {}", .0 + 1)]
    InvalidPart(u8),
    /// The tone type has no such partial or key.
    #[error("no partial or key {0}")]
    InvalidPartial(u8),
    /// The tone type (SN Acoustic) has no partials.
    #[error("this tone type has no partials")]
    NoPartials,
    /// No "Init finished" within [`INIT_TIMEOUT_MS`]: the area's content is
    /// unknown.
    #[error("the device didn't confirm the Init")]
    Timeout,
    /// Cancelled before the Init was sent; nothing changed.
    #[error("Init cancelled")]
    Cancelled,
}

/// Build the request that resets `target` to INIT.
pub fn build_init_request(device_id: u8, target: InitTarget) -> Vec<u8> {
    let (tt, part, partial) = target.bytes();
    let address = Address::new(0x0F, 0x00, 0x18, tt);
    sysex::build_rq1(
        device_id,
        &address,
        &DataSize::new(0x7F, 0x7F, part, partial),
    )
}

/// Whether a DT1 is the "Init finished" reply.
pub fn is_init_finished(address: &Address, data: &[u8]) -> bool {
    *address == INIT_FINISHED_ADDR && data == [0, 0, 0, 0]
}

/// What the host should do next for an [`Initialization`].
#[derive(Debug, Clone, PartialEq)]
pub enum InitAction {
    /// Send this message (the Init request).
    Send(Vec<u8>),
    /// Feed DT1s to [`Initialization::on_reply`] until it returns `true`,
    /// or call [`Initialization::on_timeout`] after `timeout_ms`.
    Await {
        /// How long to wait for "Init finished", in ms.
        timeout_ms: f64,
    },
    /// Over. On success, read the reset area back: the device announces
    /// none of its values.
    Done(Result<(), InitError>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Ready,
    Sent,
    Done(Result<(), InitError>),
}

/// Sends one Init and waits for "Init finished". Pure: the host does the
/// I/O.
#[derive(Debug, Clone)]
pub struct Initialization {
    device_id: u8,
    target: InitTarget,
    phase: Phase,
}

impl Initialization {
    /// Prepare resetting `target` on the device `device_id`.
    pub fn new(device_id: u8, target: InitTarget) -> Result<Self, InitError> {
        if let Some(part) = target.part()
            && part >= INTEGRA7.part_count
        {
            return Err(InitError::InvalidPart(part));
        }
        Ok(Self {
            device_id,
            target,
            phase: Phase::Ready,
        })
    }

    /// What it resets.
    pub fn target(&self) -> InitTarget {
        self.target
    }

    /// The next step: send the Init, then await "finished".
    pub fn next_action(&mut self) -> InitAction {
        match self.phase {
            Phase::Ready => {
                self.phase = Phase::Sent;
                InitAction::Send(build_init_request(self.device_id, self.target))
            }
            Phase::Sent => InitAction::Await {
                timeout_ms: INIT_TIMEOUT_MS,
            },
            Phase::Done(result) => InitAction::Done(result),
        }
    }

    /// Feed an incoming DT1. Returns `true` if it was the awaited "Init
    /// finished".
    pub fn on_reply(&mut self, address: &Address, data: &[u8]) -> bool {
        let finished = self.phase == Phase::Sent && is_init_finished(address, data);
        if finished {
            self.phase = Phase::Done(Ok(()));
        }
        finished
    }

    /// The wait ran out: give up on "finished".
    pub fn on_timeout(&mut self) {
        if self.phase == Phase::Sent {
            self.phase = Phase::Done(Err(InitError::Timeout));
        }
    }

    /// Stop before the Init is sent. Once sent, the device resets the area
    /// anyway, so "finished" is still awaited.
    pub fn cancel(&mut self) {
        if self.phase == Phase::Ready {
            self.phase = Phase::Done(Err(InitError::Cancelled));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STARTED: Address = INIT_STARTED_ADDR;

    /// `F0 41 10 00 00 64 11 0F 00 18 <TT> 7F 7F <PP> <NN> <sum> F7`.
    fn request(tt: u8, pp: u8, nn: u8, checksum: u8) -> Vec<u8> {
        vec![
            0xF0, 0x41, 0x10, 0x00, 0x00, 0x64, 0x11, 0x0F, 0x00, 0x18, tt, 0x7F, 0x7F, pp, nn,
            checksum, 0xF7,
        ]
    }

    /// Requests sent on our device (docs/midi/99-undocumented.md §7).
    #[test]
    fn builds_device_checked_requests() {
        assert_eq!(
            build_init_request(0x10, InitTarget::StudioSet),
            request(0x00, 0x00, 0x00, 0x5B)
        );
        assert_eq!(
            build_init_request(0x10, InitTarget::Tone(1)),
            request(0x20, 0x01, 0x00, 0x3A)
        );
    }

    #[test]
    fn sound_ctrl_names_no_part() {
        assert_eq!(
            build_init_request(0x10, InitTarget::SoundCtrl),
            request(0x08, 0x00, 0x00, 0x53)
        );
    }

    #[test]
    fn part_carries_the_part() {
        assert_eq!(
            build_init_request(0x10, InitTarget::Part(15)),
            request(0x10, 0x0F, 0x00, 0x3C)
        );
    }

    #[test]
    fn partial_carries_the_partial_index() {
        let target = InitTarget::partial(1, ToneType::PcmSynth, 2).unwrap();
        assert_eq!(
            target,
            InitTarget::Partial {
                part: 1,
                partial: PartialRef(2)
            }
        );
        assert_eq!(
            build_init_request(0x10, target),
            request(0x30, 0x01, 0x02, 0x28)
        );
    }

    /// On a kit `NN` is the key number: key 36 is `24` (checked in #288).
    #[test]
    fn kit_partial_carries_the_key_number() {
        for kit in [ToneType::PcmDrum, ToneType::SnDrum] {
            let target = InitTarget::partial(15, kit, 36).unwrap();
            assert_eq!(
                build_init_request(0x10, target),
                request(0x30, 0x0F, 0x24, 0x78)
            );
        }
    }

    #[test]
    fn partial_is_checked_against_the_tone_type() {
        use ToneType::*;
        let sns = INTEGRA7.sn_synth_partial_count;
        let pcms = INTEGRA7.pcm_synth_partial_count;
        assert!(InitTarget::partial(0, SnSynth, sns - 1).is_ok());
        assert_eq!(
            InitTarget::partial(0, SnSynth, sns),
            Err(InitError::InvalidPartial(sns))
        );
        assert!(InitTarget::partial(0, PcmSynth, pcms - 1).is_ok());
        assert_eq!(
            InitTarget::partial(0, PcmSynth, pcms),
            Err(InitError::InvalidPartial(pcms))
        );
        assert_eq!(
            InitTarget::partial(0, SnAcoustic, 0),
            Err(InitError::NoPartials)
        );
        for (kit, keys) in [
            (PcmDrum, INTEGRA7.pcm_drum_keys),
            (SnDrum, INTEGRA7.sn_drum_keys),
        ] {
            assert!(InitTarget::partial(0, kit, keys.first).is_ok());
            assert!(InitTarget::partial(0, kit, keys.last).is_ok());
            // A 0-based index is no key.
            assert_eq!(
                InitTarget::partial(0, kit, keys.first - 1),
                Err(InitError::InvalidPartial(keys.first - 1))
            );
            assert_eq!(
                InitTarget::partial(0, kit, keys.last + 1),
                Err(InitError::InvalidPartial(keys.last + 1))
            );
        }
    }

    #[test]
    fn recognises_only_finished() {
        assert!(is_init_finished(&INIT_FINISHED_ADDR, &[0; 4]));
        assert!(!is_init_finished(&STARTED, &[0; 4]));
        assert!(!is_init_finished(&INIT_FINISHED_ADDR, &[0, 0, 0, 1]));
    }

    #[test]
    fn rejects_a_missing_part() {
        let count = INTEGRA7.part_count;
        for target in [
            InitTarget::Part(count),
            InitTarget::Tone(count),
            InitTarget::partial(count, ToneType::SnSynth, 0).unwrap(),
        ] {
            assert_eq!(
                Initialization::new(0x10, target).err(),
                Some(InitError::InvalidPart(count))
            );
        }
        assert!(Initialization::new(0x10, InitTarget::Part(count - 1)).is_ok());
        assert!(Initialization::new(0x10, InitTarget::SoundCtrl).is_ok());
    }

    #[test]
    fn sends_then_awaits_finished() {
        let mut init = Initialization::new(0x10, InitTarget::Part(15)).unwrap();
        assert_eq!(init.target(), InitTarget::Part(15));
        assert_eq!(
            init.next_action(),
            InitAction::Send(request(0x10, 0x0F, 0x00, 0x3C))
        );
        let awaiting = InitAction::Await {
            timeout_ms: INIT_TIMEOUT_MS,
        };
        assert_eq!(init.next_action(), awaiting);
        assert!(!init.on_reply(&STARTED, &[0; 4]));
        assert_eq!(init.next_action(), awaiting);
        assert!(init.on_reply(&INIT_FINISHED_ADDR, &[0; 4]));
        assert_eq!(init.next_action(), InitAction::Done(Ok(())));
        // A late reply or timeout changes nothing.
        assert!(!init.on_reply(&INIT_FINISHED_ADDR, &[0; 4]));
        init.on_timeout();
        assert_eq!(init.next_action(), InitAction::Done(Ok(())));
    }

    #[test]
    fn finished_before_sending_is_ignored() {
        let mut init = Initialization::new(0x10, InitTarget::SoundCtrl).unwrap();
        assert!(!init.on_reply(&INIT_FINISHED_ADDR, &[0; 4]));
        init.on_timeout();
        assert!(matches!(init.next_action(), InitAction::Send(_)));
    }

    #[test]
    fn timeout_fails() {
        let mut init = Initialization::new(0x10, InitTarget::Tone(0)).unwrap();
        init.next_action();
        init.on_timeout();
        assert_eq!(
            init.next_action(),
            InitAction::Done(Err(InitError::Timeout))
        );
    }

    #[test]
    fn cancel_stops_only_before_sending() {
        let mut init = Initialization::new(0x10, InitTarget::Tone(0)).unwrap();
        init.cancel();
        assert_eq!(
            init.next_action(),
            InitAction::Done(Err(InitError::Cancelled))
        );

        let mut init = Initialization::new(0x10, InitTarget::Tone(0)).unwrap();
        init.next_action();
        init.cancel();
        assert!(matches!(init.next_action(), InitAction::Await { .. }));
        assert!(init.on_reply(&INIT_FINISHED_ADDR, &[0; 4]));
        assert_eq!(init.next_action(), InitAction::Done(Ok(())));
    }
}
