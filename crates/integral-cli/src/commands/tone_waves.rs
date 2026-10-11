//! `read tone-waves`: list every Wave a Part's temporary tone uses.
//!
//! Tone blocks are read and parsed by `integral-core` (`pcm_synth`,
//! `pcm_drum`, `sn_synth`) and named through the Wave catalog
//! ([`integral_core::waves`]); this module only reads and formats.

use anyhow::{Context, Result};
use integral_core::device_spec::INTEGRA7;
use integral_core::params::{self, ToneType};
use integral_core::pcm_drum::{self, PcmDrumPartial};
use integral_core::pcm_synth::{self, PcmSynthPartial};
use integral_core::sn_synth::{self, OSC_WAVE_PCM, SnSynthPartial};
use integral_core::waves::{self, WaveGroup};

use super::wave::format_number;
use crate::device::IntegraDevice;

/// Name printed for Wave Number 0.
const OFF: &str = "OFF";

/// `<group> <number> <name>` for a Wave picked by Group Type/ID and Number;
/// `OFF` for Wave Number 0, raw numbers for an unknown group.
fn wave_label(group: Option<WaveGroup>, raw: (u8, u16), number: u16) -> String {
    if number == 0 {
        return OFF.to_string();
    }
    match group {
        Some(g) => {
            let name = waves::name(g, number).unwrap_or("?");
            format!("{} {} {name}", g.id(), format_number(number))
        }
        None => format!(
            "group-type={} group-id={} {}",
            raw.0,
            raw.1,
            format_number(number)
        ),
    }
}

/// The Wave Group selected by a partial's or WMT's Group Type and ID.
fn group_of(group_type: u8, group_id: u16) -> Option<WaveGroup> {
    WaveGroup::from_params(group_type, u8::try_from(group_id).ok()?)
}

/// The L and R lines of one Wave pick, each prefixed by `slot`.
fn stereo_lines(slot: &str, group_type: u8, group_id: u16, l: u16, r: u16) -> [String; 2] {
    let group = group_of(group_type, group_id);
    let raw = (group_type, group_id);
    [
        format!("{slot} L {}", wave_label(group, raw, l)),
        format!("{slot} R {}", wave_label(group, raw, r)),
    ]
}

/// PCM Synth lines: partials 1–4, L then R.
pub fn pcm_synth_lines(partials: &[PcmSynthPartial]) -> Vec<String> {
    partials
        .iter()
        .enumerate()
        .flat_map(|(i, p)| {
            stereo_lines(
                &format!("P{}", i + 1),
                p.wave_group_type,
                p.wave_group_id,
                p.wave_number_l,
                p.wave_number_r,
            )
        })
        .collect()
}

/// PCM Drum lines: for each `(key, partial)`, every switched-on WMT, L then R.
pub fn pcm_drum_lines(keys: &[(u8, PcmDrumPartial)]) -> Vec<String> {
    keys.iter()
        .flat_map(|(key, partial)| {
            partial
                .wmt
                .iter()
                .enumerate()
                .filter(|(_, w)| w.switch != 0)
                .flat_map(move |(i, w)| {
                    stereo_lines(
                        &format!("{key} WMT{}", i + 1),
                        w.wave_group_type,
                        w.wave_group_id,
                        w.wave_number_l,
                        w.wave_number_r,
                    )
                })
        })
        .collect()
}

/// Line printed for an SN Synth tone with no OSC in PCM mode.
const NO_PCM_OSC: &str = "no OSC uses a PCM Wave";

/// SN Synth lines: OSCs 1–3 whose OSC Wave is PCM, or [`NO_PCM_OSC`].
pub fn sn_synth_lines(partials: &[SnSynthPartial]) -> Vec<String> {
    let group = Some(WaveGroup::SnSynth);
    let lines: Vec<String> = partials
        .iter()
        .enumerate()
        .filter(|(_, p)| p.osc_wave == OSC_WAVE_PCM)
        .map(|(i, p)| format!("OSC{} {}", i + 1, wave_label(group, (0, 0), p.wave_number)))
        .collect();
    if lines.is_empty() {
        vec![NO_PCM_OSC.to_string()]
    } else {
        lines
    }
}

/// The line printed for a tone type without Waves.
pub fn no_waves_line(tone: ToneType) -> String {
    let name = INTEGRA7
        .user_memory
        .iter()
        .find(|m| m.tone_type == Some(tone))
        .map_or("this", |m| m.cli_name);
    format!("{name} tone has no Waves")
}

