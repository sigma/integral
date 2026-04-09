import { useCallback, useMemo } from "react";
import { EqKnob } from "./EqKnob";
import {
  mfx_type_names,
  mfx_type_param_count,
  mfx_param_def,
} from "../pkg/integral_wasm.js";
import css from "./MfxEditor.module.css";

interface MfxCtrl {
  source: number;
  sens: number;
  assign: number;
}

interface Props {
  partIndex: number;
  mfxType: number;
  chorusSend: number;
  reverbSend: number;
  controls: MfxCtrl[];
  params: number[];
  onTypeChange: (type_: number) => void;
  onHeaderParam: (offset: number, value: number) => void;
  onNibParam: (paramIndex: number, value: number) => void;
}

let _typeNamesCache: string[] | null = null;
function getTypeNames(): string[] {
  if (!_typeNamesCache) _typeNamesCache = mfx_type_names();
  return _typeNamesCache;
}

const CTRL_SOURCE_NAMES: string[] = (() => {
  const names = ["OFF"];
  // CC01-CC31
  for (let i = 1; i <= 31; i++) names.push(`CC${String(i).padStart(2, "0")}`);
  // CC33-CC95
  for (let i = 33; i <= 95; i++) names.push(`CC${String(i).padStart(2, "0")}`);
  names.push("BEND", "AFT", "SYS1", "SYS2", "SYS3", "SYS4");
  return names;
})();

export function MfxEditor({
  mfxType,
  chorusSend,
  reverbSend,
  controls,
  params,
  onTypeChange,
  onHeaderParam,
  onNibParam,
}: Props) {
  const paramDefs = useMemo(() => {
    const count = mfx_type_param_count(mfxType);
    const defs = [];
    for (let i = 0; i < count; i++) {
      const d = mfx_param_def(mfxType, i);
      if (d) {
        defs.push({ index: d.index, name: d.name, min: d.min, max: d.max, defaultValue: d.defaultValue });
        d.free();
      }
    }
    return defs;
  }, [mfxType]);

  const handleTypeChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      onTypeChange(Number(e.target.value));
    },
    [onTypeChange],
  );

  return (
    <div className={css.editor}>
      {/* Type selector + sends */}
      <div className={css.header}>
        <label className={css.typeLabel}>
          MFX
          <select className={css.typeSelect} value={mfxType} onChange={handleTypeChange}>
            {getTypeNames().map((name, i) => (
              <option key={i} value={i}>{i}: {name}</option>
            ))}
          </select>
        </label>
        <EqKnob label="Cho Send" value={chorusSend} min={0} max={127} defaultValue={0}
          onChange={(v) => onHeaderParam(0x02, v)} formatValue={(v) => String(v)} color="#668" />
        <EqKnob label="Rev Send" value={reverbSend} min={0} max={127} defaultValue={0}
          onChange={(v) => onHeaderParam(0x03, v)} formatValue={(v) => String(v)} color="#686" />
      </div>

      {/* Dynamic parameter knobs */}
      {paramDefs.length > 0 && (
        <div className={css.paramGrid}>
          {paramDefs.map((def, i) => (
            <EqKnob
              key={`${mfxType}-${def.index}`}
              label={def.name}
              value={params[i] ?? def.defaultValue}
              min={def.min}
              max={def.max}
              defaultValue={def.defaultValue}
              onChange={(v) => onNibParam(i, v)}
              formatValue={(v) => String(v)}
              color="#c8a"
            />
          ))}
        </div>
      )}

      {/* MFX Control (4 slots) */}
      <div className={css.ctrlSection}>
        <span className={css.ctrlTitle}>MFX Control</span>
        <div className={css.ctrlSlots}>
          {[0, 1, 2, 3].map((slot) => {
            const ctrl = controls[slot] ?? { source: 0, sens: 64, assign: 0 };
            return (
              <div key={slot} className={css.ctrlSlot}>
                <span className={css.ctrlSlotLabel}>{slot + 1}</span>
                <select
                  className={css.ctrlSelect}
                  value={ctrl.source}
                  onChange={(e) => onHeaderParam(0x05 + slot * 2, Number(e.target.value))}
                  title={`Source ${slot + 1}`}
                >
                  {CTRL_SOURCE_NAMES.map((name, i) => (
                    <option key={i} value={i}>{name}</option>
                  ))}
                </select>
                <EqKnob
                  label="Sens"
                  value={ctrl.sens}
                  min={1}
                  max={127}
                  defaultValue={64}
                  onChange={(v) => onHeaderParam(0x06 + slot * 2, v)}
                  formatValue={(v) => String(v - 64)}
                  color="#8ac"
                />
                <select
                  className={css.ctrlSelect}
                  value={ctrl.assign}
                  onChange={(e) => onHeaderParam(0x0D + slot, Number(e.target.value))}
                  title={`Assign ${slot + 1}`}
                >
                  <option value={0}>OFF</option>
                  {paramDefs.map((def) => (
                    <option key={def.index} value={def.index}>{def.name}</option>
                  ))}
                </select>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
