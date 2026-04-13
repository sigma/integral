/**
 * INTEGRA-7 tone category definitions.
 *
 * Sourced from docs/sounds/categories.json (Roland INTEGRA-7 MIDI Implementation).
 */

/** Category ID → display name (e.g. 1 → "Ac.Piano"). */
export const CATEGORIES: Record<number, string> = {
  0: "No assign",
  1: "Ac.Piano",
  5: "E.Piano",
  6: "Organ",
  10: "Other Keyboards",
  12: "Accordion/Harmonica",
  14: "Bell/Mallet",
  16: "Ac.Guitar",
  17: "E.Guitar",
  18: "Dist.Guitar",
  19: "Ac.Bass",
  20: "E.Bass",
  21: "Synth Bass",
  22: "Plucked/Stroke",
  24: "Strings",
  26: "Brass",
  28: "Wind",
  29: "Flute",
  30: "Sax",
  31: "Recorder",
  32: "Vox/Choir",
  34: "Synth Lead",
  35: "Synth Brass",
  36: "Synth Pad/Strings",
  37: "Synth Bellpad",
  38: "Synth PolyKey",
  39: "FX",
  40: "Synth Seq/Pop",
  41: "Phrase",
  42: "Pulsating",
  43: "Beat&Groove",
  44: "Hit",
  45: "Sound FX",
  46: "Drums",
  47: "Percussion",
  48: "Combination",
};

/** Ordered list of defined category IDs (excluding 0 = "No assign"). */
export const CATEGORY_IDS: number[] = Object.keys(CATEGORIES)
  .map(Number)
  .filter((id) => id !== 0)
  .sort((a, b) => a - b);

const ABBREVS: Record<number, string> = {
  1: "PNO",
  5: "EP",
  6: "ORG",
  10: "KEY",
  12: "ACD",
  14: "BEL",
  16: "AGT",
  17: "EGT",
  18: "DGT",
  19: "ABS",
  20: "EBS",
  21: "SBS",
  22: "PLK",
  24: "STR",
  26: "BRS",
  28: "WND",
  29: "FLT",
  30: "SAX",
  31: "REC",
  32: "VOX",
  34: "SLD",
  35: "SBR",
  36: "SPD",
  37: "SBP",
  38: "SPK",
  39: "FX",
  40: "SEQ",
  41: "PHR",
  42: "PLS",
  43: "B&G",
  44: "HIT",
  45: "SFX",
  46: "DRM",
  47: "PRC",
  48: "CMB",
};

/** Short abbreviation for display in tone list columns. */
export function categoryAbbrev(id: number): string {
  return ABBREVS[id] ?? "---";
}
