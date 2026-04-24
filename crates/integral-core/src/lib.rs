//! Integral Core — Integra-7 SysEx engine.
//!
//! This crate implements the portable business logic for communicating with
//! the Roland INTEGRA-7 via SysEx messages, including address mapping,
//! checksum calculation, message construction, and state management.

use thiserror::Error;

/// Error returned when a tone data dump is too short to parse.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("tone data too short: expected at least {expected} bytes, got {got}")]
pub struct ToneParseError {
    /// Minimum required size.
    pub expected: usize,
    /// Actual size received.
    pub got: usize,
}

pub mod address;
pub mod bitstream;
pub mod catalog;
pub mod device;
pub mod device_spec;
pub mod factory_catalog;
pub mod fx_params;
pub mod mfx;
pub mod midi_bridge;
pub mod mfx_params;
pub mod param_registry;
pub mod params;
pub mod parse_helpers;
pub mod pcm_drum;
pub mod pcm_synth;
pub mod sn_acoustic;
pub mod sn_drum;
pub mod sn_synth;
pub mod sna_inst_params;
pub mod state;
pub mod svd;
pub mod svd_convert;
pub mod svd_specs;
pub mod sysex;
pub mod tone_banks;
pub mod tone_catalog;
