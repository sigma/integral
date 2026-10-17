//! Static specification of INTEGRA-7 device capabilities.
//!
//! Centralizes all device-specific constants (part count, output names,
//! FX type labels, surround options, etc.) so that views and commands
//! consume a single spec rather than hardcoding device constants.

use crate::address::{self, Address, DataSize};
use crate::params::{self, ToneType};
use crate::user_write::BankSelect;
use crate::{mfx, pcm_drum, pcm_synth, sn_acoustic, sn_drum, sn_synth};
use thiserror::Error;

/// Number of MIDI channels, fixed by the MIDI protocol.
pub const MIDI_CHANNELS: u8 = 16;

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
    /// Output assign labels (A–D), shared by Reverb and Chorus Output Assign.
    pub output_assign_names: &'static [&'static str],
    /// Motional Surround room type names.
    pub surround_room_types: &'static [&'static str],
    /// Motional Surround room size names.
    pub surround_room_sizes: &'static [&'static str],
    /// Drum Comp+EQ compressor attack time labels (msec), one per value.
    pub comp_attack_times: &'static [&'static str],
    /// Drum Comp+EQ compressor release time labels (msec), one per value.
    pub comp_release_times: &'static [&'static str],
    /// Drum Comp+EQ compressor ratio labels, one per value.
    pub comp_ratios: &'static [&'static str],
    /// Number of Partials in an SN Synth Tone.
    pub sn_synth_partial_count: u8,
    /// Number of Partials in a PCM Synth Tone.
    pub pcm_synth_partial_count: u8,
    /// Number of MFX control slots in a tone's MFX block.
    pub mfx_control_count: u8,
    /// Number of WMT layers in a PCM Drum Kit partial.
    pub pcm_drum_wmt_count: u8,
    /// Keys an SN Drum Kit has a Note block for.
    pub sn_drum_keys: KeyRange,
    /// Highest Inst Number an SN Drum Kit Note can select.
    pub sn_drum_inst_number_max: u16,
    /// Keys a PCM Drum Kit has a Partial block for.
    pub pcm_drum_keys: KeyRange,
    /// Maximum characters in a tone name.
    pub tone_name_length: u32,
    /// Maximum characters in a Studio Set name.
    pub studio_set_name_length: u32,
    /// User memory layout of each type the librarian handles.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub user_memory: &'static [UserMemorySpec],
}

impl DeviceSpec {
    /// Number of User Studio Set slots (the [`Self::user_memory`] entry with
    /// no tone type), or 0 if the device has none.
    pub fn user_studio_set_count(&self) -> u16 {
        self.user_memory
            .iter()
            .find(|m| m.tone_type.is_none())
            .map_or(0, |m| m.slot_count)
    }
}

/// An inclusive range of MIDI key numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct KeyRange {
    /// Lowest key.
    pub first: u8,
    /// Highest key.
    pub last: u8,
}

impl KeyRange {
    /// Return `key` if it lies in this range, or the error naming the range.
    pub const fn check(self, key: u8) -> Result<u8, KeyOutOfRange> {
        if key >= self.first && key <= self.last {
            Ok(key)
        } else {
            Err(KeyOutOfRange { key, range: self })
        }
    }
}

/// Error returned for a key outside a drum kit's [`KeyRange`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("key {key} out of range ({}–{})", range.first, range.last)]
pub struct KeyOutOfRange {
    /// The rejected key.
    pub key: u8,
    /// The range the key had to lie in.
    pub range: KeyRange,
}

/// A run of equally sized blocks in a Temporary layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockRun {
    /// Offset of the first block from the area base (`aa bb cc`).
    pub offset: [u8; 3],
    /// Size of each block, 7-bit.
    pub size: DataSize,
    /// Number of blocks.
    pub count: u8,
    /// Distance between consecutive blocks, in `00 xx 00` units.
    pub stride: u8,
}

const fn run(offset: [u8; 3], size: DataSize, count: u8, stride: u8) -> BlockRun {
    BlockRun {
        offset,
        size,
        count,
        stride,
    }
}

const fn one(offset: [u8; 3], size: DataSize) -> BlockRun {
    run(offset, size, 1, 0)
}

