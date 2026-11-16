/**
 * The SYSTEM-or-STUDIO SET pairs
 * ([#284](https://github.com/sigma/integral/issues/284)), as the System page
 * and the Studio Set settings drawer both show them: each side of a pair,
 * dimmed while its System selector puts the other in force, and the
 * read-only readout either shows the side it doesn't edit with.
 */

import type { ReactNode } from "react";
import {
  StudioSetSetting,
  studio_set_field_label,
  studio_set_setting_spec,
  type SystemOrStudioSet,
} from "../pkg/integral_wasm.js";
import { cached } from "./cached";
import type { StudioSetSettings } from "./types";
import { studioSetInForce, type SystemValues } from "./useSystemCommon";
import css from "./systemPairs.module.css";

/** Which side of a pair is dimmed. */
export interface PairDimming {
  studioSet: boolean;
  system: boolean;
}

/** Which side of `pair` is dimmed: none before System Common is read. */
export function pairDimming(
  values: SystemValues | null,
  pair: SystemOrStudioSet,
): PairDimming {
  const studioSet = studioSetInForce(values, pair);
  if (studioSet === null) return { studioSet: false, system: false };
  return { studioSet: !studioSet, system: studioSet };
}

/** One side of a pair, dimmed while not in force. */
export function PairSide({
  dim,
  side,
  className,
  children,
}: {
  dim: boolean;
  side: "studioSet" | "system";
  className?: string;
  children: ReactNode;
}) {
  return (
    <div
      className={[css.side, dim && css.dimmed, className]
        .filter(Boolean)
        .join(" ")}
      data-side={side}
      data-dimmed={dim || undefined}
    >
      {children}
    </div>
  );
}

/** A value shown read-only, named `label`. */
export function Readout({
  label,
  value,
  title,
}: {
  label: string;
  value: string;
  title?: string;
}) {
  return (
    <div className={css.readout} title={title}>
      <span className={css.readoutLabel}>{label}</span>
      <span className={css.readoutValue} aria-label={label}>
        {value}
      </span>
    </div>
  );
}

/** A Studio Set setting's core spec, read once. */
const settingSpec = cached((setting: StudioSetSetting) =>
  studio_set_setting_spec(setting),
);

/** Instance `index` of `setting` in a Studio Set's settings, as its raw value. */
function studioSetValue(
  settings: StudioSetSettings,
  setting: StudioSetSetting,
  index: number,
): number | undefined {
  switch (setting) {
    case StudioSetSetting.ToneControlSource:
      return settings.toneControlSources[index];
    case StudioSetSetting.Tempo:
      return index === 0 ? settings.tempo : undefined;
    case StudioSetSetting.PhaseLock: {
      const on = settings.phaseLock[index];
      return on === undefined ? undefined : Number(on);
    }
  }
}

/**
 * Instance `index` of `setting`, read-only: `—` until the Studio Set is
 * read.
 */
export function StudioSetReadout({
  settings,
  setting,
  index,
}: {
  settings: StudioSetSettings | null;
  setting: StudioSetSetting;
  index: number;
}) {
  const value = settings && studioSetValue(settings, setting, index);
  const label = studio_set_field_label(setting, index);
  return (
    <Readout
      label={label}
      title={`${label} (Studio Set; edited in the Studio Set settings)`}
      value={
        value === null || value === undefined
          ? "—"
          : settingSpec(setting).formatValue(value)
      }
    />
  );
}
