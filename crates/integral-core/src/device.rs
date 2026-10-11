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

use crate::address::Address;
use crate::block_field::{BlockField, FieldIndexError};
use crate::device_spec::KeyOutOfRange;
use thiserror::Error;

use crate::expansion_load::{self, LoadAction, LoadMachine, LoadOutcome, LoadPlan};
use crate::expansion_slots::{self, ExpansionSlots};
use crate::expansions::Expansion;
use crate::midi_bridge::RequestTracker;
use crate::params::{self, ToneType, part};
use crate::parse_helpers::encode_ascii_name;
use crate::pcm_synth::{PcmsCommonField, PcmsPartialField, PcmsPmtField};
use crate::sn_acoustic::SnaCommonField;
use crate::sn_drum::{SndCommonField, SndNoteField};
use crate::sn_synth::{SnSynthTone, SnSynthToneBlocks, SnsBlock};
use crate::state::MixerState;
use crate::sysex;

/// Minimum interval between SysEx sends (ms).
const THROTTLE_MS: f64 = 40.0;

/// Duration to suppress incoming DT1 echoes after a local send (ms).
const ECHO_SUPPRESS_MS: f64 = 150.0;

/// How long an SN Synth block request waits for its reply before it is
/// sent again (ms).
const SNS_REPLY_TIMEOUT_MS: f64 = 2000.0;

/// How many times an unanswered SN Synth block request is sent again.
const SNS_MAX_RETRIES: u8 = 2;

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

