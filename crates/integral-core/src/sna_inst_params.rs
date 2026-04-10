//! SuperNATURAL Acoustic instrument-specific parameter definitions.
//!
//! Each SN-A instrument type has a unique set of "Modify Parameters" on the
//! INST tab. This module provides the parameter metadata (names, ranges,
//! defaults) needed to build UI controls and interpret the 32 generic
//! modify-param slots in [`crate::sn_acoustic::SnAcousticCommon`].

/// Definition of a single SN-A instrument parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnaInstParamDef {
    /// Modify parameter index (1–32).
    pub index: u8,
    /// Human-readable name.
    pub name: &'static str,
    /// Minimum raw value (inclusive).
    pub min: u8,
    /// Maximum raw value (inclusive).
    pub max: u8,
    /// Default raw value.
    pub default_value: u8,
}

/// Definition of an SN-A instrument type (category).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnaInstTypeDef {
    /// Human-readable category/instrument name.
    pub name: &'static str,
    /// Parameter definitions for this instrument type.
    pub params: &'static [SnaInstParamDef],
}

// ---------------------------------------------------------------------------
// Ac.Piano — INT 001–009
// ---------------------------------------------------------------------------

const AC_PIANO_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "String Resonance",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Key Off Resonance",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 3,
        name: "Hammer Noise",
        min: 0,
        max: 4,
        default_value: 2,
    },
    SnaInstParamDef {
        index: 4,
        name: "Stereo Width",
        min: 0,
        max: 63,
        default_value: 32,
    },
    SnaInstParamDef {
        index: 5,
        name: "Nuance",
        min: 0,
        max: 2,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 6,
        name: "Tone Character",
        min: 0,
        max: 10,
        default_value: 5,
    },
];

const AC_PIANO_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Ac.Piano",
    params: AC_PIANO_PARAMS,
};

// ---------------------------------------------------------------------------
// E.Piano — INT 010–015
// ---------------------------------------------------------------------------

const E_PIANO_PARAMS: &[SnaInstParamDef] = &[SnaInstParamDef {
    index: 1,
    name: "Noise Level",
    min: 0,
    max: 127,
    default_value: 64,
}];

const E_PIANO_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "E.Piano",
    params: E_PIANO_PARAMS,
};

// ---------------------------------------------------------------------------
// Other Keyboards (Clav) — INT 016–023
// ---------------------------------------------------------------------------

const CLAV_PARAMS: &[SnaInstParamDef] = &[SnaInstParamDef {
    index: 1,
    name: "Noise Level",
    min: 0,
    max: 127,
    default_value: 64,
}];

const CLAV_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Clav",
    params: CLAV_PARAMS,
};

// ---------------------------------------------------------------------------
// Bell/Mallet — INT 024–028, ExSN1 001–002
// ---------------------------------------------------------------------------

const BELL_MALLET_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Mallet Hardness",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Roll Speed",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 3,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const BELL_MALLET_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Bell/Mallet",
    params: BELL_MALLET_PARAMS,
};

// ---------------------------------------------------------------------------
// TW Organ — INT 029
// ---------------------------------------------------------------------------

const TW_ORGAN_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Harmonic Bar 16'",
        min: 0,
        max: 8,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 2,
        name: "Harmonic Bar 5-1/3'",
        min: 0,
        max: 8,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 3,
        name: "Harmonic Bar 8'",
        min: 0,
        max: 8,
        default_value: 8,
    },
    SnaInstParamDef {
        index: 4,
        name: "Harmonic Bar 4'",
        min: 0,
        max: 8,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 5,
        name: "Harmonic Bar 2-2/3'",
        min: 0,
        max: 8,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 6,
        name: "Harmonic Bar 2'",
        min: 0,
        max: 8,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 7,
        name: "Harmonic Bar 1-3/5'",
        min: 0,
        max: 8,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 8,
        name: "Harmonic Bar 1-1/3'",
        min: 0,
        max: 8,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 9,
        name: "Harmonic Bar 1'",
        min: 0,
        max: 8,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 10,
        name: "Leakage Level",
        min: 0,
        max: 127,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 11,
        name: "Percussion Switch",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 12,
        name: "Percussion Soft",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 13,
        name: "Percussion Soft Level",
        min: 0,
        max: 15,
        default_value: 8,
    },
    SnaInstParamDef {
        index: 14,
        name: "Percussion Normal Level",
        min: 0,
        max: 15,
        default_value: 8,
    },
    SnaInstParamDef {
        index: 15,
        name: "Percussion Slow",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 16,
        name: "Percussion Slow Time",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 17,
        name: "Percussion Fast Time",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 18,
        name: "Percussion Harmonic",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 19,
        name: "Percussion Recharge Time",
        min: 0,
        max: 15,
        default_value: 8,
    },
    SnaInstParamDef {
        index: 20,
        name: "Percussion Harmonic Bar Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 21,
        name: "Key On Click Level",
        min: 0,
        max: 31,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 22,
        name: "Key Off Click Level",
        min: 0,
        max: 31,
        default_value: 0,
    },
];

