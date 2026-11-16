/**
 * The System page ([#603](https://github.com/sigma/integral/issues/603), PRD
 * [#601](https://github.com/sigma/integral/issues/601)): all of System
 * Common, in the core registry's groups and order, read when the page opens
 * and on Refresh. Its edits last until power-off, which it always says.
 *
 * Each SYSTEM-or-STUDIO SET pair ([#604](https://github.com/sigma/integral/issues/604))
 * shows the Studio Set's side in its selector's group, read-only with a link
 * to the Studio Set settings drawer that edits it; the side its selector
 * puts out of force is dimmed.
 */

import {
  ParameterFamily,
  control_source_choices,
  studio_set_setting_count,
  system_field_spec,
  system_fields,
  system_group_label,
  system_group_note,
  system_groups,
  system_pair_selector,
  system_pair_studio_set_side,
  system_pair_system_side,
  system_pairs,
  system_volatile_notice,
  type SystemField,
  type SystemGroup,
  type SystemOrStudioSet,
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
import { PairSide, StudioSetReadout, pairDimming } from "./systemPairs";
import type { StudioSetSettings } from "./types";
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

/** The SYSTEM-or-STUDIO SET pairs whose selector sits in `group`. */
const pairsOf = cached((group: SystemGroup): readonly SystemOrStudioSet[] =>
  system_pairs().filter(
    (pair) => specOf(system_pair_selector(pair)).group === group,
  ),
);

/** The pair whose System side `field` is, if any. */
const pairOf = cached((field: SystemField): SystemOrStudioSet | undefined =>
  system_pairs().find((pair) => system_pair_system_side(pair).includes(field)),
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

interface StudioSetSideProps {
  pair: SystemOrStudioSet;
  values: SystemValues;
  /** The current Studio Set's settings, none before the mixer reads them. */
  studioSetSettings: StudioSetSettings | null;
  /** Open the Studio Set settings drawer; no link without it. */
  onOpenStudioSetSettings?: () => void;
}

/** The Studio Set's side of `pair`, read-only, with its link to the drawer that edits it. */
function StudioSetSide({
  pair,
  values,
  studioSetSettings,
  onOpenStudioSetSettings,
}: StudioSetSideProps) {
  const setting = system_pair_studio_set_side(pair);
  return (
    <div className={css.studioSet}>
      {/* The link stays lit: it works whichever side is in force. */}
      <div className={css.studioSetHead}>
        <span>Studio Set</span>
        {onOpenStudioSetSettings && (
          <button className={css.link} onClick={onOpenStudioSetSettings}>
            Edit in Studio Set settings
          </button>
        )}
      </div>
      <PairSide
        dim={pairDimming(values, pair).studioSet}
        side="studioSet"
        className={css.fields}
      >
        {Array.from({ length: studio_set_setting_count(setting) }, (_, i) => (
          <StudioSetReadout
            key={i}
            settings={studioSetSettings}
            setting={setting}
            index={i}
          />
        ))}
      </PairSide>
    </div>
  );
}

interface GroupProps extends Omit<StudioSetSideProps, "pair"> {
  group: SystemGroup;
  onChange: (field: SystemField, value: number) => void;
}

/**
 * One group of the page: its fields, the Studio Set's side of the pairs it
 * selects, then what it leaves to the panel.
 */
function SystemGroupPanel({ group, onChange, ...side }: GroupProps) {
  const { values } = side;
  const note = system_group_note(group);
  return (
    <SectionPanel
      label={system_group_label(group)}
      family={ParameterFamily.Common}
    >
      <div className={css.fields}>
        {fieldsOf(group).map((field) => {
          const control = (
            <SystemFieldControl
              key={field}
              field={field}
              value={values.get(field) ?? specOf(field).min}
              onChange={(value) => onChange(field, value)}
            />
          );
          const pair = pairOf(field);
          return pair === undefined ? (
            control
          ) : (
            <PairSide
              key={field}
              dim={pairDimming(values, pair).system}
              side="system"
            >
              {control}
            </PairSide>
          );
        })}
      </div>
      {pairsOf(group).map((pair) => (
        <StudioSetSide key={pair} pair={pair} {...side} />
      ))}
      {note && <p className={css.note}>{note}</p>}
    </SectionPanel>
  );
}

interface Props {
  service: IntegraService;
  /** The current Studio Set's settings, none before the mixer reads them. */
  studioSetSettings: StudioSetSettings | null;
  /** Open the Studio Set settings drawer; no link without it. */
  onOpenStudioSetSettings?: () => void;
}

/** The System page: the power-off notice, Refresh, and every System Common group. */
export function SystemPage({
  service,
  studioSetSettings,
  onOpenStudioSetSettings,
}: Props) {
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
              studioSetSettings={studioSetSettings}
              onOpenStudioSetSettings={onOpenStudioSetSettings}
              onChange={set}
            />
          ))}
        </div>
      )}
    </div>
  );
}
