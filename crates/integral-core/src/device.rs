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
use crate::sn_synth;
use crate::state::MixerState;
use crate::sysex;
use crate::{params, params::part};

/// Minimum interval between SysEx sends (ms).
const THROTTLE_MS: f64 = 40.0;

/// Duration to suppress incoming DT1 echoes after a local send (ms).
const ECHO_SUPPRESS_MS: f64 = 150.0;

// ---------------------------------------------------------------------------
// Send queue
// ---------------------------------------------------------------------------

struct QueuedMessage {
    key: String,
    bytes: Vec<u8>,
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
            self.queue.push(QueuedMessage { key, bytes });
        }
    }

    /// Queue a raw SysEx message (non-DT1, e.g. catalog queries).
    pub fn send_raw(&mut self, key: &str, bytes: Vec<u8>) {
        self.queue.push(QueuedMessage {
            key: key.to_string(),
            bytes,
        });
    }

    /// Drain the next outbound message if the throttle window has elapsed.
    ///
    /// `now_ms` is a monotonic timestamp in milliseconds.
    /// Returns `None` if the queue is empty or the throttle hasn't elapsed.
    pub fn drain(&mut self, now_ms: f64) -> Option<Vec<u8>> {
        if self.queue.is_empty() {
            return None;
        }
        if now_ms - self.last_send_time < THROTTLE_MS {
            return None;
        }
        let msg = self.queue.remove(0);
        self.last_send_time = now_ms;

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
        }

        false
    }

    // -----------------------------------------------------------------------
    // Convenience setters (mutate state + queue DT1)
    // -----------------------------------------------------------------------

    /// Set a part's level (0–127).
    pub fn set_part_level(&mut self, part: u8, value: u8) {
        self.state.parts[part as usize].level = value;
        let addr = params::part_address(part, part::LEVEL);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a part's pan (0–127, 64=centre).
    pub fn set_part_pan(&mut self, part: u8, value: u8) {
        self.state.parts[part as usize].pan = value;
        let addr = params::part_address(part, part::PAN);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a part's mute state.
    pub fn set_part_mute(&mut self, part: u8, muted: bool) {
        self.state.parts[part as usize].muted = muted;
        let addr = params::part_address(part, part::MUTE);
        self.send_dt1(&addr, &[u8::from(muted)]);
    }

    /// Toggle a part's mute state.
    pub fn toggle_part_mute(&mut self, part: u8) {
        let muted = !self.state.parts[part as usize].muted;
        self.set_part_mute(part, muted);
    }

    /// Set a part's chorus send level (0–127).
    pub fn set_part_chorus_send(&mut self, part: u8, value: u8) {
        self.state.parts[part as usize].chorus_send = value;
        let addr = params::part_address(part, part::CHORUS_SEND);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a part's reverb send level (0–127).
    pub fn set_part_reverb_send(&mut self, part: u8, value: u8) {
        self.state.parts[part as usize].reverb_send = value;
        let addr = params::part_address(part, part::REVERB_SEND);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a part's receive channel (0–15).
    pub fn set_part_receive_channel(&mut self, part: u8, channel: u8) {
        self.state.parts[part as usize].receive_channel = channel;
        let addr = params::part_address(part, part::RECEIVE_CHANNEL);
        self.send_dt1(&addr, &[channel]);
    }

    /// Change a part's tone (bank select MSB + LSB + PC).
    ///
    /// This enqueues 3 DT1 messages for the tone bank MSB, LSB, and PC.
    pub fn change_part_tone(&mut self, part: u8, msb: u8, lsb: u8, pc: u8) {
        let p = &mut self.state.parts[part as usize];
        p.tone_bank_msb = msb;
        p.tone_bank_lsb = lsb;
        p.tone_pc = pc;
        p.tone_name = String::new(); // Clear until re-read.

        let msb_addr = params::part_address(part, part::TONE_BANK_MSB);
        self.send_dt1(&msb_addr, &[msb]);
        let lsb_addr = params::part_address(part, part::TONE_BANK_LSB);
        self.send_dt1(&lsb_addr, &[lsb]);
        let pc_addr = params::part_address(part, part::TONE_PC);
        self.send_dt1(&pc_addr, &[pc]);
    }

    /// Set the system master level (0–127).
    pub fn set_master_level(&mut self, value: u8) {
        self.state.master_level = value;
        self.send_dt1(&params::SYSTEM_MASTER_LEVEL, &[value]);
    }

    /// Set a Part EQ parameter.
    pub fn set_part_eq_param(&mut self, part: u8, param_offset: u8, value: u8) {
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
        let offset = 0x04 + (param_index as u8) * 4;
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
        let offset = 0x03 + (param_index as u8) * 4;
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
        self.send_dt1(&params::SETUP_STUDIO_SET_BS_MSB, &[85]);
        self.send_dt1(&params::SETUP_STUDIO_SET_BS_LSB, &[0]);
        self.send_dt1(&params::SETUP_STUDIO_SET_PC, &[pc]);
    }

    // -----------------------------------------------------------------------
    // MFX
    // -----------------------------------------------------------------------

    /// Set an MFX header parameter (type, sends, control slots).
    pub fn set_mfx_param(&mut self, part: u8, param_offset: u8, value: u8) {
        let addr = crate::mfx::mfx_address(part, param_offset);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a nibblized MFX effect parameter (0-based index).
    pub fn set_mfx_nib_param(&mut self, part: u8, param_index: u8, value: i32) {
        let bytes = crate::mfx::encode_mfx_param(value);
        let addr = crate::mfx::mfx_param_address(part, param_index);
        self.send_dt1(&addr, &bytes);
    }

    /// Build an RQ1 to read the full MFX block for a part.
    pub fn build_mfx_request(&self, part: u8) -> Vec<u8> {
        let addr = crate::mfx::mfx_block_address(part);
        sysex::build_rq1(self.device_id, &addr, &crate::mfx::MFX_BLOCK_SIZE)
    }

    // -----------------------------------------------------------------------
    // SN-S Tone Edit
    // -----------------------------------------------------------------------

    /// Set a single SN-S Common parameter for a part.
    pub fn set_sns_common_param(&mut self, part: u8, offset: u8, value: u8) {
        let addr = sn_synth::sns_common_param_address(part, offset);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a single SN-S Partial parameter for a part.
    pub fn set_sns_partial_param(&mut self, part: u8, partial: u8, offset: u8, value: u8) {
        let addr = sn_synth::sns_partial_param_address(part, partial, offset);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a nibblized SN-S Partial parameter (4 bytes, e.g. wave number).
    pub fn set_sns_partial_nib_param(&mut self, part: u8, partial: u8, offset: u8, value: u16) {
        let addr = sn_synth::sns_partial_param_address(part, partial, offset);
        let bytes = [
            ((value >> 12) & 0x0F) as u8,
            ((value >> 8) & 0x0F) as u8,
            ((value >> 4) & 0x0F) as u8,
            (value & 0x0F) as u8,
        ];
        self.send_dt1(&addr, &bytes);
    }

    /// Set a nibblized SN-S Common parameter (4 bytes, e.g. phrase number).
    pub fn set_sns_common_nib_param(&mut self, part: u8, offset: u8, value: u16) {
        let addr = sn_synth::sns_common_param_address(part, offset);
        let bytes = [
            ((value >> 12) & 0x0F) as u8,
            ((value >> 8) & 0x0F) as u8,
            ((value >> 4) & 0x0F) as u8,
            (value & 0x0F) as u8,
        ];
        self.send_dt1(&addr, &bytes);
    }

    /// Build an RQ1 to read the full SN-S Common block for a part.
    pub fn build_sns_common_request(&self, part: u8) -> Vec<u8> {
        let addr = sn_synth::sns_common_address(part);
        sysex::build_rq1(self.device_id, &addr, &sn_synth::SNS_COMMON_BLOCK_SIZE)
    }

    /// Build an RQ1 to read an SN-S Partial block for a part.
    pub fn build_sns_partial_request(&self, part: u8, partial: u8) -> Vec<u8> {
        let addr = sn_synth::sns_partial_address(part, partial);
        sysex::build_rq1(self.device_id, &addr, &sn_synth::SNS_PARTIAL_BLOCK_SIZE)
    }

    // -----------------------------------------------------------------------
    // SN-A Tone Edit
    // -----------------------------------------------------------------------

    /// Set a single SN-A Common parameter for a part.
    pub fn set_sna_common_param(&mut self, part: u8, offset: u8, value: u8) {
        let addr = crate::sn_acoustic::sna_common_param_address(part, offset);
        self.send_dt1(&addr, &[value]);
    }

    /// Build an RQ1 to read the full SN-A Common block for a part.
    pub fn build_sna_common_request(&self, part: u8) -> Vec<u8> {
        let addr = crate::sn_acoustic::sna_common_address(part);
        sysex::build_rq1(
            self.device_id,
            &addr,
            &crate::sn_acoustic::SNA_COMMON_BLOCK_SIZE,
        )
    }

    // -----------------------------------------------------------------------
    // SN-D Tone Edit
    // -----------------------------------------------------------------------

    /// Set a single SN-D Common parameter for a part.
    pub fn set_snd_common_param(&mut self, part: u8, offset: u8, value: u8) {
        let addr = crate::sn_drum::snd_common_param_address(part, offset);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a single SN-D Note parameter for a part and key.
    pub fn set_snd_note_param(&mut self, part: u8, key: u8, offset: u8, value: u8) {
        let addr = crate::sn_drum::snd_note_param_address(part, key, offset);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a nibblized SN-D Note parameter (4 bytes, e.g. inst number or tune).
    pub fn set_snd_note_nib_param(&mut self, part: u8, key: u8, offset: u8, value: u16) {
        let addr = crate::sn_drum::snd_note_param_address(part, key, offset);
        let bytes = [
            ((value >> 12) & 0x0F) as u8,
            ((value >> 8) & 0x0F) as u8,
            ((value >> 4) & 0x0F) as u8,
            (value & 0x0F) as u8,
        ];
        self.send_dt1(&addr, &bytes);
    }

    /// Build an RQ1 to read the full SN-D Common block for a part.
    pub fn build_snd_common_request(&self, part: u8) -> Vec<u8> {
        let addr = crate::sn_drum::snd_common_address(part);
        sysex::build_rq1(
            self.device_id,
            &addr,
            &crate::sn_drum::SND_COMMON_BLOCK_SIZE,
        )
    }

    /// Build an RQ1 to read an SN-D Note block for a part and key.
    pub fn build_snd_note_request(&self, part: u8, key: u8) -> Vec<u8> {
        let addr = crate::sn_drum::snd_note_address(part, key);
        sysex::build_rq1(self.device_id, &addr, &crate::sn_drum::SND_NOTE_BLOCK_SIZE)
    }

    // -----------------------------------------------------------------------
    // PCM Synth Tone Edit
    // -----------------------------------------------------------------------

    /// Set a single PCM Synth Common parameter for a part.
    pub fn set_pcms_common_param(&mut self, part: u8, offset: u8, value: u8) {
        let addr = crate::pcm_synth::pcms_common_param_address(part, offset);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a nibblized PCM Synth Common parameter (4 bytes).
    pub fn set_pcms_common_nib_param(&mut self, part: u8, offset: u8, value: u16) {
        let addr = crate::pcm_synth::pcms_common_param_address(part, offset);
        let bytes = [
            ((value >> 12) & 0x0F) as u8,
            ((value >> 8) & 0x0F) as u8,
            ((value >> 4) & 0x0F) as u8,
            (value & 0x0F) as u8,
        ];
        self.send_dt1(&addr, &bytes);
    }

    /// Set a single PCM Synth PMT parameter for a part.
    pub fn set_pcms_pmt_param(&mut self, part: u8, offset: u8, value: u8) {
        let addr = crate::pcm_synth::pcms_pmt_param_address(part, offset);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a single PCM Synth Partial parameter for a part.
    pub fn set_pcms_partial_param(&mut self, part: u8, partial: u8, offset: u16, value: u8) {
        let addr = crate::pcm_synth::pcms_partial_param_address(part, partial, offset);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a nibblized PCM Synth Partial parameter (4 bytes, e.g. wave number).
    pub fn set_pcms_partial_nib_param(&mut self, part: u8, partial: u8, offset: u16, value: u16) {
        let addr = crate::pcm_synth::pcms_partial_param_address(part, partial, offset);
        let bytes = [
            ((value >> 12) & 0x0F) as u8,
            ((value >> 8) & 0x0F) as u8,
            ((value >> 4) & 0x0F) as u8,
            (value & 0x0F) as u8,
        ];
        self.send_dt1(&addr, &bytes);
    }

    /// Set a nibblized PCM Synth Partial parameter (2 bytes, e.g. delay time, LFO rate).
    pub fn set_pcms_partial_nib2_param(&mut self, part: u8, partial: u8, offset: u16, value: u8) {
        let addr = crate::pcm_synth::pcms_partial_param_address(part, partial, offset);
        let bytes = [((value >> 4) & 0x0F), value & 0x0F];
        self.send_dt1(&addr, &bytes);
    }

    /// Set a single PCM Synth Common2 parameter for a part.
    pub fn set_pcms_common2_param(&mut self, part: u8, offset: u8, value: u8) {
        let addr = crate::pcm_synth::pcms_common2_param_address(part, offset);
        self.send_dt1(&addr, &[value]);
    }

    /// Set a nibblized PCM Synth Common2 parameter (4 bytes, e.g. phrase number).
    pub fn set_pcms_common2_nib_param(&mut self, part: u8, offset: u8, value: u16) {
        let addr = crate::pcm_synth::pcms_common2_param_address(part, offset);
        let bytes = [
            ((value >> 12) & 0x0F) as u8,
            ((value >> 8) & 0x0F) as u8,
            ((value >> 4) & 0x0F) as u8,
            (value & 0x0F) as u8,
        ];
        self.send_dt1(&addr, &bytes);
    }

    /// Build an RQ1 to read the PCM Synth Common block for a part.
    pub fn build_pcms_common_request(&self, part: u8) -> Vec<u8> {
        let addr = crate::pcm_synth::pcms_common_address(part);
        sysex::build_rq1(
            self.device_id,
            &addr,
            &crate::pcm_synth::PCMS_COMMON_BLOCK_SIZE,
        )
    }

    /// Build an RQ1 to read the PCM Synth PMT block for a part.
    pub fn build_pcms_pmt_request(&self, part: u8) -> Vec<u8> {
        let addr = crate::pcm_synth::pcms_pmt_address(part);
        sysex::build_rq1(
            self.device_id,
            &addr,
            &crate::pcm_synth::PCMS_PMT_BLOCK_SIZE,
        )
    }

    /// Build an RQ1 to read a PCM Synth Partial block for a part.
    pub fn build_pcms_partial_request(&self, part: u8, partial: u8) -> Vec<u8> {
        let addr = crate::pcm_synth::pcms_partial_address(part, partial);
        sysex::build_rq1(
            self.device_id,
            &addr,
            &crate::pcm_synth::PCMS_PARTIAL_BLOCK_SIZE,
        )
    }

    /// Build an RQ1 to read the PCM Synth Common2 block for a part.
    pub fn build_pcms_common2_request(&self, part: u8) -> Vec<u8> {
        let addr = crate::pcm_synth::pcms_common2_address(part);
        sysex::build_rq1(
            self.device_id,
            &addr,
            &crate::pcm_synth::PCMS_COMMON2_BLOCK_SIZE,
        )
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
    pub fn set_part_surround_param(&mut self, part_idx: u8, param: [u8; 3], value: u8) {
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

    /// Build an RQ1 to read the Motional Surround common block (13 bytes).
    pub fn build_surround_common_request(&self) -> Vec<u8> {
        sysex::build_rq1(
            self.device_id,
            &params::SURROUND_BASE,
            &params::SURROUND_COMMON_SIZE,
        )
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
        let addr = params::comp_eq_param_address(part, unit, param_offset);
        self.send_dt1(&addr, &[value]);
    }

    /// Build an RQ1 to read all 6 Comp+EQ units (84 bytes).
    pub fn build_comp_eq_block_request(&self) -> Vec<u8> {
        let part = self.state.drum_comp_eq.part;
        let addr = params::comp_eq_block_address(part);
        sysex::build_rq1(self.device_id, &addr, &params::COMP_EQ_BLOCK_SIZE)
    }

    // -----------------------------------------------------------------------
    // RQ1 request builders
    // -----------------------------------------------------------------------

    /// Build an RQ1 to read the part mixer state.
    pub fn build_part_mixer_request(&self, part: u8) -> Vec<u8> {
        let addr = params::part_address(part, part::RECEIVE_CHANNEL);
        sysex::build_rq1(self.device_id, &addr, &params::PART_MIXER_SIZE)
    }

    /// Build an RQ1 to read the system master level.
    pub fn build_master_level_request(&self) -> Vec<u8> {
        sysex::build_rq1(
            self.device_id,
            &params::SYSTEM_MASTER_LEVEL,
            &params::SINGLE_BYTE_SIZE,
        )
    }

    /// Build an RQ1 to read the studio set name.
    pub fn build_studio_set_name_request(&self) -> Vec<u8> {
        sysex::build_rq1(
            self.device_id,
            &params::STUDIO_SET_NAME,
            &params::STUDIO_SET_NAME_SIZE,
        )
    }

    /// Build an RQ1 to read the studio set PC.
    pub fn build_studio_set_pc_request(&self) -> Vec<u8> {
        sysex::build_rq1(
            self.device_id,
            &params::SETUP_STUDIO_SET_PC,
            &params::SINGLE_BYTE_SIZE,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
