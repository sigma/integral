//! High-level device state machine for the INTEGRA-7.
//!
//! [`DeviceState`] owns the mixer state, send queue, and echo suppression.
//! It is platform-agnostic: the caller feeds it incoming MIDI bytes and a
//! monotonic timestamp, and it returns outbound messages + state changes.
//!
//! # Usage pattern (from a web or VST host)
//!
//! ```text
//! loop {
//!     // 1. Feed incoming MIDI
//!     let changes = device.handle_incoming(raw_bytes, now_ms);
//!     // 2. Drain outbound queue
//!     while let Some(msg) = device.drain(now_ms) {
//!         midi_output.send(&msg);
//!     }
//!     // 3. Read state for rendering
//!     let state = device.state();
//! }
//! ```

use std::collections::HashMap;

use crate::address::{Address, DataSize};
use crate::block_field::{BlockField, FieldIndexError};
use crate::device_spec::KeyOutOfRange;
use thiserror::Error;

use crate::expansion_load::{self, LoadAction, LoadMachine, LoadOutcome, LoadPlan};
use crate::expansion_slots::{self, ExpansionSlots};
use crate::expansions::Expansion;
use crate::midi_bridge::RequestTracker;
use crate::mixer_load::{self, MixerBlock};
use crate::param_registry::{
    CompEqKit, MixerField, MixerFieldError, MixerScope, MixerScopeKind, mixer_field_address,
};
use crate::params::{self, ToneType, part};
use crate::parse_helpers::encode_ascii_name;
use crate::pcm_synth::{PcmsCommonField, PcmsPartialField, PcmsPmtField};
use crate::sn_acoustic::SnaCommonField;
use crate::sn_drum::{SndCommonField, SndNoteField};
use crate::sn_synth::{SnSynthTone, SnSynthToneBlocks, SnsBlock, SnsCommonField, SnsPartialField};
use crate::state::{EqState, MixerState, SurroundPartState};
use crate::sysex;

/// Minimum interval between SysEx sends (ms).
const THROTTLE_MS: f64 = 40.0;

/// Duration to suppress incoming DT1 echoes after a local send (ms).
const ECHO_SUPPRESS_MS: f64 = 150.0;

/// How long a tracked read (SN Synth or mixer block) waits for its reply
/// before it is sent again (ms).
const REPLY_TIMEOUT_MS: f64 = 2000.0;

/// How many times an unanswered tracked read is sent again.
const MAX_RETRIES: u8 = 2;

// ---------------------------------------------------------------------------
// Send queue
// ---------------------------------------------------------------------------

struct QueuedMessage {
    key: String,
    bytes: Vec<u8>,
    /// Whether `key` is a request whose reply the request tracker awaits
    /// once the message is sent.
    tracked: bool,
}

/// A part's SN Synth Tone being read from the device: the tone it was
/// requested for, the blocks so far, and how often each block was asked
/// again.
struct SnsToneLoad {
    /// The part's (bank MSB, bank LSB, PC) when the load started.
    tone: (u8, u8, u8),
    blocks: SnSynthToneBlocks,
    retries: [u8; SnsBlock::ALL.len()],
}

/// A mixer block being read from the device.
struct MixerRead {
    block: MixerBlock,
    address: Address,
    retries: u8,
}

/// The request tracker key of the mixer read at `address`; distinct from
/// SN Synth block keys, whose Common block starts at a tone name address.
fn mixer_request_key(address: &Address) -> String {
    format!("mixer:{address}")
}

/// The request tracker key of `part`'s `block` request.
fn sns_request_key(part: u8, block: SnsBlock) -> String {
    format!("rq1:{}", block.address(part))
}

/// The state value a [`MixerField`] writes.
enum Slot<'a> {
    /// A byte value.
    Byte(&'a mut u8),
    /// A switch or mute.
    Flag(&'a mut bool),
}

/// The state value `field` writes in `scope`, or `None` if the field is
/// not in the scope. Indices must already be in range
/// ([`mixer_field_address`] checks them).
fn mixer_slot(state: &mut MixerState, scope: MixerScope, field: MixerField) -> Option<Slot<'_>> {
    use MixerField as F;
    use Slot::{Byte, Flag};
    Some(match scope {
        MixerScope::Part(part) => {
            let part = usize::from(part);
            let p = &mut state.parts[part];
            match field {
                F::PartLevel => Byte(&mut p.level),
                F::PartPan => Byte(&mut p.pan),
                F::PartMute => Flag(&mut p.muted),
                F::PartChorusSend => Byte(&mut p.chorus_send),
                F::PartReverbSend => Byte(&mut p.reverb_send),
                F::PartOutputAssign => Byte(&mut p.output_assign),
                F::PartReceiveChannel => Byte(&mut p.receive_channel),
                F::SurroundLr | F::SurroundFb | F::SurroundWidth | F::SurroundAmbienceSend => {
                    surround_slot(&mut state.surround.parts[part], field)?
                }
                _ => eq_slot(&mut p.eq, field)?,
            }
        }
        MixerScope::Ext => match field {
            F::ExtLevel => Byte(&mut state.ext_level),
            F::ExtMute => Flag(&mut state.ext_muted),
            _ => surround_slot(&mut state.surround.ext, field)?,
        },
        MixerScope::Master => match field {
            F::MasterLevel => Byte(&mut state.master_level),
            _ => eq_slot(&mut state.master_eq, field)?,
        },
        MixerScope::CompEqUnit { unit, .. } => {
            let unit = usize::from(unit);
            if field == F::CompOutputAssign {
                return Some(Byte(&mut state.drum_comp_eq.output_assigns[unit]));
            }
            let u = &mut state.drum_comp_eq.units[unit];
            match field {
                F::CompSwitch => Flag(&mut u.comp_switch),
                F::CompAttack => Byte(&mut u.comp_attack),
                F::CompRelease => Byte(&mut u.comp_release),
                F::CompThreshold => Byte(&mut u.comp_threshold),
                F::CompRatio => Byte(&mut u.comp_ratio),
                F::CompOutputGain => Byte(&mut u.comp_output_gain),
                F::EqSwitch => Flag(&mut u.eq_switch),
                F::EqLowFreq => Byte(&mut u.eq_low_freq),
                F::EqLowGain => Byte(&mut u.eq_low_gain),
                F::EqMidFreq => Byte(&mut u.eq_mid_freq),
                F::EqMidGain => Byte(&mut u.eq_mid_gain),
                F::EqMidQ => Byte(&mut u.eq_mid_q),
                F::EqHighFreq => Byte(&mut u.eq_high_freq),
                F::EqHighGain => Byte(&mut u.eq_high_gain),
                _ => return None,
            }
        }
        MixerScope::Common => match field {
            F::ChorusSwitch => Flag(&mut state.chorus.enabled),
            F::ChorusType => Byte(&mut state.chorus.fx_type),
            F::ChorusLevel => Byte(&mut state.chorus.level),
            F::ChorusOutputAssign => Byte(&mut state.chorus.output_assign),
            F::ChorusOutputSelect => Byte(&mut state.chorus.output),
            F::ReverbSwitch => Flag(&mut state.reverb.enabled),
            F::ReverbType => Byte(&mut state.reverb.fx_type),
            F::ReverbLevel => Byte(&mut state.reverb.level),
            F::ReverbOutputAssign => Byte(&mut state.reverb.output),
            F::SoloPart => Byte(&mut state.solo_part),
            F::DrumCompEqSwitch => Flag(&mut state.drum_comp_eq.enabled),
            F::DrumCompEqPart => Byte(&mut state.drum_comp_eq.part),
            F::SurroundSwitch => Flag(&mut state.surround.enabled),
            F::SurroundRoomType => Byte(&mut state.surround.room_type),
            F::SurroundRoomSize => Byte(&mut state.surround.room_size),
            F::SurroundDepth => Byte(&mut state.surround.depth),
            F::SurroundAmbienceLevel => Byte(&mut state.surround.ambience_level),
            F::SurroundAmbienceTime => Byte(&mut state.surround.ambience_time),
            F::SurroundAmbienceDensity => Byte(&mut state.surround.ambience_density),
            F::SurroundAmbienceHfDamp => Byte(&mut state.surround.ambience_hf_damp),
            _ => return None,
        },
    })
}

/// The [`SurroundPartState`] value a part's (or the Ext Part's) surround
/// field writes.
fn surround_slot(position: &mut SurroundPartState, field: MixerField) -> Option<Slot<'_>> {
    use MixerField as F;
    Some(Slot::Byte(match field {
        F::SurroundLr => &mut position.lr,
        F::SurroundFb => &mut position.fb,
        F::SurroundWidth => &mut position.width,
        F::SurroundAmbienceSend => &mut position.ambience_send,
        _ => return None,
    }))
}

