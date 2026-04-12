/** 3-band parametric EQ state. */
export interface EqState {
  /** EQ on/off. */
  enabled: boolean;
  /** Low band frequency (0=200Hz, 1=400Hz). */
  lowFreq: number;
  /** Low band gain (0–30, display: -15 to +15 dB). */
  lowGain: number;
  /** Mid band frequency (0–16, 17 values from 200Hz to 8000Hz). */
  midFreq: number;
  /** Mid band gain (0–30, display: -15 to +15 dB). */
  midGain: number;
  /** Mid band Q (0–4, display: 0.5, 1.0, 2.0, 4.0, 8.0). */
  midQ: number;
  /** High band frequency (0=2000Hz, 1=4000Hz, 2=8000Hz). */
  highFreq: number;
  /** High band gain (0–30, display: -15 to +15 dB). */
  highGain: number;
}

/** Default EQ state (flat, enabled). */
export function defaultEqState(): EqState {
  return {
    enabled: true,
    lowFreq: 1,   // 400 Hz
    lowGain: 15,  // 0 dB
    midFreq: 7,   // 1000 Hz
    midGain: 15,  // 0 dB
    midQ: 0,      // 0.5
    highFreq: 1,  // 4000 Hz
    highGain: 15, // 0 dB
  };
}

/** Chorus/Reverb FX state. */
export interface FxState {
  /** Effect on/off. */
  enabled: boolean;
  /** Effect type index (Chorus: 0-3, Reverb: 0-6). */
  type: number;
  /** Effect level (0-127). */
  level: number;
  /** Output routing (Chorus: 0-2 MAIN/REV/MAIN+REV; Reverb: 0-3 A/B/C/D). */
  output: number;
  /** Type-dependent parameters (nibblized values, decoded to display range). */
  params: number[];
}

/** Default FX state. */
export function defaultFxState(): FxState {
  return {
    enabled: true,
    type: 0,
    level: 0,
    output: 0,
    params: [],
  };
}

/** State of a single Part in the mixer. */
export interface PartState {
  /** Part level / volume (0–127). */
  level: number;
  /** Part pan (0–127, 64 = center). */
  pan: number;
  /** Whether the part is muted. */
  muted: boolean;
  /** Tone bank MSB. */
  toneBankMsb: number;
  /** Tone bank LSB. */
  toneBankLsb: number;
  /** Tone program number. */
  tonePC: number;
  /** MIDI receive channel (0–15). */
  receiveChannel: number;
  /** Chorus send level (0–127). */
  chorusSend: number;
  /** Reverb send level (0–127). */
  reverbSend: number;
  /** Tone name read from the device. */
  toneName: string;
  /** Per-part EQ settings. */
  eq: EqState;
}

/** State of a single Comp+EQ unit. */
export interface CompEqUnit {
  compSwitch: boolean;
  compAttack: number;
  compRelease: number;
  compThreshold: number;
  compRatio: number;
  compOutputGain: number;
  eqSwitch: boolean;
  eqLowFreq: number;
  eqLowGain: number;
  eqMidFreq: number;
  eqMidGain: number;
  eqMidQ: number;
  eqHighFreq: number;
  eqHighGain: number;
}

/** Drum Comp+EQ state (6 units + Studio Set common settings). */
export interface DrumCompEqState {
  enabled: boolean;
  part: number;
  outputAssigns: number[];
  units: CompEqUnit[];
}

/** Default Comp+EQ unit. */
export function defaultCompEqUnit(): CompEqUnit {
  return {
    compSwitch: false,
    compAttack: 10,
    compRelease: 10,
    compThreshold: 127,
    compRatio: 0,
    compOutputGain: 0,
    eqSwitch: false,
    eqLowFreq: 1,
    eqLowGain: 15,
    eqMidFreq: 7,
    eqMidGain: 15,
    eqMidQ: 0,
    eqHighFreq: 1,
    eqHighGain: 15,
  };
}

/** Default Drum Comp+EQ state. */
export function defaultDrumCompEqState(): DrumCompEqState {
  return {
    enabled: false,
    part: 9,
    outputAssigns: [0, 0, 0, 0, 0, 0],
    units: Array.from({ length: 6 }, () => defaultCompEqUnit()),
  };
}

/** Per-part surround positioning. */
export interface SurroundPartState {
  lr: number;
  fb: number;
  width: number;
  ambienceSend: number;
}

/** Motional Surround global state. */
export interface SurroundState {
  enabled: boolean;
  roomType: number;
  roomSize: number;
  depth: number;
  ambienceLevel: number;
  ambienceTime: number;
  ambienceDensity: number;
  ambienceHfDamp: number;
  parts: SurroundPartState[];
  ext: SurroundPartState;
  extControlChannel: number;
}

function defaultSurroundPartState(): SurroundPartState {
  return { lr: 64, fb: 64, width: 16, ambienceSend: 0 };
}

export function defaultSurroundState(): SurroundState {
  return {
    enabled: false,
    roomType: 0,
    roomSize: 1,
    depth: 50,
    ambienceLevel: 64,
    ambienceTime: 50,
    ambienceDensity: 50,
    ambienceHfDamp: 50,
    parts: Array.from({ length: 16 }, () => defaultSurroundPartState()),
    ext: defaultSurroundPartState(),
    extControlChannel: 16,
  };
}

/** Full mixer state. */
export interface MixerState {
  /** Studio Set name (up to 16 ASCII chars). */
  studioSetName: string;
  /** Current Studio Set PC (0–63). */
  studioSetPC: number;
  /** System master level (0–127). */
  masterLevel: number;
  /** Solo Part (0=OFF, 1–16=Part 1–16). */
  soloPart: number;
  /** All 16 parts. */
  parts: PartState[];
  /** Currently selected part index (0–15). */
  selectedPart: number;
  /** Chorus (FX1) state. */
  chorus: FxState;
  /** Reverb (FX2) state. */
  reverb: FxState;
  /** External input level (0–127). */
  extLevel: number;
  /** External input mute. */
  extMuted: boolean;
  /** Master EQ settings. */
  masterEq: EqState;
  /** Motional Surround state. */
  surround: SurroundState;
  /** Drum Comp+EQ (6 units, assigned to one part). */
  drumCompEq: DrumCompEqState;
  /** Preview part (0 = off, 1–16 = part being previewed). */
  previewPart: number;
  /** Whether the EQ section is expanded in all strips. */
  eqExpanded: boolean;
  /** Whether initial state is still loading from the device. */
  loading: boolean;
  /** All 64 Studio Set names (indexed 0–63). Populated via catalog query. */
  studioSetNames: Map<number, string>;
}

/** Default state for a single part. */
export function defaultPartState(): PartState {
  return {
    level: 100,
    pan: 64,
    muted: false,
    toneBankMsb: 0,
    toneBankLsb: 0,
    tonePC: 0,
    chorusSend: 0,
    reverbSend: 0,
    receiveChannel: 0,
    toneName: "",
    eq: defaultEqState(),
  };
}

/** Default mixer state (before device sync). */
export function defaultMixerState(): MixerState {
  return {
    studioSetName: "",
    studioSetPC: 0,
    masterLevel: 100,
    soloPart: 0,
    parts: Array.from({ length: 16 }, () => defaultPartState()),
    selectedPart: 0,
    chorus: defaultFxState(),
    reverb: defaultFxState(),
    extLevel: 100,
    extMuted: false,
    masterEq: defaultEqState(),
    surround: defaultSurroundState(),
    drumCompEq: defaultDrumCompEqState(),
    previewPart: 0,
    eqExpanded: false,
    loading: true,
    studioSetNames: new Map(),
  };
}
