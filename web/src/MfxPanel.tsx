import { useMemo } from "react";
import {
  ControlSourceKind,
  MfxHeaderField,
  control_source_choices,
  mfx_type_names,
  mfx_control_targets,
  mfx_params,
} from "../pkg/integral_wasm.js";
import type { WasmControlSource } from "../pkg/integral_wasm.js";
import { SynthKnob, OutputStrip, ParametricEq, SectionPanel } from "./synth-ui";
import type { EqBand, EqControl } from "./synth-ui";
import { ParamControl, pairParams, type ParamLook, type SpecParam } from "./ParamControl";
import { OUTPUT_PARAMS, itemParam, resolveSkin, type Skin } from "./mfxSkins";
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
// Spec-driven parameter panel
// ---------------------------------------------------------------------------

function useSpecParams(mfxType: number): SpecParam[] {
  return useMemo(() => pairParams(mfx_params(mfxType)), [mfxType]);
}

interface SkinPanelProps {
  skin: Skin;
  params: SpecParam[];
  slots: number[];
  onNibParam: (slot: number, value: number) => void;
}

function SkinPanel({ skin, params, slots, onNibParam }: SkinPanelProps) {
  const byName = new Map(params.map((p) => [p.def.name, p]));
  const eqControl = (name: string): EqControl | undefined => {
    const def = byName.get(name)?.def;
    return def && {
      value: slots[def.index] ?? def.defaultValue, min: def.min, max: def.max, defaultValue: def.defaultValue,
      onChange: (v) => onNibParam(def.index, v), formatValue: (v) => def.formatValue(v),
    };
  };
  const bands = (skin.eq ?? []).flatMap((b): EqBand[] => {
    const gain = eqControl(b.gain), freq = eqControl(b.freq);
    return gain && freq ? [{ label: b.label, color: b.color, gain, freq, q: b.q ? eqControl(b.q) : undefined }] : [];
  });
  return (
    <div className={css.customLayout}>
      {bands.length > 0 && <ParametricEq bands={bands} />}
      {skin.rows.map((row, ri) => (
        <div key={ri} className={css.customRow}>
          {row.items.map((item) => {
            const param = byName.get(itemParam(item));
            if (!param) return null;
            const look: ParamLook = typeof item === "string" ? {} : item;
            return (
              <ParamControl key={param.def.index} param={param} slots={slots} onNibParam={onNibParam}
                look={{ ...look, color: look.color ?? row.color }} />
            );
          })}
        </div>
      ))}
    </div>
  );
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
  const params = useSpecParams(mfx.mfxType);
  const skin = useMemo(() => resolveSkin(mfx.mfxType, params), [mfx.mfxType, params]);
  const controlTargets = useMemo(() => mfx_control_targets(mfx.mfxType), [mfx.mfxType]);
  const levelParamIndex = params.find((p) => OUTPUT_PARAMS.has(p.def.name))?.def.index ?? -1;

  return (
    <SectionPanel label="MFX" accentColor="#4ce0e0">
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

          {/* Spec-driven parameter panel, laid out by the type's skin */}
          <SkinPanel skin={skin} params={params} slots={mfx.params} onNibParam={onNibParam} />

          {/* Output section — sends + level, like a mixer channel strip */}
          <div className={css.mfxOutput}>
            <OutputStrip
              fx1={mfx.chorusSend} fx2={mfx.reverbSend}
              level={levelParamIndex >= 0 ? (mfx.params[levelParamIndex] ?? 127) : 127}
              onFx1Change={(v) => onHeaderParam(MfxHeaderField.ChorusSend, 0, v)}
              onFx2Change={(v) => onHeaderParam(MfxHeaderField.ReverbSend, 0, v)}
              onLevelChange={levelParamIndex >= 0 ? (v) => onNibParam(levelParamIndex, v) : () => {}}
              trackHeight={250}
            />
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
                  color="#8ac"
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
