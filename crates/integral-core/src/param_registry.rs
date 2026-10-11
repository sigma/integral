//! Parameter registry — definitions, formatting, and lookup for all
//! controllable parameters on the Integra-7.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::sync::{Mutex, OnceLock};

use crate::device_spec::{INTEGRA7, MIDI_CHANNELS};

/// How a parameter value should be displayed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ParamFormat {
    /// Display value as-is.
    Raw,
    /// Display as value minus center (e.g. pan 64 -> "0", 0 -> "-64", 127 -> "+63").
    SignedCenter(i32),
    /// Index into a static name array.
    Named(&'static [&'static str]),
    /// Boolean: 0=OFF, 1=ON.
    OnOff,
}

/// Logical grouping for a parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ParamGroup {
    /// Per-part parameter (0-based part index).
    Part(u8),
    /// Master output.
    Master,
    /// External input.
    Ext,
    /// Chorus effect.
    Chorus,
    /// Reverb effect.
    Reverb,
    /// Mixer 3-band EQ, shared by parts, master and Drum Comp+EQ units.
    Eq,
    /// Global / top-level.
    Global,
    /// Tone-level common parameters.
    ToneCommon,
    /// Tone-level per-partial parameters (partial index 0-based).
    TonePartial(u8),
}

/// A single parameter definition.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ParamDef {
    /// Dot-separated identifier, e.g. `"part.0.level"`.
    pub id: &'static str,
    /// Human-readable display name.
    pub name: &'static str,
    /// Which group this parameter belongs to.
    pub group: ParamGroup,
    /// Minimum raw value (inclusive).
    pub min: i32,
    /// Maximum raw value (inclusive).
    pub max: i32,
    /// Default raw value.
    #[cfg_attr(feature = "serde", serde(rename = "default_value"))]
    pub default: i32,
    /// Display format.
    pub format: ParamFormat,
}

/// Format a raw parameter value according to the parameter's [`ParamFormat`].
///
/// Returns the formatted string. Out-of-range named indices fall back to the
/// raw numeric representation.
pub fn format_value(def: &ParamDef, value: i32) -> String {
    match &def.format {
        ParamFormat::Raw => value.to_string(),
        ParamFormat::SignedCenter(center) => {
            let offset = value - center;
            if offset > 0 {
                let mut s = String::new();
                let _ = write!(s, "+{offset}");
                s
            } else {
                offset.to_string()
            }
        }
        ParamFormat::Named(names) => {
            let idx = value as usize;
            if idx < names.len() {
                names[idx].to_string()
            } else {
                value.to_string()
            }
        }
        ParamFormat::OnOff => {
            if value == 0 {
                "OFF".to_string()
            } else {
                "ON".to_string()
            }
        }
    }
}

/// Per-part parameter id suffixes, in registry order.
const PART_PARAM_SUFFIXES: [&str; 7] = [
    "level",
    "pan",
    "mute",
    "chorus_send",
    "reverb_send",
    "output_assign",
    "receive_channel",
];

/// `part.<i>.<suffix>` ids for a given part count, one row per part.
type PartIds = Vec<[&'static str; 7]>;

/// Interns the part ids per part count, so `ParamDef` can borrow
/// `'static` strings while each count is leaked at most once.
fn part_ids(part_count: u8) -> &'static PartIds {
    static CACHE: Mutex<BTreeMap<u8, &'static PartIds>> = Mutex::new(BTreeMap::new());
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    cache.entry(part_count).or_insert_with(|| {
        Box::leak(Box::new(
            (0..part_count)
                .map(|i| {
                    PART_PARAM_SUFFIXES
                        .map(|suffix| &*Box::leak(format!("part.{i}.{suffix}").into_boxed_str()))
                })
                .collect(),
        ))
    })
}

/// Receive-channel labels `"1"..="16"`, one per [`MIDI_CHANNELS`].
fn channel_names() -> &'static [&'static str] {
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| {
        (1..=MIDI_CHANNELS)
            .map(|n| &*Box::leak(n.to_string().into_boxed_str()))
            .collect()
    })
}

/// Generate all mixer parameter definitions.
///
/// This includes per-part parameters ([`DeviceSpec::part_count`] parts x 7
/// params each), master, external input, chorus, reverb, the solo-part
/// selector and the shared mixer EQ bands.
///
/// [`DeviceSpec::part_count`]: crate::device_spec::DeviceSpec::part_count
pub fn mixer_params() -> Vec<ParamDef> {
    mixer_params_for(INTEGRA7.part_count)
}

