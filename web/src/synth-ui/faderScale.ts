/**
 * A level fader's scale: the marks it draws beside its track and their
 * labels. One rule for every fader that draws one, the mixer's FX faders'
 * (pre-Grid `VolumeFader`), so the level faders read alike (#392).
 */

/** One mark on a fader's scale. */
export interface ScaleMark {
  /** The raw value it marks. */
  value: number;
  /** What it reads. */
  label: string;
  /** Whether it is the 0 dB mark, drawn emphasised. */
  zero: boolean;
}

/** The marks of a 0–127 level, the MIDI ruler. */
const MIDI_MARKS = [127, 100, 80, 60, 40, 20, 0];

/**
 * The 0–127 level the scale marks 0 dB, as the pre-Grid mixer's faders did.
 * Core has no dB for these raw levels; this is that UI's convention, kept
 * in this one place.
 */
export const LEVEL_0DB = 100;

/** What a scale is drawn for: the field's range, its formatter and its nominal value. */
export interface ScaleOf {
  /** Minimum raw value. */
  min: number;
  /** Maximum raw value. */
  max: number;
  /** A raw value's label; default its number. */
  format?: (value: number) => string;
  /** The 0 dB value of a range that isn't the 0–127 ruler (a dB field's 0). */
  nominal?: number;
}

/**
 * The scale of a fader over `min`–`max`: the MIDI ruler for 0–127, its 0 dB
 * mark at `LEVEL_0DB`, else quarters of the range labelled by `format`, the
 * mark at `nominal` emphasised.
 */
export function faderScale({ min, max, format = String, nominal }: ScaleOf): ScaleMark[] {
  const midi = min === 0 && max === 127;
  const zero = midi ? LEVEL_0DB : nominal;
  const values = midi ? MIDI_MARKS : [0, 0.25, 0.5, 0.75, 1].map((f) => Math.round(max - f * (max - min)));
  return values.map((value) => ({
    value,
    zero: value === zero,
    label: midi && value === zero ? "0dB" : format(value),
  }));
}
