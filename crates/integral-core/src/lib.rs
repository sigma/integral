//! Integral Core — Integra-7 SysEx engine.
//!
//! This crate implements the portable business logic for communicating with
//! the Roland INTEGRA-7 via SysEx messages, including address mapping,
//! checksum calculation, message construction, and state management.

pub mod address;
pub mod bitstream;
pub mod catalog;
pub mod device;
pub mod fx_params;
pub mod mfx;
pub mod mfx_params;
pub mod params;
pub mod pcm_drum;
pub mod pcm_synth;
pub mod sn_acoustic;
pub mod sn_drum;
pub mod sn_synth;
pub mod sna_inst_params;
pub mod state;
pub mod svd;
pub mod sysex;
pub mod tone_banks;
pub mod tone_catalog;