/// [`mixer_params`] for an arbitrary part count.
fn mixer_params_for(part_count: u8) -> Vec<ParamDef> {
    // 7 per part + 1 master + 2 ext + 4 chorus + 4 reverb + 1 solo + 7 EQ
    let mut params = Vec::with_capacity(usize::from(part_count) * 7 + 19);

    let spec = &INTEGRA7;
    let last = |names: &[&str]| names.len() as i32 - 1;
    let channel_names = channel_names();

    for (i, ids) in (0u8..).zip(part_ids(part_count)) {
        let [
            level_id,
            pan_id,
            mute_id,
            chorus_send_id,
            reverb_send_id,
            output_assign_id,
            receive_channel_id,
        ] = *ids;
        params.push(ParamDef {
            id: level_id,
            name: "Level",
            group: ParamGroup::Part(i),
            min: 0,
            max: 127,
            default: 100,
            format: ParamFormat::Raw,
        });
        params.push(ParamDef {
            id: pan_id,
            name: "Pan",
            group: ParamGroup::Part(i),
            min: 0,
            max: 127,
            default: 64,
            format: ParamFormat::SignedCenter(64),
        });
        params.push(ParamDef {
            id: mute_id,
            name: "Mute",
            group: ParamGroup::Part(i),
            min: 0,
            max: 1,
            default: 0,
            format: ParamFormat::OnOff,
        });
        params.push(ParamDef {
            id: chorus_send_id,
            name: "FX1",
            group: ParamGroup::Part(i),
            min: 0,
            max: 127,
            default: 0,
            format: ParamFormat::Raw,
        });
        params.push(ParamDef {
            id: reverb_send_id,
            name: "FX2",
            group: ParamGroup::Part(i),
            min: 0,
            max: 127,
            default: 0,
            format: ParamFormat::Raw,
        });
        params.push(ParamDef {
            id: output_assign_id,
            name: "Output Assign",
            group: ParamGroup::Part(i),
            min: 0,
            max: last(spec.output_assigns),
            default: 0,
            format: ParamFormat::Named(spec.output_assigns),
        });
        params.push(ParamDef {
            id: receive_channel_id,
            name: "Receive Channel",
            group: ParamGroup::Part(i),
            min: 0,
            max: last(channel_names),
            default: i32::from(i),
            format: ParamFormat::Named(channel_names),
        });
    }

    // Master
    params.push(ParamDef {
        id: "master.level",
        name: "Master Level",
        group: ParamGroup::Master,
        min: 0,
        max: 127,
        default: 100,
        format: ParamFormat::Raw,
    });

    // Ext
    params.push(ParamDef {
        id: "ext.level",
        name: "Ext Level",
        group: ParamGroup::Ext,
        min: 0,
        max: 127,
        default: 100,
        format: ParamFormat::Raw,
    });
    params.push(ParamDef {
        id: "ext.mute",
        name: "Ext Mute",
        group: ParamGroup::Ext,
        min: 0,
        max: 1,
        default: 0,
        format: ParamFormat::OnOff,
    });

    // Chorus
    params.push(ParamDef {
        id: "chorus.enabled",
        name: "Chorus",
        group: ParamGroup::Chorus,
        min: 0,
        max: 1,
        default: 1,
        format: ParamFormat::OnOff,
    });
    params.push(ParamDef {
        id: "chorus.type",
        name: "Chorus Type",
        group: ParamGroup::Chorus,
        min: 0,
        max: last(spec.chorus_type_names),
        default: 0,
        format: ParamFormat::Named(spec.chorus_type_names),
    });
    params.push(ParamDef {
        id: "chorus.level",
        name: "Chorus Level",
        group: ParamGroup::Chorus,
        min: 0,
        max: 127,
        default: 0,
        format: ParamFormat::Raw,
    });
    params.push(ParamDef {
        id: "chorus.output",
        name: "Chorus Output",
        group: ParamGroup::Chorus,
        min: 0,
        max: last(spec.chorus_output_names),
        default: 0,
        format: ParamFormat::Named(spec.chorus_output_names),
    });

    // Reverb
    params.push(ParamDef {
        id: "reverb.enabled",
        name: "Reverb",
        group: ParamGroup::Reverb,
        min: 0,
        max: 1,
        default: 1,
        format: ParamFormat::OnOff,
    });
    params.push(ParamDef {
        id: "reverb.type",
        name: "Reverb Type",
        group: ParamGroup::Reverb,
        min: 0,
        max: last(spec.reverb_type_names),
        default: 0,
        format: ParamFormat::Named(spec.reverb_type_names),
    });
    params.push(ParamDef {
        id: "reverb.level",
        name: "Reverb Level",
        group: ParamGroup::Reverb,
        min: 0,
        max: 127,
        default: 0,
        format: ParamFormat::Raw,
    });
    params.push(ParamDef {
        id: "reverb.output",
        name: "Reverb Output",
        group: ParamGroup::Reverb,
        min: 0,
        max: last(spec.reverb_output_names),
        default: 0,
        format: ParamFormat::Named(spec.reverb_output_names),
    });

    // Solo part
    params.push(ParamDef {
        id: "solo_part",
        name: "Solo Part",
        group: ParamGroup::Global,
        min: 0,
        max: i32::from(part_count),
        default: 0,
        format: ParamFormat::Raw,
    });

    // Mixer EQ: part, master and Drum Comp+EQ units share these ranges.
    static EQ_LOW_FREQ_NAMES: &[&str] = &["200", "400"];
    static EQ_MID_FREQ_NAMES: &[&str] = &[
        "200", "250", "315", "400", "500", "630", "800", "1.0k", "1.2k", "1.6k", "2.0k", "2.5k",
        "3.1k", "4.0k", "5.0k", "6.3k", "8.0k",
    ];
    static EQ_HIGH_FREQ_NAMES: &[&str] = &["2.0k", "4.0k", "8.0k"];
    static EQ_Q_NAMES: &[&str] = &["0.5", "1.0", "2.0", "4.0", "8.0"];

    let eq = |id, name, max, default, format| ParamDef {
        id,
        name,
        group: ParamGroup::Eq,
        min: 0,
        max,
        default,
        format,
    };
    let gain = || ParamFormat::SignedCenter(15);
    params.push(eq(
        "eq.low_freq",
        "Low Freq",
        1,
        1,
        ParamFormat::Named(EQ_LOW_FREQ_NAMES),
    ));
    params.push(eq("eq.low_gain", "Low Gain", 30, 15, gain()));
    params.push(eq(
        "eq.mid_freq",
        "Mid Freq",
        16,
        7,
        ParamFormat::Named(EQ_MID_FREQ_NAMES),
    ));
    params.push(eq("eq.mid_gain", "Mid Gain", 30, 15, gain()));
    params.push(eq(
        "eq.mid_q",
        "Mid Q",
        4,
        0,
        ParamFormat::Named(EQ_Q_NAMES),
    ));
    params.push(eq(
        "eq.high_freq",
        "High Freq",
        2,
        1,
        ParamFormat::Named(EQ_HIGH_FREQ_NAMES),
    ));
    params.push(eq("eq.high_gain", "High Gain", 30, 15, gain()));

    params
}

