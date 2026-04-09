//! Integral Core — Integra-7 SysEx engine.
//!
//! This crate implements the portable business logic for communicating with
//! the Roland INTEGRA-7 via SysEx messages, including address mapping,
//! checksum calculation, message construction, and state management.

pub mod address;
pub mod catalog;
pub mod device;
pub mod fx_params;
pub mod params;
pub mod state;
pub mod sysex;
pub mod tone_banks;
