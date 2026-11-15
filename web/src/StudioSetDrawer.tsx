/**
 * The Studio Set settings drawer ([#608](https://github.com/sigma/integral/issues/608),
 * PRD [#606](https://github.com/sigma/integral/issues/606)): the current
 * Studio Set's settings the Mixer doesn't edit, opened from ⚙ in the
 * navigation row's Studio Set group.
 *
 * - CONTROL: Tone Control 1–4 Source, each beside System Control of the same
 *   number, under System's Control Source.
 * - TEMPO: the Studio Set Tempo beside the System Tempo, under System's Tempo
 *   Assign Source.
 * - PHASE LOCK: one switch per MIDI channel, with the manual's caveat.
 *
 * Fields, ranges and labels come from the core registries. System's side of
 * each SYSTEM-or-STUDIO SET pair is read-only here (the System page edits
 * it), read from the device each time the drawer opens; whichever side isn't
 * in force is dimmed.
 */

import { useMemo, type ReactNode } from "react";
import {
  StudioSetSetting,
  SystemField,
  SystemOrStudioSet,
  control_source_choices,
  phase_lock_notice,
  studio_set_field_label,
  studio_set_setting_count,
  studio_set_setting_spec,
  system_control_source_field,
  system_field_spec,
  system_pair_selector,
} from "../pkg/integral_wasm.js";
import { Drawer, DrawerGroup } from "./Drawer";
import { SynthKnob, SynthPicker, SynthSwitch } from "./synth-ui";
import { switchOptions } from "./fieldSpec";
import {
  studioSetInForce,
  systemDisplay,
  useSystemCommon,
  type SystemValues,
} from "./useSystemCommon";
import type { IntegraService } from "./integra";
import type { UseMixerResult } from "./useMixer";
import css from "./StudioSetDrawer.module.css";

interface StudioSetDrawerProps {
  mixer: UseMixerResult;
  service: IntegraService;
  onClose: () => void;
  /** Open the System page, where System's side of each pair is edited; no link without it. */
  onOpenSystem?: () => void;
}

/** The instances 0…count-1 of a Studio Set setting. */
const instances = (setting: StudioSetSetting) =>
  Array.from({ length: studio_set_setting_count(setting) }, (_, i) => i);

/** The drawer of the current Studio Set's settings. */
export function StudioSetDrawer({ mixer, service, onClose, onOpenSystem }: StudioSetDrawerProps) {
  const { state, setStudioSetField } = mixer;
  const { read: system, refresh: refreshSystem } = useSystemCommon(service);
  const settings = state.studioSetSettings;
  const name = state.studioSetName.trim();
  return (
    <Drawer
      label="Studio Set settings"
      className={css.drawer}
      hint={`Follows the current Studio Set${name ? `: ${name}` : ""}.`}
      onClose={onClose}
    >
      {system.error !== null && (
        <p className={css.notice} role="alert">
          System Common didn't answer: System's side is unknown.{" "}
          <button className={css.link} onClick={refreshSystem}>
            Retry
          </button>
        </p>
      )}
      {settings === null ? (
        <p className={css.notice}>Reading the Studio Set…</p>
      ) : (
        <>
          <DrawerGroup
            title="CONTROL"
            actions={
              onOpenSystem && (
                <button className={css.link} onClick={onOpenSystem}>
                  System page
                </button>
              )
            }
          >
            <ControlGroup
              sources={settings.toneControlSources}
              values={system.values}
              onChange={(i, v) => setStudioSetField(StudioSetSetting.ToneControlSource, i, v)}
            />
          </DrawerGroup>
          <DrawerGroup title="TEMPO">
            <TempoGroup
              tempo={settings.tempo}
              values={system.values}
              onChange={(v) => setStudioSetField(StudioSetSetting.Tempo, 0, v)}
            />
          </DrawerGroup>
          <DrawerGroup title="PHASE LOCK">
            <PhaseLockGroup
              phaseLock={settings.phaseLock}
              onChange={(i, on) => setStudioSetField(StudioSetSetting.PhaseLock, i, on)}
            />
          </DrawerGroup>
        </>
      )}
    </Drawer>
  );
}

/** Which side of `pair` is dimmed: none before System Common is read. */
function dimmed(values: SystemValues | null, pair: SystemOrStudioSet) {
  const studioSet = studioSetInForce(values, pair);
  if (studioSet === null) return { studioSet: false, system: false };
  return { studioSet: !studioSet, system: studioSet };
}

