import { VolumeFader } from "./VolumeFader";
import { PanKnob } from "./PanKnob";
import { EqKnob } from "./EqKnob";
import { EqSection } from "./EqSection";
import { defaultEqState } from "./types";
import type { FxState } from "./types";
import type { FxParamDef } from "./fxParams";
import css from "./ChannelStrip.module.css";
import fxCss from "./FxStrip.module.css";
import masterCss from "./MasterStrip.module.css";

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

const noop = () => {};
const noopParam = (_o: number, _v: number) => {};

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

      {eqExpanded ? (
        /* FX-specific controls shown in place of EQ section */
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
      ) : (
        /* When collapsed, hidden EQ section placeholder for height alignment */
        <div className={masterCss.hidden}>
          <EqSection eq={defaultEqState()} onToggleSwitch={noop} onParam={noopParam} />
        </div>
      )}

      {/* Hidden PAN + sends + MUTE spacers for fader alignment */}
      <div className={masterCss.hidden}>
        <PanKnob value={64} onChange={noop} />
        <div className={css.sends}>
          <EqKnob label="FX1" value={0} min={0} max={127} defaultValue={0}
            onChange={noop} formatValue={(v) => String(v)} />
          <EqKnob label="FX2" value={0} min={0} max={127} defaultValue={0}
            onChange={noop} formatValue={(v) => String(v)} />
        </div>
        <span className={css.muteLabel}>MUTE</span>
        <button className={css.muteButton}>M</button>
      </div>

      <div className={css.faderArea}>
        <VolumeFader value={fx.level} onChange={(v) => onParam(1, v)} />
      </div>
    </div>
  );
}
