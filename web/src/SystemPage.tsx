/**
 * The System page ([#603](https://github.com/sigma/integral/issues/603), PRD
 * [#601](https://github.com/sigma/integral/issues/601)): all of System
 * Common, in the core registry's groups and order, read when the page opens
 * and on Refresh. Its edits last until power-off, which it always says.
 */

import {
  ParameterFamily,
  control_source_choices,
  system_field_spec,
  system_fields,
  system_group_label,
  system_group_note,
  system_groups,
  system_volatile_notice,
  type SystemField,
  type SystemGroup,
  type WasmSystemFieldSpec,
} from "../pkg/integral_wasm.js";
import type { PickOption } from "./SearchPopover";
import {
  FOOTPRINT,
  SectionPanel,
  SynthKnob,
  SynthPicker,
  SynthSwitch,
  cellsPx,
} from "./synth-ui";
import { cached } from "./cached";
import { switchOptions } from "./fieldSpec";
import type { IntegraService } from "./integra";
import { useSystemCommon, type SystemValues } from "./useSystemCommon";
import css from "./SystemPage.module.css";

/** A picker's box: wide enough for a System field's full name and its value. */
const PICKER_BOX = cellsPx({
  w: 2 * FOOTPRINT.picker.w,
  h: FOOTPRINT.picker.h,
});

/** A field's core spec, read once. */
const specOf = cached((field: SystemField): WasmSystemFieldSpec =>
  system_field_spec(field),
);

/** The fields of `group`, in page order. */
const fieldsOf = cached((group: SystemGroup): readonly SystemField[] =>
  system_fields().filter((field) => specOf(field).group === group),
);

/** The values a field takes, named as core formats them (its Control Sources, for a source). */
const optionsOf = cached((field: SystemField): readonly PickOption[] => {
  const spec = specOf(field);
  if (spec.controlSource !== undefined) {
    return control_source_choices(spec.controlSource).map((source) => {
      const option = { value: source.value, label: source.name };
      source.free();
      return option;
    });
  }
  return switchOptions(spec);
});

interface FieldProps {
  field: SystemField;
  value: number;
  onChange: (value: number) => void;
}

/** One System field's house Control, chosen from its spec: a switch, a picker or a knob. */
function SystemFieldControl({ field, value, onChange }: FieldProps) {
  const spec = specOf(field);
  // A value the page doesn't write (a USB rate set on the panel) still shows as read.
  const inRange = value >= spec.min && value <= spec.max;
  const control =
    spec.max - spec.min === 1 ? (
      // Out of range, the two values are listed with neither lit.
      <SynthSwitch
        label={spec.label}
        value={value}
        options={[...optionsOf(field)]}
        onChange={onChange}
        vertical={!inRange}
      />
    ) : spec.namedList || spec.controlSource !== undefined ? (
      <SynthPicker
        label={spec.label}
        value={value}
        options={optionsOf(field)}
        onChange={onChange}
        box={PICKER_BOX}
      />
    ) : (
      <SynthKnob
        label={spec.label}
        value={value}
        min={spec.min}
        max={spec.max}
        // The registry gives no default: a knob without a centre resets to
        // where it stands, so a double-click sends nothing new.
        defaultValue={spec.center ?? value}
        center={spec.center}
        formatValue={(v) => spec.formatValue(v)}
        family={ParameterFamily.Common}
        onChange={onChange}
      />
    );
  return (
    <div className={css.field} data-field={spec.name}>
      {control}
      {!inRange && (
        <span className={css.unwritten}>Reads {spec.formatValue(value)}</span>
      )}
    </div>
  );
}

interface GroupProps {
  group: SystemGroup;
  values: SystemValues;
  onChange: (field: SystemField, value: number) => void;
}

/** One group of the page: its fields, then what it leaves to the panel. */
// The SYSTEM-or-STUDIO SET pairs' Studio Set side (Tone Control 1–4 next to
// Control Source, Studio Set Tempo next to Tempo Assign Source) joins its
// group here in #604.
function SystemGroupPanel({ group, values, onChange }: GroupProps) {
  const note = system_group_note(group);
  return (
    <SectionPanel
      label={system_group_label(group)}
      family={ParameterFamily.Common}
    >
      <div className={css.fields}>
        {fieldsOf(group).map((field) => (
          <SystemFieldControl
            key={field}
            field={field}
            value={values.get(field) ?? specOf(field).min}
            onChange={(value) => onChange(field, value)}
          />
        ))}
      </div>
      {note && <p className={css.note}>{note}</p>}
    </SectionPanel>
  );
}

interface Props {
  service: IntegraService;
}

/** The System page: the power-off notice, Refresh, and every System Common group. */
export function SystemPage({ service }: Props) {
  const { read, refresh, set } = useSystemCommon(service);
  const { values } = read;
  return (
    <div className={css.page}>
      <div className={css.bar}>
        <p className={css.notice} role="note">
          {system_volatile_notice()}
        </p>
        <button
          className={css.refresh}
          onClick={refresh}
          disabled={read.loading}
        >
          Refresh
        </button>
      </div>
      {read.loading && !values && (
        <p className={css.status}>Reading the System settings…</p>
      )}
      {read.error && (
        <p className={css.error} role="alert">
          Could not read the System settings: {read.error}
        </p>
      )}
      {values && (
        <div className={css.groups}>
          {system_groups().map((group) => (
            <SystemGroupPanel
              key={group}
              group={group}
              values={values}
              onChange={set}
            />
          ))}
        </div>
      )}
    </div>
  );
}