/** One side of a pair, dimmed while not in force. */
function Side({ dim, side, children }: { dim: boolean; side: "studioSet" | "system"; children: ReactNode }) {
  return (
    <div className={`${css.side} ${dim ? css.dimmed : ""}`} data-side={side} data-dimmed={dim || undefined}>
      {children}
    </div>
  );
}

/** A System field's value, read-only: `—` until System Common is read. */
function SystemReadout({ field, values }: { field: SystemField; values: SystemValues | null }) {
  const label = system_field_spec(field).label;
  return (
    <div className={css.readout} title={`${label} (System; edited on the System page)`}>
      <span className={css.readoutLabel}>{label}</span>
      <span className={css.readoutValue} aria-label={label}>
        {systemDisplay(values, field)}
      </span>
    </div>
  );
}

function ControlGroup({
  sources,
  values,
  onChange,
}: {
  sources: number[];
  values: SystemValues | null;
  onChange: (index: number, value: number) => void;
}) {
  // The Control Source list the registry names the sources from.
  const { spec, options } = useMemo(() => {
    const spec = studio_set_setting_spec(StudioSetSetting.ToneControlSource);
    const kind = spec.controlSource;
    const options =
      kind === undefined ? [] : control_source_choices(kind).map(({ value, name }) => ({ value, label: name }));
    return { spec, options };
  }, []);
  const dim = dimmed(values, SystemOrStudioSet.Control);
  return (
    <>
      <SystemReadout field={system_pair_selector(SystemOrStudioSet.Control)} values={values} />
      {instances(StudioSetSetting.ToneControlSource).map((i) => {
        const systemSide = system_control_source_field(i);
        return (
          <div key={i} className={css.pair}>
            <Side dim={dim.studioSet} side="studioSet">
              <SynthPicker
                label={`TC${i + 1}`}
                title={studio_set_field_label(StudioSetSetting.ToneControlSource, i)}
                value={sources[i] ?? spec.defaultValue}
                options={options}
                onChange={(v) => onChange(i, v)}
              />
            </Side>
            {systemSide !== undefined && (
              <Side dim={dim.system} side="system">
                <SystemReadout field={systemSide} values={values} />
              </Side>
            )}
          </div>
        );
      })}
    </>
  );
}

function TempoGroup({
  tempo,
  values,
  onChange,
}: {
  tempo: number;
  values: SystemValues | null;
  onChange: (value: number) => void;
}) {
  const spec = studio_set_setting_spec(StudioSetSetting.Tempo);
  const label = studio_set_field_label(StudioSetSetting.Tempo, 0);
  const dim = dimmed(values, SystemOrStudioSet.Tempo);
  return (
    <>
      <SystemReadout field={system_pair_selector(SystemOrStudioSet.Tempo)} values={values} />
      <div className={css.pair}>
        <Side dim={dim.studioSet} side="studioSet">
          <SynthKnob
            label="Tempo"
            title={label}
            accessibleName={label}
            value={tempo}
            min={spec.min}
            max={spec.max}
            defaultValue={spec.defaultValue}
            formatValue={(v) => spec.formatValue(v)}
            family={spec.family}
            onChange={onChange}
          />
        </Side>
        <Side dim={dim.system} side="system">
          <SystemReadout field={SystemField.SystemTempo} values={values} />
        </Side>
      </div>
    </>
  );
}

function PhaseLockGroup({
  phaseLock,
  onChange,
}: {
  phaseLock: boolean[];
  onChange: (index: number, value: number) => void;
}) {
  const { spec, options } = useMemo(() => {
    const spec = studio_set_setting_spec(StudioSetSetting.PhaseLock);
    return { spec, options: switchOptions(spec) };
  }, []);
  return (
    <>
      <p className={css.notice}>{phase_lock_notice()}</p>
      <div className={css.channels}>
        {instances(StudioSetSetting.PhaseLock).map((i) => (
          <SynthSwitch
            key={i}
            label={`CH ${i + 1}`}
            title={studio_set_field_label(StudioSetSetting.PhaseLock, i)}
            value={phaseLock[i] ? spec.max : spec.min}
            options={options}
            onChange={(v) => onChange(i, v)}
          />
        ))}
      </div>
    </>
  );
}