/// The [`EqState`] value an EQ field writes.
fn eq_slot(eq: &mut EqState, field: MixerField) -> Option<Slot<'_>> {
    use MixerField as F;
    use Slot::{Byte, Flag};
    Some(match field {
        F::EqSwitch => Flag(&mut eq.enabled),
        F::EqLowFreq => Byte(&mut eq.low_freq),
        F::EqLowGain => Byte(&mut eq.low_gain),
        F::EqMidFreq => Byte(&mut eq.mid_freq),
        F::EqMidGain => Byte(&mut eq.mid_gain),
        F::EqMidQ => Byte(&mut eq.mid_q),
        F::EqHighFreq => Byte(&mut eq.high_freq),
        F::EqHighGain => Byte(&mut eq.high_gain),
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// DeviceState
// ---------------------------------------------------------------------------

/// High-level state machine for INTEGRA-7 communication.
///
/// Owns the mixer state, send queue with coalescing/throttle, and echo
/// suppression.  All methods are synchronous and return outbound messages
/// rather than performing I/O.
pub struct DeviceState {
    device_id: u8,
    state: MixerState,
    queue: Vec<QueuedMessage>,
    last_send_time: f64,
    /// Timestamps of recently sent DT1 messages, keyed by address hex.
    echo_tracker: HashMap<String, f64>,
    /// Loaded Expansion slots, or `None` while unknown.
    expansion_slots: Option<ExpansionSlots>,
    /// The running Expansion load; the send queue is held while set.
    load: Option<LoadMachine>,
    /// Each part's SN Synth Tone, by part index, once a load started.
    sns_tones: HashMap<u8, SnsToneLoad>,
    /// Mixer blocks being read (see [`load_mixer`](Self::load_mixer)).
    mixer_reads: Vec<MixerRead>,
    /// Mixer blocks whose read gave up, or that nothing could build a read
    /// for (see [`queue_mixer_reads`](Self::queue_mixer_reads)).
    mixer_failed: Vec<MixerBlock>,
    /// Sent SN Synth block and mixer requests awaiting their reply.
    requests: RequestTracker,
}

/// Why an Expansion load can't start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum LoadError {
    /// The current slots are unknown; read them first.
    #[error("Expansion slots are unknown; read them first")]
    SlotsUnknown,
    /// Another load is running.
    #[error("an Expansion load is already running")]
    Busy,
}

impl DeviceState {
    /// Create a new device state with the given SysEx device ID.
    pub fn new(device_id: u8) -> Self {
        Self {
            device_id,
            state: MixerState::default(),
            queue: Vec::new(),
            last_send_time: -THROTTLE_MS,
            echo_tracker: HashMap::new(),
            expansion_slots: None,
            load: None,
            sns_tones: HashMap::new(),
            mixer_reads: Vec::new(),
            mixer_failed: Vec::new(),
            requests: RequestTracker::new(),
        }
    }

    /// Read-only access to the current mixer state.
    pub fn state(&self) -> &MixerState {
        &self.state
    }

    /// Mutable access to the mixer state (for direct patching during load).
    pub fn state_mut(&mut self) -> &mut MixerState {
        &mut self.state
    }

    // -----------------------------------------------------------------------
    // Expansion slots
    // -----------------------------------------------------------------------

    /// The loaded Expansion slots, or `None` while unknown.
    pub fn expansion_slots(&self) -> Option<&ExpansionSlots> {
        self.expansion_slots.as_ref()
    }

    /// Build the RQ1 that reads the Expansion slots.
    pub fn build_expansion_slots_request(&self) -> Vec<u8> {
        expansion_slots::build_read_request(self.device_id)
    }

    /// Record the slots a slot read answered.
    pub fn set_expansion_slots(&mut self, slots: ExpansionSlots) {
        self.expansion_slots = Some(slots);
    }

    /// Record that a slot read got no answer: the slots are unknown.
    pub fn expansion_slots_timed_out(&mut self) {
        self.expansion_slots = None;
    }

    /// Whether `expansion` is loaded, or `None` while the slots are unknown.
    pub fn is_loaded(&self, expansion: &Expansion) -> Option<bool> {
        self.expansion_slots.map(|slots| slots.contains(expansion))
    }

    /// Feed a DT1 that may concern the Expansion slots: a load reply, or a
    /// slot read answer. Returns `true` if it did.
    pub fn handle_expansion_dt1(&mut self, address: &Address, data: &[u8], now_ms: f64) -> bool {
        if let Some(load) = &mut self.load
            && load.on_dt1(address, data, now_ms)
        {
            return true;
        }
        let Some(reply) = expansion_slots::parse_read_reply(address, data) else {
            return false;
        };
        self.expansion_slots = reply.ok();
        true
    }

    /// Start loading `target` into the Expansion slots.
    ///
    /// Holds the send queue until the load is over. Drive it with
    /// [`next_load_action`](Self::next_load_action).
    pub fn start_expansion_load(&mut self, target: ExpansionSlots) -> Result<(), LoadError> {
        if self.load.is_some() {
            return Err(LoadError::Busy);
        }
        let current = self.expansion_slots.ok_or(LoadError::SlotsUnknown)?;
        self.load = Some(LoadMachine::new(self.device_id, current, target));
        Ok(())
    }

    /// What loading `target` over the current slots would change, given the
    /// Parts' current tone banks.
    pub fn plan_expansion_load(&self, target: &ExpansionSlots) -> Result<LoadPlan, LoadError> {
        let current = self.expansion_slots.ok_or(LoadError::SlotsUnknown)?;
        Ok(expansion_load::plan(&current, target, &self.part_banks()))
    }

    /// Parts (1-based) whose current tone comes from `expansion`.
    pub fn parts_using_expansion(&self, expansion: &Expansion) -> Vec<u8> {
        expansion_load::parts_using(expansion, &self.part_banks())
    }

    /// The slot (0 = A) to stage another Expansion into over `slots`: an
    /// OFF slot, else one whose Expansion no current Part uses, else A.
    pub fn preselect_expansion_slot(&self, slots: &ExpansionSlots) -> usize {
        expansion_load::preselect_slot(slots, &self.part_banks())
    }

    /// Each Part's tone bank (MSB, LSB), Part 1 first.
    fn part_banks(&self) -> Vec<(u8, u8)> {
        self.state
            .parts
            .iter()
            .map(|p| (p.tone_bank_msb, p.tone_bank_lsb))
            .collect()
    }

    /// The running load's target, if a load is running.
    pub fn expansion_load_target(&self) -> Option<&ExpansionSlots> {
        self.load.as_ref().map(LoadMachine::target)
    }

    /// The next step of the running load, or `None` if none runs.
    ///
    /// `Send` and `Poll` bytes go out at once, bypassing the held queue.
    /// On `Done` the slots are updated (unknown after a timeout) and the
    /// queue is released.
    ///
    /// A `Send` or `Poll` due within the SysEx throttle of the previous
    /// send is deferred: a `Wait` until the throttle elapses comes back
    /// instead.
    pub fn next_load_action(&mut self, now_ms: f64) -> Option<LoadAction> {
        let load = self.load.as_mut()?;
        let mut next = load.clone();
        let action = next.next_action(now_ms);
        if let LoadAction::Send(_) | LoadAction::Poll(_) = action {
            let since_send = now_ms - self.last_send_time;
            if since_send < THROTTLE_MS {
                return Some(LoadAction::Wait(THROTTLE_MS - since_send));
            }
            self.last_send_time = now_ms;
        }
        *load = next;
        if let LoadAction::Done(outcome) = action {
            self.load = None;
            match outcome {
                LoadOutcome::NoOp => {}
                LoadOutcome::Loaded(slots) | LoadOutcome::Failed(slots) => {
                    self.expansion_slots = Some(slots);
                }
                LoadOutcome::TimedOut => self.expansion_slots = None,
            }
        }
        Some(action)
    }

    // -----------------------------------------------------------------------
    // Send queue
    // -----------------------------------------------------------------------

    /// Queue a DT1 message.  If a DT1 for the same address is already
    /// queued, replace it (coalescing).
    pub fn send_dt1(&mut self, address: &Address, data: &[u8]) {
        let bytes = sysex::build_dt1(self.device_id, address, data);
        let key = format!("dt1:{address}");

        // Mark for echo suppression (sentinel = not yet sent).
        self.echo_tracker
            .insert(format!("{address}"), f64::NEG_INFINITY);

        if let Some(existing) = self.queue.iter_mut().find(|m| m.key == key) {
            existing.bytes = bytes;
        } else {
            self.queue.push(QueuedMessage {
                key,
                bytes,
                tracked: false,
            });
        }
    }

    /// Queue a raw SysEx message (non-DT1, e.g. catalog queries).
    pub fn send_raw(&mut self, key: &str, bytes: Vec<u8>) {
        self.queue.push(QueuedMessage {
            key: key.to_string(),
            bytes,
            tracked: false,
        });
    }

    /// Drain the next outbound message if the throttle window has elapsed.
    ///
    /// `now_ms` is a monotonic timestamp in milliseconds. Before draining,
    /// SN Synth block and mixer requests whose reply timed out are queued
    /// again (see [`load_sns_tone`](Self::load_sns_tone) and
    /// [`load_mixer`](Self::load_mixer)).
    ///
    /// Returns `None` if the queue is empty, the throttle hasn't elapsed, or
    /// an Expansion load holds the queue.
    pub fn drain(&mut self, now_ms: f64) -> Option<Vec<u8>> {
        if self.load.is_some() {
            return None;
        }
        self.retry_requests(now_ms);
        if self.queue.is_empty() || now_ms - self.last_send_time < THROTTLE_MS {
            return None;
        }
        let msg = self.queue.remove(0);
        self.last_send_time = now_ms;
        if msg.tracked {
            self.requests.register_request(msg.key, now_ms);
        }

        // Stamp echo tracker entries that were queued but not yet sent.
        for ts in self.echo_tracker.values_mut() {
            if *ts == f64::NEG_INFINITY {
                *ts = now_ms;
            }
        }

        Some(msg.bytes)
    }

    /// Returns `true` if there are queued messages waiting to be sent.
    pub fn has_pending(&self) -> bool {
        !self.queue.is_empty()
    }

    // -----------------------------------------------------------------------
    // Echo suppression
    // -----------------------------------------------------------------------

    fn is_suppressed(&mut self, address: &Address, now_ms: f64) -> bool {
        let key = format!("{address}");
        if let Some(&ts) = self.echo_tracker.get(&key) {
            if ts != f64::NEG_INFINITY && now_ms - ts < ECHO_SUPPRESS_MS {
                return true;
            }
            self.echo_tracker.remove(&key);
        }
        false
    }

    // -----------------------------------------------------------------------
    // Incoming DT1 handling
    // -----------------------------------------------------------------------

    /// Process an incoming DT1 message and update state.
    ///
    /// Returns `true` if the state was changed.
    pub fn handle_dt1(&mut self, address: &Address, data: &[u8], now_ms: f64) -> bool {
        // A block reply is never an echo of a single-parameter edit.
        if self.handle_sns_block(address, data) || self.handle_mixer_reply(address, data) {
            return true;
        }
        if self.is_suppressed(address, now_ms) {
            return false;
        }
        if data.is_empty() {
            return false;
        }
        let value = data[0];

        // Match against part parameters.
        for part_idx in 0u8..16 {
            let level_addr = params::part_address(part_idx, part::LEVEL);
            if *address == level_addr {
                self.state.parts[part_idx as usize].level = value;
                return true;
            }
            let pan_addr = params::part_address(part_idx, part::PAN);
            if *address == pan_addr {
                self.state.parts[part_idx as usize].pan = value;
                return true;
            }
            let mute_addr = params::part_address(part_idx, part::MUTE);
            if *address == mute_addr {
                self.state.parts[part_idx as usize].muted = value == 1;
                return true;
            }
            let oa_addr = params::part_address(part_idx, part::OUTPUT_ASSIGN);
            if *address == oa_addr {
                self.state.parts[part_idx as usize].output_assign = value;
                return true;
            }
        }

        false
    }

    // -----------------------------------------------------------------------
    // Convenience setters (mutate state + queue DT1)
    // -----------------------------------------------------------------------

    /// Number of parts on the INTEGRA-7.
    pub const NUM_PARTS: u8 = 16;

    /// Set a part's level (0–127).
    ///
    /// # Panics
    /// Debug-asserts that `part` < 16.
    pub fn set_part_level(&mut self, part: u8, value: u8) {
        debug_assert!(part < Self::NUM_PARTS, "part index {part} out of range");
        self.state.parts[part as usize].level = value;
        let addr = params::part_address(part, part::LEVEL);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a part's pan (0–127, 64=centre).
    ///
    /// # Panics
    /// Debug-asserts that `part` < 16.
    pub fn set_part_pan(&mut self, part: u8, value: u8) {
        debug_assert!(part < Self::NUM_PARTS, "part index {part} out of range");
        self.state.parts[part as usize].pan = value;
        let addr = params::part_address(part, part::PAN);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a part's mute state.
    ///
    /// # Panics
    /// Debug-asserts that `part` < 16.
    pub fn set_part_mute(&mut self, part: u8, muted: bool) {
        debug_assert!(part < Self::NUM_PARTS, "part index {part} out of range");
        self.state.parts[part as usize].muted = muted;
        let addr = params::part_address(part, part::MUTE);
        self.send_dt1(&addr, &[u8::from(muted)]);
    }

    /// Toggle a part's mute state.
    ///
    /// # Panics
    /// Debug-asserts that `part` < 16.
    pub fn toggle_part_mute(&mut self, part: u8) {
        debug_assert!(part < Self::NUM_PARTS, "part index {part} out of range");
        let muted = !self.state.parts[part as usize].muted;
        self.set_part_mute(part, muted);
    }

    /// Set a part's chorus send level (0–127).
    ///
    /// # Panics
    /// Debug-asserts that `part` < 16.
    pub fn set_part_chorus_send(&mut self, part: u8, value: u8) {
        debug_assert!(part < Self::NUM_PARTS, "part index {part} out of range");
        self.state.parts[part as usize].chorus_send = value;
        let addr = params::part_address(part, part::CHORUS_SEND);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a part's reverb send level (0–127).
    ///
    /// # Panics
    /// Debug-asserts that `part` < 16.
    pub fn set_part_reverb_send(&mut self, part: u8, value: u8) {
        debug_assert!(part < Self::NUM_PARTS, "part index {part} out of range");
        self.state.parts[part as usize].reverb_send = value;
        let addr = params::part_address(part, part::REVERB_SEND);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a part's receive channel (0–15).
    ///
    /// # Panics
    /// Debug-asserts that `part` < 16.
    pub fn set_part_receive_channel(&mut self, part: u8, channel: u8) {
        debug_assert!(part < Self::NUM_PARTS, "part index {part} out of range");
        self.state.parts[part as usize].receive_channel = channel;
        let addr = params::part_address(part, part::RECEIVE_CHANNEL);
        self.send_dt1(&addr, &[channel]);
    }

    /// Set a part's output assign (0=A, 1=B, 2=C, 3=D, 4–11=1–8).
    ///
    /// # Panics
    /// Debug-asserts that `part` < 16.
    pub fn set_part_output_assign(&mut self, part: u8, value: u8) {
        debug_assert!(part < Self::NUM_PARTS, "part index {part} out of range");
        self.state.parts[part as usize].output_assign = value;
        let addr = params::part_address(part, part::OUTPUT_ASSIGN);
        self.send_dt1(&addr, &[value]);
    }

    /// Change a part's tone (bank select MSB + LSB + PC).
    ///
    /// This enqueues 3 DT1 messages for the tone bank MSB, LSB, and PC.
    ///
    /// # Panics
    /// Debug-asserts that `part` < 16.
    pub fn change_part_tone(&mut self, part: u8, msb: u8, lsb: u8, pc: u8) {
        debug_assert!(part < Self::NUM_PARTS, "part index {part} out of range");
        mixer_load::set_tone(&mut self.state.parts[part as usize], msb, lsb, pc);
        self.invalidate_sns_tone(part);

        let msb_addr = params::part_address(part, part::TONE_BANK_MSB);
        self.send_dt1(&msb_addr, &[msb]);
        let lsb_addr = params::part_address(part, part::TONE_BANK_LSB);
        self.send_dt1(&lsb_addr, &[lsb]);
        let pc_addr = params::part_address(part, part::TONE_PC);
        self.send_dt1(&pc_addr, &[pc]);

        // Reads of the old tone would undo the change: read the new one,
        // after the change is sent.
        let stale: Vec<_> = (self.mixer_reads.iter())
            .filter(|r| r.block.reads_tone_of(part))
            .map(|r| (r.block, r.address))
            .collect();
        for (_, address) in &stale {
            self.drop_mixer_read(address);
        }
        let mut reads = if stale.iter().any(|(b, _)| *b == MixerBlock::Part(part)) {
            vec![MixerBlock::Part(part)]
        } else {
            MixerBlock::tone_reads(part, msb).to_vec()
        };
        if stale
            .iter()
            .any(|(b, _)| matches!(b, MixerBlock::CompEq { .. }))
            // A part that left the drum kit banks has no kit block to read;
            // queueing it anyway would report it failed.
            && params::comp_eq_block_address_for_bank(part, msb).is_some()
        {
            reads.push(MixerBlock::CompEq { part, msb });
        }
        self.queue_mixer_reads(reads);
    }

    /// Set a part's Temporary tone name (ASCII, truncated or space-padded
    /// to the device's 12 characters), e.g. before saving it to a user slot.
    ///
    /// Returns `false` and sends nothing if the part's bank MSB doesn't map
    /// to a known tone type.
    ///
    /// # Panics
    /// Debug-asserts that `part` < 16.
    pub fn rename_tone(&mut self, part: u8, name: &str) -> bool {
        debug_assert!(part < Self::NUM_PARTS, "part index {part} out of range");
        let p = &mut self.state.parts[part as usize];
        let Some(tone_type) = params::tone_type_from_bank_msb(p.tone_bank_msb) else {
            return false;
        };
        let data = encode_ascii_name(name, params::TONE_NAME_SIZE.value() as usize);
        p.tone_name = stored_name(&data);
        self.send_dt1(&params::tone_name_address(part, tone_type), &data);
        true
    }

    /// Set the Temporary Studio Set name (ASCII, truncated or space-padded
    /// to the device's 16 characters).
    pub fn rename_studio_set(&mut self, name: &str) {
        let data = encode_ascii_name(name, params::STUDIO_SET_NAME_SIZE.value() as usize);
        self.state.studio_set_name = stored_name(&data);
        self.send_dt1(&params::STUDIO_SET_NAME, &data);
    }

    /// Set the system master level (0–127).
    pub fn set_master_level(&mut self, value: u8) {
        self.state.master_level = value;
        self.send_dt1(&params::SYSTEM_MASTER_LEVEL, &[value]);
    }

    /// Set a chorus core parameter (type, level, output assign or output
    /// select, at the [`params::chorus`] offsets).
    pub fn set_chorus_param(&mut self, offset: u8, value: u8) {
        match offset {
            params::chorus::TYPE => self.state.chorus.fx_type = value,
            params::chorus::LEVEL => self.state.chorus.level = value,
            params::chorus::OUTPUT_ASSIGN => self.state.chorus.output_assign = value,
            params::chorus::OUTPUT_SELECT => self.state.chorus.output = value,
            _ => return,
        }
        let addr = params::chorus_address(offset);
        self.send_dt1(&addr, &[value]);
    }

    /// Toggle the chorus switch.
    pub fn toggle_chorus_switch(&mut self) {
        self.state.chorus.enabled = !self.state.chorus.enabled;
        self.send_dt1(
            &params::CHORUS_SWITCH,
            &[u8::from(self.state.chorus.enabled)],
        );
    }

    /// Write a nibblized chorus FX parameter.
    pub fn set_chorus_nib_param(&mut self, param_index: usize, value: i32) {
        if param_index < self.state.chorus.params.len() {
            self.state.chorus.params[param_index] = value;
        }
        let bytes = crate::state::parse::encode_nib_param(value);
        let offset = params::chorus::PARAM_BASE + (param_index as u8) * 4;
        let addr = params::chorus_address(offset);
        self.send_dt1(&addr, &bytes);
    }

    /// Set a reverb core parameter (type, level or output assign, at the
    /// [`params::reverb`] offsets).
    pub fn set_reverb_param(&mut self, offset: u8, value: u8) {
        match offset {
            params::reverb::TYPE => self.state.reverb.fx_type = value,
            params::reverb::LEVEL => self.state.reverb.level = value,
            params::reverb::OUTPUT_ASSIGN => self.state.reverb.output = value,
            _ => return,
        }
        let addr = params::reverb_address(offset);
        self.send_dt1(&addr, &[value]);
    }

    /// Toggle the reverb switch.
    pub fn toggle_reverb_switch(&mut self) {
        self.state.reverb.enabled = !self.state.reverb.enabled;
        self.send_dt1(
            &params::REVERB_SWITCH,
            &[u8::from(self.state.reverb.enabled)],
        );
    }

    /// Write a nibblized reverb FX parameter.
    pub fn set_reverb_nib_param(&mut self, param_index: usize, value: i32) {
        if param_index < self.state.reverb.params.len() {
            self.state.reverb.params[param_index] = value;
        }
        let bytes = crate::state::parse::encode_nib_param(value);
        let offset = params::reverb::PARAM_BASE + (param_index as u8) * 4;
        let addr = params::reverb_address(offset);
        self.send_dt1(&addr, &bytes);
    }

    /// Set external part level (0–127).
    pub fn set_ext_level(&mut self, value: u8) {
        self.state.ext_level = value;
        self.send_dt1(&params::EXT_PART_LEVEL, &[value]);
    }

    /// Toggle external part mute.
    pub fn toggle_ext_mute(&mut self) {
        self.state.ext_muted = !self.state.ext_muted;
        self.send_dt1(&params::EXT_PART_MUTE, &[u8::from(self.state.ext_muted)]);
    }

    /// Set the Solo Part (0=OFF, 1–16=solo that part).
    pub fn set_solo_part(&mut self, value: u8) {
        self.state.solo_part = value;
        self.send_dt1(&params::SOLO_PART, &[value]);
    }

    /// Toggle solo for a part (0-indexed).
    ///
    /// If the part is already soloed, turns solo off.
    /// Otherwise, solos that part (exclusive — only one at a time).
    pub fn toggle_solo(&mut self, part: u8) {
        if self.state.solo_part == part + 1 {
            self.set_solo_part(0);
        } else {
            self.set_solo_part(part + 1);
        }
    }

    /// Switch studio set by program change.
    ///
    /// Sends BS MSB=85, BS LSB=0, PC=pc.
    pub fn switch_studio_set(&mut self, pc: u8) {
        self.state.studio_set_pc = pc;
        for part in 0..Self::NUM_PARTS {
            self.invalidate_sns_tone(part);
        }
        self.send_dt1(&params::SETUP_STUDIO_SET_BS_MSB, &[85]);
        self.send_dt1(&params::SETUP_STUDIO_SET_BS_LSB, &[0]);
        self.send_dt1(&params::SETUP_STUDIO_SET_PC, &[pc]);
    }

    // -----------------------------------------------------------------------
    // MFX
    // -----------------------------------------------------------------------

    /// Set instance `index` (the 0-based control slot for the per-control
    /// fields, 0 otherwise) of an MFX header field of a part's tone of type
    /// `tone_type`.
    pub fn set_mfx_header_param(
        &mut self,
        part: u8,
        tone_type: ToneType,
        field: crate::mfx::MfxHeaderField,
        index: u8,
        value: u8,
    ) -> Result<(), crate::mfx::MfxHeaderIndexError> {
        let offset = crate::mfx::mfx_header_offset(field, index)?;
        let addr = crate::mfx::mfx_address(part, tone_type, offset);
        self.send_dt1(&addr, &[value]);
        Ok(())
    }

    /// Change the MFX type of a part's tone of type `tone_type`, and load the
    /// type's default parameters: the device keeps the old slot values when
    /// only the type byte changes. Also sets Control Assign 1–4 to OFF, since
    /// their values index the old type's destination list. Returns the new
    /// slot values, or `None` for an unknown type.
    pub fn set_mfx_type(
        &mut self,
        part: u8,
        tone_type: ToneType,
        mfx_type: u8,
    ) -> Option<[i32; crate::mfx::MFX_PARAM_COUNT]> {
        use crate::mfx::{MfxHeaderField, mfx_header_offset};
        let slots = crate::mfx_params::default_slots(mfx_type)?;
        self.send_dt1(&crate::mfx::mfx_address(part, tone_type, 0), &[mfx_type]);
        // Control Assign 1-4 back to OFF (each type has its own destination
        // list), then the 32 parameter slots right after them.
        let mut data = vec![0; MfxHeaderField::ControlAssign.count()];
        data.extend(slots.iter().flat_map(|&v| crate::mfx::encode_mfx_param(v)));
        let assign = mfx_header_offset(MfxHeaderField::ControlAssign, 0).ok()?;
        self.send_dt1(&crate::mfx::mfx_address(part, tone_type, assign), &data);
        Some(slots)
    }

    /// Set a nibblized MFX effect parameter (0-based index) of a part's tone
    /// of type `tone_type`.
    pub fn set_mfx_nib_param(
        &mut self,
        part: u8,
        tone_type: ToneType,
        param_index: u8,
        value: i32,
    ) {
        let bytes = crate::mfx::encode_mfx_param(value);
        let addr = crate::mfx::mfx_param_address(part, tone_type, param_index);
        self.send_dt1(&addr, &bytes);
    }

    /// Build an RQ1 to read the full MFX block of a part's tone of type
    /// `tone_type`.
    pub fn build_mfx_request(&self, part: u8, tone_type: ToneType) -> Vec<u8> {
        let addr = crate::mfx::tone_mfx_block_address(part, tone_type);
        sysex::build_rq1(self.device_id, &addr, &crate::mfx::MFX_BLOCK_SIZE)
    }

    // -----------------------------------------------------------------------
    // SN-S Tone Edit
    // -----------------------------------------------------------------------

    /// Set an SN-S Common field for a part, sent nibblized when the field
    /// is, in the cached tone too.
    pub fn set_sns_common_param(&mut self, part: u8, field: SnsCommonField, value: u16) {
        self.set_sns_bytes(part, SnsBlock::Common, field as u8, &field.encode(value));
    }

    /// Set an SN-S Partial field for a part, sent nibblized when the field
    /// is (e.g. the Wave Number), in the cached tone too.
    pub fn set_sns_partial_param(
        &mut self,
        part: u8,
        partial: u8,
        field: SnsPartialField,
        value: u16,
    ) {
        let bytes = field.encode(value);
        self.set_sns_bytes(part, SnsBlock::Partial(partial), field as u8, &bytes);
    }

    /// Send `bytes` at `offset` in `part`'s SN-S `block`, and patch the
    /// cached block with them.
    fn set_sns_bytes(&mut self, part: u8, block: SnsBlock, offset: u8, bytes: &[u8]) {
        let addr = block.address(part).offset([0x00, 0x00, 0x00, offset]);
        self.send_dt1(&addr, bytes);
        if let Some(load) = self.sns_tones.get_mut(&part) {
            load.blocks.patch(block, offset, bytes);
        }
    }

    /// Start reading `part`'s SN Synth Tone (Common and Partials) into the
    /// cache, unless it is already read or being read for the part's
    /// current tone. Returns whether a new read started.
    ///
    /// Requests whose reply doesn't arrive within a timeout are sent again
    /// by [`drain`](Self::drain), a bounded number of times. A tone change
    /// drops the cached tone and its outstanding requests.
    ///
    /// # Panics
    /// Debug-asserts that `part` < 16.
    pub fn load_sns_tone(&mut self, part: u8) -> bool {
        debug_assert!(part < Self::NUM_PARTS, "part index {part} out of range");
        let tone = self.part_tone(part);
        if self.sns_tones.get(&part).is_some_and(|l| l.tone == tone) {
            return false;
        }
        self.invalidate_sns_tone(part);
        self.sns_tones.insert(
            part,
            SnsToneLoad {
                tone,
                blocks: SnSynthToneBlocks::default(),
                retries: [0; SnsBlock::ALL.len()],
            },
        );
        for block in SnsBlock::ALL {
            self.queue_sns_request(part, block);
        }
        true
    }

    /// Drop `part`'s cached SN Synth Tone and read it again, as
    /// [`load_sns_tone`](Self::load_sns_tone) does. Editors call this on
    /// open, so a tone edited on the device's front panel since, or a read
    /// that ran out of retries, is read anew.
    ///
    /// # Panics
    /// Debug-asserts that `part` < 16.
    pub fn reload_sns_tone(&mut self, part: u8) {
        self.invalidate_sns_tone(part);
        self.load_sns_tone(part);
    }

    /// `part`'s cached SN Synth Tone, or `None` until every block of it has
    /// arrived (see [`load_sns_tone`](Self::load_sns_tone)).
    pub fn sns_tone(&self, part: u8) -> Option<SnSynthTone> {
        self.sns_tones.get(&part)?.blocks.tone()
    }

    /// The part's (bank MSB, bank LSB, PC).
    fn part_tone(&self, part: u8) -> (u8, u8, u8) {
        let p = &self.state.parts[part as usize];
        (p.tone_bank_msb, p.tone_bank_lsb, p.tone_pc)
    }

    /// Queue the RQ1 for `part`'s `block`, unless it is queued already.
    fn queue_sns_request(&mut self, part: u8, block: SnsBlock) {
        let key = sns_request_key(part, block);
        if self.queue.iter().any(|m| m.key == key) {
            return;
        }
        let bytes = sysex::build_rq1(self.device_id, &block.address(part), &block.size());
        self.queue.push(QueuedMessage {
            key,
            bytes,
            tracked: true,
        });
    }

    /// Drop `part`'s cached SN Synth Tone and its outstanding requests.
    fn invalidate_sns_tone(&mut self, part: u8) {
        if self.sns_tones.remove(&part).is_none() {
            return;
        }
        for block in SnsBlock::ALL {
            let key = sns_request_key(part, block);
            self.requests.cancel(&key);
            self.queue.retain(|m| m.key != key);
        }
    }

    /// Queue again the requests whose reply timed out, while retries remain.
    fn retry_requests(&mut self, now_ms: f64) {
        for key in self.requests.expire_old(now_ms, REPLY_TIMEOUT_MS) {
            self.retry_sns_request(&key);
            self.retry_mixer_read(&key);
        }
    }

    /// Queue again the SN Synth block request of tracker key `key`, if it is
    /// one, while retries remain and the part's tone hasn't changed.
    fn retry_sns_request(&mut self, key: &str) {
        let Some((part, block)) = self.sns_tones.keys().find_map(|&part| {
            SnsBlock::ALL
                .into_iter()
                .find(|&block| sns_request_key(part, block) == key)
                .map(|block| (part, block))
        }) else {
            return;
        };
        let tone = self.part_tone(part);
        let Some(load) = self.sns_tones.get_mut(&part) else {
            return;
        };
        if load.tone != tone {
            self.invalidate_sns_tone(part);
            return;
        }
        let retries = &mut load.retries[block.index()];
        if load.blocks.has(block) || *retries >= MAX_RETRIES {
            return;
        }
        *retries += 1;
        self.queue_sns_request(part, block);
    }

    /// Store an SN Synth block reply in the cache of a part being read.
    /// Returns whether `address` and `data` were such a reply.
    fn handle_sns_block(&mut self, address: &Address, data: &[u8]) -> bool {
        for (&part, load) in &mut self.sns_tones {
            for block in SnsBlock::ALL {
                if *address == block.address(part) && load.blocks.store(block, data) {
                    self.requests.cancel(&sns_request_key(part, block));
                    return true;
                }
            }
        }
        false
    }

    // -----------------------------------------------------------------------
    // Mixer load
    // -----------------------------------------------------------------------

    /// Start reading the whole mixer state from the device, dropping any
    /// read in progress.
    ///
    /// Replies update [`state`](Self::state) as they arrive, through
    /// [`handle_dt1`](Self::handle_dt1). Requests whose reply doesn't arrive
    /// within a timeout are sent again by [`drain`](Self::drain), a bounded
    /// number of times.
    pub fn load_mixer(&mut self) {
        for read in std::mem::take(&mut self.mixer_reads) {
            self.drop_mixer_read(&read.address);
        }
        self.mixer_failed.clear();
        self.queue_mixer_reads(MixerBlock::initial());
    }

    /// Whether a mixer load still waits for a block the mixer can't be shown
    /// without (see [`MixerBlock::is_primary`]).
    pub fn mixer_loading(&self) -> bool {
        self.mixer_reads.iter().any(|r| r.block.is_primary())
    }

    /// The mixer blocks whose read gave up after `MAX_RETRIES` tries, or
    /// that nothing could build a read for (the kit Comp+EQ block when the
    /// part dump naming it is missing). A fresh
    /// [`load_mixer`](Self::load_mixer) clears them.
    pub fn mixer_failed_blocks(&self) -> &[MixerBlock] {
        &self.mixer_failed
    }

    /// Queue the RQ1 of each block that has something to read.
    fn queue_mixer_reads(&mut self, blocks: impl IntoIterator<Item = MixerBlock>) {
        for block in blocks {
            let Some((address, size)) = block.request() else {
                // Only the kit Comp+EQ block has no read because its
                // prerequisite (the part dump naming the kit's bank) is
                // missing; the other unbuildable blocks (e.g. a GM2 tone's
                // name) simply have nothing to read on the device.
                if let MixerBlock::CompEq { .. } = block {
                    self.mixer_failed.push(block);
                }
                continue;
            };
            self.drop_mixer_read(&address);
            self.mixer_failed.retain(|b| *b != block);
            self.mixer_reads.push(MixerRead {
                block,
                address,
                retries: 0,
            });
            self.queue_mixer_request(&address, &size);
        }
    }

    /// Queue the RQ1 of the mixer read at `address`, unless it is queued
    /// already.
    fn queue_mixer_request(&mut self, address: &Address, size: &DataSize) {
        let key = mixer_request_key(address);
        if self.queue.iter().any(|m| m.key == key) {
            return;
        }
        let bytes = sysex::build_rq1(self.device_id, address, size);
        self.queue.push(QueuedMessage {
            key,
            bytes,
            tracked: true,
        });
    }

    /// Forget the mixer read at `address` and its outstanding request.
    fn drop_mixer_read(&mut self, address: &Address) {
        self.mixer_reads.retain(|r| r.address != *address);
        let key = mixer_request_key(address);
        self.requests.cancel(&key);
        self.queue.retain(|m| m.key != key);
    }

    /// Queue again the mixer read of tracker key `key`, if it is one, while
    /// retries remain; else give it up.
    fn retry_mixer_read(&mut self, key: &str) {
        let Some(i) = (self.mixer_reads.iter()).position(|r| mixer_request_key(&r.address) == key)
        else {
            return;
        };
        let read = &mut self.mixer_reads[i];
        if read.retries >= MAX_RETRIES {
            let block = read.block;
            self.mixer_reads.remove(i);
            self.mixer_failed.push(block);
            return;
        }
        read.retries += 1;
        let (address, size) = read.block.request().expect("a queued read has a request");
        self.queue_mixer_request(&address, &size);
    }

    /// Apply a reply to a pending mixer read, queueing the reads it makes
    /// possible. Returns whether `address` and `data` were such a reply.
    fn handle_mixer_reply(&mut self, address: &Address, data: &[u8]) -> bool {
        let Some(i) = self.mixer_reads.iter().position(|r| {
            r.address == *address
                && r.block
                    .request()
                    .is_some_and(|(_, size)| size.value() as usize == data.len())
        }) else {
            return false;
        };
        let read = self.mixer_reads.remove(i);
        self.requests.cancel(&mixer_request_key(address));
        let next = read.block.apply(&mut self.state, data);
        self.queue_mixer_reads(next);
        true
    }

    // -----------------------------------------------------------------------
    // SN-A Tone Edit
    // -----------------------------------------------------------------------

    /// Set value `index` (0-based; 0 unless the field is the Modify
    /// Parameters) of an SN-A Common field for a part, sent nibblized when
    /// the field is.
    pub fn set_sna_common_param(
        &mut self,
        part: u8,
        field: SnaCommonField,
        index: u8,
        value: u16,
    ) -> Result<(), FieldIndexError<SnaCommonField>> {
        let position = field.element_position(index)?;
        let addr = crate::sn_acoustic::sna_common_param_address(part, position);
        self.send_dt1(&addr, &field.encode(value));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // SN-D Tone Edit
    // -----------------------------------------------------------------------

    /// Set an SN-D Common field for a part.
    pub fn set_snd_common_param(&mut self, part: u8, field: SndCommonField, value: u8) {
        let addr = crate::sn_drum::snd_common_param_address(part, field as u8);
        self.send_dt1(&addr, &field.encode(value.into()));
    }

    /// Set an SN-D Note field for a part and key, sent nibblized when the
    /// field is.
    pub fn set_snd_note_param(
        &mut self,
        part: u8,
        key: u8,
        field: SndNoteField,
        value: u16,
    ) -> Result<(), KeyOutOfRange> {
        let addr = crate::sn_drum::snd_note_param_address(part, key, field as u8)?;
        self.send_dt1(&addr, &field.encode(value));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // PCM Synth Tone Edit
    // -----------------------------------------------------------------------

    /// Set value `index` (0-based; 0 unless the field holds one value per
    /// Matrix Control) of a PCM Synth Common field for a part.
    pub fn set_pcms_common_param(
        &mut self,
        part: u8,
        field: PcmsCommonField,
        index: u8,
        value: u8,
    ) -> Result<(), FieldIndexError<PcmsCommonField>> {
        let position = field.element_position(index)?;
        let addr = crate::pcm_synth::pcms_common_param_address(part, position);
        self.send_dt1(&addr, &field.encode(value.into()));
        Ok(())
    }

    /// Set value `index` (0-based; the Partial for the per-Partial fields,
    /// else 0) of a PCM Synth PMT field for a part.
    pub fn set_pcms_pmt_param(
        &mut self,
        part: u8,
        field: PcmsPmtField,
        index: u8,
        value: u8,
    ) -> Result<(), FieldIndexError<PcmsPmtField>> {
        let position = field.element_position(index)?;
        let addr = crate::pcm_synth::pcms_pmt_param_address(part, position);
        self.send_dt1(&addr, &field.encode(value.into()));
        Ok(())
    }

    /// Set value `index` (0-based; 0 unless the field is an envelope's
    /// times or levels, a control's switches or the LFO steps) of a PCM
    /// Synth Partial field for a part, sent nibblized when the field is.
    pub fn set_pcms_partial_param(
        &mut self,
        part: u8,
        partial: u8,
        field: PcmsPartialField,
        index: u8,
        value: u16,
    ) -> Result<(), FieldIndexError<PcmsPartialField>> {
        let position = field.element_position(index)?;
        let addr = crate::pcm_synth::pcms_partial_param_address(part, partial, position);
        self.send_dt1(&addr, &field.encode(value));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // PCM Drum Kit Edit
    // -----------------------------------------------------------------------

    /// Set a part's PCM Drum Kit Level (0–127).
    pub fn set_pcmd_kit_level(&mut self, part: u8, value: u8) {
        let addr = crate::pcm_drum::pcmd_common_param_address(
            part,
            crate::pcm_drum::PcmdCommonField::KitLevel as u8,
        );
        self.send_dt1(&addr, &[value]);
    }

    /// Set value `wmt` (0-based WMT layer) of a WMT field in the Partial
    /// for `key`, sent nibblized when the field is.
    pub fn set_pcmd_wmt_param(
        &mut self,
        part: u8,
        key: u8,
        wmt: u8,
        field: crate::pcm_drum::PcmdWmtField,
        value: u16,
    ) -> Result<(), crate::pcm_drum::PcmdFieldError<crate::pcm_drum::PcmdWmtField>> {
        let position = field.element_position(wmt)?;
        let addr = crate::pcm_drum::pcmd_partial_param_address(part, key, position)?;
        self.send_dt1(&addr, &field.encode(value));
        Ok(())
    }

    /// Set element `index` (0-based; 0 unless the field is an envelope's
    /// times or levels) of a non-WMT field in the Partial for `key`.
    pub fn set_pcmd_partial_field(
        &mut self,
        part: u8,
        key: u8,
        field: crate::pcm_drum::PcmdPartialField,
        index: u8,
        value: u8,
    ) -> Result<(), crate::pcm_drum::PcmdFieldError<crate::pcm_drum::PcmdPartialField>> {
        let position = field.element_position(index)?;
        let addr = crate::pcm_drum::pcmd_partial_param_address(part, key, position)?;
        self.send_dt1(&addr, &[value]);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Drum Comp+EQ
    // -----------------------------------------------------------------------

    /// Set the Drum Comp+EQ global switch.
    pub fn set_drum_comp_eq_switch(&mut self, enabled: bool) {
        self.state.drum_comp_eq.enabled = enabled;
        self.send_dt1(&params::DRUM_COMP_EQ_SWITCH, &[u8::from(enabled)]);
    }

    /// Set the Drum Comp+EQ assigned part (0–15).
    pub fn set_drum_comp_eq_part(&mut self, part: u8) {
        self.state.drum_comp_eq.part = part;
        self.send_dt1(&params::DRUM_COMP_EQ_PART, &[part]);
    }

    /// Set a Comp+EQ unit output assign (0–12).
    pub fn set_drum_comp_eq_output_assign(&mut self, unit: u8, value: u8) {
        self.state.drum_comp_eq.output_assigns[unit as usize] = value;
        let addr = params::drum_comp_eq_output_assign(unit);
        self.send_dt1(&addr, &[value]);
    }

    /// The drum kit holding the Drum Comp+EQ units: the assigned part and
    /// its tone type (PCM Drum when its bank is not a known tone type).
    pub fn comp_eq_kit(&self) -> CompEqKit {
        let part = self.state.drum_comp_eq.part;
        let msb = self.state.parts[part as usize].tone_bank_msb;
        CompEqKit {
            part,
            tone_type: params::tone_type_from_bank_msb(msb).unwrap_or(ToneType::PcmDrum),
        }
    }

    /// The [`MixerScope`] of a scope kind and index: the part or Comp+EQ
    /// unit index, ignored for the other kinds. A Comp+EQ unit lives in
    /// the current [`comp_eq_kit`](Self::comp_eq_kit).
    pub fn mixer_scope(&self, kind: MixerScopeKind, index: u8) -> MixerScope {
        match kind {
            MixerScopeKind::Part => MixerScope::Part(index),
            MixerScopeKind::Ext => MixerScope::Ext,
            MixerScopeKind::Master => MixerScope::Master,
            MixerScopeKind::CompEqUnit => MixerScope::CompEqUnit {
                kit: self.comp_eq_kit(),
                unit: index,
            },
            MixerScopeKind::Common => MixerScope::Common,
        }
    }

    /// Set a mixer field in a scope: update the state and send it to the
    /// address core resolves for the scope ([`mixer_field_address`]).
    ///
    /// A switch or mute is on for any non-zero value.
    ///
    /// # Errors
    ///
    /// The [`MixerFieldError`] of [`mixer_field_address`]; nothing is
    /// changed or sent.
    pub fn set_mixer_field(
        &mut self,
        scope: MixerScope,
        field: MixerField,
        value: u8,
    ) -> Result<(), MixerFieldError> {
        let address = mixer_field_address(field, scope)?;
        let slot =
            mixer_slot(&mut self.state, scope, field).ok_or(MixerFieldError::NotInScope {
                field,
                scope: scope.kind(),
            })?;
        match slot {
            Slot::Byte(byte) => *byte = value,
            Slot::Flag(flag) => *flag = value != 0,
        }
        self.send_dt1(&address, &[value]);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Preview
    // -----------------------------------------------------------------------

    /// Start phrase preview on the given part (1–16).
    ///
    /// Sends the undocumented DT1 at `0F 00 20 00` that simulates pressing
    /// the hardware VOLUME knob. If preview is already active on another
    /// part, sends OFF first.
    pub fn preview_start(&mut self, part: u8) {
        assert!((1..=16).contains(&part), "part must be 1–16");
        if self.state.preview_part != 0 {
            self.preview_stop();
        }
        self.state.preview_part = part;
        let addr = Address::new(0x0F, 0x00, 0x20, 0x00);
        let bytes = sysex::build_dt1(self.device_id, &addr, &[part]);
        // Use send_raw with a unique key to avoid coalescing with stop.
        self.send_raw("preview:start", bytes);
    }

    /// Stop any active phrase preview.
    pub fn preview_stop(&mut self) {
        self.state.preview_part = 0;
        let addr = Address::new(0x0F, 0x00, 0x20, 0x00);
        let bytes = sysex::build_dt1(self.device_id, &addr, &[0x00]);
        self.send_raw("preview:stop", bytes);
    }

    /// Toggle phrase preview for the given part (1–16).
    ///
    /// If the same part is already previewing, stops it. Otherwise starts
    /// preview on the new part.
    pub fn preview_toggle(&mut self, part: u8) {
        if self.state.preview_part == part {
            self.preview_stop();
        } else {
            self.preview_start(part);
        }
    }
}

/// The name as the device will report it: the encoded bytes, trailing
/// padding trimmed.
fn stored_name(encoded: &[u8]) -> String {
    String::from_utf8_lossy(encoded).trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expansions;
    use crate::sn_synth;

    #[test]
    fn set_mixer_field_writes_part_and_ext_surround() {
        let mut dev = DeviceState::new(0x10);
        let part = dev.mixer_scope(MixerScopeKind::Part, 3);
        dev.set_mixer_field(part, MixerField::SurroundWidth, 20)
            .unwrap();
        assert_eq!(dev.state().surround.parts[3].width, 20);
        let expected = sysex::build_dt1(
            0x10,
            &params::part_address(3, params::part_surround::WIDTH),
            &[20],
        );
        assert_eq!(dev.drain(0.0), Some(expected));

        let ext = dev.mixer_scope(MixerScopeKind::Ext, 0);
        dev.set_mixer_field(ext, MixerField::SurroundLr, 10)
            .unwrap();
        assert_eq!(dev.state().surround.ext.lr, 10);
        let common = dev.mixer_scope(MixerScopeKind::Common, 0);
        dev.set_mixer_field(common, MixerField::SurroundSwitch, 1)
            .unwrap();
        assert!(dev.state().surround.enabled);
    }

    #[test]
    fn set_mixer_field_writes_part_eq() {
        let mut dev = DeviceState::new(0x10);
        let scope = dev.mixer_scope(MixerScopeKind::Part, 4);
        dev.set_mixer_field(scope, MixerField::EqMidGain, 20)
            .unwrap();
        assert_eq!(dev.state().parts[4].eq.mid_gain, 20);
        let expected = sysex::build_dt1(
            0x10,
            &params::part_eq_address(4, params::part_eq::MID_GAIN),
            &[20],
        );
        assert_eq!(dev.drain(0.0), Some(expected));
    }

    #[test]
    fn set_mixer_field_writes_the_strip_fields_outside_their_blocks() {
        let mut dev = DeviceState::new(0x10);
        let part = dev.mixer_scope(MixerScopeKind::Part, 5);
        dev.set_mixer_field(part, MixerField::SurroundAmbienceSend, 90)
            .unwrap();
        assert_eq!(dev.state().surround.parts[5].ambience_send, 90);
        let unit = dev.mixer_scope(MixerScopeKind::CompEqUnit, 2);
        dev.set_mixer_field(unit, MixerField::CompOutputAssign, 7)
            .unwrap();
        assert_eq!(dev.state().drum_comp_eq.output_assigns[2], 7);
        dev.set_mixer_field(MixerScope::Common, MixerField::DrumCompEqSwitch, 1)
            .unwrap();
        assert!(dev.state().drum_comp_eq.enabled);
    }

    #[test]
    fn set_mixer_field_switches_master_eq() {
        let mut dev = DeviceState::new(0x10);
        dev.set_mixer_field(MixerScope::Master, MixerField::EqSwitch, 0)
            .unwrap();
        assert!(!dev.state().master_eq.enabled);
        let expected = sysex::build_dt1(0x10, &params::MASTER_EQ_SWITCH, &[0]);
        assert_eq!(dev.drain(0.0), Some(expected));
    }

    #[test]
    fn set_mixer_field_writes_the_assigned_kits_comp_eq_unit() {
        let mut dev = DeviceState::new(0x10);
        dev.state_mut().drum_comp_eq.part = 9;
        dev.state_mut().parts[9].tone_bank_msb = 88; // SN Drum
        let scope = dev.mixer_scope(MixerScopeKind::CompEqUnit, 3);
        let kit = CompEqKit {
            part: 9,
            tone_type: ToneType::SnDrum,
        };
        assert_eq!(scope, MixerScope::CompEqUnit { kit, unit: 3 });
        dev.set_mixer_field(scope, MixerField::EqHighFreq, 2)
            .unwrap();
        assert_eq!(dev.state().drum_comp_eq.units[3].eq_high_freq, 2);
        let expected = sysex::build_dt1(
            0x10,
            &params::comp_eq_param_address(9, ToneType::SnDrum, 3, params::comp_eq::EQ_HIGH_FREQ),
            &[2],
        );
        assert_eq!(dev.drain(0.0), Some(expected));
    }

    #[test]
    fn set_mixer_field_rejects_a_field_outside_its_scope() {
        let mut dev = DeviceState::new(0x10);
        assert_eq!(
            dev.set_mixer_field(MixerScope::Ext, MixerField::EqLowGain, 3),
            Err(MixerFieldError::NotInScope {
                field: MixerField::EqLowGain,
                scope: MixerScopeKind::Ext,
            })
        );
        assert!(!dev.has_pending());
    }

    #[test]
    fn typed_setters_agree_with_mixer_field_addresses() {
        type Set = fn(&mut DeviceState);
        let unit = DeviceState::new(0x10).mixer_scope(MixerScopeKind::CompEqUnit, 4);
        let cases: [(Set, MixerScope, MixerField); 16] = [
            (
                |d| d.set_part_level(2, 1),
                MixerScope::Part(2),
                MixerField::PartLevel,
            ),
            (
                |d| d.set_part_pan(2, 1),
                MixerScope::Part(2),
                MixerField::PartPan,
            ),
            (
                |d| d.set_part_mute(2, true),
                MixerScope::Part(2),
                MixerField::PartMute,
            ),
            (
                |d| d.set_part_chorus_send(2, 1),
                MixerScope::Part(2),
                MixerField::PartChorusSend,
            ),
            (
                |d| d.set_part_reverb_send(2, 1),
                MixerScope::Part(2),
                MixerField::PartReverbSend,
            ),
            (
                |d| d.set_part_receive_channel(2, 1),
                MixerScope::Part(2),
                MixerField::PartReceiveChannel,
            ),
            (
                |d| d.set_part_output_assign(2, 1),
                MixerScope::Part(2),
                MixerField::PartOutputAssign,
            ),
            (
                |d| d.set_master_level(1),
                MixerScope::Master,
                MixerField::MasterLevel,
            ),
            (
                |d| d.set_ext_level(1),
                MixerScope::Ext,
                MixerField::ExtLevel,
            ),
            (
                |d| d.toggle_ext_mute(),
                MixerScope::Ext,
                MixerField::ExtMute,
            ),
            (
                |d| d.toggle_chorus_switch(),
                MixerScope::Common,
                MixerField::ChorusSwitch,
            ),
            (
                |d| d.set_reverb_param(params::reverb::LEVEL, 1),
                MixerScope::Common,
                MixerField::ReverbLevel,
            ),
            (
                |d| d.set_solo_part(1),
                MixerScope::Common,
                MixerField::SoloPart,
            ),
            (
                |d| d.set_drum_comp_eq_part(3),
                MixerScope::Common,
                MixerField::DrumCompEqPart,
            ),
            (
                |d| d.set_drum_comp_eq_switch(true),
                MixerScope::Common,
                MixerField::DrumCompEqSwitch,
            ),
            (
                |d| d.set_drum_comp_eq_output_assign(4, 1),
                unit,
                MixerField::CompOutputAssign,
            ),
        ];
        for (set, scope, field) in cases {
            let mut typed = DeviceState::new(0x10);
            set(&mut typed);
            let sent = typed.drain(0.0).unwrap();
            let address = mixer_field_address(field, scope).unwrap();
            // F0 41 <dev> 00 00 64 12, then the 4 address bytes.
            assert_eq!(&sent[7..11], address.as_bytes(), "{field:?}");
        }
    }

    #[test]
    fn every_scoped_mixer_field_has_a_state_slot() {
        let kinds = [
            MixerScopeKind::Part,
            MixerScopeKind::Ext,
            MixerScopeKind::Master,
            MixerScopeKind::CompEqUnit,
            MixerScopeKind::Common,
        ];
        let mut dev = DeviceState::new(0x10);
        for kind in kinds {
            let scope = dev.mixer_scope(kind, 1);
            for field in MixerField::ALL {
                assert_eq!(
                    mixer_slot(&mut dev.state, scope, field).is_some(),
                    field.in_scope(kind),
                    "{field:?} in {kind:?}"
                );
            }
        }
    }

    #[test]
    fn chorus_output_assign_writes_its_own_offset() {
        let mut dev = DeviceState::new(0x10);
        dev.set_chorus_param(params::chorus::OUTPUT_ASSIGN, 3);
        assert_eq!(dev.state().chorus.output_assign, 3);
        let expected = sysex::build_dt1(
            0x10,
            &params::chorus_address(params::chorus::OUTPUT_ASSIGN),
            &[3],
        );
        assert_eq!(dev.drain(0.0), Some(expected));
    }

    #[test]
    fn chorus_output_select_writes_its_own_offset() {
        let mut dev = DeviceState::new(0x10);
        dev.set_chorus_param(params::chorus::OUTPUT_SELECT, 2);
        assert_eq!(dev.state().chorus.output, 2);
        let expected = sysex::build_dt1(
            0x10,
            &params::chorus_address(params::chorus::OUTPUT_SELECT),
            &[2],
        );
        assert_eq!(dev.drain(0.0), Some(expected));
        // The output assign at 0x02 is independent of the select.
        dev.set_chorus_param(params::chorus::OUTPUT_ASSIGN, 1);
        assert_eq!(dev.state().chorus.output, 2);
        assert_eq!(dev.state().chorus.output_assign, 1);
        assert!(dev.has_pending());
    }

    #[test]
    fn expansion_slots_start_unknown_and_follow_reads() {
        let srx03 = expansions::by_name("SRX-03").unwrap();
        let mut dev = DeviceState::new(0x10);
        assert_eq!(dev.expansion_slots(), None);
        assert_eq!(dev.is_loaded(srx03), None);

        let slots = ExpansionSlots::from_values(&[0x03, 0x05, 0x07, 0x08]).unwrap();
        dev.set_expansion_slots(slots);
        assert_eq!(dev.expansion_slots(), Some(&slots));
        assert_eq!(dev.is_loaded(srx03), Some(true));
        assert_eq!(
            dev.is_loaded(expansions::by_name("ExPCM").unwrap()),
            Some(false)
        );

        dev.expansion_slots_timed_out();
        assert_eq!(dev.is_loaded(srx03), None);

        assert_eq!(
            dev.build_expansion_slots_request(),
            expansion_slots::build_read_request(0x10)
        );
        let read = expansion_slots::SLOT_READ_ADDR;
        assert!(dev.handle_expansion_dt1(&read, &[0x03, 0x05, 0x07, 0x08], 0.0));
        assert_eq!(dev.expansion_slots(), Some(&slots));
        assert!(dev.handle_expansion_dt1(&read, &[0x7F, 0, 0, 0], 0.0));
        assert_eq!(dev.expansion_slots(), None);
    }

    #[test]
    fn expansion_load_holds_the_queue_until_done() {
        let mut dev = DeviceState::new(0x10);
        let target = ExpansionSlots::from_values(&[0x01, 0x05, 0x07, 0x08]).unwrap();
        assert_eq!(
            dev.start_expansion_load(target),
            Err(LoadError::SlotsUnknown)
        );

        let current = ExpansionSlots::from_values(&[0x03, 0x05, 0x07, 0x08]).unwrap();
        dev.set_expansion_slots(current);
        dev.start_expansion_load(target).unwrap();
        assert_eq!(dev.start_expansion_load(target), Err(LoadError::Busy));
        dev.set_part_level(0, 100);

        assert!(matches!(
            dev.next_load_action(0.0),
            Some(LoadAction::Send(_))
        ));
        assert_eq!(dev.drain(1_000.0), None, "queue held during the load");
        let started = Address::new(0x0F, 0x00, 0x30, 0x01);
        assert!(dev.handle_expansion_dt1(&started, &[0; 4], 70.0));
        assert!(matches!(
            dev.next_load_action(570.0),
            Some(LoadAction::Poll(_))
        ));
        let read = expansion_slots::SLOT_READ_ADDR;
        assert!(dev.handle_expansion_dt1(&read, &target.values(), 5_000.0));
        assert_eq!(
            dev.next_load_action(5_000.0),
            Some(LoadAction::Done(LoadOutcome::Loaded(target)))
        );
        assert_eq!(dev.expansion_slots(), Some(&target));
        assert_eq!(dev.next_load_action(5_000.0), None);
        assert!(dev.drain(5_000.0).is_some(), "queue released");
    }

    #[test]
    fn expansion_load_sends_respect_the_throttle() {
        let mut dev = DeviceState::new(0x10);
        let current = ExpansionSlots::from_values(&[0x03, 0x05, 0x07, 0x08]).unwrap();
        dev.set_expansion_slots(current);
        dev.set_part_level(0, 100);
        assert!(dev.drain(1_000.0).is_some());
        dev.start_expansion_load(current.with(0, None)).unwrap();
        assert_eq!(
            dev.next_load_action(1_010.0),
            Some(LoadAction::Wait(THROTTLE_MS - 10.0))
        );
        assert!(matches!(
            dev.next_load_action(1_000.0 + THROTTLE_MS),
            Some(LoadAction::Send(_))
        ));
    }

    #[test]
    fn plan_expansion_load_uses_part_banks() {
        let mut dev = DeviceState::new(0x10);
        let current = ExpansionSlots::from_values(&[0x03, 0x05, 0x07, 0x08]).unwrap();
        let target = current.with(0, None);
        assert_eq!(
            dev.plan_expansion_load(&target),
            Err(LoadError::SlotsUnknown)
        );
        dev.set_expansion_slots(current);
        // Part 2 on SRX-03 (MSB 93, LSB 2), which the load unloads.
        dev.state_mut().parts[1].tone_bank_msb = 93;
        dev.state_mut().parts[1].tone_bank_lsb = 2;
        let plan = dev.plan_expansion_load(&target).unwrap();
        assert_eq!(plan.changes.len(), 1);
        assert_eq!(plan.silenced_parts, [2]);
    }

    #[test]
    fn parts_using_expansion_follows_part_tones() {
        let mut dev = DeviceState::new(0x10);
        let srx03 = expansions::by_name("SRX-03").unwrap();
        assert!(dev.parts_using_expansion(srx03).is_empty());
        for part in [1, 5] {
            dev.state_mut().parts[part].tone_bank_msb = 93;
            dev.state_mut().parts[part].tone_bank_lsb = 2;
        }
        assert_eq!(dev.parts_using_expansion(srx03), [2, 6]);
    }

    #[test]
    fn preselect_expansion_slot_skips_slots_parts_use() {
        let mut dev = DeviceState::new(0x10);
        let slots = ExpansionSlots::from_values(&[0x03, 0x05, 0x07, 0x08]).unwrap();
        assert_eq!(dev.preselect_expansion_slot(&slots), 0);
        // Part 1 on SRX-03 (MSB 93, LSB 2), in slot A.
        dev.state_mut().parts[0].tone_bank_msb = 93;
        dev.state_mut().parts[0].tone_bank_lsb = 2;
        assert_eq!(dev.preselect_expansion_slot(&slots), 1);
        assert_eq!(dev.preselect_expansion_slot(&slots.with(3, None)), 3);
    }

    #[test]
    fn expansion_load_timeout_leaves_slots_unknown() {
        let mut dev = DeviceState::new(0x10);
        let current = ExpansionSlots::from_values(&[0x03, 0x05, 0x07, 0x08]).unwrap();
        dev.set_expansion_slots(current);
        dev.start_expansion_load(current.with(0, None)).unwrap();
        dev.next_load_action(0.0);
        assert_eq!(
            dev.next_load_action(60_000.0),
            Some(LoadAction::Done(LoadOutcome::TimedOut))
        );
        assert_eq!(dev.expansion_slots(), None);
    }

    #[test]
    fn send_dt1_coalesces_same_address() {
        let mut dev = DeviceState::new(0x10);
        dev.set_part_level(0, 50);
        dev.set_part_level(0, 80);
        // Should have coalesced to one message.
        assert_eq!(dev.queue.len(), 1);
    }

    #[test]
    fn drain_respects_throttle() {
        let mut dev = DeviceState::new(0x10);
        dev.set_part_level(0, 100);
        dev.set_part_pan(0, 64);

        // First drain should succeed.
        assert!(dev.drain(0.0).is_some());
        // Second drain within throttle window should fail.
        assert!(dev.drain(10.0).is_none());
        // After throttle window.
        assert!(dev.drain(50.0).is_some());
    }

    #[test]
    fn echo_suppression() {
        let mut dev = DeviceState::new(0x10);
        dev.set_part_level(0, 100);

        // Drain to stamp the echo tracker.
        let _msg = dev.drain(0.0);

        // Incoming DT1 for the same address within the window should be suppressed.
        let addr = params::part_address(0, part::LEVEL);
        assert!(!dev.handle_dt1(&addr, &[50], 50.0));
        assert_eq!(dev.state().parts[0].level, 100); // Unchanged.

        // After the window, it should be accepted.
        assert!(dev.handle_dt1(&addr, &[50], 200.0));
        assert_eq!(dev.state().parts[0].level, 50);
    }

    #[test]
    fn change_part_tone_sends_the_tone_then_reads_its_name_and_category() {
        let mut dev = DeviceState::new(0x10);
        dev.change_part_tone(0, 89, 64, 12);
        let name = params::tone_name_address(0, ToneType::SnAcoustic);
        let category = params::tone_category_address(0, 89).unwrap();
        let sent = drain_until(&mut dev, 0.0, 1_000.0);
        assert_eq!(sent.len(), 5);
        assert_eq!(
            sent[3],
            sysex::build_rq1(0x10, &name, &params::TONE_NAME_SIZE)
        );
        assert_eq!(sent[4], sysex::build_rq1(0x10, &category, &DataSize::ONE));
        assert_eq!(dev.state().parts[0].tone_bank_msb, 89);
        assert_eq!(dev.state().parts[0].tone_bank_lsb, 64);
        assert_eq!(dev.state().parts[0].tone_pc, 12);
    }

    #[test]
    fn change_part_tone_shows_factory_name_until_read_back() {
        let mut dev = DeviceState::new(0x10);
        dev.change_part_tone(0, 89, 64, 0);
        assert_eq!(dev.state().parts[0].tone_name, "Full Grand 1");
        dev.change_part_tone(0, 87, 0, 0); // User bank: no factory name
        assert_eq!(dev.state().parts[0].tone_name, "");
    }

    #[test]
    fn rename_tone_writes_padded_name_for_tone_type() {
        let mut dev = DeviceState::new(0x10);
        dev.change_part_tone(1, 95, 0, 0); // SN Synth
        drain_until(&mut dev, 0.0, 200.0); // Tone change, name and category reads
        assert!(dev.rename_tone(1, "My Pad"));
        assert_eq!(dev.state().parts[1].tone_name, "My Pad");
        let addr = params::tone_name_address(1, ToneType::SnSynth);
        assert_eq!(
            dev.drain(300.0).unwrap(),
            sysex::build_dt1(0x10, &addr, b"My Pad      ")
        );
    }

    #[test]
    fn rename_tone_unknown_bank_sends_nothing() {
        let mut dev = DeviceState::new(0x10);
        dev.change_part_tone(0, 0, 0, 0);
        let _ = (dev.drain(0.0), dev.drain(100.0), dev.drain(200.0));
        assert!(!dev.rename_tone(0, "X"));
        assert!(dev.drain(300.0).is_none());
    }

    #[test]
    fn set_mfx_header_param_writes_the_tone_types_mfx_block() {
        use crate::mfx::MfxHeaderField;
        let mut dev = DeviceState::new(0x10);
        dev.set_mfx_header_param(0, ToneType::SnAcoustic, MfxHeaderField::Type, 0, 5)
            .unwrap();
        assert_eq!(
            dev.drain(0.0).unwrap(),
            sysex::build_dt1(0x10, &Address::new(0x19, 0x02, 0x02, 0x00), &[5])
        );
        dev.set_mfx_header_param(0, ToneType::PcmDrum, MfxHeaderField::ControlAssign, 3, 7)
            .unwrap();
        // PCM Drum MFX block at `10 02 00`, Control Assign 4 at `00 10`.
        assert_eq!(
            dev.drain(100.0).unwrap(),
            sysex::build_dt1(0x10, &Address::new(0x19, 0x10, 0x02, 0x10), &[7])
        );
    }

    #[test]
    fn set_mfx_type_also_sends_the_types_defaults() {
        let mut dev = DeviceState::new(0x10);
        let slots = dev.set_mfx_type(0, ToneType::SnSynth, 34).unwrap();
        assert_eq!(slots, crate::mfx_params::default_slots(34).unwrap());
        // SN-S MFX block at `19 01 02 00`: type at `00`, then Control
        // Assign 1-4 (OFF) at `0D` and the 32 parameters from `11`.
        assert_eq!(
            dev.drain(0.0).unwrap(),
            sysex::build_dt1(0x10, &Address::new(0x19, 0x01, 0x02, 0x00), &[34])
        );
        let mut data = vec![0; 4];
        data.extend(slots.iter().flat_map(|&v| crate::mfx::encode_mfx_param(v)));
        assert_eq!(
            dev.drain(100.0).unwrap(),
            sysex::build_dt1(0x10, &Address::new(0x19, 0x01, 0x02, 0x0D), &data)
        );
        assert!(dev.set_mfx_type(0, ToneType::SnSynth, 68).is_none());
        assert!(dev.drain(200.0).is_none());
    }

    #[test]
    fn set_pcmd_kit_level_writes_the_kit_common() {
        let mut dev = DeviceState::new(0x10);
        dev.set_pcmd_kit_level(1, 100);
        // Part 2: PCM Drum Kit Common at `19 30 00 00`, Kit Level at `00 0C`.
        assert_eq!(
            dev.drain(0.0).unwrap(),
            sysex::build_dt1(0x10, &Address::new(0x19, 0x30, 0x00, 0x0C), &[100])
        );
    }

    #[test]
    fn set_pcmd_partial_field_writes_the_keys_partial() {
        use crate::pcm_drum::PcmdPartialField;
        let mut dev = DeviceState::new(0x10);
        dev.set_pcmd_partial_field(0, 22, PcmdPartialField::TvfCutoffFrequency, 0, 90)
            .unwrap();
        // Part 1, key 22: Partial at `10 12 00`, TVF Cutoff Frequency at `01 23`.
        assert_eq!(
            dev.drain(0.0).unwrap(),
            sysex::build_dt1(0x10, &Address::new(0x19, 0x10, 0x13, 0x23), &[90])
        );
    }

    #[test]
    fn set_mfx_nib_param_writes_the_tone_types_mfx_block() {
        let mut dev = DeviceState::new(0x10);
        dev.set_mfx_nib_param(4, ToneType::SnSynth, 1, 0x1234);
        assert_eq!(
            dev.drain(0.0).unwrap(),
            sysex::build_dt1(
                0x10,
                &Address::new(0x1A, 0x01, 0x02, 0x15),
                &crate::mfx::encode_mfx_param(0x1234)
            )
        );
    }

    #[test]
    fn build_mfx_request_reads_the_tone_types_mfx_block() {
        let dev = DeviceState::new(0x10);
        assert_eq!(
            dev.build_mfx_request(0, ToneType::SnSynth),
            sysex::build_rq1(
                0x10,
                &Address::new(0x19, 0x01, 0x02, 0x00),
                &crate::mfx::MFX_BLOCK_SIZE
            )
        );
    }

    #[test]
    fn rename_studio_set_writes_sixteen_chars() {
        let mut dev = DeviceState::new(0x10);
        dev.rename_studio_set("A Much Too Long Studio Set Name");
        assert_eq!(dev.state().studio_set_name, "A Much Too Long");
        assert_eq!(
            dev.drain(0.0).unwrap(),
            sysex::build_dt1(0x10, &params::STUDIO_SET_NAME, b"A Much Too Long ")
        );
    }

    #[test]
    fn toggle_mute() {
        let mut dev = DeviceState::new(0x10);
        assert!(!dev.state().parts[0].muted);
        dev.toggle_part_mute(0);
        assert!(dev.state().parts[0].muted);
        dev.toggle_part_mute(0);
        assert!(!dev.state().parts[0].muted);
    }

    #[test]
    fn handle_dt1_updates_state() {
        let mut dev = DeviceState::new(0x10);
        let addr = params::part_address(3, part::PAN);
        assert!(dev.handle_dt1(&addr, &[100], 0.0));
        assert_eq!(dev.state().parts[3].pan, 100);
    }

    #[test]
    fn preview_toggle() {
        let mut dev = DeviceState::new(0x10);
        assert_eq!(dev.state().preview_part, 0);

        // Start preview on part 1.
        dev.preview_toggle(1);
        assert_eq!(dev.state().preview_part, 1);
        assert_eq!(dev.queue.len(), 1);

        // Toggle same part → stops.
        dev.preview_toggle(1);
        assert_eq!(dev.state().preview_part, 0);

        // Toggle different part while another is active.
        dev.queue.clear();
        dev.preview_start(3);
        assert_eq!(dev.state().preview_part, 3);
        dev.preview_toggle(5);
        assert_eq!(dev.state().preview_part, 5);
    }

    #[test]
    fn snd_note_setters_reject_keys_outside_the_kit() {
        use crate::sn_drum::{SND_FIRST_KEY, SND_LAST_KEY};
        let mut dev = DeviceState::new(0x10);
        for key in [SND_FIRST_KEY - 1, SND_LAST_KEY + 1] {
            assert!(
                dev.set_snd_note_param(0, key, SndNoteField::Level, 100)
                    .is_err()
            );
            assert!(
                dev.set_snd_note_param(0, key, SndNoteField::InstNumber, 1)
                    .is_err()
            );
        }
        assert!(!dev.has_pending());
        assert!(
            dev.set_snd_note_param(0, SND_LAST_KEY, SndNoteField::Level, 100)
                .is_ok()
        );
        assert!(dev.has_pending());
    }

    #[test]
    fn snd_setters_write_each_field_at_its_address() {
        let mut dev = DeviceState::new(0x10);
        dev.set_snd_common_param(0, SndCommonField::AmbienceLevel, 80);
        dev.set_snd_note_param(0, 28, SndNoteField::InstNumber, 0x1FF)
            .unwrap();
        // Part 1: SN-D Common Ambience Level at `19 03 00 11`; key 28's
        // Note at `03 11 00`, Inst Number (4 nibbles) at `00 00`.
        assert_eq!(
            dev.drain(0.0),
            Some(sysex::build_dt1(
                0x10,
                &Address::new(0x19, 0x03, 0x00, 0x11),
                &[80]
            ))
        );
        assert_eq!(
            dev.drain(100.0),
            Some(sysex::build_dt1(
                0x10,
                &Address::new(0x19, 0x03, 0x11, 0x00),
                &[0x00, 0x01, 0x0F, 0x0F]
            ))
        );
    }

    #[test]
    fn pcmd_partial_setters_reject_keys_outside_the_kit() {
        use crate::pcm_drum::{PCMD_FIRST_KEY, PCMD_LAST_KEY, PcmdPartialField, PcmdWmtField};
        let mut dev = DeviceState::new(0x10);
        for key in [PCMD_FIRST_KEY - 1, PCMD_LAST_KEY + 1] {
            assert!(
                dev.set_pcmd_partial_field(0, key, PcmdPartialField::Level, 0, 1)
                    .is_err()
            );
            assert!(
                dev.set_pcmd_wmt_param(0, key, 0, PcmdWmtField::WaveNumberL, 1)
                    .is_err()
            );
        }
        assert!(!dev.has_pending());
        assert!(
            dev.set_pcmd_partial_field(0, PCMD_FIRST_KEY, PcmdPartialField::Level, 0, 1)
                .is_ok()
        );
        assert!(dev.has_pending());
    }

    #[test]
    fn pcmd_wmt_param_sends_nibblized_fields_as_four_bytes() {
        use crate::pcm_drum::PcmdWmtField;
        let mut dev = DeviceState::new(0x10);
        dev.set_pcmd_wmt_param(0, 21, 3, PcmdWmtField::WaveNumberR, 0x1234)
            .unwrap();
        dev.set_pcmd_wmt_param(0, 21, 0, PcmdWmtField::Level, 100)
            .unwrap();
        // Part 1, key 21: WMT4 Wave Number R at `01 02`, WMT1 Level at `00 39`.
        assert_eq!(
            dev.drain(0.0),
            Some(sysex::build_dt1(
                0x10,
                &Address::new(0x19, 0x10, 0x11, 0x02),
                &[1, 2, 3, 4]
            ))
        );
        assert_eq!(
            dev.drain(100.0),
            Some(sysex::build_dt1(
                0x10,
                &Address::new(0x19, 0x10, 0x10, 0x39),
                &[100]
            ))
        );
    }

    #[test]
    fn pcms_setters_write_each_field_at_its_address() {
        let mut dev = DeviceState::new(0x10);
        dev.set_pcms_common_param(0, PcmsCommonField::MatrixControlSens1, 2, 70)
            .unwrap();
        dev.set_pcms_pmt_param(0, PcmsPmtField::PartialSwitch, 3, 1)
            .unwrap();
        dev.set_pcms_partial_param(0, 1, PcmsPartialField::Lfo1Rate, 0, 0x95)
            .unwrap();
        dev.set_pcms_partial_param(0, 3, PcmsPartialField::LfoStep, 15, 100)
            .unwrap();
        // Part 1: Common Matrix Control 3 Sens 1 at `00 00 3F`; PMT4
        // Partial Switch at `00 10 20`; Partial 2 LFO1 Rate (2 nibbles) at
        // `00 22 6E`; Partial 4 LFO Step 16 at `00 27 19`.
        let expected = [
            (Address::new(0x19, 0x00, 0x00, 0x3F), vec![70]),
            (Address::new(0x19, 0x00, 0x10, 0x20), vec![1]),
            (Address::new(0x19, 0x00, 0x22, 0x6E), vec![0x09, 0x05]),
            (Address::new(0x19, 0x00, 0x27, 0x19), vec![100]),
        ];
        for (i, (addr, data)) in expected.into_iter().enumerate() {
            assert_eq!(
                dev.drain(100.0 * i as f64),
                Some(sysex::build_dt1(0x10, &addr, &data))
            );
        }
        assert!(
            dev.set_pcms_partial_param(0, 0, PcmsPartialField::TvaEnvLevel, 3, 0)
                .is_err()
        );
    }

    #[test]
    fn sna_setter_writes_each_field_at_its_address() {
        let mut dev = DeviceState::new(0x10);
        dev.set_sna_common_param(0, SnaCommonField::ModifyParameter, 31, 50)
            .unwrap();
        dev.set_sna_common_param(0, SnaCommonField::PhraseNumber, 0, 0x2A)
            .unwrap();
        // Part 1: SN-A Common at `19 02 00 00`; Modify Parameter 32 at
        // `00 41`, Phrase Number (2 nibbles) at `00 1C`.
        assert_eq!(
            dev.drain(0.0),
            Some(sysex::build_dt1(
                0x10,
                &Address::new(0x19, 0x02, 0x00, 0x41),
                &[50]
            ))
        );
        assert_eq!(
            dev.drain(100.0),
            Some(sysex::build_dt1(
                0x10,
                &Address::new(0x19, 0x02, 0x00, 0x1C),
                &[0x02, 0x0A]
            ))
        );
        assert!(
            dev.set_sna_common_param(0, SnaCommonField::ModifyParameter, 32, 0)
                .is_err()
        );
    }

    #[test]
    fn set_mixer_field_writes_surround_common() {
        let mut dev = DeviceState::new(0x10);
        let common = dev.mixer_scope(MixerScopeKind::Common, 0);
        dev.set_mixer_field(common, MixerField::SurroundDepth, 80)
            .unwrap();
        assert_eq!(dev.state().surround.depth, 80);
        // Motional Surround at `18 00 08 00`, Depth at `00 0C`.
        assert_eq!(
            dev.drain(0.0),
            Some(sysex::build_dt1(
                0x10,
                &Address::new(0x18, 0x00, 0x08, 0x0C),
                &[80]
            ))
        );
    }

    // -- SN Synth Tone cache ------------------------------------------------

    /// The RQ1 that reads `part`'s `block`.
    fn sns_request(part: u8, block: SnsBlock) -> Vec<u8> {
        sysex::build_rq1(0x10, &block.address(part), &block.size())
    }

    /// A reply for `block` whose every byte is `fill`, with tone name
    /// "Reply" in a Common block.
    fn sns_reply(block: SnsBlock, fill: u8) -> Vec<u8> {
        let mut data = match block {
            SnsBlock::Common => vec![fill; sn_synth::SNS_COMMON_SIZE],
            SnsBlock::Partial(_) => vec![fill; sn_synth::SNS_PARTIAL_SIZE],
        };
        if block == SnsBlock::Common {
            data[..12].copy_from_slice(b"Reply       ");
        }
        data
    }

    /// How many of `sent` are SN Synth block requests for `part`.
    fn sns_requests_in(sent: &[Vec<u8>], part: u8) -> usize {
        let requests = SnsBlock::ALL.map(|b| sns_request(part, b));
        sent.iter().filter(|m| requests.contains(m)).count()
    }

    /// Everything `dev` sends from `from_ms` until `until_ms`, draining
    /// every 10 ms; `now` ends at `until_ms`.
    fn drain_until(dev: &mut DeviceState, from_ms: f64, until_ms: f64) -> Vec<Vec<u8>> {
        let mut sent = Vec::new();
        let mut now = from_ms;
        while now <= until_ms {
            if let Some(msg) = dev.drain(now) {
                sent.push(msg);
            }
            now += 10.0;
        }
        sent
    }

    /// A device with SN Synth Tone (95, 64, 3) on part 3 whose read started.
    fn loading_sns(part: u8) -> DeviceState {
        let mut dev = DeviceState::new(0x10);
        let p = &mut dev.state_mut().parts[part as usize];
        (p.tone_bank_msb, p.tone_bank_lsb, p.tone_pc) = (95, 64, 3);
        assert!(dev.load_sns_tone(part));
        dev
    }

    #[test]
    fn sns_tone_parses_once_every_block_arrived() {
        let mut dev = loading_sns(2);
        assert!(!dev.load_sns_tone(2), "the same tone is read once");
        assert_eq!(
            drain_until(&mut dev, 0.0, 200.0),
            SnsBlock::ALL.map(|b| sns_request(2, b)).to_vec()
        );

        for block in SnsBlock::ALL {
            assert_eq!(dev.sns_tone(2), None);
            // Another part's block isn't this part's.
            assert!(!dev.handle_dt1(&block.address(5), &sns_reply(block, 1), 0.0));
            let reply = sns_reply(block, 0x20);
            assert!(dev.handle_dt1(&block.address(2), &reply, 0.0));
        }

        let tone = dev.sns_tone(2).unwrap();
        assert_eq!(
            tone.common,
            sn_synth::parse_sns_common(&sns_reply(SnsBlock::Common, 0x20)).unwrap()
        );
        assert_eq!(tone.common.tone_name, "Reply");
        for partial in &tone.partials {
            assert_eq!(partial.filter_cutoff, 0x20);
        }
        assert_eq!(dev.sns_tone(5), None);
    }

    #[test]
    fn sns_edits_show_in_the_cached_tone() {
        let mut dev = loading_sns(0);
        drain_until(&mut dev, 0.0, 200.0);
        for block in SnsBlock::ALL {
            dev.handle_dt1(&block.address(0), &sns_reply(block, 0), 300.0);
        }

        dev.set_sns_common_param(0, SnsCommonField::ToneLevel, 99);
        dev.set_sns_partial_param(0, 1, SnsPartialField::FilterCutoff, 77);
        dev.set_sns_partial_param(0, 2, SnsPartialField::WaveNumber, 1234);

        let tone = dev.sns_tone(0).unwrap();
        assert_eq!(tone.common.tone_level, 99);
        assert_eq!(tone.partials[0].filter_cutoff, 0);
        assert_eq!(tone.partials[1].filter_cutoff, 77);
        assert_eq!(tone.partials[2].wave_number, 1234);

        // Each edit goes out once, as encoded for the cache.
        let sent = drain_until(&mut dev, 1000.0, 1200.0);
        assert!(sent.contains(&sysex::build_dt1(
            0x10,
            // Partial 3 Wave Number at `00 35`, as 4 nibbles.
            &Address::new(0x19, 0x01, 0x22, 0x35),
            &[0x00, 0x04, 0x0D, 0x02],
        )));
        assert_eq!(sent.len(), 3);
    }

    #[test]
    fn a_lost_sns_reply_is_requested_again() {
        let mut dev = loading_sns(3);
        drain_until(&mut dev, 0.0, 200.0);
        let lost = SnsBlock::Partial(1);
        for block in SnsBlock::ALL.into_iter().filter(|&b| b != lost) {
            dev.handle_dt1(&block.address(3), &sns_reply(block, 0), 300.0);
        }
        assert_eq!(dev.sns_tone(3), None);

        // Partial 2's request went out at 80 ms; nothing is resent before
        // its reply times out.
        assert!(drain_until(&mut dev, 300.0, 80.0 + REPLY_TIMEOUT_MS - 10.0).is_empty());
        assert_eq!(
            drain_until(&mut dev, 80.0 + REPLY_TIMEOUT_MS, 3000.0),
            vec![sns_request(3, lost)]
        );

        dev.handle_dt1(&lost.address(3), &sns_reply(lost, 0), 3000.0);
        assert!(dev.sns_tone(3).is_some());
        assert!(drain_until(&mut dev, 3000.0, 20_000.0).is_empty());
    }

    #[test]
    fn sns_retries_are_bounded() {
        let mut dev = loading_sns(0);
        let sent = drain_until(&mut dev, 0.0, 60_000.0);
        let tries = 1 + MAX_RETRIES as usize;
        assert_eq!(sent.len(), SnsBlock::ALL.len() * tries);
        for block in SnsBlock::ALL {
            let request = sns_request(0, block);
            assert_eq!(sent.iter().filter(|m| **m == request).count(), tries);
        }
        assert_eq!(dev.sns_tone(0), None);
    }

    #[test]
    fn reload_reads_the_same_tone_again() {
        let mut dev = loading_sns(2);
        drain_until(&mut dev, 0.0, 200.0);
        for block in SnsBlock::ALL {
            dev.handle_dt1(&block.address(2), &sns_reply(block, 1), 0.0);
        }
        assert!(dev.sns_tone(2).is_some());

        // Edited on the front panel since: the editor reopens.
        dev.reload_sns_tone(2);
        assert_eq!(dev.sns_tone(2), None, "the stale copy is dropped");
        assert_eq!(
            drain_until(&mut dev, 300.0, 600.0),
            SnsBlock::ALL.map(|b| sns_request(2, b)).to_vec()
        );
    }

    #[test]
    fn reload_asks_again_once_retries_are_used_up() {
        let mut dev = loading_sns(0);
        drain_until(&mut dev, 0.0, 60_000.0);
        assert!(drain_until(&mut dev, 60_000.0, 70_000.0).is_empty());
        dev.reload_sns_tone(0);
        assert_eq!(
            sns_requests_in(&drain_until(&mut dev, 70_000.0, 70_500.0), 0),
            SnsBlock::ALL.len()
        );
    }

    #[test]
    fn a_tone_change_stops_the_old_tone_requests() {
        // Queued but not yet sent.
        let mut dev = loading_sns(0);
        dev.change_part_tone(0, 87, 0, 0);
        let sent = drain_until(&mut dev, 0.0, 60_000.0);
        assert_eq!(sns_requests_in(&sent, 0), 0, "no SN Synth read goes out");

        // Sent, awaiting replies.
        let mut dev = loading_sns(0);
        drain_until(&mut dev, 0.0, 200.0);
        dev.change_part_tone(0, 87, 0, 0);
        assert_eq!(
            sns_requests_in(&drain_until(&mut dev, 200.0, 60_000.0), 0),
            0
        );
        assert_eq!(dev.sns_tone(0), None);

        // Changed behind the cache's back (a part dump): the timed-out
        // requests aren't sent again, and the next load reads anew.
        let mut dev = loading_sns(0);
        drain_until(&mut dev, 0.0, 200.0);
        dev.state_mut().parts[0].tone_pc = 4;
        assert!(drain_until(&mut dev, 200.0, 60_000.0).is_empty());
        assert!(dev.load_sns_tone(0));
    }

    // -- Mixer load ---------------------------------------------------------

    /// Answer every RQ1 `dev` sends until `until_ms` from `memory` (a
    /// block's bytes by start address, zeros elsewhere), draining every
    /// 10 ms. Returns the addresses read.
    fn serve(
        dev: &mut DeviceState,
        memory: &HashMap<Address, Vec<u8>>,
        until_ms: f64,
    ) -> Vec<Address> {
        let mut read = Vec::new();
        let mut now = 0.0;
        while now <= until_ms {
            if let Some(msg) = dev.drain(now)
                && msg[6] == 0x11
            {
                let address = Address::new(msg[7], msg[8], msg[9], msg[10]);
                let size = DataSize::new(msg[11], msg[12], msg[13], msg[14]).value();
                let data = memory
                    .get(&address)
                    .cloned()
                    .unwrap_or_else(|| vec![0; size as usize]);
                dev.handle_dt1(&address, &data, now);
                read.push(address);
            }
            now += 10.0;
        }
        read
    }

    /// A part mixer dump playing tone `msb`/`lsb`/`pc` at `level`.
    fn part_dump(msb: u8, lsb: u8, pc: u8, level: u8) -> Vec<u8> {
        let mut data = vec![0; params::PART_MIXER_SIZE.value() as usize];
        data[0x06..0x0A].copy_from_slice(&[msb, lsb, pc, level]);
        data[0x27] = 33; // Chorus send
        data[0x29] = 4; // Output assign
        data
    }

    #[test]
    fn load_mixer_reads_the_device_state() {
        // Part 3 plays User PCM tone 5; part 10 an SN-D kit using Comp+EQ.
        let memory = HashMap::from([
            (params::SYSTEM_MASTER_LEVEL, vec![90]),
            (params::STUDIO_SET_NAME, b"My Set          ".to_vec()),
            (
                params::part_address(2, part::RECEIVE_CHANNEL),
                part_dump(87, 0, 5, 77),
            ),
            (
                params::part_address(9, part::RECEIVE_CHANNEL),
                part_dump(88, 64, 0, 100),
            ),
            (
                params::tone_name_address(2, ToneType::PcmSynth),
                b"User Lead   ".to_vec(),
            ),
            (params::tone_category_address(2, 87).unwrap(), vec![7]),
            (
                params::chorus_address(params::chorus::TYPE),
                vec![2, 100, 3, 1],
            ),
            (params::REVERB_SWITCH, vec![1]),
            (params::reverb_address(params::reverb::TYPE), vec![3, 80, 2]),
            (params::EXT_PART_LEVEL, vec![55]),
            (params::DRUM_COMP_EQ_SWITCH, vec![1, 9, 1, 2, 3, 4, 5, 6]),
        ]);
        let mut dev = DeviceState::new(0x10);
        dev.load_mixer();
        assert!(dev.mixer_loading());

        let read = serve(&mut dev, &memory, 10_000.0);

        assert!(!dev.mixer_loading());
        let s = dev.state();
        assert_eq!((s.master_level, s.studio_set_name.as_str()), (90, "My Set"));
        let p = &s.parts[2];
        assert_eq!((p.tone_bank_msb, p.tone_pc, p.level), (87, 5, 77));
        assert_eq!((p.chorus_send, p.output_assign), (33, 4));
        assert_eq!((p.tone_name.as_str(), p.tone_category), ("User Lead", 7));
        assert_eq!(s.parts[9].tone_category, crate::svd::DRUMS_CATEGORY);
        assert_eq!(
            (
                s.chorus.fx_type,
                s.chorus.level,
                s.chorus.output_assign,
                s.chorus.output
            ),
            (2, 100, 3, 1)
        );
        assert!(s.reverb.enabled);
        assert_eq!(
            (s.reverb.fx_type, s.reverb.level, s.reverb.output),
            (3, 80, 2)
        );
        assert_eq!(s.ext_level, 55);
        assert_eq!(s.drum_comp_eq.output_assigns, [1, 2, 3, 4, 5, 6]);
        let kit = params::comp_eq_block_address_for_bank(9, 88).unwrap();
        assert!(read.contains(&kit), "reads the Comp+EQ part's kit block");
        assert_eq!(read.last(), Some(&kit), "the kit block is read last");
    }

    #[test]
    fn load_mixer_stops_loading_once_retries_are_used_up() {
        let mut dev = DeviceState::new(0x10);
        dev.load_mixer();
        let sent = drain_until(&mut dev, 0.0, 60_000.0);
        assert!(!dev.mixer_loading());
        let master = sysex::build_rq1(0x10, &params::SYSTEM_MASTER_LEVEL, &DataSize::ONE);
        let tries = sent.iter().filter(|m| **m == master).count();
        assert_eq!(tries, 1 + usize::from(MAX_RETRIES));
    }

    /// Drain `dev` from 0 until `until_ms`, answering every RQ1 except the
    /// addresses in `lost` (from `memory`, zeros elsewhere).
    fn serve_but(
        dev: &mut DeviceState,
        memory: &HashMap<Address, Vec<u8>>,
        until_ms: f64,
        lost: &[Address],
    ) {
        let mut now = 0.0;
        while now <= until_ms {
            if let Some(msg) = dev.drain(now)
                && msg[6] == 0x11
            {
                let address = Address::new(msg[7], msg[8], msg[9], msg[10]);
                let size = DataSize::new(msg[11], msg[12], msg[13], msg[14]).value();
                if !lost.contains(&address) {
                    let data = memory
                        .get(&address)
                        .cloned()
                        .unwrap_or_else(|| vec![0; size as usize]);
                    dev.handle_dt1(&address, &data, now);
                }
            }
            now += 10.0;
        }
    }

    #[test]
    fn a_lost_mixer_block_is_reported_failed_and_cleared_on_reload() {
        let lost = params::SYSTEM_MASTER_LEVEL;
        let mut dev = DeviceState::new(0x10);
        dev.load_mixer();
        serve_but(&mut dev, &HashMap::new(), 60_000.0, &[lost]);
        assert_eq!(dev.mixer_failed_blocks(), &[MixerBlock::MasterLevel]);
        assert!(!dev.mixer_loading());

        // A fresh load starts over: the old failure no longer shows.
        let memory = HashMap::from([(lost, vec![90])]);
        dev.load_mixer();
        assert!(dev.mixer_failed_blocks().is_empty());
        serve_but(&mut dev, &memory, 60_000.0, &[]);
        assert!(dev.mixer_failed_blocks().is_empty());
        assert_eq!(dev.state().master_level, 90);
    }

    #[test]
    fn a_lost_part_dump_reports_the_kit_comp_eq_block_failed() {
        // The Comp+EQ common dump names part 10, whose own dump is lost:
        // the kit Comp+EQ read can't be built, so it is reported too.
        let memory = HashMap::from([(params::DRUM_COMP_EQ_SWITCH, vec![1, 9, 1, 2, 3, 4, 5, 6])]);
        let mut dev = DeviceState::new(0x10);
        dev.load_mixer();
        serve_but(
            &mut dev,
            &memory,
            60_000.0,
            &[params::part_address(9, part::RECEIVE_CHANNEL)],
        );
        let failed = dev.mixer_failed_blocks();
        assert!(
            failed.contains(&MixerBlock::CompEq { part: 9, msb: 0 }),
            "the kit Comp+EQ block is failed, not silently skipped: {failed:?}"
        );
        assert!(failed.contains(&MixerBlock::Part(9)));
    }

    #[test]
    fn changing_to_a_non_kit_tone_queues_no_comp_eq_read() {
        let kit = params::comp_eq_block_address_for_bank(9, 88).unwrap();
        let memory = HashMap::from([
            (params::SYSTEM_MASTER_LEVEL, vec![90]),
            (params::STUDIO_SET_NAME, b"My Set          ".to_vec()),
            (
                params::part_address(9, part::RECEIVE_CHANNEL),
                part_dump(88, 64, 0, 100),
            ),
            (params::DRUM_COMP_EQ_SWITCH, vec![1, 9, 1, 2, 3, 4, 5, 6]),
            (kit, vec![0; params::COMP_EQ_BLOCK_SIZE.value() as usize]),
        ]);
        let mut dev = DeviceState::new(0x10);
        dev.load_mixer();
        serve(&mut dev, &memory, 10_000.0);
        assert!(dev.mixer_failed_blocks().is_empty());

        // Part 10 leaves the drum kit banks: no Comp+EQ read exists to
        // queue, and none is reported failed. (The new tone's name and
        // category reads go unanswered and fail, as they should.)
        dev.change_part_tone(9, 87, 0, 3);
        let sent = drain_until(&mut dev, 10_000.0, 20_000.0);
        assert!(!sent.contains(&sysex::build_rq1(0x10, &kit, &params::COMP_EQ_BLOCK_SIZE)));
        assert!(
            !dev.mixer_failed_blocks()
                .iter()
                .any(|b| matches!(b, MixerBlock::CompEq { .. })),
            "no Comp+EQ read was queued, so none fails: {:?}",
            dev.mixer_failed_blocks()
        );
    }

    #[test]
    fn load_mixer_again_restarts_the_read() {
        let mut dev = DeviceState::new(0x10);
        dev.load_mixer();
        dev.load_mixer();
        let first = drain_until(&mut dev, 0.0, 1_000.0);
        let mut fresh = DeviceState::new(0x10);
        fresh.load_mixer();
        assert_eq!(first, drain_until(&mut fresh, 0.0, 1_000.0));
    }

    #[test]
    fn a_tone_change_during_a_load_reads_the_part_after_the_change() {
        let mut dev = DeviceState::new(0x10);
        dev.load_mixer();
        dev.change_part_tone(3, 87, 0, 3);
        let sent = drain_until(&mut dev, 0.0, 10_000.0);
        let dump = params::part_address(3, part::RECEIVE_CHANNEL);
        let dump = sysex::build_rq1(0x10, &dump, &params::PART_MIXER_SIZE);
        let pc = sysex::build_dt1(0x10, &params::part_address(3, part::TONE_PC), &[3]);
        let at = |m: &Vec<u8>| sent.iter().position(|s| s == m).unwrap();
        assert!(at(&dump) > at(&pc), "the dump shows the new tone");
    }

    #[test]
    fn change_part_tone_reads_the_new_tone_name() {
        let name = params::tone_name_address(0, ToneType::PcmSynth);
        let memory = HashMap::from([(name, b"User Bass   ".to_vec())]);
        let mut dev = DeviceState::new(0x10);
        dev.change_part_tone(0, 87, 0, 3);
        assert!(serve(&mut dev, &memory, 1_000.0).contains(&name));
        assert_eq!(dev.state().parts[0].tone_name, "User Bass");
        assert!(!dev.mixer_loading(), "a tone read alone isn't a mixer load");
    }
}
