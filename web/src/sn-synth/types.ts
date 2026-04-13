// ---------------------------------------------------------------------------
// Types mirroring the Rust serde output
// ---------------------------------------------------------------------------

export interface SnSynthCommon {
  toneName: string;
  toneLevel: number;
  portamentoSwitch: number;
  portamentoTime: number;
  monoSwitch: number;
  octaveShift: number;
  pitchBendRangeUp: number;
  pitchBendRangeDown: number;
  partial1Switch: number;
  partial1Select: number;
  partial2Switch: number;
  partial2Select: number;
  partial3Switch: number;
  partial3Select: number;
  ringSwitch: number;
  tfxSwitch: number;
  unisonSwitch: number;
  portamentoMode: number;
  legatoSwitch: number;
  analogFeel: number;
  waveShape: number;
  toneCategory: number;
  phraseNumber: number;
  phraseOctaveShift: number;
  unisonSize: number;
}

export interface SnSynthPartial {
  oscWave: number;
  oscWaveVariation: number;
  oscPitch: number;
  oscDetune: number;
  oscPwModDepth: number;
  oscPulseWidth: number;
  oscPitchEnvAttack: number;
  oscPitchEnvDecay: number;
  oscPitchEnvDepth: number;
  filterMode: number;
  filterSlope: number;
  filterCutoff: number;
  filterKeyfollow: number;
  filterEnvVelSens: number;
  filterResonance: number;
  filterEnvAttack: number;
  filterEnvDecay: number;
  filterEnvSustain: number;
  filterEnvRelease: number;
  filterEnvDepth: number;
  ampLevel: number;
  ampVelSens: number;
  ampEnvAttack: number;
  ampEnvDecay: number;
  ampEnvSustain: number;
  ampEnvRelease: number;
  ampPan: number;
  lfoShape: number;
  lfoRate: number;
  lfoTempoSync: number;
  lfoTempoSyncNote: number;
  lfoFadeTime: number;
  lfoKeyTrigger: number;
  lfoPitchDepth: number;
  lfoFilterDepth: number;
  lfoAmpDepth: number;
  lfoPanDepth: number;
  modLfoShape: number;
  modLfoRate: number;
  modLfoTempoSync: number;
  modLfoTempoSyncNote: number;
  pwShift: number;
  modLfoPitchDepth: number;
  modLfoFilterDepth: number;
  modLfoAmpDepth: number;
  modLfoPanDepth: number;
  aftertouchCutoff: number;
  aftertouchLevel: number;
  waveGain: number;
  waveNumber: number;
  hpfCutoff: number;
  superSawDetune: number;
  modLfoRateControl: number;
  ampLevelKeyfollow: number;
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

export const OSC_WAVE_NAMES = ["SAW", "SQR", "PW-SQR", "TRI", "SINE", "NOISE", "SP-SAW", "PCM"];
export const OSC_VARIATION_NAMES = ["A", "B", "C"];
export const FILTER_MODE_NAMES = ["BYPASS", "LPF", "HPF", "BPF", "PKG", "LPF2", "LPF3", "LPF4"];
export const FILTER_SLOPE_NAMES = ["-12dB", "-24dB"];
export const LFO_SHAPE_NAMES = ["TRI", "SIN", "SAW", "SQR", "S&H", "RND"];
export const UNISON_SIZE_NAMES = ["2", "4", "6", "8"];
export const WAVE_GAIN_NAMES = ["-6dB", "0dB", "+6dB", "+12dB"];
export const TEMPO_SYNC_NOTE_NAMES = [
  "16", "12", "8", "4", "2", "1", "3/4", "2/3", "1/2", "3/8",
  "1/3", "1/4", "3/16", "1/6", "1/8", "3/32", "1/12", "1/16", "1/24", "1/32",
];

export const ON_OFF_OPTIONS = [{ value: 0, label: "OFF" }, { value: 1, label: "ON" }];

export function signedFmt(raw: number, center: number): string {
  const v = raw - center;
  return v > 0 ? `+${v}` : String(v);
}

export function panFmt(v: number): string {
  if (v === 64) return "C";
  if (v < 64) return `L${64 - v}`;
  return `${v - 64}R`;
}