/// Read Part `part` (0-based)'s temporary tone and return its Wave lines.
pub fn read(dev: &mut IntegraDevice, part: u8) -> Result<Vec<String>> {
    let bank_addr = params::part_address(part, params::part::TONE_BANK_MSB);
    let msb = dev.request_data(&bank_addr, &params::SINGLE_BYTE_SIZE)?[0];
    let tone = params::tone_type_from_bank_msb(msb)
        .with_context(|| format!("unknown tone type for bank MSB {msb}"))?;

    match tone {
        ToneType::PcmSynth => {
            let partials = (0..pcm_synth::PCMS_PARTIAL_COUNT as u8)
                .map(|i| {
                    let addr = pcm_synth::pcms_partial_address(part, i);
                    let data = dev.request_data(&addr, &pcm_synth::PCMS_PARTIAL_BLOCK_SIZE)?;
                    Ok(pcm_synth::parse_pcms_partial(&data)?)
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(pcm_synth_lines(&partials))
        }
        ToneType::PcmDrum => {
            let range = INTEGRA7.pcm_drum_keys;
            let keys = (range.first..=range.last)
                .map(|key| {
                    let addr = pcm_drum::pcmd_partial_address(part, key)?;
                    let data = dev.request_data(&addr, &pcm_drum::PCMD_PARTIAL_BLOCK_SIZE)?;
                    Ok((key, pcm_drum::parse_pcmd_partial(&data)?))
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(pcm_drum_lines(&keys))
        }
        ToneType::SnSynth => {
            let partials = (0..sn_synth::SNS_PARTIAL_COUNT as u8)
                .map(|i| {
                    let addr = sn_synth::sns_partial_address(part, i);
                    let data = dev.request_data(&addr, &sn_synth::SNS_PARTIAL_BLOCK_SIZE)?;
                    Ok(sn_synth::parse_sns_partial(&data)?)
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(sn_synth_lines(&partials))
        }
        other => Ok(vec![no_waves_line(other)]),
    }
}

#[cfg(test)]
mod tests {
    use integral_core::pcm_drum::PcmDrumWmt;

    use super::*;

    fn pcm_partial(group: (u8, u16), l: u16, r: u16) -> PcmSynthPartial {
        PcmSynthPartial {
            wave_group_type: group.0,
            wave_group_id: group.1,
            wave_number_l: l,
            wave_number_r: r,
            ..Default::default()
        }
    }

    #[test]
    fn pcm_synth_lists_four_partials_l_and_r() {
        let partials = [
            pcm_partial((0, 0), 1, 2),
            pcm_partial((1, 7), 1, 0),
            pcm_partial((0, 0), 0, 0),
            pcm_partial((2, 5), 12, 13),
        ];
        assert_eq!(
            pcm_synth_lines(&partials),
            [
                "P1 L INT 0001 StGrand pA L",
                "P1 R INT 0002 StGrand pA R",
                "P2 L SRX-07 0001 StGrand L pA",
                "P2 R OFF",
                "P3 L OFF",
                "P3 R OFF",
                "P4 L group-type=2 group-id=5 0012",
                "P4 R group-type=2 group-id=5 0013",
            ]
        );
    }

    #[test]
    fn unknown_wave_number_has_no_name() {
        let past_end = waves::waves(WaveGroup::Srx(2)).len() as u16 + 1;
        let lines = pcm_synth_lines(&[pcm_partial((1, 2), past_end, 0)]);
        assert_eq!(
            lines[0],
            format!("P1 L SRX-02 {} ?", format_number(past_end))
        );
    }

    #[test]
    fn pcm_drum_lists_switched_on_wmts_per_key() {
        let wmt = |switch, l, r| PcmDrumWmt {
            switch,
            wave_number_l: l,
            wave_number_r: r,
            ..Default::default()
        };
        let kick = PcmDrumPartial {
            wmt: vec![wmt(1, 1, 2), wmt(0, 3, 4), wmt(1, 5, 0), wmt(0, 0, 0)],
            ..Default::default()
        };
        let silent = PcmDrumPartial {
            wmt: vec![wmt(0, 1, 2); 4],
            ..Default::default()
        };
        assert_eq!(
            pcm_drum_lines(&[(36, kick), (37, silent)]),
            [
                "36 WMT1 L INT 0001 StGrand pA L",
                "36 WMT1 R INT 0002 StGrand pA R",
                "36 WMT3 L INT 0005 StGrand pC L",
                "36 WMT3 R OFF",
            ]
        );
    }

    #[test]
    fn sn_synth_lists_pcm_oscs_only() {
        let osc = |osc_wave, wave_number| SnSynthPartial {
            osc_wave,
            wave_number,
            ..Default::default()
        };
        let first = waves::name(WaveGroup::SnSynth, 1).unwrap();
        assert_eq!(
            sn_synth_lines(&[osc(0, 1), osc(OSC_WAVE_PCM, 1), osc(OSC_WAVE_PCM, 0)]),
            [
                format!("OSC2 SN-Synth 0001 {first}"),
                "OSC3 OFF".to_string()
            ]
        );
        assert_eq!(sn_synth_lines(&[osc(0, 1), osc(6, 0)]), [NO_PCM_OSC]);
    }

    #[test]
    fn other_tone_types_have_no_waves() {
        assert_eq!(
            no_waves_line(ToneType::SnAcoustic),
            "sn-acoustic tone has no Waves"
        );
        assert_eq!(no_waves_line(ToneType::SnDrum), "sn-drum tone has no Waves");
    }
}
