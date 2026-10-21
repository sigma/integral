import { useMemo } from "react";
import {
  ControlSourceKind,
  MfxHeaderField,
  ParameterFamily,
  control_source_choices,
  mfxHeaderFamily,
  mfx_type_names,
  mfx_control_targets,
} from "../pkg/integral_wasm.js";
import type { WasmControlSource } from "../pkg/integral_wasm.js";
import { SynthKnob, SectionPanel } from "./synth-ui";
import { GridSection, MFX_SET, variantLayout } from "./layouts";
import { useFxBind } from "./fxBinding";
import css from "./MfxPanel.module.css";
import { deviceSpec } from "./deviceSpec";
import type { MfxHandlers } from "./useMfxHandlers";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export interface MfxCtrl {
  source: number;
  sens: number;
  assign: number;
}

export interface MfxState {
  mfxType: number;
  chorusSend: number;
  reverbSend: number;
  controls: MfxCtrl[];
  params: number[];
}

interface Props extends MfxHandlers {
  mfx: MfxState;
}

// ---------------------------------------------------------------------------
// Cached MFX type names
// ---------------------------------------------------------------------------

let _typeNamesCache: string[] | null = null;
function getMfxTypeNames(): string[] {
  if (!_typeNamesCache) _typeNamesCache = mfx_type_names();
  return _typeNamesCache;
}

// ---------------------------------------------------------------------------
// Cached MFX control sources
// ---------------------------------------------------------------------------

let _ctrlSourcesCache: WasmControlSource[] | null = null;
function getMfxControlSources(): WasmControlSource[] {
  if (!_ctrlSourcesCache) _ctrlSourcesCache = control_source_choices(ControlSourceKind.Mfx);
  return _ctrlSourcesCache;
}

// ---------------------------------------------------------------------------
// Main MfxPanel component
// ---------------------------------------------------------------------------

export function MfxPanel({
  mfx,
  onTypeChange,
  onHeaderParam,
  onNibParam,
}: Props) {
  const layout = useMemo(() => variantLayout(MFX_SET, mfx.mfxType), [mfx.mfxType]);
  const bind = useFxBind("mfx", mfx.mfxType, mfx.params, onNibParam);
  const controlTargets = useMemo(() => mfx_control_targets(mfx.mfxType), [mfx.mfxType]);

  return (
    <SectionPanel label="MFX" family={ParameterFamily.Common}>
      <div className={css.columns}>
        <div className={css.mainColumn}>
          {/* Type selector */}
          <div className={css.mfxTypeRow}>
            <span className={css.mfxTypeLabel}>Type</span>
            <select
              className={css.mfxTypeSelect}
              value={mfx.mfxType}
              onChange={(e) => onTypeChange(Number(e.target.value))}
            >
              {getMfxTypeNames().map((name, i) => (
                <option key={i} value={i}>{i}: {name}</option>
              ))}
            </select>
          </div>

          {/* The type's parameters, Level included, in the variant set's one footprint */}
          <GridSection layout={layout} bind={bind} />

          {/* Sends, like a mixer channel strip */}
          <div className={css.mfxOutput}>
            <div className={css.mfxSends}>
              {/* shortcut: 0–127 literal ranges — MFX header fields have no core
                  field_spec yet; swap for `mfx_header_spec` knobs when integral-core grows one. */}
              <SynthKnob label="FX1" title="Chorus Send Level" value={mfx.chorusSend}
                min={0} max={127} defaultValue={0} formatValue={String}
                family={mfxHeaderFamily(MfxHeaderField.ChorusSend)}
                onChange={(v) => onHeaderParam(MfxHeaderField.ChorusSend, 0, v)} />
              <SynthKnob label="FX2" title="Reverb Send Level" value={mfx.reverbSend}
                min={0} max={127} defaultValue={0} formatValue={String}
                family={mfxHeaderFamily(MfxHeaderField.ReverbSend)}
                onChange={(v) => onHeaderParam(MfxHeaderField.ReverbSend, 0, v)} />
            </div>
          </div>
        </div>

        {/* Macros — CC-to-parameter mappings */}
        <div className={css.macroSection}>
          <div className={css.macroHeader}>
            <span className={css.macroTitle}>MACROS</span>
            <span className={css.macroColLabel}>SRC / DST</span>
            <span className={css.macroColLabel}>DEPTH</span>
          </div>
          {Array.from({ length: deviceSpec().mfx_control_count }, (_, slot) => {
            const ctrl = mfx.controls[slot] ?? { source: 0, sens: 64, assign: 0 };
            return (
              <div key={slot} className={css.macroSlot}>
                <span className={css.macroNum}>{slot + 1}</span>
                <div className={css.macroSelectors}>
                  <select
                    className={css.macroSelect}
                    value={ctrl.source}
                    onChange={(e) => onHeaderParam(MfxHeaderField.ControlSource, slot, Number(e.target.value))}
                    title="Source"
                  >
                    {getMfxControlSources().map(({ value, name }) => (
                      <option key={value} value={value}>{name}</option>
                    ))}
                  </select>
                  <select
                    className={css.macroSelect}
                    value={ctrl.assign}
                    onChange={(e) => onHeaderParam(MfxHeaderField.ControlAssign, slot, Number(e.target.value))}
                    title="Destination"
                  >
                    {controlTargets.map((name, i) => (
                      <option key={i} value={i}>{name}</option>
                    ))}
                  </select>
                </div>
                <SynthKnob
                  label=""
                  value={ctrl.sens}
                  min={1}
                  max={127}
                  defaultValue={64}
                  center={64}
                  onChange={(v) => onHeaderParam(MfxHeaderField.ControlSens, slot, v)}
                  formatValue={(v) => String(v - 64)}
                  family={mfxHeaderFamily(MfxHeaderField.ControlSens)}
                  title={`Macro ${slot + 1} Depth`}
                />
              </div>
            );
          })}
        </div>
      </div>
    </SectionPanel>
  );
}
