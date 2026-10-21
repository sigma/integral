import { PcmsCommonField, PcmsPartialField, PcmsPmtField } from "../../pkg/integral_wasm.js";
import type { FieldEntry } from "../fieldSpec";

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
  reserve0c: number;
  reserve1f: number;
  reserve27: number;
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

/** Sets a Common field. */
export type CommonParamHandler = (field: PcmsCommonField, value: number) => void;

/** Sets a PMT field common to all Partials. */
export type PmtParamHandler = (field: PcmsPmtField, value: number) => void;

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

/** Sets value `index` (0 unless the field holds a list) of a Partial field. */
export type PartialParamHandler = (field: PcmsPartialField, index: number, value: number) => void;

/** Every Common field the editor places, in {@link PcmSynthCommon}. */
export const PCMS_COMMON_ENTRIES: Partial<Record<PcmsCommonField, FieldEntry<PcmSynthCommon>>> = {
  [PcmsCommonField.ToneLevel]: { key: "toneLevel", name: "Tone level" },
  [PcmsCommonField.TonePan]: { key: "tonePan", name: "Tone pan" },
  [PcmsCommonField.TonePriority]: { key: "tonePriority", name: "Tone priority" },
  [PcmsCommonField.CoarseTune]: { key: "coarseTune", name: "Coarse tune (semitones)" },
  [PcmsCommonField.FineTune]: { key: "fineTune", name: "Fine tune (cents)" },
  [PcmsCommonField.OctaveShift]: { key: "octaveShift", name: "Octave shift" },
  [PcmsCommonField.StretchTuneDepth]: { key: "stretchTuneDepth", name: "Stretch tune depth" },
  [PcmsCommonField.AnalogFeel]: { key: "analogFeel", name: "Analog feel: subtle pitch and timing randomness, like a vintage synth" },
  [PcmsCommonField.MonoPoly]: { key: "monoPoly", name: "Voice mode: monophonic or polyphonic" },
  [PcmsCommonField.LegatoSwitch]: { key: "legatoSwitch", name: "Legato: overlapping notes share one voice" },
  [PcmsCommonField.LegatoRetrigger]: { key: "legatoRetrigger", name: "Legato retrigger" },
  [PcmsCommonField.PortamentoSwitch]: { key: "portamentoSwitch", name: "Portamento: a smooth pitch glide between notes" },
  [PcmsCommonField.PortamentoMode]: { key: "portamentoMode", name: "Portamento mode" },
  [PcmsCommonField.PortamentoType]: { key: "portamentoType", name: "Portamento type" },
  [PcmsCommonField.PortamentoStart]: { key: "portamentoStart", name: "Portamento start" },
  [PcmsCommonField.PortamentoTime]: { key: "portamentoTime", name: "Portamento time: the speed of the glide" },
  [PcmsCommonField.CutoffOffset]: { key: "cutoffOffset", name: "Cutoff offset: shifts every partial's filter cutoff" },
  [PcmsCommonField.ResonanceOffset]: { key: "resonanceOffset", name: "Resonance offset: shifts every partial's resonance" },
  [PcmsCommonField.AttackTimeOffset]: { key: "attackTimeOffset", name: "Attack time offset: shifts every partial's envelope attack" },
  [PcmsCommonField.ReleaseTimeOffset]: { key: "releaseTimeOffset", name: "Release time offset: shifts every partial's envelope release" },
  [PcmsCommonField.VelocitySensOffset]: { key: "velocitySensOffset", name: "Velocity sensitivity offset: shifts every partial's velocity response" },
  [PcmsCommonField.PmtControlSwitch]: { key: "pmtControlSwitch", name: "PMT control switch" },
  [PcmsCommonField.PitchBendRangeUp]: { key: "pitchBendRangeUp", name: "Pitch bend range up (semitones)" },
  [PcmsCommonField.PitchBendRangeDown]: { key: "pitchBendRangeDown", name: "Pitch bend range down (semitones)" },
};

/** The PMT fields common to all Partials, in {@link PcmSynthPmt}. */
export const PCMS_PMT_ENTRIES: Partial<Record<PcmsPmtField, FieldEntry<PcmSynthPmt>>> = {
  [PcmsPmtField.StructureType12]: { key: "structureType12", name: "Structure 1&2: how partials 1 and 2 are wired" },
  [PcmsPmtField.Booster12]: { key: "booster12", name: "Booster 1&2" },
  [PcmsPmtField.StructureType34]: { key: "structureType34", name: "Structure 3&4: how partials 3 and 4 are wired" },
  [PcmsPmtField.Booster34]: { key: "booster34", name: "Booster 3&4" },
  [PcmsPmtField.PmtVelocityControl]: { key: "pmtVelocityControl", name: "Velocity control: how key velocity selects partials" },
};

