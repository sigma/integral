//! Mixer state types for the INTEGRA-7.
//!
//! These structs represent the runtime state of the mixer — parts, EQ, FX,
//! master settings.  They are plain data with sensible defaults; parsing
//! from SysEx dumps is provided by [`crate::state::parse`].

use std::collections::HashMap;

/// Number of parts in the INTEGRA-7 mixer.
pub const NUM_PARTS: usize = 16;

// ---------------------------------------------------------------------------
// EQ
// ---------------------------------------------------------------------------

/// 3-band parametric EQ state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EqState {
    /// EQ on/off.
    pub enabled: bool,
    /// Low band frequency (0=200 Hz, 1=400 Hz).
    pub low_freq: u8,
    /// Low band gain (0–30, display: −15 to +15 dB).
    pub low_gain: u8,
    /// Mid band frequency (0–16, 17 values from 200 Hz to 8000 Hz).
    pub mid_freq: u8,
    /// Mid band gain (0–30, display: −15 to +15 dB).
    pub mid_gain: u8,
    /// Mid band Q (0–4, display: 0.5, 1.0, 2.0, 4.0, 8.0).
    pub mid_q: u8,
    /// High band frequency (0=2000 Hz, 1=4000 Hz, 2=8000 Hz).
    pub high_freq: u8,
    /// High band gain (0–30, display: −15 to +15 dB).
    pub high_gain: u8,
}

impl Default for EqState {
    fn default() -> Self {
        Self {
            enabled: true,
            low_freq: 1,   // 400 Hz
            low_gain: 15,  // 0 dB
            mid_freq: 7,   // 1000 Hz
            mid_gain: 15,  // 0 dB
            mid_q: 0,      // 0.5
            high_freq: 1,  // 4000 Hz
            high_gain: 15, // 0 dB
        }
    }
}

// ---------------------------------------------------------------------------
// FX (Chorus / Reverb)
// ---------------------------------------------------------------------------

/// Chorus or Reverb FX block state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FxState {
    /// Effect on/off.
    pub enabled: bool,
    /// Effect type index (Chorus: 0–3, Reverb: 0–6).
    pub fx_type: u8,
    /// Effect level (0–127).
    pub level: u8,
    /// Output routing (Chorus: 0–2 MAIN/REV/MAIN+REV; Reverb: 0–3 A/B/C/D).
    pub output: u8,
    /// Type-dependent parameters (nibblized values, decoded to display range).
    pub params: Vec<i32>,
}

impl Default for FxState {
    fn default() -> Self {
        Self {
            enabled: true,
            fx_type: 0,
            level: 0,
            output: 0,
            params: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Part
// ---------------------------------------------------------------------------

/// State of a single Part in the mixer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartState {
    /// Part level / volume (0–127).
    pub level: u8,
    /// Part pan (0–127, 64 = centre).
    pub pan: u8,
    /// Whether the part is muted.
    pub muted: bool,
    /// Tone bank MSB.
    pub tone_bank_msb: u8,
    /// Tone bank LSB.
    pub tone_bank_lsb: u8,
    /// Tone program number.
    pub tone_pc: u8,
    /// MIDI receive channel (0–15).
    pub receive_channel: u8,
    /// Chorus send level (0–127).
    pub chorus_send: u8,
    /// Reverb send level (0–127).
    pub reverb_send: u8,
    /// Tone name read from the device.
    pub tone_name: String,
    /// Per-part EQ settings.
    pub eq: EqState,
}

impl Default for PartState {
    fn default() -> Self {
        Self {
            level: 100,
            pan: 64,
            muted: false,
            tone_bank_msb: 0,
            tone_bank_lsb: 0,
            tone_pc: 0,
            receive_channel: 0,
            chorus_send: 0,
            reverb_send: 0,
            tone_name: String::new(),
            eq: EqState::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Full mixer state
// ---------------------------------------------------------------------------

/// Full mixer state.
#[derive(Debug, Clone, PartialEq)]
pub struct MixerState {
    /// Studio Set name (up to 16 ASCII chars).
    pub studio_set_name: String,
    /// Current Studio Set PC (0–63).
    pub studio_set_pc: u8,
    /// System master level (0–127).
    pub master_level: u8,
    /// All 16 parts.
    pub parts: [PartState; NUM_PARTS],
    /// Chorus (FX1) state.
    pub chorus: FxState,
    /// Reverb (FX2) state.
    pub reverb: FxState,
    /// External input level (0–127).
    pub ext_level: u8,
    /// External input mute.
    pub ext_muted: bool,
    /// Master EQ settings.
    pub master_eq: EqState,
    /// All 64 Studio Set names (indexed 0–63).  Populated via catalog query.
    pub studio_set_names: HashMap<u8, String>,
}

impl Default for MixerState {
    fn default() -> Self {
        Self {
            studio_set_name: String::new(),
            studio_set_pc: 0,
            master_level: 100,
            parts: std::array::from_fn(|_| PartState::default()),
            chorus: FxState::default(),
            reverb: FxState::default(),
            ext_level: 100,
            ext_muted: false,
            master_eq: EqState::default(),
            studio_set_names: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mixer_has_16_parts() {
        let m = MixerState::default();
        assert_eq!(m.parts.len(), NUM_PARTS);
    }

    #[test]
    fn default_part_values() {
        let p = PartState::default();
        assert_eq!(p.level, 100);
        assert_eq!(p.pan, 64);
        assert!(!p.muted);
        assert_eq!(p.tone_name, "");
    }

    #[test]
    fn default_eq_is_flat() {
        let eq = EqState::default();
        assert!(eq.enabled);
        assert_eq!(eq.low_gain, 15); // 0 dB
        assert_eq!(eq.mid_gain, 15);
        assert_eq!(eq.high_gain, 15);
    }

    #[test]
    fn default_fx_is_off_type() {
        let fx = FxState::default();
        assert!(fx.enabled);
        assert_eq!(fx.fx_type, 0);
        assert!(fx.params.is_empty());
    }
}
