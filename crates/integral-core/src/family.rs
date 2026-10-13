//! Parameter families and their palette.
//!
//! A **Parameter family** is the role a parameter plays in shaping the sound,
//! whatever editor or effect it sits in (see `CONTEXT.md`). Every parameter
//! definition declares one, and both the web UI and the VST colour a control
//! by its family's [`ParameterFamily::colour`] (ADR 0002).

use core::fmt;

/// An sRGB colour, one byte per channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl fmt::Display for Rgb {
    /// Formats as a CSS hex colour, `#rrggbb`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

/// The role a parameter plays in shaping the sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParameterFamily {
    /// Pitch / Source: what is sounding and at what pitch.
    Pitch,
    /// Filter / Tone: the spectrum, including EQ.
    Filter,
    /// Amp / Level: loudness and dynamics.
    Amp,
    /// Modulation: anything that generates movement (LFOs, step sequencers).
    Modulation,
    /// Time / Space: delay, reverb, pan and stereo placement.
    Time,
    /// Common / Global: switches, structure and routing, with no sound dimension.
    Common,
}

impl ParameterFamily {
    /// Every family, in display order.
    pub const ALL: [Self; 6] = [
        Self::Pitch,
        Self::Filter,
        Self::Amp,
        Self::Modulation,
        Self::Time,
        Self::Common,
    ];

    /// The family's colour, from the colour-blind-safe Okabe-Ito palette.
    pub const fn colour(self) -> Rgb {
        match self {
            Self::Pitch => Rgb(0xE6, 0x9F, 0x00),
            Self::Filter => Rgb(0x56, 0xB4, 0xE9),
            Self::Amp => Rgb(0x00, 0x9E, 0x73),
            Self::Modulation => Rgb(0xCC, 0x79, 0xA7),
            Self::Time => Rgb(0xF0, 0xE4, 0x42),
            Self::Common => Rgb(0x99, 0x99, 0x99),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_is_okabe_ito() {
        let hex: Vec<String> = ParameterFamily::ALL
            .iter()
            .map(|f| f.colour().to_string())
            .collect();
        assert_eq!(
            hex,
            [
                "#e69f00", "#56b4e9", "#009e73", "#cc79a7", "#f0e442", "#999999"
            ]
        );
    }
}
