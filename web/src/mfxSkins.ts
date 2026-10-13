import { isSwitch, type ParamLook, type SpecParam } from "./ParamControl";

/**
 * MFX panel skins: layout only. A skin names spec params; ranges, formats,
 * slots and tempo sync come from the core. Params a skin leaves out still
 * appear, in the default layout below it.
 */

/** A param by its spec name, optionally with a look. */
export type SkinItem = string | ({ param: string } & ParamLook);

/** A row of controls. */
export interface SkinRow {
  items: SkinItem[];
}

/** One band of the parametric EQ display. */
export interface EqBandSkin {
  label: string;
  gain: string;
  freq: string;
  q?: string;
}

/** Layout of one MFX type's panel. */
export interface Skin {
  eq?: EqBandSkin[];
  rows: SkinRow[];
}

/** Params the shared output section draws. */
export const OUTPUT_PARAMS = new Set(["Level"]);
const TONE_EQ = new Set(["Low Gain", "High Gain"]);
const BALANCE = new Set(["Balance", "Mix"]);

const TONE_ROW: SkinRow = {
  items: [
    { param: "Low Gain", label: "Lo Gain" },
    { param: "High Gain", label: "Hi Gain" },
  ],
};

const DRIVE: Skin = {
  rows: [
    { items: ["Drive", "Tone"] },
    { items: [{ param: "Amp Sw", label: "Amp" }, { param: "Amp Type", label: "Type" }] },
    { items: ["Pan"] },
    TONE_ROW,
  ],
};

/** Hand-made skins by MFX type; every other type uses `defaultRows`. */
export const MFX_SKINS: Record<number, Skin> = {
  1: {
    eq: [
      { label: "HI", gain: "High Gain", freq: "High Freq" },
      { label: "MID2", gain: "Mid2 Gain", freq: "Mid2 Freq", q: "Mid2 Q" },
      { label: "MID1", gain: "Mid1 Gain", freq: "Mid1 Freq", q: "Mid1 Q" },
      { label: "LO", gain: "Low Gain", freq: "Low Freq" },
    ],
    rows: [],
  },
  22: {
    rows: [
      { items: [{ param: "Filter Type", label: "Filter" }, { param: "Cutoff Freq", label: "Cutoff" }] },
      { items: ["Rate", "Depth"] },
      { items: [{ param: "Pre Delay", label: "Pre Dly" }, "Phase"] },
      { items: ["Balance"] },
      TONE_ROW,
    ],
  },
  28: DRIVE,
  29: DRIVE,
  31: {
    rows: [
      TONE_ROW,
      {
        items: [
          { param: "Attack", label: "Atk", as: "fader" },
          { param: "Threshold", label: "Thr", as: "fader" },
          { param: "Post Gain", label: "Gain", as: "fader" },
        ],
      },
    ],
  },
  34: {
    rows: [
      {
        items: [
          { param: "Phase Left", label: "Ph L" },
          { param: "Phase Right", label: "Ph R" },
          { param: "Feedback Mode", label: "FB" },
        ],
      },
      {
        items: [
          { param: "Feedback", label: "Fdbk" },
          "HF Damp",
          "Balance",
        ],
      },
      TONE_ROW,
      {
        items: [
          { param: "Delay Left", label: "Time L", as: "fader" },
          { param: "Delay Right", label: "Time R", as: "fader" },
        ],
      },
    ],
  },
};

/** Spec name of a skin item. */
export function itemParam(item: SkinItem): string {
  return typeof item === "string" ? item : item.param;
}

/** Every spec name a skin refers to. */
export function skinParams(skin: Skin): string[] {
  return [
    ...(skin.eq ?? []).flatMap((b) => [b.gain, b.freq, ...(b.q ? [b.q] : [])]),
    ...skin.rows.flatMap((r) => r.items.map(itemParam)),
  ];
}

/** Default layout: switches, then knobs in pairs, then balance, then the tone EQ. */
export function defaultRows(params: SpecParam[]): SkinRow[] {
  const names = (ps: SpecParam[]) => ps.map((p) => p.def.name);
  const rest = params.filter((p) => !OUTPUT_PARAMS.has(p.def.name));
  const toneEq = rest.filter((p) => TONE_EQ.has(p.def.name));
  const balance = rest.filter((p) => BALANCE.has(p.def.name));
  const others = rest.filter((p) => !TONE_EQ.has(p.def.name) && !BALANCE.has(p.def.name));
  const switches = others.filter((p) => isSwitch(p.def));
  const knobs = others.filter((p) => !isSwitch(p.def));
  const rows: SkinRow[] = [];
  if (switches.length > 0) rows.push({ items: names(switches) });
  for (let i = 0; i < knobs.length; i += 2) rows.push({ items: names(knobs.slice(i, i + 2)) });
  if (balance.length > 0) rows.push({ items: names(balance) });
  if (toneEq.length > 0) rows.push({ items: names(toneEq) });
  return rows;
}

/** The skin for `params` of one type: its hand-made rows, then the default layout of what they leave out. */
export function resolveSkin(mfxType: number, params: SpecParam[]): Skin {
  const skin = MFX_SKINS[mfxType] ?? { rows: [] };
  const used = new Set(skinParams(skin));
  return { ...skin, rows: [...skin.rows, ...defaultRows(params.filter((p) => !used.has(p.def.name)))] };
}