/// Where one type (Studio Set, or a tone or kit type) lives in User memory
/// and in its Temporary area.
///
/// Reference: `docs/midi/99-undocumented.md` §2, `docs/midi/04-address-map.md`
/// and the per-type `docs/midi/` pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserMemorySpec {
    /// Type name on the CLI (e.g. `sn-synth`).
    pub cli_name: &'static str,
    /// Bank Select MSB of the User bank. Slot `n` is LSB `n / 128`, PC `n % 128`.
    pub bank_msb: u8,
    /// Number of User slots.
    pub slot_count: u16,
    /// First address byte of User slot 0.
    pub user_base: u8,
    /// Distance between User slots in the second address byte.
    pub slot_stride: u8,
    /// Tone type this memory holds in a Part's Temporary Tone, or `None` for the
    /// Temporary Studio Set.
    pub tone_type: Option<ToneType>,
    /// Blocks of the Temporary layout, which User slots share.
    pub blocks: &'static [BlockRun],
    /// Names the device gives an initialized slot.
    pub init_names: &'static [&'static str],
}

impl UserMemorySpec {
    /// Address of User slot `slot` (0-indexed), or `None` past the last slot.
    pub fn user_address(&self, slot: u16) -> Option<Address> {
        let per_unit = 128 / u16::from(self.slot_stride);
        (slot < self.slot_count).then(|| {
            Address::new(
                self.user_base + (slot / per_unit) as u8,
                (slot % per_unit) as u8 * self.slot_stride,
                0,
                0,
            )
        })
    }

    /// Bank Select of User slot `slot` (0-indexed), or `None` past the last slot.
    pub fn bank_select(&self, slot: u16) -> Option<BankSelect> {
        (slot < self.slot_count).then_some(BankSelect {
            msb: self.bank_msb,
            lsb: (slot / 128) as u8,
            pc: (slot % 128) as u8,
        })
    }

    /// Whether a slot named `name` is free: it holds INIT.
    pub fn is_free(&self, name: &str) -> bool {
        self.init_names.contains(&name.trim_end())
    }

    /// Base of the Temporary area this type loads into, for Part `part`
    /// (0-indexed; ignored for the Studio Set).
    pub fn temporary_base(&self, part: u8) -> Address {
        match self.tone_type {
            Some(tone_type) => tone_type.base_address(part),
            None => address::STUDIO_SET,
        }
    }

    /// Every block of the layout rebased at `base`, as `(address, size)`.
    pub fn blocks(&self, base: Address) -> impl Iterator<Item = (Address, DataSize)> + '_ {
        self.blocks.iter().flat_map(move |r| {
            let [a, b, c] = r.offset;
            (0..r.count).map(move |i| {
                let step = u16::from(i) * u16::from(r.stride);
                let addr = base.offset([0, a, b, c]).offset([
                    0,
                    (step / 128) as u8,
                    (step % 128) as u8,
                    0,
                ]);
                (addr, r.size)
            })
        })
    }
}

const fn size(n: u8) -> DataSize {
    DataSize::new(0, 0, 0, n)
}

