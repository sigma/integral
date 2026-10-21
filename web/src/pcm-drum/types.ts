import { PcmdCommonField, PcmdPartialField, PcmdWmtField } from "../../pkg/integral_wasm.js";
import type { FieldEntry } from "../fieldSpec";

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

/** Every Common field in {@link PcmDrumCommon}. */
export const PCMD_COMMON_ENTRIES: Record<PcmdCommonField, FieldEntry<PcmDrumCommon>> = {
  [PcmdCommonField.KitLevel]: { key: "kitLevel", name: "Kit Level" },
};

/** Every WMT layer field in {@link PcmDrumWmt}. */
export const PCMD_WMT_ENTRIES: Record<PcmdWmtField, FieldEntry<PcmDrumWmt>> = {
  [PcmdWmtField.Switch]: { key: "wmtSwitch", name: "Wave layer switch" },
  [PcmdWmtField.WaveGroupType]: { key: "waveGroupType", name: "Wave Group Type" },
  [PcmdWmtField.WaveGroupId]: { key: "waveGroupId", name: "Wave Group ID" },
  [PcmdWmtField.WaveNumberL]: { key: "waveNumberL", name: "Wave Number L" },
  [PcmdWmtField.WaveNumberR]: { key: "waveNumberR", name: "Wave Number R" },
  [PcmdWmtField.WaveGain]: { key: "waveGain", name: "Wave Gain" },
  [PcmdWmtField.WaveFxmSwitch]: { key: "waveFxmSwitch", name: "FXM Switch" },
  [PcmdWmtField.WaveFxmColor]: { key: "waveFxmColor", name: "FXM Color" },
  [PcmdWmtField.WaveFxmDepth]: { key: "waveFxmDepth", name: "FXM Depth" },
  [PcmdWmtField.WaveTempoSync]: { key: "waveTempoSync", name: "Tempo Sync" },
  [PcmdWmtField.CoarseTune]: { key: "coarseTune", name: "Coarse Tune" },
  [PcmdWmtField.FineTune]: { key: "fineTune", name: "Fine Tune" },
  [PcmdWmtField.Pan]: { key: "pan", name: "Wave Pan" },
  [PcmdWmtField.RandomPanSwitch]: { key: "randomPanSwitch", name: "Random Pan Switch" },
  [PcmdWmtField.AlternatePanSwitch]: { key: "alternatePanSwitch", name: "Alternate Pan Switch" },
  [PcmdWmtField.Level]: { key: "level", name: "Wave Level" },
  [PcmdWmtField.VelocityRangeLower]: { key: "velocityRangeLower", name: "Velocity Range Lower" },
  [PcmdWmtField.VelocityRangeUpper]: { key: "velocityRangeUpper", name: "Velocity Range Upper" },
  [PcmdWmtField.VelocityFadeLower]: { key: "velocityFadeLower", name: "Velocity Fade Width Lower" },
  [PcmdWmtField.VelocityFadeUpper]: { key: "velocityFadeUpper", name: "Velocity Fade Width Upper" },
};

