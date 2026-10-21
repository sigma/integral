/**
 * The MFX parameter Section, keyed by MFX type. Six types have hand layouts
 *; every other type's layout is generated from its params.
 * All of them fit one footprint: six by four knob places, and the output
 * Level fader at the right on the same Cells for every type.
 */

import { FOOTPRINT } from "../../synth-ui/grid";
import { fxFields, isTwoState } from "../fields";
import type { ControlLayout, FaderLayout, SectionBody, VariantSet } from "../types";
import { fxVariantSet } from "./set";

/** One knob's place: a run of switches may share it. */
const PLACE = FOOTPRINT.knobSm;
/** Knob places across. */
const COLS = 6;
/** Knob places down. */
const ROWS = 4;
/** Two-state switches stacked in one knob place. */
const SWITCHES_PER_PLACE = PLACE.h / FOOTPRINT.switch.h;
/** Width of the knob places, in Cells; the Level fader sits right of them. */
const PARAMS_W = COLS * PLACE.w;
/** Height of the Section, in Cells. */
const H = ROWS * PLACE.h;

/** Core's name for the output level every MFX type but Thru ends with. */
export const MFX_LEVEL = "Level";

/** The output level: a full-height fader right of the knob places. */
const LEVEL_FADER: FaderLayout<string> = { kind: "fader", field: MFX_LEVEL, x: PARAMS_W, y: 0, h: H };

/**
 * A type's generated layout: Level on its fader, every other param in core's
 * order through the knob places, row by row. A knob takes a place; a run of
 * two-state params stacks as switches, up to three to a place.
 */
function generate(type: number): SectionBody<string> {
  const fields = fxFields("mfx", type);
  const items: ControlLayout<string>[] = [];
  let place = 0;
  let stacked = 0;
  const origin = () => ({ x: (place % COLS) * PLACE.w, y: Math.floor(place / COLS) * PLACE.h });
  for (const field of fields.ids) {
    if (field === MFX_LEVEL) {
      items.push(LEVEL_FADER);
    } else if (isTwoState(fields, field)) {
      const { x, y } = origin();
      items.push({ kind: "switch", field, x, y: y + stacked });
      stacked++;
      if (stacked === SWITCHES_PER_PLACE) {
        place++;
        stacked = 0;
      }
    } else {
      if (stacked > 0) {
        place++;
        stacked = 0;
      }
      items.push({ kind: "knob", field, ...origin() });
      place++;
    }
  }
  return { items };
}

/**
 * Equalizer, in the mixer EQ's idiom (`../mixer/eq.ts`): one knob per 2-Cell
 * row, zig-zagging between two columns, frequency and Q left, gain right, each
 * band starting in the column the previous one ended. Four bands don't fit the
 * footprint's height in one zig-zag, so HI and MID2 run down the left pair of
 * columns and MID1 and LO down the right pair. Diagonal rules divide the bands
 * within a pair, a straight one divides the pairs; band names sit in the free
 * slot beside a knob.
 */
const EQUALIZER: SectionBody<string> = {
  items: [
    { kind: "knob", field: "High Freq", x: 0, y: 0, label: "Freq" },
    { kind: "knob", field: "High Gain", x: 2, y: 2, label: "Gain" },
    { kind: "knob", field: "Mid2 Freq", x: 0, y: 4, label: "Freq" },
    { kind: "knob", field: "Mid2 Gain", x: 2, y: 6, label: "Gain" },
    { kind: "knob", field: "Mid2 Q", x: 0, y: 8, label: "Q" },
    { kind: "knob", field: "Mid1 Freq", x: 6, y: 0, label: "Freq" },
    { kind: "knob", field: "Mid1 Gain", x: 8, y: 2, label: "Gain" },
    { kind: "knob", field: "Mid1 Q", x: 6, y: 4, label: "Q" },
    { kind: "knob", field: "Low Gain", x: 8, y: 6, label: "Gain" },
    { kind: "knob", field: "Low Freq", x: 6, y: 8, label: "Freq" },
    LEVEL_FADER,
  ],
  // Rules run through the column centres and the gaps between knobs.
  silkscreen: [
    { kind: "rule", points: [[0, 3.5], [1, 3.5], [3, 5.5], [4, 5.5]] },
    { kind: "rule", points: [[5, 0], [5, H]] },
    { kind: "rule", points: [[6, 7.5], [7, 7.5], [9, 5.5], [10, 5.5]] },
    { kind: "label", text: "HI", x: 3, y: 1 },
    { kind: "label", text: "MID2", x: 1, y: 7.5 },
    { kind: "label", text: "MID1", x: 7, y: 3.5 },
    { kind: "label", text: "LO", x: 9, y: 10 },
  ],
};