const TW_ORGAN_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "TW Organ",
    params: TW_ORGAN_PARAMS,
};

// ---------------------------------------------------------------------------
// Accordion — INT 030–031, INT 033
// ---------------------------------------------------------------------------

const ACCORDION_PARAMS: &[SnaInstParamDef] = &[SnaInstParamDef {
    index: 1,
    name: "Noise Level",
    min: 0,
    max: 127,
    default_value: 64,
}];

const ACCORDION_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Accordion",
    params: ACCORDION_PARAMS,
};

// ---------------------------------------------------------------------------
// Harmonica — INT 032
// ---------------------------------------------------------------------------

const HARMONICA_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Noise Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Growl Sens",
        min: 0,
        max: 127,
        default_value: 64,
    },
];

const HARMONICA_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Harmonica",
    params: HARMONICA_PARAMS,
};

// ---------------------------------------------------------------------------
// Ac.Guitar — INT 034–036, ExSN3 001–002, ExSN4 001–003, ExSN4 005–006
// ---------------------------------------------------------------------------

const AC_GUITAR_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Noise Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Strum Speed",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 3,
        name: "Strum Mode",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 4,
        name: "Sub String Tune",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 5,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const AC_GUITAR_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Ac.Guitar",
    params: AC_GUITAR_PARAMS,
};

// ---------------------------------------------------------------------------
// Mandolin — ExSN4 004
// ---------------------------------------------------------------------------

const MANDOLIN_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Noise Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Tremolo Speed",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 3,
        name: "Strum Mode",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 4,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const MANDOLIN_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Mandolin",
    params: MANDOLIN_PARAMS,
};

// ---------------------------------------------------------------------------
// E.Guitar — INT 037–040, ExSN3 003–005
// ---------------------------------------------------------------------------

const E_GUITAR_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Noise Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Strum Speed",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 3,
        name: "Strum Mode",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 4,
        name: "Picking Harmonics",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 5,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const E_GUITAR_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "E.Guitar",
    params: E_GUITAR_PARAMS,
};

// ---------------------------------------------------------------------------
// Ac.Bass — INT 041, ExSN3 006
// ---------------------------------------------------------------------------

const AC_BASS_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Noise Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const AC_BASS_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Ac.Bass",
    params: AC_BASS_PARAMS,
};

// ---------------------------------------------------------------------------
// E.Bass — INT 042–044, ExSN3 007–008
// ---------------------------------------------------------------------------

const E_BASS_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Noise Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const E_BASS_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "E.Bass",
    params: E_BASS_PARAMS,
};

// ---------------------------------------------------------------------------
// Strings Solo — INT 045–050
// ---------------------------------------------------------------------------

const STRINGS_SOLO_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Noise Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const STRINGS_SOLO_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Strings Solo",
    params: STRINGS_SOLO_PARAMS,
};

// ---------------------------------------------------------------------------
// Strings Ensemble — INT 053–054
// ---------------------------------------------------------------------------

const STRINGS_ENSEMBLE_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Hold Legato Mode",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 2,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const STRINGS_ENSEMBLE_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Strings Ensemble",
    params: STRINGS_ENSEMBLE_PARAMS,
};

// ---------------------------------------------------------------------------
// Strings Erhu — INT 076
// ---------------------------------------------------------------------------

const STRINGS_ERHU_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Noise Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const STRINGS_ERHU_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Strings Erhu",
    params: STRINGS_ERHU_PARAMS,
};

// ---------------------------------------------------------------------------
// Strings Sarangi — ExSN1 010
// ---------------------------------------------------------------------------

const STRINGS_SARANGI_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Resonance Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Tambura Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 3,
        name: "Tambura Pitch",
        min: 0,
        max: 24,
        default_value: 12,
    },
];

const STRINGS_SARANGI_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Strings Sarangi",
    params: STRINGS_SARANGI_PARAMS,
};

// ---------------------------------------------------------------------------
// Harp — INT 051
// ---------------------------------------------------------------------------

const HARP_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Glissando Mode",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 2,
        name: "Play Scale",
        min: 0,
        max: 5,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 3,
        name: "Scale Key",
        min: 0,
        max: 11,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 4,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const HARP_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Harp",
    params: HARP_PARAMS,
};

