/**
 * SN Synth Common, drawn from its layouts (`layouts/sn-synth`): the COMMON
 * Section, and the switch strip heading each partial's row, as the Partial
 * Switches are Common fields.
 */

import { useMemo } from "react";
import {
  ParameterFamily,
  sns_common_spec,
  sns_common_value,
  sns_partial_switch_field,
  type SnsCommonField,
} from "../../pkg/integral_wasm.js";
import { cached } from "../cached";
import { GridSection, specBinding, type Binding } from "../layouts";
import { SNS_COMMON_FIELDS, SNS_COMMON_LAYOUT } from "../layouts/sn-synth/common";
import { SNS_PARTIAL_SWITCH_LAYOUT } from "../layouts/sn-synth/partial";
import type { CommonParamHandler, SnSynthCommon } from "./types";

/** Each Common field's spec, read from core once. */
const commonSpec = cached(sns_common_spec);

/** `field`'s binding at `value`, writing through `onChange`. */
function commonBinding(field: SnsCommonField, value: number, onChange: CommonParamHandler): Binding {
  return specBinding(SNS_COMMON_FIELDS.nameOf(field), commonSpec(field), value, (v) => onChange(field, v));
}

/** Props of {@link CommonSection}. */
export interface CommonSectionProps {
  /** The tone's Common, as core serialized it. */
  common: SnSynthCommon;
  /** Writes one of its fields. */
  onChange: CommonParamHandler;
}

/** The SN Synth COMMON Section. */
export function CommonSection({ common, onChange }: CommonSectionProps) {
  const values = useMemo(
    () => new Map(SNS_COMMON_FIELDS.ids.map((f) => [f, sns_common_value(f, common)])),
    [common],
  );
  const bind = (field: SnsCommonField): Binding => commonBinding(field, values.get(field) ?? 0, onChange);
  return <GridSection layout={SNS_COMMON_LAYOUT} bind={bind} family={ParameterFamily.Common} />;
}

/** Props of {@link PartialSwitch}. */
export interface PartialSwitchProps extends CommonSectionProps {
  /** Which partial, 0-based: the strip's Scope. */
  idx: number;
}

/** The strip heading a partial's row: its on/off switch, live whether the partial is on or off. */
export function PartialSwitch({ idx, common, onChange }: PartialSwitchProps) {
  // The layout names Partial 1's switch; this Scope binds its own.
  const field = sns_partial_switch_field(idx);
  const binding = commonBinding(field, sns_common_value(field, common), onChange);
  return <GridSection layout={SNS_PARTIAL_SWITCH_LAYOUT} scope={{ partial: idx }} bind={() => binding} />;
}