/// Generate all SuperNATURAL Synth Tone parameter definitions.
///
/// This includes 13 common parameters and 3 partials x 8 parameters each = 37 total.
pub fn sn_synth_params() -> Vec<ParamDef> {
    let mut params = Vec::with_capacity(37);

    static OSC_WAVE_NAMES: &[&str] = &[
        "SAW", "SQR", "PW-SQR", "TRI", "SINE", "NOISE", "SP-SAW", "PCM",
    ];
    static FILTER_MODE_NAMES: &[&str] =
        &["BYP", "LPF", "HPF", "BPF", "PKG", "LPF2", "LPF3", "LPF4"];
    static RING_NAMES: &[&str] = &["OFF", "ON1", "ON2"];
    static UNISON_SIZE_NAMES: &[&str] = &["2", "4", "6", "8"];

    // Common parameters
    params.push(ParamDef {
        id: "sns.common.tone_level",
        name: "Level",
        group: ParamGroup::ToneCommon,
        min: 0,
        max: 127,
        default: 127,
        format: ParamFormat::Raw,
    });
    params.push(ParamDef {
        id: "sns.common.portamento_switch",
        name: "Porta",
        group: ParamGroup::ToneCommon,
        min: 0,
        max: 1,
        default: 0,
        format: ParamFormat::OnOff,
    });
    params.push(ParamDef {
        id: "sns.common.portamento_time",
        name: "Porta Time",
        group: ParamGroup::ToneCommon,
        min: 0,
        max: 127,
        default: 0,
        format: ParamFormat::Raw,
    });
    params.push(ParamDef {
        id: "sns.common.mono_switch",
        name: "Mono",
        group: ParamGroup::ToneCommon,
        min: 0,
        max: 1,
        default: 0,
        format: ParamFormat::OnOff,
    });
    params.push(ParamDef {
        id: "sns.common.octave_shift",
        name: "Oct Shift",
        group: ParamGroup::ToneCommon,
        min: 61,
        max: 67,
        default: 64,
        format: ParamFormat::SignedCenter(64),
    });
    params.push(ParamDef {
        id: "sns.common.pitch_bend_range_up",
        name: "Bend Up",
        group: ParamGroup::ToneCommon,
        min: 0,
        max: 24,
        default: 2,
        format: ParamFormat::Raw,
    });
    params.push(ParamDef {
        id: "sns.common.pitch_bend_range_down",
        name: "Bend Down",
        group: ParamGroup::ToneCommon,
        min: 0,
        max: 24,
        default: 2,
        format: ParamFormat::Raw,
    });
    params.push(ParamDef {
        id: "sns.common.unison_switch",
        name: "Unison",
        group: ParamGroup::ToneCommon,
        min: 0,
        max: 1,
        default: 0,
        format: ParamFormat::OnOff,
    });
    params.push(ParamDef {
        id: "sns.common.unison_size",
        name: "Uni Size",
        group: ParamGroup::ToneCommon,
        min: 0,
        max: 3,
        default: 0,
        format: ParamFormat::Named(UNISON_SIZE_NAMES),
    });
    params.push(ParamDef {
        id: "sns.common.legato_switch",
        name: "Legato",
        group: ParamGroup::ToneCommon,
        min: 0,
        max: 1,
        default: 0,
        format: ParamFormat::OnOff,
    });
    params.push(ParamDef {
        id: "sns.common.analog_feel",
        name: "Analog",
        group: ParamGroup::ToneCommon,
        min: 0,
        max: 127,
        default: 0,
        format: ParamFormat::Raw,
    });
    params.push(ParamDef {
        id: "sns.common.wave_shape",
        name: "Wave Shape",
        group: ParamGroup::ToneCommon,
        min: 0,
        max: 127,
        default: 0,
        format: ParamFormat::Raw,
    });
    params.push(ParamDef {
        id: "sns.common.ring_switch",
        name: "Ring",
        group: ParamGroup::ToneCommon,
        min: 0,
        max: 2,
        default: 0,
        format: ParamFormat::Named(RING_NAMES),
    });

    // Per-partial parameters (3 partials)
    macro_rules! partial_ids {
        ($suffix:expr) => {
            [
                concat!("sns.partial.0.", $suffix),
                concat!("sns.partial.1.", $suffix),
                concat!("sns.partial.2.", $suffix),
            ]
        };
    }

    static OSC_WAVE_IDS: [&str; 3] = partial_ids!("osc_wave");
    static OSC_PITCH_IDS: [&str; 3] = partial_ids!("osc_pitch");
    static OSC_DETUNE_IDS: [&str; 3] = partial_ids!("osc_detune");
    static FILTER_MODE_IDS: [&str; 3] = partial_ids!("filter_mode");
    static FILTER_CUTOFF_IDS: [&str; 3] = partial_ids!("filter_cutoff");
    static FILTER_RESONANCE_IDS: [&str; 3] = partial_ids!("filter_resonance");
    static AMP_LEVEL_IDS: [&str; 3] = partial_ids!("amp_level");
    static AMP_PAN_IDS: [&str; 3] = partial_ids!("amp_pan");

    for n in 0u8..3 {
        let idx = n as usize;
        params.push(ParamDef {
            id: OSC_WAVE_IDS[idx],
            name: "Wave",
            group: ParamGroup::TonePartial(n),
            min: 0,
            max: 7,
            default: 0,
            format: ParamFormat::Named(OSC_WAVE_NAMES),
        });
        params.push(ParamDef {
            id: OSC_PITCH_IDS[idx],
            name: "Pitch",
            group: ParamGroup::TonePartial(n),
            min: 40,
            max: 88,
            default: 64,
            format: ParamFormat::SignedCenter(64),
        });
        params.push(ParamDef {
            id: OSC_DETUNE_IDS[idx],
            name: "Detune",
            group: ParamGroup::TonePartial(n),
            min: 14,
            max: 114,
            default: 64,
            format: ParamFormat::SignedCenter(64),
        });
        params.push(ParamDef {
            id: FILTER_MODE_IDS[idx],
            name: "Flt Mode",
            group: ParamGroup::TonePartial(n),
            min: 0,
            max: 7,
            default: 0,
            format: ParamFormat::Named(FILTER_MODE_NAMES),
        });
        params.push(ParamDef {
            id: FILTER_CUTOFF_IDS[idx],
            name: "Cutoff",
            group: ParamGroup::TonePartial(n),
            min: 0,
            max: 127,
            default: 127,
            format: ParamFormat::Raw,
        });
        params.push(ParamDef {
            id: FILTER_RESONANCE_IDS[idx],
            name: "Reso",
            group: ParamGroup::TonePartial(n),
            min: 0,
            max: 127,
            default: 0,
            format: ParamFormat::Raw,
        });
        params.push(ParamDef {
            id: AMP_LEVEL_IDS[idx],
            name: "Level",
            group: ParamGroup::TonePartial(n),
            min: 0,
            max: 127,
            default: 127,
            format: ParamFormat::Raw,
        });
        params.push(ParamDef {
            id: AMP_PAN_IDS[idx],
            name: "Pan",
            group: ParamGroup::TonePartial(n),
            min: 0,
            max: 127,
            default: 64,
            format: ParamFormat::SignedCenter(64),
        });
    }

    params
}

