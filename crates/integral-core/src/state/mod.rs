//! Mixer state types for the INTEGRA-7.
//!
//! These structs represent the runtime state of the mixer — parts, EQ, FX,
//! master settings.  They are plain data with sensible defaults; parsing
//! from SysEx dumps is provided by [`parse`].

pub mod parse;

use std::collections::HashMap;

/// Number of parts in the INTEGRA-7 mixer.
pub const NUM_PARTS: usize = 16;

// ---------------------------------------------------------------------------
// EQ
// ---------------------------------------------------------------------------

/// 3-band parametric EQ state.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FxState {
    /// Effect on/off.
    pub enabled: bool,
    /// Effect type index (Chorus: 0–3, Reverb: 0–6).
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
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
    #[cfg_attr(feature = "serde", serde(rename = "tonePC"))]
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
// Motional Surround
// ---------------------------------------------------------------------------

/// Per-part surround positioning.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SurroundPartState {
    /// L-R position (0–127, display: −64 to +63).
    pub lr: u8,
    /// F-B position (0–127, display: −64 to +63).
    pub fb: u8,
    /// Width (0–32).
    pub width: u8,
    /// Ambience send level (0–127).
    pub ambience_send: u8,
}

impl Default for SurroundPartState {
    fn default() -> Self {
        Self {
            lr: 64,
            fb: 64,
            width: 16,
            ambience_send: 0,
        }
    }
}

/// Motional Surround global state.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SurroundState {
    /// Global on/off.
    pub enabled: bool,
    /// Room type (0–3: ROOM1, ROOM2, HALL1, HALL2).
    pub room_type: u8,
    /// Room size (0–2: SMALL, MEDIUM, LARGE).
    pub room_size: u8,
    /// Depth (0–100).
    pub depth: u8,
    /// Ambience level (0–127).
    pub ambience_level: u8,
    /// Ambience time (0–100).
    pub ambience_time: u8,
    /// Ambience density (0–100).
    pub ambience_density: u8,
    /// Ambience HF damp (0–100).
    pub ambience_hf_damp: u8,
    /// Per-part positioning (16 parts).
    pub parts: [SurroundPartState; NUM_PARTS],
    /// Ext part positioning.
    pub ext: SurroundPartState,
    /// Ext part control channel (0–16: 1–16, OFF).
    pub ext_control_channel: u8,
}

impl Default for SurroundState {
    fn default() -> Self {
        Self {
            enabled: false,
            room_type: 0,
            room_size: 1,
            depth: 50,
            ambience_level: 64,
            ambience_time: 50,
            ambience_density: 50,
            ambience_hf_damp: 50,
            parts: std::array::from_fn(|_| SurroundPartState::default()),
            ext: SurroundPartState::default(),
            ext_control_channel: 16, // OFF
        }
    }
}

// ---------------------------------------------------------------------------
// Drum Comp+EQ
// ---------------------------------------------------------------------------

/// State of a single Comp+EQ unit (compressor + 3-band EQ).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CompEqUnit {
    /// Compressor on/off.
    pub comp_switch: bool,
    /// Compressor attack time (0–31).
    pub comp_attack: u8,
    /// Compressor release time (0–23).
    pub comp_release: u8,
    /// Compressor threshold (0–127).
    pub comp_threshold: u8,
    /// Compressor ratio (0–19).
    pub comp_ratio: u8,
    /// Compressor output gain (0–24, display: 0 to +24 dB).
    pub comp_output_gain: u8,
    /// EQ on/off.
    pub eq_switch: bool,
    /// EQ low freq (0=200 Hz, 1=400 Hz).
    pub eq_low_freq: u8,
    /// EQ low gain (0–30, display: −15 to +15 dB).
    pub eq_low_gain: u8,
    /// EQ mid freq (0–16).
    pub eq_mid_freq: u8,
    /// EQ mid gain (0–30).
    pub eq_mid_gain: u8,
    /// EQ mid Q (0–4).
    pub eq_mid_q: u8,
    /// EQ high freq (0–2).
    pub eq_high_freq: u8,
    /// EQ high gain (0–30).
    pub eq_high_gain: u8,
}

impl Default for CompEqUnit {
    fn default() -> Self {
        Self {
            comp_switch: false,
            comp_attack: 10,
            comp_release: 10,
            comp_threshold: 127,
            comp_ratio: 0,
            comp_output_gain: 0,
            eq_switch: false,
            eq_low_freq: 1,
            eq_low_gain: 15,
            eq_mid_freq: 7,
            eq_mid_gain: 15,
            eq_mid_q: 0,
            eq_high_freq: 1,
            eq_high_gain: 15,
        }
    }
}

/// Drum Comp+EQ state (6 units + Studio Set common settings).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct DrumCompEqState {
    /// Global on/off (Studio Set Common).
    pub enabled: bool,
    /// Assigned part index (0–15).
    pub part: u8,
    /// Per-unit output assign (0–12: PART, A, B, C, D, 1–8).
    pub output_assigns: [u8; 6],
    /// The 6 Comp+EQ units.
    pub units: [CompEqUnit; 6],
}

impl Default for DrumCompEqState {
    fn default() -> Self {
        Self {
            enabled: false,
            part: 9, // Part 10 (0-indexed)
            output_assigns: [0; 6],
            units: std::array::from_fn(|_| CompEqUnit::default()),
        }
    }
}

// ---------------------------------------------------------------------------
// Full mixer state
// ---------------------------------------------------------------------------

/// Full mixer state.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct MixerState {
    /// Studio Set name (up to 16 ASCII chars).
    pub studio_set_name: String,
    /// Current Studio Set PC (0–63).
    #[cfg_attr(feature = "serde", serde(rename = "studioSetPC"))]
    pub studio_set_pc: u8,
    /// System master level (0–127).
    pub master_level: u8,
    /// Solo Part (0=OFF, 1–16=Part 1–16).
    pub solo_part: u8,
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
    /// Motional Surround state.
    pub surround: SurroundState,
    /// Drum Comp+EQ (6 units, assigned to one part).
    pub drum_comp_eq: DrumCompEqState,
    /// All 64 Studio Set names (indexed 0–63).  Populated via catalog query.
    pub studio_set_names: HashMap<u8, String>,
    /// Preview part (0 = off, 1–16 = part being previewed).
    pub preview_part: u8,
}

impl Default for MixerState {
    fn default() -> Self {
        Self {
            studio_set_name: String::new(),
            studio_set_pc: 0,
            master_level: 100,
            solo_part: 0,
            parts: std::array::from_fn(|_| PartState::default()),
            chorus: FxState::default(),
            reverb: FxState::default(),
            ext_level: 100,
            ext_muted: false,
            master_eq: EqState::default(),
            surround: SurroundState::default(),
            drum_comp_eq: DrumCompEqState::default(),
            studio_set_names: HashMap::new(),
            preview_part: 0,
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
