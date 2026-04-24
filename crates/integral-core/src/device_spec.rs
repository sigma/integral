//! Static specification of INTEGRA-7 device capabilities.
//!
//! Centralizes all device-specific constants (part count, output names,
//! FX type labels, surround options, etc.) so that views and commands
//! consume a single spec rather than hardcoding device constants.

/// Static specification of the INTEGRA-7's capabilities.
///
/// Views and commands consume this spec rather than hardcoding device constants.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct DeviceSpec {
    /// Device model name.
    pub name: &'static str,
    /// Number of mixer parts.
    pub part_count: u8,
    /// Number of Drum Comp+EQ units.
    pub comp_eq_unit_count: u8,
    /// Output assign labels for parts (e.g. "A", "B", ... "8").
    pub output_assigns: &'static [&'static str],
    /// Output assign labels for Comp+EQ units (includes "PART" prefix).
    pub comp_eq_output_assigns: &'static [&'static str],
    /// Chorus type display names.
    pub chorus_type_names: &'static [&'static str],
    /// Chorus output routing names.
    pub chorus_output_names: &'static [&'static str],
    /// Reverb type display names.
    pub reverb_type_names: &'static [&'static str],
    /// Reverb output routing names.
    pub reverb_output_names: &'static [&'static str],
    /// Motional Surround room type names.
    pub surround_room_types: &'static [&'static str],
    /// Motional Surround room size names.
    pub surround_room_sizes: &'static [&'static str],
}

/// The INTEGRA-7 device specification with all factory constants.
pub const INTEGRA7: DeviceSpec = DeviceSpec {
    name: "INTEGRA-7",
    part_count: 16,
    comp_eq_unit_count: 6,
    output_assigns: &["A", "B", "C", "D", "1", "2", "3", "4", "5", "6", "7", "8"],
    comp_eq_output_assigns: &[
        "PART", "A", "B", "C", "D", "1", "2", "3", "4", "5", "6", "7", "8",
    ],
    chorus_type_names: &["OFF", "Chorus", "Delay", "GM2 Cho"],
    chorus_output_names: &["MAIN", "REV", "MAIN+REV"],
    reverb_type_names: &[
        "OFF", "Room 1", "Room 2", "Hall 1", "Hall 2", "Plate", "GM2 Rev",
    ],
    reverb_output_names: &["A", "B", "C", "D"],
    surround_room_types: &["Room 1", "Room 2", "Hall 1", "Hall 2"],
    surround_room_sizes: &["Small", "Medium", "Large"],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integra7_name() {
        assert_eq!(INTEGRA7.name, "INTEGRA-7");
    }

    #[test]
    fn integra7_part_count() {
        assert_eq!(INTEGRA7.part_count, 16);
    }

    #[test]
    fn integra7_comp_eq_unit_count() {
        assert_eq!(INTEGRA7.comp_eq_unit_count, 6);
    }

    #[test]
    fn integra7_output_assigns() {
        assert_eq!(INTEGRA7.output_assigns.len(), 12);
        assert_eq!(INTEGRA7.output_assigns[0], "A");
        assert_eq!(INTEGRA7.output_assigns[11], "8");
    }

    #[test]
    fn integra7_comp_eq_output_assigns() {
        assert_eq!(INTEGRA7.comp_eq_output_assigns.len(), 13);
        assert_eq!(INTEGRA7.comp_eq_output_assigns[0], "PART");
        assert_eq!(INTEGRA7.comp_eq_output_assigns[12], "8");
    }

    #[test]
    fn integra7_chorus_types() {
        assert_eq!(INTEGRA7.chorus_type_names.len(), 4);
        assert_eq!(INTEGRA7.chorus_type_names[0], "OFF");
        assert_eq!(INTEGRA7.chorus_type_names[3], "GM2 Cho");
    }

    #[test]
    fn integra7_chorus_outputs() {
        assert_eq!(INTEGRA7.chorus_output_names.len(), 3);
        assert_eq!(INTEGRA7.chorus_output_names[0], "MAIN");
        assert_eq!(INTEGRA7.chorus_output_names[2], "MAIN+REV");
    }

    #[test]
    fn integra7_reverb_types() {
        assert_eq!(INTEGRA7.reverb_type_names.len(), 7);
        assert_eq!(INTEGRA7.reverb_type_names[0], "OFF");
        assert_eq!(INTEGRA7.reverb_type_names[5], "Plate");
        assert_eq!(INTEGRA7.reverb_type_names[6], "GM2 Rev");
    }

    #[test]
    fn integra7_reverb_outputs() {
        assert_eq!(INTEGRA7.reverb_output_names.len(), 4);
        assert_eq!(INTEGRA7.reverb_output_names[0], "A");
        assert_eq!(INTEGRA7.reverb_output_names[3], "D");
    }

    #[test]
    fn integra7_surround_room_types() {
        assert_eq!(INTEGRA7.surround_room_types.len(), 4);
        assert_eq!(INTEGRA7.surround_room_types[0], "Room 1");
        assert_eq!(INTEGRA7.surround_room_types[3], "Hall 2");
    }

    #[test]
    fn integra7_surround_room_sizes() {
        assert_eq!(INTEGRA7.surround_room_sizes.len(), 3);
        assert_eq!(INTEGRA7.surround_room_sizes[0], "Small");
        assert_eq!(INTEGRA7.surround_room_sizes[2], "Large");
    }
}
