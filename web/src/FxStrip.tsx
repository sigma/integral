import { VolumeFader } from "./VolumeFader";
import { EqKnob } from "./EqKnob";
import type { FxState } from "./types";
import { useMemo } from "react";
import type { WasmFxParamDef } from "../pkg/integral_wasm.js";
import { stripIcon } from "./categoryIcons";
import css from "./ChannelStrip.module.css";
import fxCss from "./FxStrip.module.css";
import box from "./PanelBox.module.css";
import { RouteChip } from "./RouteChip";

interface Props {
  label: string;
  fx: FxState;
  typeNames: string[];
  outputNames: string[];
  /** Visible parameter definitions for a type (from the core, via WASM). */
  paramDefs: (type: number) => WasmFxParamDef[];
  onToggleSwitch: () => void;
  onParam: (offset: number, value: number) => void;
  onNibParam: (paramIndex: number, value: number) => void;
}

/** Two-way msec/Hz-vs-note switch; `labels` come from the core. */
function SyncToggle({
  labels,
  noteMode,
  onChange,
}: {
  labels: string[];
  noteMode: boolean;
  onChange: (noteMode: boolean) => void;
}) {
  return (
    <span className={fxCss.sync}>
      {labels.map((l, i) => (
        <button
          key={l}
          className={(i === 1) === noteMode ? fxCss.syncOn : undefined}
          aria-pressed={(i === 1) === noteMode}
          onClick={() => onChange(i === 1)}
        >
          {l}
        </button>
      ))}
    </span>
  );
}

export function FxStrip({
  label,
  fx,
  typeNames,
  outputNames,
  paramDefs,
  onToggleSwitch,
  onParam,
  onNibParam,
}: Props) {
  // Per type: each visible param, paired with its note-mode view when it has one.
  const defsByType = useMemo(
    () => typeNames.map((_, t) => paramDefs(t).map((def) => ({ def, note: def.noteParam() }))),
    [paramDefs, typeNames],
  );
  const currentParamDefs = defsByType[fx.type] ?? [];
  const slots = Int32Array.from(fx.params);
  // Reserve rows for the type with the most params, so the box keeps its height when the type changes.
  const maxRows = Math.max(0, ...defsByType.map((defs) => defs.length));

  return (
    <div className={`${css.strip} ${fxCss.override}`}>
      <div className={css.partNumber}>{label}</div>

      <div className={`${box.box} ${fxCss.box} ${fx.enabled ? "" : box.off}`}>
        <button
          className={`${box.switchButton} ${fx.enabled ? box.switchOn : box.switchOff}`}
          onClick={onToggleSwitch}
        >
          {fx.enabled ? "FX ON" : "FX OFF"}
        </button>

        <div className={fxCss.knobs} style={{ gridTemplateRows: `repeat(${maxRows}, var(--knob-pitch))` }}>
          {currentParamDefs.map(({ def, note }, i) => {
            // A msec/Hz-vs-note param edits whichever slot its switch selects.
            const active = note && def.noteMode(slots) ? note : def;
            const sync = def.syncIndex;
            return (
              <div key={def.index} style={{ gridRow: i + 1 }}>
                <EqKnob
                  label={def.name}
                  value={fx.params[active.index] ?? active.defaultValue}
                  min={active.min}
                  max={active.max}
                  defaultValue={active.defaultValue}
                  onChange={(v) => onNibParam(active.index, v)}
                  formatValue={(v) => active.formatValue(v)}
                  addon={
                    note && sync !== undefined && (
                      <SyncToggle
                        labels={def.syncLabels}
                        noteMode={active === note}
                        onChange={(on) => onNibParam(sync, on ? 1 : 0)}
                      />
                    )
                  }
                />
              </div>
            );
          })}
        </div>
      </div>

      {/* Type/Out: a separate box pinned to the fader. */}
      <div className={css.bottom}>
        <div className={box.box}>
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
        </div>
        {/* Route chip shares the slot above M/S with part and Comp+EQ strips; the hidden M/S row aligns it. */}
        <div className={css.toggles}>
          <div className={css.muteRow}>
            <RouteChip value={fx.output} names={outputNames}
              onChange={(v) => onParam(label === "FX1" ? 3 : 2, v)} title={`${label} Output`} />
          </div>
          <div className={css.muteRow} style={{ visibility: "hidden" }}>
            <button className={css.muteButton}>M</button>
          </div>
        </div>
      </div>

      <div className={css.faderArea}>
        <span className={css.faderLabel}>{typeNames[fx.type]}</span>
        <VolumeFader value={fx.level} onChange={(v) => onParam(1, v)} />
      </div>

      {/* Role icon for alignment with part strips */}
      <div className={css.categoryArea}>
        <div
          className={css.categoryIcon}
          dangerouslySetInnerHTML={{ __html: stripIcon(label === "FX1" ? "fx1" : "fx2") }}
        />
        <span className={css.categoryLabel}>{label === "FX1" ? "Chorus/Delay" : "Reverb"}</span>
      </div>
    </div>
  );
}
