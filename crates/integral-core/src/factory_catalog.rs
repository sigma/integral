//! Static factory sound catalog, generated at build time from `docs/sounds/*.json`.
//!
//! Provides instant access to all factory preset tone names without requiring
//! a MIDI connection. Use [`factory_tones`] to look up tones by bank.

/// A factory preset tone entry.
#[derive(Debug, Clone, Copy)]
pub struct FactoryTone {
    /// Bank Select MSB.
    pub msb: u8,
    /// Bank Select LSB.
    pub lsb: u8,
    /// Program Change (0-indexed).
    pub pc: u8,
    /// Tone name (up to 16 ASCII characters).
    pub name: &'static str,
    /// Numeric tone category (use `svd::tone_category_name()` to display).
    pub category: u8,
}

include!(concat!(env!("OUT_DIR"), "/factory_catalog_data.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sn_acoustic_preset_first() {
        let tones = factory_tones(89, 64);
        assert_eq!(tones.len(), 128);
        assert_eq!(tones[0].name, "Full Grand 1");
        assert_eq!(tones[0].pc, 0);
        assert_eq!(tones[0].category, 1); // Ac.Piano
    }

    #[test]
    fn sn_synth_preset_first() {
        let tones = factory_tones(95, 64);
        assert_eq!(tones.len(), 128);
        assert_eq!(tones[0].name, "JP8 Strings1");
        assert_eq!(tones[0].pc, 0);
        assert_eq!(tones[0].category, 36); // Synth Pad/Strings
    }

    #[test]
    fn unknown_bank_returns_empty() {
        assert!(factory_tones(0, 0).is_empty());
        assert!(factory_tones(255, 255).is_empty());
    }

    #[test]
    fn total_entry_count() {
        // Verify we have a reasonable number of entries.
        let mut total = 0;
        for msb in 0..=127 {
            for lsb in 0..=127 {
                total += factory_tones(msb, lsb).len();
            }
        }
        assert!(total > 5000, "expected >5000 factory tones, got {total}");
    }
}