/// The request tracker key of `part`'s `block` request.
fn sns_request_key(part: u8, block: SnsBlock) -> String {
    format!("rq1:{}", block.address(part))
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
    /// Sent SN Synth block requests awaiting their reply.
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
    /// SN Synth block requests whose reply timed out are queued again (see
    /// [`load_sns_tone`](Self::load_sns_tone)).
    ///
    /// Returns `None` if the queue is empty, the throttle hasn't elapsed, or
    /// an Expansion load holds the queue.
    pub fn drain(&mut self, now_ms: f64) -> Option<Vec<u8>> {
        if self.load.is_some() {
            return None;
        }
        self.retry_sns_requests(now_ms);
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
        if self.handle_sns_block(address, data) {
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
        let p = &mut self.state.parts[part as usize];
        p.tone_bank_msb = msb;
        p.tone_bank_lsb = lsb;
        p.tone_pc = pc;
        p.tone_name = String::new(); // Clear until re-read.
        self.invalidate_sns_tone(part);

        let msb_addr = params::part_address(part, part::TONE_BANK_MSB);
        self.send_dt1(&msb_addr, &[msb]);
        let lsb_addr = params::part_address(part, part::TONE_BANK_LSB);
        self.send_dt1(&lsb_addr, &[lsb]);
        let pc_addr = params::part_address(part, part::TONE_PC);
        self.send_dt1(&pc_addr, &[pc]);
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

    /// Set a Part EQ parameter.
    ///
    /// # Panics
    /// Debug-asserts that `part` < 16.
    pub fn set_part_eq_param(&mut self, part: u8, param_offset: u8, value: u8) {
        debug_assert!(part < Self::NUM_PARTS, "part index {part} out of range");
        let eq = &mut self.state.parts[part as usize].eq;
        match param_offset {
            0 => eq.enabled = value == 1,
            1 => eq.low_freq = value,
            2 => eq.low_gain = value,
            3 => eq.mid_freq = value,
            4 => eq.mid_gain = value,
            5 => eq.mid_q = value,
            6 => eq.high_freq = value,
            7 => eq.high_gain = value,
            _ => return,
        }
        let addr = params::part_eq_address(part, param_offset);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a Master EQ parameter.
    pub fn set_master_eq_param(&mut self, param_offset: u8, value: u8) {
        let eq = &mut self.state.master_eq;
        match param_offset {
            0 => eq.low_freq = value,
            1 => eq.low_gain = value,
            2 => eq.mid_freq = value,
            3 => eq.mid_gain = value,
            4 => eq.mid_q = value,
            5 => eq.high_freq = value,
            6 => eq.high_gain = value,
            _ => return,
        }
        let addr = params::master_eq_address(param_offset);
        self.send_dt1(&addr, &[value]);
    }

    /// Toggle the Master EQ switch.
    pub fn toggle_master_eq_switch(&mut self) {
        self.state.master_eq.enabled = !self.state.master_eq.enabled;
        self.send_dt1(
            &params::MASTER_EQ_SWITCH,
            &[u8::from(self.state.master_eq.enabled)],
        );
    }

    /// Set a chorus core parameter (type, level, output at offsets 0–2).
    pub fn set_chorus_param(&mut self, offset: u8, value: u8) {
        match offset {
            0 => self.state.chorus.fx_type = value,
            1 => self.state.chorus.level = value,
            2 => self.state.chorus.output = value,
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

    /// Set a reverb core parameter (type, level, output at offsets 0–2).
    pub fn set_reverb_param(&mut self, offset: u8, value: u8) {
        match offset {
            0 => self.state.reverb.fx_type = value,
            1 => self.state.reverb.level = value,
            2 => self.state.reverb.output = value,
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

    /// Set a single SN-S Common parameter for a part, in the cached tone
    /// too.
    pub fn set_sns_common_param(&mut self, part: u8, offset: u8, value: u8) {
        self.set_sns_bytes(part, SnsBlock::Common, offset, &[value]);
    }

    /// Set a single SN-S Partial parameter for a part, in the cached tone
    /// too.
    pub fn set_sns_partial_param(&mut self, part: u8, partial: u8, offset: u8, value: u8) {
        self.set_sns_bytes(part, SnsBlock::Partial(partial), offset, &[value]);
    }

    /// Set a nibblized SN-S Partial parameter (4 bytes, e.g. wave number),
    /// in the cached tone too.
    pub fn set_sns_partial_nib_param(&mut self, part: u8, partial: u8, offset: u8, value: u16) {
        let bytes = crate::parse_helpers::encode_nibble4(value);
        self.set_sns_bytes(part, SnsBlock::Partial(partial), offset, &bytes);
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

    /// Queue again the SN Synth block requests whose reply timed out, while
    /// retries remain and the part's tone hasn't changed.
    fn retry_sns_requests(&mut self, now_ms: f64) {
        for key in self.requests.expire_old(now_ms, SNS_REPLY_TIMEOUT_MS) {
            let Some((part, block)) = self.sns_tones.keys().find_map(|&part| {
                SnsBlock::ALL
                    .into_iter()
                    .find(|&block| sns_request_key(part, block) == key)
                    .map(|block| (part, block))
            }) else {
                continue;
            };
            let tone = self.part_tone(part);
            let Some(load) = self.sns_tones.get_mut(&part) else {
                continue;
            };
            if load.tone != tone {
                self.invalidate_sns_tone(part);
                continue;
            }
            let retries = &mut load.retries[block.index()];
            if load.blocks.has(block) || *retries >= SNS_MAX_RETRIES {
                continue;
            }
            *retries += 1;
            self.queue_sns_request(part, block);
        }
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
        let addr = crate::pcm_drum::pcmd_common_param_address(part, crate::pcm_drum::KIT_LEVEL);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a single PCM Drum Kit Partial parameter for a part and key.
    fn set_pcmd_partial_param(
        &mut self,
        part: u8,
        key: u8,
        offset: u16,
        value: u8,
    ) -> Result<(), KeyOutOfRange> {
        let addr = crate::pcm_drum::pcmd_partial_param_address(part, key, offset)?;
        self.send_dt1(&addr, &[value]);
        Ok(())
    }

    /// Set a nibblized PCM Drum Kit Partial parameter (4 bytes, e.g. wave number).
    fn set_pcmd_partial_nib_param(
        &mut self,
        part: u8,
        key: u8,
        offset: u16,
        value: u16,
    ) -> Result<(), KeyOutOfRange> {
        let addr = crate::pcm_drum::pcmd_partial_param_address(part, key, offset)?;
        self.send_dt1(&addr, &crate::parse_helpers::encode_nibble4(value));
        Ok(())
    }

    /// Set a field of WMT `wmt` (0-based) in the Partial for `key`, sent
    /// nibblized when the field is.
    pub fn set_pcmd_wmt_param(
        &mut self,
        part: u8,
        key: u8,
        wmt: u8,
        field: crate::pcm_drum::PcmdWmtField,
        value: u16,
    ) -> Result<(), crate::pcm_drum::PcmdFieldError> {
        let offset = crate::pcm_drum::pcmd_wmt_field_offset(wmt, field)?;
        if field.is_nibblized() {
            self.set_pcmd_partial_nib_param(part, key, offset, value)?;
        } else {
            self.set_pcmd_partial_param(part, key, offset, value as u8)?;
        }
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
    ) -> Result<(), crate::pcm_drum::PcmdFieldError> {
        let offset = crate::pcm_drum::pcmd_partial_field_offset(field, index)?;
        self.set_pcmd_partial_param(part, key, offset, value)?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Motional Surround
    // -----------------------------------------------------------------------

    /// Set a Motional Surround common parameter.
    pub fn set_surround_param(&mut self, param_offset: u8, value: u8) {
        let s = &mut self.state.surround;
        match param_offset {
            0x00 => s.enabled = value == 1,
            0x01 => s.room_type = value,
            0x02 => s.ambience_level = value,
            0x03 => s.room_size = value,
            0x04 => s.ambience_time = value,
            0x05 => s.ambience_density = value,
            0x06 => s.ambience_hf_damp = value,
            0x07 => s.ext.lr = value,
            0x08 => s.ext.fb = value,
            0x09 => s.ext.width = value,
            0x0A => s.ext.ambience_send = value,
            0x0B => s.ext_control_channel = value,
            0x0C => s.depth = value,
            _ => return,
        }
        let addr = params::surround_address(param_offset);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a per-part surround parameter.
    ///
    /// # Panics
    /// Debug-asserts that `part_idx` < 16.
    pub fn set_part_surround_param(&mut self, part_idx: u8, param: [u8; 3], value: u8) {
        debug_assert!(
            part_idx < Self::NUM_PARTS,
            "part index {part_idx} out of range"
        );
        let ps = &mut self.state.surround.parts[part_idx as usize];
        match param {
            params::part_surround::LR => ps.lr = value,
            params::part_surround::FB => ps.fb = value,
            params::part_surround::WIDTH => ps.width = value,
            params::part_surround::AMBIENCE_SEND => ps.ambience_send = value,
            _ => return,
        }
        let addr = params::part_address(part_idx, param);
        self.send_dt1(&addr, &[value]);
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

    /// Set a parameter within a Comp+EQ unit.
    ///
    /// `unit` is 0–5, `param_offset` is a `comp::` or `comp_eq::` constant.
    pub fn set_comp_eq_param(&mut self, unit: u8, param_offset: u8, value: u8) {
        let u = &mut self.state.drum_comp_eq.units[unit as usize];
        match param_offset {
            0x00 => u.comp_switch = value == 1,
            0x01 => u.comp_attack = value,
            0x02 => u.comp_release = value,
            0x03 => u.comp_threshold = value,
            0x04 => u.comp_ratio = value,
            0x05 => u.comp_output_gain = value,
            0x06 => u.eq_switch = value == 1,
            0x07 => u.eq_low_freq = value,
            0x08 => u.eq_low_gain = value,
            0x09 => u.eq_mid_freq = value,
            0x0A => u.eq_mid_gain = value,
            0x0B => u.eq_mid_q = value,
            0x0C => u.eq_high_freq = value,
            0x0D => u.eq_high_gain = value,
            _ => return,
        }
        let part = self.state.drum_comp_eq.part;
        let tone_type = self.comp_eq_tone_type();
        let addr = params::comp_eq_param_address(part, tone_type, unit, param_offset);
        self.send_dt1(&addr, &[value]);
    }

    /// Determine the tone type for the Comp+EQ assigned part.
    fn comp_eq_tone_type(&self) -> ToneType {
        let part = self.state.drum_comp_eq.part;
        let msb = self.state.parts[part as usize].tone_bank_msb;
        params::tone_type_from_bank_msb(msb).unwrap_or(ToneType::PcmDrum)
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
    fn change_part_tone_enqueues_three_messages() {
        let mut dev = DeviceState::new(0x10);
        dev.change_part_tone(0, 89, 64, 12);
        assert_eq!(dev.queue.len(), 3);
        assert_eq!(dev.state().parts[0].tone_bank_msb, 89);
        assert_eq!(dev.state().parts[0].tone_bank_lsb, 64);
        assert_eq!(dev.state().parts[0].tone_pc, 12);
    }

    #[test]
    fn rename_tone_writes_padded_name_for_tone_type() {
        let mut dev = DeviceState::new(0x10);
        dev.change_part_tone(1, 95, 0, 0); // SN Synth
        let _ = (dev.drain(0.0), dev.drain(100.0), dev.drain(200.0));
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
        use crate::pcm_drum::{PCMD_FIRST_KEY, PCMD_LAST_KEY};
        let mut dev = DeviceState::new(0x10);
        for key in [PCMD_FIRST_KEY - 1, PCMD_LAST_KEY + 1] {
            assert!(dev.set_pcmd_partial_param(0, key, 0x0021, 1).is_err());
            assert!(dev.set_pcmd_partial_nib_param(0, key, 0x0027, 1).is_err());
        }
        assert!(!dev.has_pending());
        assert!(
            dev.set_pcmd_partial_param(0, PCMD_FIRST_KEY, 0x0021, 1)
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

        dev.set_sns_common_param(0, 0x0C, 99); // Tone Level
        dev.set_sns_partial_param(0, 1, 0x0C, 77); // Partial 2 Filter Cutoff
        dev.set_sns_partial_nib_param(0, 2, 0x35, 1234); // Partial 3 Wave Number

        let tone = dev.sns_tone(0).unwrap();
        assert_eq!(tone.common.tone_level, 99);
        assert_eq!(tone.partials[0].filter_cutoff, 0);
        assert_eq!(tone.partials[1].filter_cutoff, 77);
        assert_eq!(tone.partials[2].wave_number, 1234);

        // Each edit goes out once, as encoded for the cache.
        let sent = drain_until(&mut dev, 1000.0, 1200.0);
        assert!(sent.contains(&sysex::build_dt1(
            0x10,
            &sn_synth::sns_partial_param_address(0, 2, 0x35),
            &crate::parse_helpers::encode_nibble4(1234),
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
        assert!(drain_until(&mut dev, 300.0, 80.0 + SNS_REPLY_TIMEOUT_MS - 10.0).is_empty());
        assert_eq!(
            drain_until(&mut dev, 80.0 + SNS_REPLY_TIMEOUT_MS, 3000.0),
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
        let tries = 1 + SNS_MAX_RETRIES as usize;
        assert_eq!(sent.len(), SnsBlock::ALL.len() * tries);
        for block in SnsBlock::ALL {
            let request = sns_request(0, block);
            assert_eq!(sent.iter().filter(|m| **m == request).count(), tries);
        }
        assert_eq!(dev.sns_tone(0), None);
    }

    #[test]
    fn a_tone_change_stops_the_old_tone_requests() {
        // Queued but not yet sent.
        let mut dev = loading_sns(0);
        dev.change_part_tone(0, 87, 0, 0);
        let sent = drain_until(&mut dev, 0.0, 60_000.0);
        assert_eq!(sent.len(), 3, "only the tone change goes out: {sent:?}");

        // Sent, awaiting replies.
        let mut dev = loading_sns(0);
        drain_until(&mut dev, 0.0, 200.0);
        dev.change_part_tone(0, 87, 0, 0);
        assert_eq!(drain_until(&mut dev, 200.0, 60_000.0).len(), 3);
        assert_eq!(dev.sns_tone(0), None);

        // Changed behind the cache's back (a part dump): the timed-out
        // requests aren't sent again, and the next load reads anew.
        let mut dev = loading_sns(0);
        drain_until(&mut dev, 0.0, 200.0);
        dev.state_mut().parts[0].tone_pc = 4;
        assert!(drain_until(&mut dev, 200.0, 60_000.0).is_empty());
        assert!(dev.load_sns_tone(0));
    }
}