// ---------------------------------------------------------------------------
// Sitar — INT 073
// ---------------------------------------------------------------------------

const SITAR_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Resonance Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Tambura Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 3,
        name: "Tambura Pitch",
        min: 0,
        max: 24,
        default_value: 12,
    },
];

const SITAR_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Sitar",
    params: SITAR_PARAMS,
};

// ---------------------------------------------------------------------------
// Shamisen — ExSN1 005–006
// ---------------------------------------------------------------------------

const SHAMISEN_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Resonance Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Bend Depth",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 3,
        name: "Buzz Key Switch",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 4,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const SHAMISEN_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Shamisen",
    params: SHAMISEN_PARAMS,
};

// ---------------------------------------------------------------------------
// Koto — ExSN1 007
// ---------------------------------------------------------------------------

const KOTO_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Tremolo Speed",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Glissando Mode",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 3,
        name: "Play Scale",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 4,
        name: "Scale Key",
        min: 0,
        max: 11,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 5,
        name: "Buzz Key Switch",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 6,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const KOTO_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Koto",
    params: KOTO_PARAMS,
};

// ---------------------------------------------------------------------------
// Taishou Koto — ExSN1 008
// ---------------------------------------------------------------------------

const TAISHOU_KOTO_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Noise Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Tremolo Speed",
        min: 0,
        max: 127,
        default_value: 64,
    },
];

const TAISHOU_KOTO_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Taishou Koto",
    params: TAISHOU_KOTO_PARAMS,
};

// ---------------------------------------------------------------------------
// Kalimba — ExSN1 009
// ---------------------------------------------------------------------------

const KALIMBA_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Resonance Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const KALIMBA_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Kalimba",
    params: KALIMBA_PARAMS,
};

// ---------------------------------------------------------------------------
// Vox/Choir — INT 055–056
// ---------------------------------------------------------------------------

const VOX_CHOIR_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Hold Legato Mode",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 2,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const VOX_CHOIR_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Vox/Choir",
    params: VOX_CHOIR_PARAMS,
};

// ---------------------------------------------------------------------------
// Brass — INT 057–061, ExSN5 001–011
// ---------------------------------------------------------------------------

const BRASS_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Noise Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Crescendo Depth",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 3,
        name: "Growl Sens",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 4,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const BRASS_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Brass",
    params: BRASS_PARAMS,
};

// ---------------------------------------------------------------------------
// Wind — INT 066–068, ExSN2 005–006
// ---------------------------------------------------------------------------

const WIND_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Noise Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Growl Sens",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 3,
        name: "Play Scale",
        min: 0,
        max: 5,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 4,
        name: "Scale Key",
        min: 0,
        max: 11,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 5,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const WIND_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Wind",
    params: WIND_PARAMS,
};

// ---------------------------------------------------------------------------
// Wind Pipes — INT 074–075
// ---------------------------------------------------------------------------

const WIND_PIPES_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Drone Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Drone Pitch",
        min: 0,
        max: 24,
        default_value: 12,
    },
    SnaInstParamDef {
        index: 3,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const WIND_PIPES_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Wind Pipes",
    params: WIND_PIPES_PARAMS,
};

// ---------------------------------------------------------------------------
// Flute — INT 069–071, ExSN2 007
// ---------------------------------------------------------------------------

const FLUTE_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Noise Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Growl Sens",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 3,
        name: "Play Scale",
        min: 0,
        max: 5,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 4,
        name: "Scale Key",
        min: 0,
        max: 11,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 5,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const FLUTE_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Flute",
    params: FLUTE_PARAMS,
};

// ---------------------------------------------------------------------------
// Flute Shakuhachi/Tin Whistle/Ryuteki — INT 072, ExSN1 003–004
// ---------------------------------------------------------------------------

const FLUTE_ETHNIC_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Noise Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Growl Sens",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 3,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const FLUTE_ETHNIC_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Flute Ethnic",
    params: FLUTE_ETHNIC_PARAMS,
};

// ---------------------------------------------------------------------------
// Sax — INT 062–065, ExSN2 001–004
// ---------------------------------------------------------------------------

const SAX_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Noise Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Growl Sens",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 3,
        name: "Play Scale",
        min: 0,
        max: 5,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 4,
        name: "Scale Key",
        min: 0,
        max: 11,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 5,
        name: "Glide",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 6,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const SAX_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Sax",
    params: SAX_PARAMS,
};

// ---------------------------------------------------------------------------
// Recorder — ExSN2 008–011
// ---------------------------------------------------------------------------

const RECORDER_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Noise Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Growl Sens",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 3,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const RECORDER_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Recorder",
    params: RECORDER_PARAMS,
};