/** Every Partial field in {@link PcmSynthPartial}. */
export const PCMS_PARTIAL_ENTRIES: Record<PcmsPartialField, FieldEntry<PcmSynthPartial, PartialFieldKey>> = {
  [PcmsPartialField.Level]: { key: "level", name: "Partial level" },
  [PcmsPartialField.CoarseTune]: { key: "coarseTune", name: "Coarse tune (semitones)" },
  [PcmsPartialField.FineTune]: { key: "fineTune", name: "Fine tune (cents)" },
  [PcmsPartialField.RandomPitchDepth]: { key: "randomPitchDepth", name: "Random pitch depth: variation on each note (cents)" },
  [PcmsPartialField.Pan]: { key: "pan", name: "Partial pan" },
  [PcmsPartialField.PanKeyfollow]: { key: "panKeyfollow", name: "Pan keyfollow: notes above C4 pan right (+) or left (−)" },
  [PcmsPartialField.RandomPanDepth]: { key: "randomPanDepth", name: "Random pan depth: each note lands at a random position" },
  [PcmsPartialField.AlternatePanDepth]: { key: "alternatePanDepth", name: "Alternate pan depth: successive notes alternate left and right" },
  [PcmsPartialField.EnvMode]: { key: "envMode", name: "Envelope mode: NO-SUS plays once, SUSTAIN holds at the sustain level" },
  [PcmsPartialField.DelayMode]: { key: "delayMode", name: "Delay mode: when the partial sounds relative to note-on" },
  [PcmsPartialField.DelayTime]: { key: "delayTime", name: "Delay time before the partial sounds" },
  [PcmsPartialField.OutputLevel]: { key: "outputLevel", name: "Output level" },
  [PcmsPartialField.ChorusSend]: { key: "chorusSend", name: "Chorus send" },
  [PcmsPartialField.ReverbSend]: { key: "reverbSend", name: "Reverb send" },
  [PcmsPartialField.ReceiveBender]: { key: "receiveBender", name: "Receive pitch bend" },
  [PcmsPartialField.ReceiveExpression]: { key: "receiveExpression", name: "Receive expression (CC11)" },
  [PcmsPartialField.ReceiveHold1]: { key: "receiveHold1", name: "Receive hold (CC64)" },
  [PcmsPartialField.RedamperSwitch]: { key: "redamperSwitch", name: "Redamper switch" },
  [PcmsPartialField.Control1Switch]: { key: "control1Switches", name: "Control 1 switch" },
  [PcmsPartialField.Control2Switch]: { key: "control2Switches", name: "Control 2 switch" },
  [PcmsPartialField.Control3Switch]: { key: "control3Switches", name: "Control 3 switch" },
  [PcmsPartialField.Control4Switch]: { key: "control4Switches", name: "Control 4 switch" },
  [PcmsPartialField.WaveGroupType]: { key: "waveGroupType", name: "Wave group type" },
  [PcmsPartialField.WaveGroupId]: { key: "waveGroupId", name: "Wave group ID" },
  [PcmsPartialField.WaveNumberL]: { key: "waveNumberL", name: "Wave number L (mono)" },
  [PcmsPartialField.WaveNumberR]: { key: "waveNumberR", name: "Wave number R" },
  [PcmsPartialField.WaveGain]: { key: "waveGain", name: "Wave gain" },
  [PcmsPartialField.WaveFxmSwitch]: { key: "waveFxmSwitch", name: "FXM: frequency cross modulation" },
  [PcmsPartialField.WaveFxmColor]: { key: "waveFxmColor", name: "FXM color: its tonal character" },
  [PcmsPartialField.WaveFxmDepth]: { key: "waveFxmDepth", name: "FXM depth: its intensity" },
  [PcmsPartialField.WaveTempoSync]: { key: "waveTempoSync", name: "Wave tempo sync: play the wave in time with the song" },
  [PcmsPartialField.WavePitchKeyfollow]: { key: "wavePitchKeyfollow", name: "Pitch keyfollow" },
  [PcmsPartialField.PitchEnvDepth]: { key: "pitchEnvDepth", name: "Pitch envelope depth (semitones)" },
  [PcmsPartialField.PitchEnvVelocitySens]: { key: "pitchEnvVelocitySens", name: "Pitch envelope velocity sensitivity" },
  [PcmsPartialField.PitchEnvT1VelocitySens]: { key: "pitchEnvT1VelocitySens", name: "Pitch envelope Time 1 velocity sensitivity" },
  [PcmsPartialField.PitchEnvT4VelocitySens]: { key: "pitchEnvT4VelocitySens", name: "Pitch envelope Time 4 velocity sensitivity" },
  [PcmsPartialField.PitchEnvTimeKeyfollow]: { key: "pitchEnvTimeKeyfollow", name: "Pitch envelope time keyfollow: higher keys, faster envelope" },
  [PcmsPartialField.PitchEnvTime]: { key: "pitchEnvTime", name: "Pitch envelope time" },
  [PcmsPartialField.PitchEnvLevel]: { key: "pitchEnvLevel", name: "Pitch envelope level" },
  [PcmsPartialField.TvfFilterType]: { key: "tvfFilterType", name: "Filter type" },
  [PcmsPartialField.TvfCutoffFrequency]: { key: "tvfCutoffFrequency", name: "Filter cutoff frequency" },
  [PcmsPartialField.TvfCutoffKeyfollow]: { key: "tvfCutoffKeyfollow", name: "Cutoff keyfollow: higher keys, higher cutoff" },
  [PcmsPartialField.TvfCutoffVelocityCurve]: { key: "tvfCutoffVelocityCurve", name: "Cutoff velocity curve" },
  [PcmsPartialField.TvfCutoffVelocitySens]: { key: "tvfCutoffVelocitySens", name: "Cutoff velocity sensitivity" },
  [PcmsPartialField.TvfResonance]: { key: "tvfResonance", name: "Filter resonance" },
  [PcmsPartialField.TvfResonanceVelocitySens]: { key: "tvfResonanceVelocitySens", name: "Resonance velocity sensitivity" },
  [PcmsPartialField.TvfEnvDepth]: { key: "tvfEnvDepth", name: "Filter envelope depth" },
  [PcmsPartialField.TvfEnvVelocityCurve]: { key: "tvfEnvVelocityCurve", name: "Filter envelope velocity curve" },
  [PcmsPartialField.TvfEnvVelocitySens]: { key: "tvfEnvVelocitySens", name: "Filter envelope velocity sensitivity" },
  [PcmsPartialField.TvfEnvT1VelocitySens]: { key: "tvfEnvT1VelocitySens", name: "Filter envelope Time 1 velocity sensitivity" },
  [PcmsPartialField.TvfEnvT4VelocitySens]: { key: "tvfEnvT4VelocitySens", name: "Filter envelope Time 4 velocity sensitivity" },
  [PcmsPartialField.TvfEnvTimeKeyfollow]: { key: "tvfEnvTimeKeyfollow", name: "Filter envelope time keyfollow: higher keys, faster envelope" },
  [PcmsPartialField.TvfEnvTime]: { key: "tvfEnvTime", name: "Filter envelope time" },
  [PcmsPartialField.TvfEnvLevel]: { key: "tvfEnvLevel", name: "Filter envelope level" },
  [PcmsPartialField.TvaBiasLevel]: { key: "tvaBiasLevel", name: "Bias level: how steeply volume changes away from the bias key" },
  [PcmsPartialField.TvaBiasPosition]: { key: "tvaBiasPosition", name: "Bias key: the pivot the volume change is measured from" },
  [PcmsPartialField.TvaBiasDirection]: { key: "tvaBiasDirection", name: "Bias direction: the keys the change applies to" },
  [PcmsPartialField.TvaLevelVelocityCurve]: { key: "tvaLevelVelocityCurve", name: "Level velocity curve" },
  [PcmsPartialField.TvaLevelVelocitySens]: { key: "tvaLevelVelocitySens", name: "Level velocity sensitivity" },
  [PcmsPartialField.TvaEnvT1VelocitySens]: { key: "tvaEnvT1VelocitySens", name: "Amp envelope Time 1 velocity sensitivity" },
  [PcmsPartialField.TvaEnvT4VelocitySens]: { key: "tvaEnvT4VelocitySens", name: "Amp envelope Time 4 velocity sensitivity" },
  [PcmsPartialField.TvaEnvTimeKeyfollow]: { key: "tvaEnvTimeKeyfollow", name: "Amp envelope time keyfollow: higher keys, faster envelope" },
  [PcmsPartialField.TvaEnvTime]: { key: "tvaEnvTime", name: "Amp envelope time" },
  [PcmsPartialField.TvaEnvLevel]: { key: "tvaEnvLevel", name: "Amp envelope level" },
  [PcmsPartialField.Lfo1Waveform]: { key: "lfo1Waveform", name: "LFO1 waveform" },
  [PcmsPartialField.Lfo1Rate]: { key: "lfo1Rate", name: "LFO1 rate" },
  [PcmsPartialField.Lfo1Offset]: { key: "lfo1Offset", name: "LFO1 offset" },
  [PcmsPartialField.Lfo1RateDetune]: { key: "lfo1RateDetune", name: "LFO1 rate detune: random variation" },
  [PcmsPartialField.Lfo1DelayTime]: { key: "lfo1DelayTime", name: "LFO1 delay: time before it starts after note-on" },
  [PcmsPartialField.Lfo1DelayTimeKeyfollow]: { key: "lfo1DelayTimeKeyfollow", name: "LFO1 delay keyfollow: higher keys, shorter delay" },
  [PcmsPartialField.Lfo1FadeMode]: { key: "lfo1FadeMode", name: "LFO1 fade mode" },
  [PcmsPartialField.Lfo1FadeTime]: { key: "lfo1FadeTime", name: "LFO1 fade time" },
  [PcmsPartialField.Lfo1KeyTrigger]: { key: "lfo1KeyTrigger", name: "LFO1 key trigger: restart on each note" },
  [PcmsPartialField.Lfo1PitchDepth]: { key: "lfo1PitchDepth", name: "LFO1 pitch depth (vibrato)" },
  [PcmsPartialField.Lfo1TvfDepth]: { key: "lfo1TvfDepth", name: "LFO1 filter depth (wah)" },
  [PcmsPartialField.Lfo1TvaDepth]: { key: "lfo1TvaDepth", name: "LFO1 amp depth (tremolo)" },
  [PcmsPartialField.Lfo1PanDepth]: { key: "lfo1PanDepth", name: "LFO1 pan depth (auto-pan)" },
  [PcmsPartialField.Lfo2Waveform]: { key: "lfo2Waveform", name: "LFO2 waveform" },
  [PcmsPartialField.Lfo2Rate]: { key: "lfo2Rate", name: "LFO2 rate" },
  [PcmsPartialField.Lfo2Offset]: { key: "lfo2Offset", name: "LFO2 offset" },
  [PcmsPartialField.Lfo2RateDetune]: { key: "lfo2RateDetune", name: "LFO2 rate detune: random variation" },
  [PcmsPartialField.Lfo2DelayTime]: { key: "lfo2DelayTime", name: "LFO2 delay: time before it starts after note-on" },
  [PcmsPartialField.Lfo2DelayTimeKeyfollow]: { key: "lfo2DelayTimeKeyfollow", name: "LFO2 delay keyfollow: higher keys, shorter delay" },
  [PcmsPartialField.Lfo2FadeMode]: { key: "lfo2FadeMode", name: "LFO2 fade mode" },
  [PcmsPartialField.Lfo2FadeTime]: { key: "lfo2FadeTime", name: "LFO2 fade time" },
  [PcmsPartialField.Lfo2KeyTrigger]: { key: "lfo2KeyTrigger", name: "LFO2 key trigger: restart on each note" },
  [PcmsPartialField.Lfo2PitchDepth]: { key: "lfo2PitchDepth", name: "LFO2 pitch depth (vibrato)" },
  [PcmsPartialField.Lfo2TvfDepth]: { key: "lfo2TvfDepth", name: "LFO2 filter depth (wah)" },
  [PcmsPartialField.Lfo2TvaDepth]: { key: "lfo2TvaDepth", name: "LFO2 amp depth (tremolo)" },
  [PcmsPartialField.Lfo2PanDepth]: { key: "lfo2PanDepth", name: "LFO2 pan depth (auto-pan)" },
  [PcmsPartialField.LfoStepType]: { key: "lfoStepType", name: "Step LFO shape: hard steps or smooth; LFO1 or LFO2 plays it with the STEP waveform" },
  [PcmsPartialField.LfoStep]: { key: "lfoStepValues", name: "Step LFO step" },
};
