import { VolumeFader } from "./VolumeFader";
import { EqKnob } from "./EqKnob";
import type { FxState } from "./types";
import type { FxParamDef } from "./fxParams";
import { stripIcon } from "./categoryIcons";
import css from "./ChannelStrip.module.css";
import fxCss from "./FxStrip.module.css";

interface Props {
  label: string;
  fx: FxState;
  eqExpanded: boolean;
  typeNames: string[];
  outputNames: string[];
  paramDefs: Record<number, FxParamDef[]>;
  onToggleSwitch: () => void;
  onParam: (offset: number, value: number) => void;
  onNibParam: (paramIndex: number, value: number) => void;
}

export function FxStrip({
  label,
  fx,
  eqExpanded,
  typeNames,
  outputNames,
  paramDefs,
  onToggleSwitch,
  onParam,
  onNibParam,
}: Props) {
  const currentParamDefs = paramDefs[fx.type] ?? [];

  return (
    <div className={`${css.strip} ${fxCss.override}`}>
      <div className={css.partNumber}>{label}</div>

      {eqExpanded && (
        <div className={fxCss.controls}>
          <button
            className={`${fxCss.switchButton} ${fx.enabled ? fxCss.switchOn : fxCss.switchOff}`}
            onClick={onToggleSwitch}
          >
            {fx.enabled ? "ON" : "OFF"}
          </button>

          <div className={fxCss.row}>
            <span className={fxCss.label}>Type</span>
            <select
              className={fxCss.select}
              value={fx.type}
              onChange={(e) => onParam(0, Number(e.target.value))}
            >
              {typeNames.map((name, i) => (
                <option key={i} value={i}>{name}</option>
              ))}
            </select>
          </div>

          <div className={fxCss.row}>
            <span className={fxCss.label}>Out</span>
            <select
              className={fxCss.select}
              value={fx.output}
              onChange={(e) => onParam(label === "FX1" ? 3 : 2, Number(e.target.value))}
            >
              {outputNames.map((name, i) => (
                <option key={i} value={i}>{name}</option>
              ))}
            </select>
          </div>

          {currentParamDefs.length > 0 && (
            <div className={fxCss.paramsArea}>
              {currentParamDefs.map((def) => (
                <EqKnob
                  key={def.index}
                  label={def.name}
                  value={fx.params[def.index] ?? def.defaultValue}
                  min={def.min}
                  max={def.max}
                  defaultValue={def.defaultValue}
                  onChange={(v) => onNibParam(def.index, v)}
                  formatValue={def.format}
                />
              ))}
            </div>
          )}
        </div>
      )}

      <div className={css.faderArea}>
        <VolumeFader value={fx.level} onChange={(v) => onParam(1, v)} />
      </div>

      {/* Role icon for alignment with part strips */}
      <div className={css.categoryArea}>
        <div
          className={css.categoryIcon}
          dangerouslySetInnerHTML={{ __html: stripIcon(label === "FX1" ? "fx1" : "fx2") }}
        />
        <span className={css.categoryLabel}>{label === "FX1" ? "Chorus" : "Reverb"}</span>
      </div>
    </div>
  );
}
