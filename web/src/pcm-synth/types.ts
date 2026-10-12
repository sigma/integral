import type {
  PcmsCommonField,
  PcmsPartialField,
  PcmsPmtField,
} from "../../pkg/integral_wasm.js";

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

/** The keys of `T` holding a number (or, with `number[]`, a list of them). */
type KeysOf<T, V> = { [K in keyof T]: T[K] extends V ? K : never }[keyof T];

/** Sets a Common field: `field` addresses it on the device, `key` names it
 *  in {@link PcmSynthCommon}. */
export type CommonParamHandler = (
  field: PcmsCommonField,
  key: KeysOf<PcmSynthCommon, number>,
  value: number,
) => void;

/** Sets a PMT field common to all Partials: `field` addresses it on the
 *  device, `key` names it in {@link PcmSynthPmt}. */
export type PmtParamHandler = (
  field: PcmsPmtField,
  key: KeysOf<PcmSynthPmt, number>,
  value: number,
) => void;

/** Sets Partial `partial`'s (0-based) entry of a per-Partial PMT field:
 *  `field` addresses it on the device, `key` names it in
 *  {@link PmtPartialEntry}. */
export type PmtEntryParamHandler = (
  partial: number,
  field: PcmsPmtField,
  key: keyof PmtPartialEntry,
  value: number,
) => void;

/** A {@link PcmSynthPartial} key holding a value or a list of values. */
export type PartialFieldKey = KeysOf<PcmSynthPartial, number | number[]>;

/** Sets value `index` (0 unless the field holds a list) of a Partial field:
 *  `field` addresses it on the device, `key` names it in
 *  {@link PcmSynthPartial}. */
export type PartialParamHandler = (
  field: PcmsPartialField,
  key: PartialFieldKey,
  index: number,
  value: number,
) => void;

/** Builds the change callback for value `index` (default 0) of a field. */
export type PartialSetter = (
  field: PcmsPartialField,
  key: PartialFieldKey,
  index?: number,
) => (value: number) => void;