/// Look up a parameter definition by its string id.
pub fn lookup<'a>(params: &'a [ParamDef], id: &str) -> Option<&'a ParamDef> {
    params.iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixer_param_count() {
        let params = mixer_params();
        // 16 parts * 7 params + 1 master + 2 ext + 4 chorus + 4 reverb + 1 solo + 7 EQ = 131
        assert_eq!(params.len(), 131);
    }

    #[test]
    fn part_count_drives_mixer_params() {
        let params = mixer_params_for(4);
        assert_eq!(params.len(), 4 * 7 + 19);
        assert!(lookup(&params, "part.3.level").is_some());
        assert!(lookup(&params, "part.4.level").is_none());
        let rx = lookup(&params, "part.3.receive_channel").unwrap();
        // Receive channels follow MIDI, not the part count.
        assert_eq!((rx.max, rx.default), (i32::from(MIDI_CHANNELS) - 1, 3));
        assert_eq!(format_value(rx, 15), "16");
        assert_eq!(lookup(&params, "solo_part").unwrap().max, 4);
    }

    #[test]
    fn lookup_part_level() {
        let params = mixer_params();
        let def = lookup(&params, "part.0.level").expect("part.0.level should exist");
        assert_eq!(def.min, 0);
        assert_eq!(def.max, 127);
        assert_eq!(def.default, 100);
        assert_eq!(def.group, ParamGroup::Part(0));
    }

    #[test]
    fn format_pan_center() {
        let params = mixer_params();
        let def = lookup(&params, "part.0.pan").unwrap();
        assert_eq!(format_value(def, 64), "0");
    }

    #[test]
    fn format_on_off() {
        let params = mixer_params();
        let def = lookup(&params, "part.0.mute").unwrap();
        assert_eq!(format_value(def, 0), "OFF");
        assert_eq!(format_value(def, 1), "ON");
    }

    #[test]
    fn sn_synth_param_count() {
        let params = sn_synth_params();
        // 13 common + 3 partials * 8 params = 37
        assert_eq!(params.len(), 37);
    }

    #[test]
    fn format_named() {
        let params = mixer_params();
        let def = lookup(&params, "part.0.output_assign").unwrap();
        assert_eq!(format_value(def, 0), "A");
        assert_eq!(format_value(def, 3), "D");
    }

    #[test]
    fn fx_type_zero_is_off() {
        let params = mixer_params();
        for id in ["chorus.type", "reverb.type"] {
            let def = lookup(&params, id).unwrap();
            assert_eq!(format_value(def, 0), "OFF", "{id}");
        }
    }

    #[test]
    fn eq_named_tables_cover_their_range() {
        let params = mixer_params();
        for id in ["eq.low_freq", "eq.mid_freq", "eq.mid_q", "eq.high_freq"] {
            let def = lookup(&params, id).unwrap();
            let ParamFormat::Named(names) = def.format else {
                panic!("{id} should be named");
            };
            assert_eq!(names.len() as i32, def.max - def.min + 1, "{id}");
        }
        let gain = lookup(&params, "eq.mid_gain").unwrap();
        assert_eq!(format_value(gain, 0), "-15");
        assert_eq!(format_value(gain, 30), "+15");
    }
}