/// User memory of every librarian type, Studio Set first.
pub const USER_MEMORY: &[UserMemorySpec] = &[
    UserMemorySpec {
        cli_name: "studio-set",
        bank_msb: 85,
        slot_count: 64,
        user_base: 0x2F,
        slot_stride: 1,
        tone_type: None,
        blocks: &[
            one([0x00, 0x00, 0x00], size(0x54)), // Common
            one([0x00, 0x04, 0x00], size(0x54)), // Chorus
            one([0x00, 0x06, 0x00], size(0x63)), // Reverb
            one([0x00, 0x08, 0x00], size(0x10)), // Motional Surround
            one([0x00, 0x09, 0x00], params::MASTER_EQ_SIZE),
            run([0x00, 0x10, 0x00], DataSize::ONE, 16, 1), // MIDI 1–16
            run([0x00, 0x20, 0x00], size(0x4D), 16, 1),    // Parts 1–16
            run([0x00, 0x50, 0x00], params::PART_EQ_SIZE, 16, 1),
        ],
        init_names: &["INIT STUDIO"],
    },
    UserMemorySpec {
        cli_name: "sn-synth",
        bank_msb: 95,
        slot_count: 512,
        user_base: 0x50,
        slot_stride: 1,
        tone_type: Some(ToneType::SnSynth),
        blocks: &[
            one([0x00, 0x00, 0x00], sn_synth::SNS_COMMON_BLOCK_SIZE),
            one(mfx::MFX_BLOCK_OFFSET, mfx::MFX_BLOCK_SIZE),
            run(
                [0x00, 0x20, 0x00],
                sn_synth::SNS_PARTIAL_BLOCK_SIZE,
                sn_synth::SNS_PARTIAL_COUNT as u8,
                1,
            ),
        ],
        init_names: &["INIT TONE"],
    },
    UserMemorySpec {
        cli_name: "sn-acoustic",
        bank_msb: 89,
        slot_count: 256,
        user_base: 0x60,
        slot_stride: 1,
        tone_type: Some(ToneType::SnAcoustic),
        blocks: &[
            one([0x00, 0x00, 0x00], sn_acoustic::SNA_COMMON_BLOCK_SIZE),
            one(mfx::MFX_BLOCK_OFFSET, mfx::MFX_BLOCK_SIZE),
        ],
        init_names: &["INIT TONE"],
    },
    UserMemorySpec {
        cli_name: "sn-drum",
        bank_msb: 88,
        slot_count: 64,
        user_base: 0x6E,
        slot_stride: 1,
        tone_type: Some(ToneType::SnDrum),
        blocks: &[
            one([0x00, 0x00, 0x00], sn_drum::SND_COMMON_BLOCK_SIZE),
            one(mfx::MFX_BLOCK_OFFSET, mfx::MFX_BLOCK_SIZE),
            one([0x00, 0x08, 0x00], params::COMP_EQ_BLOCK_SIZE),
            run(
                [0x00, 0x10, 0x00],
                sn_drum::SND_NOTE_BLOCK_SIZE,
                sn_drum::SND_NOTE_COUNT as u8,
                1,
            ),
        ],
        init_names: &["INIT KIT", "INITIAL KIT"],
    },
    UserMemorySpec {
        cli_name: "pcm-synth",
        bank_msb: 87,
        slot_count: 256,
        user_base: 0x30,
        slot_stride: 1,
        tone_type: Some(ToneType::PcmSynth),
        blocks: &[
            one([0x00, 0x00, 0x00], pcm_synth::PCMS_COMMON_BLOCK_SIZE),
            one(mfx::MFX_BLOCK_OFFSET, mfx::MFX_BLOCK_SIZE),
            one([0x00, 0x10, 0x00], pcm_synth::PCMS_PMT_BLOCK_SIZE),
            run(
                [0x00, 0x20, 0x00],
                pcm_synth::PCMS_PARTIAL_BLOCK_SIZE,
                pcm_synth::PCMS_PARTIAL_COUNT as u8,
                2,
            ),
            one([0x00, 0x30, 0x00], pcm_synth::PCMS_COMMON2_BLOCK_SIZE),
        ],
        init_names: &["INIT TONE"],
    },
    UserMemorySpec {
        cli_name: "pcm-drum",
        bank_msb: 86,
        slot_count: 32,
        user_base: 0x40,
        slot_stride: 0x10,
        tone_type: Some(ToneType::PcmDrum),
        blocks: &[
            one([0x00, 0x00, 0x00], pcm_drum::PCMD_COMMON_BLOCK_SIZE),
            one(mfx::MFX_BLOCK_OFFSET, mfx::MFX_BLOCK_SIZE),
            one([0x00, 0x08, 0x00], params::COMP_EQ_BLOCK_SIZE),
            run(
                [0x00, 0x10, 0x00],
                pcm_drum::PCMD_PARTIAL_BLOCK_SIZE,
                pcm_drum::PCMD_KEY_COUNT as u8,
                2,
            ),
            one([0x02, 0x00, 0x00], pcm_drum::PCMD_COMMON2_BLOCK_SIZE),
        ],
        init_names: &["INIT KIT", "INIT RHYTHM"],
    },
];

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
    output_assign_names: &["A", "B", "C", "D"],
    surround_room_types: &["Room 1", "Room 2", "Hall 1", "Hall 2"],
    surround_room_sizes: &["Small", "Medium", "Large"],
    comp_attack_times: &[
        "0.05", "0.06", "0.07", "0.08", "0.09", "0.10", "0.12", "0.14", "0.16", "0.18", "0.20",
        "0.25", "0.30", "0.35", "0.40", "0.50", "0.60", "0.70", "0.80", "0.90", "1.0", "2.0",
        "4.0", "6.0", "8.0", "10", "15", "20", "25", "30", "40", "50",
    ],
    comp_release_times: &[
        "0.05", "0.07", "0.10", "0.50", "1", "5", "10", "17", "25", "50", "75", "100", "150",
        "200", "300", "400", "500", "600", "700", "800", "900", "1000", "1200", "2000",
    ],
    comp_ratios: &[
        "1:1", "1.5:1", "2:1", "3:1", "4:1", "5:1", "6:1", "7:1", "8:1", "9:1", "10:1", "12:1",
        "14:1", "16:1", "20:1", "24:1", "30:1", "40:1", "100:1", "inf:1",
    ],
    sn_synth_partial_count: sn_synth::SNS_PARTIAL_COUNT as u8,
    pcm_synth_partial_count: pcm_synth::PCMS_PARTIAL_COUNT as u8,
    mfx_control_count: mfx::MFX_CONTROL_COUNT as u8,
    pcm_drum_wmt_count: pcm_drum::PCMD_WMT_COUNT as u8,
    sn_drum_keys: sn_drum::SND_KEYS,
    sn_drum_inst_number_max: sn_drum::SND_INST_NUMBER_MAX,
    pcm_drum_keys: pcm_drum::PCMD_KEYS,
    tone_name_length: params::TONE_NAME_SIZE.value(),
    studio_set_name_length: params::STUDIO_SET_NAME_SIZE.value(),
    user_memory: USER_MEMORY,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integra7_name() {
        assert_eq!(INTEGRA7.name, "INTEGRA-7");
    }

    #[test]
    fn integra7_tone_counts_follow_the_midi_implementation() {
        // SN-S Partials 1-3, PCM Synth Partials 1-4, MFX Control 1-4,
        // PCM Drum Kit WMT1-4.
        assert_eq!(INTEGRA7.sn_synth_partial_count, 3);
        assert_eq!(INTEGRA7.pcm_synth_partial_count, 4);
        assert_eq!(INTEGRA7.mfx_control_count, 4);
        assert_eq!(INTEGRA7.pcm_drum_wmt_count, 4);
    }

    #[test]
    fn integra7_sn_drum_inst_numbers_follow_the_midi_implementation() {
        // SN Drum Kit Note Inst Number: 0-512.
        assert_eq!(INTEGRA7.sn_drum_inst_number_max, 512);
    }

    #[test]
    fn integra7_drum_key_ranges_follow_the_midi_implementation() {
        // SN Drum Kit Note: key 27-88; PCM Drum Kit Partial: key 21-108.
        assert_eq!(
            INTEGRA7.sn_drum_keys,
            KeyRange {
                first: 27,
                last: 88
            }
        );
        assert_eq!(
            INTEGRA7.pcm_drum_keys,
            KeyRange {
                first: 21,
                last: 108
            }
        );
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
    fn integra7_comp_tables() {
        // Value ranges from the Drum Comp+EQ block: attack 0-31, release 0-23, ratio 0-19.
        assert_eq!(INTEGRA7.comp_attack_times.len(), 32);
        assert_eq!(INTEGRA7.comp_release_times.len(), 24);
        assert_eq!(INTEGRA7.comp_ratios.len(), 20);
        assert_eq!(INTEGRA7.comp_ratios[19], "inf:1");
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
    fn integra7_output_assign_names() {
        assert_eq!(INTEGRA7.output_assign_names.len(), 4);
        assert_eq!(INTEGRA7.output_assign_names[0], "A");
        assert_eq!(INTEGRA7.output_assign_names[3], "D");
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

    #[test]
    fn integra7_user_studio_set_count() {
        assert_eq!(INTEGRA7.user_studio_set_count(), 64);
    }

    fn spec(cli_name: &str) -> &'static UserMemorySpec {
        INTEGRA7
            .user_memory
            .iter()
            .find(|t| t.cli_name == cli_name)
            .unwrap()
    }

    /// First, a middle and the last slot of each type, against
    /// `docs/midi/99-undocumented.md` §2.
    #[test]
    fn user_addresses_match_doc_table() {
        for (name, cases) in [
            (
                "studio-set",
                [(0, [0x2F, 0x00]), (31, [0x2F, 0x1F]), (63, [0x2F, 0x3F])],
            ),
            (
                "pcm-synth",
                [(0, [0x30, 0x00]), (128, [0x31, 0x00]), (255, [0x31, 0x7F])],
            ),
            (
                "pcm-drum",
                [(0, [0x40, 0x00]), (9, [0x41, 0x10]), (31, [0x43, 0x70])],
            ),
            (
                "sn-synth",
                [(0, [0x50, 0x00]), (129, [0x51, 0x01]), (511, [0x53, 0x7F])],
            ),
            (
                "sn-acoustic",
                [(0, [0x60, 0x00]), (130, [0x61, 0x02]), (255, [0x61, 0x7F])],
            ),
            (
                "sn-drum",
                [(0, [0x6E, 0x00]), (32, [0x6E, 0x20]), (63, [0x6E, 0x3F])],
            ),
        ] {
            let t = spec(name);
            for (slot, [a, b]) in cases {
                assert_eq!(
                    t.user_address(slot),
                    Some(Address::new(a, b, 0, 0)),
                    "{name} {slot}"
                );
            }
            assert_eq!(t.user_address(t.slot_count), None, "{name}");
        }
    }

    #[test]
    fn user_banks_match_tone_banks() {
        assert_eq!(INTEGRA7.user_memory.len(), 6);
        for t in INTEGRA7.user_memory {
            let last = t.bank_select(t.slot_count - 1).unwrap();
            let lsbs: Vec<u8> = (0..=last.lsb).collect();
            match crate::tone_banks::user_bank_for(t.bank_msb) {
                Some(bank) => assert_eq!((bank.msb, bank.lsbs), (t.bank_msb, &lsbs[..])),
                None => assert_eq!(
                    (t.cli_name, t.bank_msb, &lsbs[..]),
                    ("studio-set", 85, &[0][..])
                ),
            }
            assert_eq!(t.bank_select(t.slot_count), None);
        }
        let ss = spec("studio-set").bank_select(63).unwrap();
        assert_eq!((ss.msb, ss.lsb, ss.pc), (0x55, 0x00, 0x3F));
    }

    /// Block count and total bytes per type, summed from the `docs/midi`
    /// "Total size" of each block.
    #[test]
    fn block_sizes_match_doc_totals() {
        for (name, count, total) in [
            // Common 54 + Chorus 54 + Reverb 63 + Surround 10 + Master EQ 07
            // + 16 × MIDI 01 + 16 × Part 4D + 16 × Part EQ 08
            (
                "studio-set",
                53,
                84 + 84 + 99 + 16 + 7 + 16 + 16 * 77 + 16 * 8,
            ),
            ("sn-synth", 5, 64 + 145 + 3 * 61),
            ("sn-acoustic", 2, 70 + 145),
            ("sn-drum", 65, 20 + 145 + 84 + 62 * 19),
            ("pcm-synth", 8, 80 + 145 + 41 + 4 * 154 + 60),
            ("pcm-drum", 92, 18 + 145 + 84 + 88 * 195 + 50),
        ] {
            let blocks: Vec<_> = spec(name).blocks(Address::new(0, 0, 0, 0)).collect();
            let sum: u32 = blocks.iter().map(|(_, s)| s.value()).sum();
            assert_eq!((blocks.len(), sum), (count, total), "{name}");
        }
    }

    #[test]
    fn blocks_rebase_on_slot_and_temporary_area() {
        let sns = spec("sn-synth");
        let blocks: Vec<_> = sns.blocks(sns.user_address(0).unwrap()).collect();
        assert_eq!(
            blocks[1],
            (
                Address::new(0x50, 0x00, 0x02, 0x00),
                DataSize::new(0, 0, 1, 0x11)
            )
        );
        assert_eq!(blocks[4].0, Address::new(0x50, 0x00, 0x22, 0x00));
        assert_eq!(sns.temporary_base(2), Address::new(0x19, 0x41, 0x00, 0x00));

        let pcmd = spec("pcm-drum");
        let blocks: Vec<_> = pcmd.blocks(pcmd.user_address(1).unwrap()).collect();
        assert_eq!(blocks[3].0, Address::new(0x40, 0x10, 0x10, 0x00)); // key 21
        assert_eq!(blocks[90].0, Address::new(0x40, 0x11, 0x3E, 0x00)); // key 108
        assert_eq!(blocks[91].0, Address::new(0x40, 0x12, 0x00, 0x00)); // Common 2
        assert_eq!(pcmd.temporary_base(0), Address::new(0x19, 0x10, 0x00, 0x00));

        let ss = spec("studio-set");
        let last = ss.blocks(ss.temporary_base(5)).last().unwrap();
        assert_eq!(
            last,
            (Address::new(0x18, 0x00, 0x5F, 0x00), params::PART_EQ_SIZE)
        );
    }
}