/** Chorus: filter and modulation, then delay and mix, then the tone EQ. */
const CHORUS: SectionBody<string> = {
  items: [
    { kind: "knob", field: "Filter Type", x: 0, y: 0, label: "Filter" },
    { kind: "knob", field: "Cutoff Freq", x: 2, y: 0, label: "Cutoff" },
    { kind: "knob", field: "Rate", x: 6, y: 0 },
    { kind: "knob", field: "Depth", x: 8, y: 0 },
    { kind: "knob", field: "Pre Delay", x: 0, y: 3, label: "Pre Dly" },
    { kind: "knob", field: "Phase", x: 2, y: 3 },
    { kind: "knob", field: "Balance", x: 6, y: 3 },
    { kind: "knob", field: "Low Gain", x: 0, y: 6, label: "Lo Gain" },
    { kind: "knob", field: "High Gain", x: 2, y: 6, label: "Hi Gain" },
    LEVEL_FADER,
  ],
};

/** Overdrive and Distortion: drive and tone, the amp, pan, then the tone EQ. */
const DRIVE: SectionBody<string> = {
  items: [
    { kind: "knob", field: "Drive", x: 0, y: 0 },
    { kind: "knob", field: "Tone", x: 2, y: 0 },
    { kind: "switch", field: "Amp Sw", x: 6, y: 1, label: "Amp" },
    { kind: "knob", field: "Amp Type", x: 8, y: 0, label: "Type" },
    { kind: "knob", field: "Pan", x: 0, y: 3 },
    { kind: "knob", field: "Low Gain", x: 0, y: 6, label: "Lo Gain" },
    { kind: "knob", field: "High Gain", x: 2, y: 6, label: "Hi Gain" },
    LEVEL_FADER,
  ],
};

/** Compressor: the tone EQ over the attack, threshold and gain faders. */
const COMPRESSOR: SectionBody<string> = {
  items: [
    { kind: "knob", field: "Low Gain", x: 0, y: 0, label: "Lo Gain" },
    { kind: "knob", field: "High Gain", x: 2, y: 0, label: "Hi Gain" },
    { kind: "fader", field: "Attack", x: 0, y: 3, h: H - 3, label: "Atk" },
    { kind: "fader", field: "Threshold", x: 2, y: 3, h: H - 3, label: "Thr" },
    { kind: "fader", field: "Post Gain", x: 4, y: 3, h: H - 3, label: "Gain" },
    LEVEL_FADER,
  ],
};

/** Delay: phase and feedback switches, feedback and mix, the tone EQ, and the delay times as faders. */
const DELAY: SectionBody<string> = {
  items: [
    { kind: "switch", field: "Phase Left", x: 0, y: 0, label: "Ph L" },
    { kind: "switch", field: "Phase Right", x: 2, y: 0, label: "Ph R" },
    { kind: "switch", field: "Feedback Mode", x: 4, y: 0, label: "FB" },
    { kind: "knob", field: "Feedback", x: 0, y: 1, label: "Fdbk" },
    { kind: "knob", field: "HF Damp", x: 2, y: 1 },
    { kind: "knob", field: "Balance", x: 4, y: 1 },
    { kind: "knob", field: "Low Gain", x: 0, y: 4, label: "Lo Gain" },
    { kind: "knob", field: "High Gain", x: 2, y: 4, label: "Hi Gain" },
    { kind: "fader", field: "Delay Left", x: 7, y: 0, h: H, label: "Time L" },
    { kind: "fader", field: "Delay Right", x: 9, y: 0, h: H, label: "Time R" },
    LEVEL_FADER,
  ],
};

/** The MFX parameter Section, keyed by MFX type. */
export const MFX_SET: VariantSet<number, string> = fxVariantSet(
  "mfx",
  { title: "Parameters", titleEdge: "side", w: PARAMS_W + FOOTPRINT.faderW, h: H },
  new Map([
    [1, EQUALIZER],
    [22, CHORUS],
    [28, DRIVE],
    [29, DRIVE],
    [31, COMPRESSOR],
    [34, DELAY],
  ]),
  generate,
);
