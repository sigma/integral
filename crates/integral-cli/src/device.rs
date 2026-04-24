//! High-level device interface for the INTEGRA-7.
//!
//! [`IntegraDevice`] wraps the raw MIDI connection and provides typed methods
//! for common read/write operations (studio-set name, master level, part
//! mixer state, etc.).

use std::sync::mpsc::Receiver;
use std::time::Duration;

use anyhow::{Context, Result};
use integral_core::address::{Address, DataSize};
use integral_core::params;
use integral_core::state::PartState;
use integral_core::sysex;
use midir::MidiOutputConnection;

use crate::midi;

/// A high-level handle to a connected INTEGRA-7 device.
///
/// Encapsulates the MIDI output connection, an inbound message receiver,
/// the resolved SysEx device ID, and a response timeout.
pub struct IntegraDevice {
    /// Guard that keeps the MIDI input connection alive.
    _conn_in: midir::MidiInputConnection<()>,
    conn_out: MidiOutputConnection,
    rx: Receiver<Vec<u8>>,
    device_id: u8,
    timeout: Duration,
}

#[allow(dead_code)]
impl IntegraDevice {
    /// Open MIDI ports matching `port_pattern` and return a connected device.
    pub fn connect(port_pattern: &str, device_id: u8, timeout: Duration) -> Result<Self> {
        let (conn_in, conn_out, rx) = midi::open_midi(port_pattern)?;
        Ok(Self {
            _conn_in: conn_in,
            conn_out,
            rx,
            device_id,
            timeout,
        })
    }

    /// Send an RQ1 request and wait for the corresponding DT1 response.
    pub fn request_data(&mut self, address: &Address, size: &DataSize) -> Result<Vec<u8>> {
        midi::request_data(
            &mut self.conn_out,
            &self.rx,
            self.device_id,
            address,
            size,
            self.timeout,
        )
    }

    /// Send a DT1 data-set message to the device.
    pub fn send_data(&mut self, address: &Address, data: &[u8]) -> Result<()> {
        let dt1 = sysex::build_dt1(self.device_id, address, data);
        self.conn_out.send(&dt1).context("failed to send DT1")?;
        Ok(())
    }

    /// Read the current studio set name (up to 16 ASCII characters).
    pub fn read_studio_set_name(&mut self) -> Result<String> {
        let data = self.request_data(&params::STUDIO_SET_NAME, &params::STUDIO_SET_NAME_SIZE)?;
        Ok(String::from_utf8_lossy(&data).trim_end().to_string())
    }

    /// Read the system master level (0-127).
    pub fn read_master_level(&mut self) -> Result<u8> {
        let data = self.request_data(&params::SYSTEM_MASTER_LEVEL, &params::SINGLE_BYTE_SIZE)?;
        Ok(data[0])
    }

    /// Read the mixer state for a part (0-indexed `part_index`).
    pub fn read_part_mixer(&mut self, part_index: u8) -> Result<PartState> {
        let addr = params::part_address(part_index, params::part::RECEIVE_CHANNEL);
        let data = self.request_data(&addr, &params::PART_MIXER_SIZE)?;
        Ok(PartState {
            receive_channel: *data.first().unwrap_or(&0),
            tone_bank_msb: *data.get(0x06).unwrap_or(&0),
            tone_bank_lsb: *data.get(0x07).unwrap_or(&0),
            tone_pc: *data.get(0x08).unwrap_or(&0),
            level: *data.get(0x09).unwrap_or(&0),
            pan: *data.get(0x0A).unwrap_or(&0),
            muted: *data.get(0x25).unwrap_or(&0) != 0,
            chorus_send: *data.get(0x27).unwrap_or(&0),
            reverb_send: *data.get(0x28).unwrap_or(&0),
            ..PartState::default()
        })
    }

    /// Return the SysEx device ID used for this connection.
    pub fn device_id(&self) -> u8 {
        self.device_id
    }

    /// Return a mutable reference to the underlying MIDI output connection.
    ///
    /// Useful for callers that need low-level access (e.g. sending raw
    /// messages) while still holding the device handle.
    pub fn conn_out_mut(&mut self) -> &mut MidiOutputConnection {
        &mut self.conn_out
    }

    /// Return a reference to the inbound message receiver.
    pub fn rx(&self) -> &Receiver<Vec<u8>> {
        &self.rx
    }
}
