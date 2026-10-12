import type { SnsCommonField, SnsPartialField } from "../../pkg/integral_wasm.js";

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

/** A part's SN Synth Tone as cached in core (its MFX is read apart). */
export interface SnSynthTone {
  common: SnSynthCommon;
  partials: SnSynthPartial[];
}

/** Sets a Common field (sent nibblized when the field is). */
export type CommonParamHandler = (field: SnsCommonField, value: number) => void;

/** Sets a field of one Partial (sent nibblized when the field is). */
export type PartialParamHandler = (field: SnsPartialField, value: number) => void;