// ---------------------------------------------------------------------------
// Recorder Ocarina — ExSN2 012–015
// ---------------------------------------------------------------------------

const OCARINA_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Noise Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Growl Sens",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 3,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const OCARINA_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Ocarina",
    params: OCARINA_PARAMS,
};

// ---------------------------------------------------------------------------
// Percussion Timpani — INT 052
// ---------------------------------------------------------------------------

const TIMPANI_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Roll Speed",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const TIMPANI_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Timpani",
    params: TIMPANI_PARAMS,
};

// ---------------------------------------------------------------------------
// Percussion Steel Drums — INT 077
// ---------------------------------------------------------------------------

const STEEL_DRUMS_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Resonance Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Roll Speed",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 3,
        name: "Variation",
        min: 0,
        max: 127,
        default_value: 0,
    },
];

const STEEL_DRUMS_TYPE: SnaInstTypeDef = SnaInstTypeDef {
    name: "Steel Drums",
    params: STEEL_DRUMS_PARAMS,
};

// ---------------------------------------------------------------------------
// Lookup table
// ---------------------------------------------------------------------------

/// All known SN-A instrument type definitions, indexed by category name.
const ALL_INST_TYPES: &[&SnaInstTypeDef] = &[
    &AC_PIANO_TYPE,
    &E_PIANO_TYPE,
    &CLAV_TYPE,
    &BELL_MALLET_TYPE,
    &TW_ORGAN_TYPE,
    &ACCORDION_TYPE,
    &HARMONICA_TYPE,
    &AC_GUITAR_TYPE,
    &MANDOLIN_TYPE,
    &E_GUITAR_TYPE,
    &AC_BASS_TYPE,
    &E_BASS_TYPE,
    &STRINGS_SOLO_TYPE,
    &STRINGS_ENSEMBLE_TYPE,
    &STRINGS_ERHU_TYPE,
    &STRINGS_SARANGI_TYPE,
    &HARP_TYPE,
    &SITAR_TYPE,
    &SHAMISEN_TYPE,
    &KOTO_TYPE,
    &TAISHOU_KOTO_TYPE,
    &KALIMBA_TYPE,
    &VOX_CHOIR_TYPE,
    &BRASS_TYPE,
    &WIND_TYPE,
    &WIND_PIPES_TYPE,
    &FLUTE_TYPE,
    &FLUTE_ETHNIC_TYPE,
    &SAX_TYPE,
    &RECORDER_TYPE,
    &OCARINA_TYPE,
    &TIMPANI_TYPE,
    &STEEL_DRUMS_TYPE,
];

/// Look up an SN-A instrument type definition by category name.
///
/// Returns `None` if the category is not recognized. The match is exact and
/// case-sensitive.
pub fn sna_inst_type_by_category(category: &str) -> Option<&'static SnaInstTypeDef> {
    ALL_INST_TYPES.iter().find(|t| t.name == category).copied()
}

/// List all known SN-A instrument type definitions.
pub fn sna_all_inst_types() -> &'static [&'static SnaInstTypeDef] {
    ALL_INST_TYPES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_ac_piano() {
        let def = sna_inst_type_by_category("Ac.Piano").unwrap();
        assert_eq!(def.name, "Ac.Piano");
        assert_eq!(def.params.len(), 6);
        assert_eq!(def.params[0].name, "String Resonance");
    }

    #[test]
    fn lookup_tw_organ() {
        let def = sna_inst_type_by_category("TW Organ").unwrap();
        assert_eq!(def.name, "TW Organ");
        assert_eq!(def.params.len(), 22);
        assert_eq!(def.params[0].name, "Harmonic Bar 16'");
        assert_eq!(def.params[8].name, "Harmonic Bar 1'");
        assert_eq!(def.params[10].name, "Percussion Switch");
    }

    #[test]
    fn lookup_sax() {
        let def = sna_inst_type_by_category("Sax").unwrap();
        assert_eq!(def.params.len(), 6);
        assert_eq!(def.params[4].name, "Glide");
    }

    #[test]
    fn lookup_unknown_returns_none() {
        assert!(sna_inst_type_by_category("NonExistent").is_none());
    }

    #[test]
    fn all_inst_types_count() {
        assert_eq!(sna_all_inst_types().len(), 33);
    }

    #[test]
    fn param_indices_start_at_one() {
        for t in ALL_INST_TYPES {
            if !t.params.is_empty() {
                assert_eq!(
                    t.params[0].index, 1,
                    "first param of {} should be index 1",
                    t.name
                );
            }
        }
    }
}
