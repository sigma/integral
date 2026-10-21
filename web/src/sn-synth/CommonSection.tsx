/** SN Synth Common, drawn from its layout (`layouts/sn-synth/common`). */

import { useMemo } from "react";
import { ParameterFamily, sns_common_spec, sns_common_value, type SnsCommonField } from "../../pkg/integral_wasm.js";
import { cached } from "../cached";
import { GridSection, specBinding, type Binding } from "../layouts";
import { SNS_COMMON_LAYOUT } from "../layouts/sn-synth/common";
import type { CommonParamHandler, SnSynthCommon } from "./types";

/** Each Common field's spec, read from core once. */
const commonSpec = cached(sns_common_spec);

/** Props of {@link CommonSection}. */
export interface CommonSectionProps {
  /** The tone's Common, as core serialized it. */
  common: SnSynthCommon;
  /** Writes one of its fields. */
  onChange: CommonParamHandler;
}

/** The SN Synth Common Section. */
export function CommonSection({ common, onChange }: CommonSectionProps) {
  const values = useMemo(
    () => new Map(SNS_COMMON_LAYOUT.fields.ids.map((f) => [f, sns_common_value(f, common)])),
    [common],
  );
  const bind = (field: SnsCommonField): Binding =>
    specBinding(SNS_COMMON_LAYOUT.fields.nameOf(field), commonSpec(field), values.get(field) ?? 0, (v) =>
      onChange(field, v),
    );
  return <GridSection layout={SNS_COMMON_LAYOUT} bind={bind} family={ParameterFamily.Common} />;
}