/** Every Partial field outside the WMT layers in {@link PcmDrumPartial}. */
export const PCMD_PARTIAL_ENTRIES: Record<PcmdPartialField, FieldEntry<PcmDrumPartial, PartialFieldKey>> = {
  [PcmdPartialField.AssignType]: { key: "assignType", name: "Assign Type" },
  [PcmdPartialField.MuteGroup]: { key: "muteGroup", name: "Mute Group" },
  [PcmdPartialField.Level]: { key: "level", name: "Partial Level" },
  [PcmdPartialField.CoarseTune]: { key: "coarseTune", name: "Partial Coarse Tune" },
  [PcmdPartialField.FineTune]: { key: "fineTune", name: "Partial Fine Tune" },
  [PcmdPartialField.RandomPitchDepth]: { key: "randomPitchDepth", name: "Random Pitch Depth (cents)" },
  [PcmdPartialField.Pan]: { key: "pan", name: "Pan" },
  [PcmdPartialField.RandomPanDepth]: { key: "randomPanDepth", name: "Random Pan Depth" },
  [PcmdPartialField.AlternatePanDepth]: { key: "alternatePanDepth", name: "Alternate Pan Depth" },
  [PcmdPartialField.EnvMode]: { key: "envMode", name: "Envelope Mode" },
  [PcmdPartialField.OutputLevel]: { key: "outputLevel", name: "Output Level" },
  [PcmdPartialField.ChorusSend]: { key: "chorusSend", name: "Chorus Send" },
  [PcmdPartialField.ReverbSend]: { key: "reverbSend", name: "Reverb Send" },
  [PcmdPartialField.OutputAssign]: { key: "outputAssign", name: "Output Assign" },
  [PcmdPartialField.PitchBendRange]: { key: "pitchBendRange", name: "Pitch Bend Range" },
  [PcmdPartialField.ReceiveExpression]: { key: "receiveExpression", name: "Receive Expression" },
  [PcmdPartialField.ReceiveHold1]: { key: "receiveHold1", name: "Receive Hold-1" },
  [PcmdPartialField.WmtVelocityControl]: { key: "wmtVelocityControl", name: "WMT Velocity Control" },
  [PcmdPartialField.PitchEnvDepth]: { key: "pitchEnvDepth", name: "Pitch envelope depth" },
  [PcmdPartialField.PitchEnvVelocitySens]: { key: "pitchEnvVelocitySens", name: "Pitch envelope velocity sensitivity" },
  [PcmdPartialField.PitchEnvT1VelocitySens]: {
    key: "pitchEnvT1VelocitySens",
    name: "Pitch envelope Time 1 velocity sensitivity",
  },
  [PcmdPartialField.PitchEnvT4VelocitySens]: {
    key: "pitchEnvT4VelocitySens",
    name: "Pitch envelope Time 4 velocity sensitivity",
  },
  [PcmdPartialField.PitchEnvTime]: { key: "pitchEnvTime", name: "Pitch envelope time" },
  [PcmdPartialField.PitchEnvLevel]: { key: "pitchEnvLevel", name: "Pitch envelope level" },
  [PcmdPartialField.TvfFilterType]: { key: "tvfFilterType", name: "Filter type" },
  [PcmdPartialField.TvfCutoffFrequency]: { key: "tvfCutoffFrequency", name: "Filter cutoff frequency" },
  [PcmdPartialField.TvfCutoffVelocityCurve]: { key: "tvfCutoffVelocityCurve", name: "Cutoff velocity curve" },
  [PcmdPartialField.TvfCutoffVelocitySens]: { key: "tvfCutoffVelocitySens", name: "Cutoff velocity sensitivity" },
  [PcmdPartialField.TvfResonance]: { key: "tvfResonance", name: "Filter resonance" },
  [PcmdPartialField.TvfResonanceVelocitySens]: {
    key: "tvfResonanceVelocitySens",
    name: "Resonance velocity sensitivity",
  },
  [PcmdPartialField.TvfEnvDepth]: { key: "tvfEnvDepth", name: "Filter envelope depth" },
  [PcmdPartialField.TvfEnvVelocityCurve]: { key: "tvfEnvVelocityCurve", name: "Filter envelope velocity curve" },
  [PcmdPartialField.TvfEnvVelocitySens]: { key: "tvfEnvVelocitySens", name: "Envelope velocity sensitivity" },
  [PcmdPartialField.TvfEnvT1VelocitySens]: {
    key: "tvfEnvT1VelocitySens",
    name: "Filter envelope Time 1 velocity sensitivity",
  },
  [PcmdPartialField.TvfEnvT4VelocitySens]: {
    key: "tvfEnvT4VelocitySens",
    name: "Filter envelope Time 4 velocity sensitivity",
  },
  [PcmdPartialField.TvfEnvTime]: { key: "tvfEnvTime", name: "Filter envelope time" },
  [PcmdPartialField.TvfEnvLevel]: { key: "tvfEnvLevel", name: "Filter envelope level" },
  [PcmdPartialField.TvaLevelVelocityCurve]: { key: "tvaLevelVelocityCurve", name: "Amplitude velocity curve" },
  [PcmdPartialField.TvaLevelVelocitySens]: { key: "tvaLevelVelocitySens", name: "Amplitude velocity sensitivity" },
  [PcmdPartialField.TvaEnvT1VelocitySens]: {
    key: "tvaEnvT1VelocitySens",
    name: "Amp envelope Time 1 velocity sensitivity",
  },
  [PcmdPartialField.TvaEnvT4VelocitySens]: {
    key: "tvaEnvT4VelocitySens",
    name: "Amp envelope Time 4 velocity sensitivity",
  },
  [PcmdPartialField.TvaEnvTime]: { key: "tvaEnvTime", name: "Amp envelope time" },
  [PcmdPartialField.TvaEnvLevel]: { key: "tvaEnvLevel", name: "Amp envelope level" },
  [PcmdPartialField.OneShotMode]: { key: "oneShotMode", name: "One Shot Mode" },
};

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
