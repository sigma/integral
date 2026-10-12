import type { PcmdPartialField, PcmdWmtField } from "../../pkg/integral_wasm.js";

// ---------------------------------------------------------------------------
// Types mirroring the Rust serde output (PcmDrumCommon, PcmDrumPartial, etc.)
// ---------------------------------------------------------------------------

export interface PcmDrumCommon {
  kitName: string;
  kitLevel: number;
}

export interface PcmDrumWmt {
  wmtSwitch: number;
  waveGroupType: number;
  waveGroupId: number;
  waveNumberL: number;
  waveNumberR: number;
  waveGain: number;
  waveFxmSwitch: number;
  waveFxmColor: number;
  waveFxmDepth: number;
  waveTempoSync: number;
  coarseTune: number;
  fineTune: number;
  pan: number;
  randomPanSwitch: number;
  alternatePanSwitch: number;
  level: number;
  velocityRangeLower: number;
  velocityRangeUpper: number;
  velocityFadeLower: number;
  velocityFadeUpper: number;
}

/** Sets one field of WMT `wmt` (0-based): `field` addresses it on the
 *  device, `key` names it in {@link PcmDrumWmt}. */
export type WmtParamHandler = (
  wmt: number,
  field: PcmdWmtField,
  key: keyof PcmDrumWmt,
  value: number,
) => void;

export interface PcmDrumPartial {
  partialName: string;
  assignType: number;
  muteGroup: number;
  level: number;
  coarseTune: number;
  fineTune: number;
  randomPitchDepth: number;
  pan: number;
  randomPanDepth: number;
  alternatePanDepth: number;
  envMode: number;
  outputLevel: number;
  chorusSend: number;
  reverbSend: number;
  outputAssign: number;
  pitchBendRange: number;
  receiveExpression: number;
  receiveHold1: number;
  wmtVelocityControl: number;
  wmt: PcmDrumWmt[];
  pitchEnvDepth: number;
  pitchEnvVelocitySens: number;
  pitchEnvT1VelocitySens: number;
  pitchEnvT4VelocitySens: number;
  pitchEnvTime: number[];
  pitchEnvLevel: number[];
  tvfFilterType: number;
  tvfCutoffFrequency: number;
  tvfCutoffVelocityCurve: number;
  tvfCutoffVelocitySens: number;
  tvfResonance: number;
  tvfResonanceVelocitySens: number;
  tvfEnvDepth: number;
  tvfEnvVelocityCurve: number;
  tvfEnvVelocitySens: number;
  tvfEnvT1VelocitySens: number;
  tvfEnvT4VelocitySens: number;
  tvfEnvTime: number[];
  tvfEnvLevel: number[];
  tvaLevelVelocityCurve: number;
  tvaLevelVelocitySens: number;
  tvaEnvT1VelocitySens: number;
  tvaEnvT4VelocitySens: number;
  tvaEnvTime: number[];
  tvaEnvLevel: number[];
  oneShotMode: number;
}

/** A {@link PcmDrumPartial} key holding a non-WMT field (a value or an
 *  envelope's times or levels). */
export type PartialFieldKey = {
  [K in keyof PcmDrumPartial]: PcmDrumPartial[K] extends number | number[] ? K : never;
}[keyof PcmDrumPartial];

/** Sets element `index` (0 unless an envelope's times or levels) of a
 *  non-WMT Partial field: `field` addresses it on the device, `key` names it
 *  in {@link PcmDrumPartial}. */
export type PartialParamHandler = (
  field: PcmdPartialField,
  key: PartialFieldKey,
  index: number,
  value: number,
) => void;

/** Builds the change callback for element `index` (default 0) of a field. */
export type PartialSetter = (
  field: PcmdPartialField,
  key: PartialFieldKey,
  index?: number,
) => (value: number) => void;

export interface PcmDrumCommon2 {
  phraseNumber: number;
  tfxSwitch: number;
}

// ---------------------------------------------------------------------------
// Note name helpers
// ---------------------------------------------------------------------------

const NOTE_NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

export function noteName(key: number): string {
  const name = NOTE_NAMES[key % 12]!;
  const octave = Math.floor(key / 12) - 1;
  return `${name}${octave}`;
}

export function isBlackKey(key: number): boolean {
  const n = key % 12;
  return n === 1 || n === 3 || n === 6 || n === 8 || n === 10;
}
