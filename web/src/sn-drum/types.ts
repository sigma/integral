import { SndCommonField, SndNoteField } from "../../pkg/integral_wasm.js";
import type { FieldEntry } from "../fieldSpec";

// ---------------------------------------------------------------------------
// Types mirroring the Rust serde output (SnDrumCommon, SnDrumNote)
// ---------------------------------------------------------------------------

export interface SnDrumCommon {
  kitName: string;
  kitLevel: number;
  ambienceLevel: number;
  phraseNumber: number;
  tfxSwitch: number;
}

export interface SnDrumNote {
  instNumber: number;
  level: number;
  pan: number;
  chorusSend: number;
  reverbSend: number;
  tune: number;
  attack: number;
  decay: number;
  brilliance: number;
  variation: number;
  dynamicRange: number;
  stereoWidth: number;
  outputAssign: number;
}

/** Every SN Drum Kit Common field in {@link SnDrumCommon}. */
export const SND_COMMON_ENTRIES: Record<SndCommonField, FieldEntry<SnDrumCommon>> = {
  [SndCommonField.KitLevel]: { key: "kitLevel", name: "Kit Level" },
  [SndCommonField.AmbienceLevel]: { key: "ambienceLevel", name: "Ambience Level" },
  [SndCommonField.PhraseNumber]: { key: "phraseNumber", name: "Phrase Number" },
  [SndCommonField.TfxSwitch]: { key: "tfxSwitch", name: "TFX Switch" },
};

/** Every SN Drum Kit Note field in {@link SnDrumNote}. */
export const SND_NOTE_ENTRIES: Record<SndNoteField, FieldEntry<SnDrumNote>> = {
  [SndNoteField.InstNumber]: { key: "instNumber", name: "Inst" },
  [SndNoteField.Level]: { key: "level", name: "Level" },
  [SndNoteField.Pan]: { key: "pan", name: "Pan" },
  [SndNoteField.ChorusSend]: { key: "chorusSend", name: "Chorus Send" },
  [SndNoteField.ReverbSend]: { key: "reverbSend", name: "Reverb Send" },
  [SndNoteField.Tune]: { key: "tune", name: "Tune" },
  [SndNoteField.Attack]: { key: "attack", name: "Attack" },
  [SndNoteField.Decay]: { key: "decay", name: "Decay" },
  [SndNoteField.Brilliance]: { key: "brilliance", name: "Brilliance" },
  [SndNoteField.Variation]: { key: "variation", name: "Variation" },
  [SndNoteField.DynamicRange]: { key: "dynamicRange", name: "Dynamic Range" },
  [SndNoteField.StereoWidth]: { key: "stereoWidth", name: "Stereo Width" },
  [SndNoteField.OutputAssign]: { key: "outputAssign", name: "Output" },
};
