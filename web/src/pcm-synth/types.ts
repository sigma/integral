// ---------------------------------------------------------------------------
// Types mirroring the Rust serde output
// ---------------------------------------------------------------------------

export interface PcmSynthCommon {
  toneName: string;
  toneLevel: number;
  tonePan: number;
  tonePriority: number;
  coarseTune: number;
  fineTune: number;
  octaveShift: number;
  stretchTuneDepth: number;
  analogFeel: number;
  monoPoly: number;
  legatoSwitch: number;
  legatoRetrigger: number;
  portamentoSwitch: number;
  portamentoMode: number;
  portamentoType: number;
  portamentoStart: number;
  portamentoTime: number;
  cutoffOffset: number;
  resonanceOffset: number;
  attackTimeOffset: number;
  releaseTimeOffset: number;
  velocitySensOffset: number;
  pmtControlSwitch: number;
  pitchBendRangeUp: number;
  pitchBendRangeDown: number;
}

export interface PmtPartialEntry {
  partialSwitch: number;
  keyRangeLower: number;
  keyRangeUpper: number;
  keyFadeLower: number;
  keyFadeUpper: number;
  velocityRangeLower: number;
  velocityRangeUpper: number;
  velocityFadeLower: number;
  velocityFadeUpper: number;
}

export interface PcmSynthPmt {
  structureType12: number;
  booster12: number;
  structureType34: number;
  booster34: number;
  pmtVelocityControl: number;
  partialEntries: PmtPartialEntry[];
}

export interface PcmSynthPartial {
  level: number;
  coarseTune: number;
  fineTune: number;
  randomPitchDepth: number;
  pan: number;
  panKeyfollow: number;
  randomPanDepth: number;
  alternatePanDepth: number;
  envMode: number;
  delayMode: number;
  delayTime: number;
  outputLevel: number;
  chorusSend: number;
  reverbSend: number;
  receiveBender: number;
  receiveExpression: number;
  receiveHold1: number;
  redamperSwitch: number;
  control1Switches: number[];
  control2Switches: number[];
  control3Switches: number[];
  control4Switches: number[];
  waveGroupType: number;
  waveGroupId: number;
  waveNumberL: number;
  waveNumberR: number;
  waveGain: number;
  waveFxmSwitch: number;
  waveFxmColor: number;
  waveFxmDepth: number;
  waveTempoSync: number;
  wavePitchKeyfollow: number;
  pitchEnvDepth: number;
  pitchEnvVelocitySens: number;
  pitchEnvT1VelocitySens: number;
  pitchEnvT4VelocitySens: number;
  pitchEnvTimeKeyfollow: number;
  pitchEnvTime: number[];
  pitchEnvLevel: number[];
  tvfFilterType: number;
  tvfCutoffFrequency: number;
  tvfCutoffKeyfollow: number;
  tvfCutoffVelocityCurve: number;
  tvfCutoffVelocitySens: number;
  tvfResonance: number;
  tvfResonanceVelocitySens: number;
  tvfEnvDepth: number;
  tvfEnvVelocityCurve: number;
  tvfEnvVelocitySens: number;
  tvfEnvT1VelocitySens: number;
  tvfEnvT4VelocitySens: number;
  tvfEnvTimeKeyfollow: number;
  tvfEnvTime: number[];
  tvfEnvLevel: number[];
  tvaBiasLevel: number;
  tvaBiasPosition: number;
  tvaBiasDirection: number;
  tvaLevelVelocityCurve: number;
  tvaLevelVelocitySens: number;
  tvaEnvT1VelocitySens: number;
  tvaEnvT4VelocitySens: number;
  tvaEnvTimeKeyfollow: number;
  tvaEnvTime: number[];
  tvaEnvLevel: number[];
  lfo1Waveform: number;
  lfo1Rate: number;
  lfo1Offset: number;
  lfo1RateDetune: number;
  lfo1DelayTime: number;
  lfo1DelayTimeKeyfollow: number;
  lfo1FadeMode: number;
  lfo1FadeTime: number;
  lfo1KeyTrigger: number;
  lfo1PitchDepth: number;
  lfo1TvfDepth: number;
  lfo1TvaDepth: number;
  lfo1PanDepth: number;
  lfo2Waveform: number;
  lfo2Rate: number;
  lfo2Offset: number;
  lfo2RateDetune: number;
  lfo2DelayTime: number;
  lfo2DelayTimeKeyfollow: number;
  lfo2FadeMode: number;
  lfo2FadeTime: number;
  lfo2KeyTrigger: number;
  lfo2PitchDepth: number;
  lfo2TvfDepth: number;
  lfo2TvaDepth: number;
  lfo2PanDepth: number;
  lfoStepType: number;
  lfoStepValues: number[];
}

export interface PcmSynthCommon2 {
  toneCategory: number;
  phraseOctaveShift: number;
  tfxSwitch: number;
  phraseNumber: number;
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

export const ON_OFF_OPTIONS = [{ value: 0, label: "OFF" }, { value: 1, label: "ON" }];

export const WAVE_GAIN_NAMES = ["-6dB", "0dB", "+6dB", "+12dB"];

export const FILTER_TYPE_NAMES = ["OFF", "LPF", "BPF", "HPF", "PKG", "LPF2", "LPF3"];

export const LFO_WAVEFORM_NAMES = [
  "SIN", "TRI", "SAW-UP", "SAW-DW", "SQR", "RND",
  "BEND-UP", "BEND-DW", "TRP", "S&H", "CHS", "VSIN", "STEP",
];
export const LFO_WAVEFORM_STEP = LFO_WAVEFORM_NAMES.indexOf("STEP");

export const LFO_OFFSET_NAMES = ["-100", "-50", "0", "+50", "+100"];

export const LFO_FADE_MODE_NAMES = ["ON-IN", "ON-OUT", "OFF-IN", "OFF-OUT"];

export const VELOCITY_CURVE_NAMES = ["FIXED", "1", "2", "3", "4", "5", "6", "7"];

export const STRUCTURE_TYPE_NAMES = [
  "1: Independent",
  "2: Stacked Filt",
  "3: Boost \u2192 Filt",
  "4: Boost + Bal",
  "5: Ring \u2192 Filt",
  "6: Ring + Bal",
  "7: Filt \u2192 Ring",
  "8: Filt+Ring+Mix",
  "9: Ring+Mix \u2192 Filt",
  "10: Mix \u2192 Boost",
];

export const BOOSTER_NAMES = ["0", "+6", "+12", "+18"];

export const PMT_VEL_CTRL_NAMES = ["OFF", "ON", "RANDOM", "CYCLE"];

export const BIAS_DIR_NAMES = ["LOWER", "UPPER", "LOWER&UPPER", "ALL"];

export const ENV_MODE_NAMES = ["NO-SUS", "SUSTAIN"];

export const DELAY_MODE_NAMES = ["NORMAL", "HOLD", "KEY-OFF-NORMAL", "KEY-OFF-DECAY"];

export function signedFmt(raw: number, center: number): string {
  const v = raw - center;
  return v > 0 ? `+${v}` : String(v);
}

export function panFmt(v: number): string {
  if (v === 64) return "C";
  if (v < 64) return `L${64 - v}`;
  return `${v - 64}R`;
}

export function noteName(midi: number): string {
  const names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
  const oct = Math.floor(midi / 12) - 1;
  return `${names[midi % 12]}${oct}`;
}
