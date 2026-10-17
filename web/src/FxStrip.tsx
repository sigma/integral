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
import { SynthSwitch } from "./synth-ui";
import { SyncToggle } from "./SyncToggle";

/** One output-routing select on an FX strip, wired to a core block offset. */
interface FxRoute {
  /** Raw offset within the FX core block. */
  offset: number;
  names: string[];
  value: number;
  title: string;
}

interface Props {
  label: string;
  fx: FxState;
  typeNames: string[];
  routes: FxRoute[];
  /** Visible parameter definitions for a type (from the core, via WASM). */
  paramDefs: (type: number) => WasmFxParamDef[];
  onToggleSwitch: () => void;
  onParam: (offset: number, value: number) => void;
  onNibParam: (paramIndex: number, value: number) => void;
}

export function FxStrip({
  label,
  fx,
  typeNames,
  routes,
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
                  family={def.family}
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

      {/* Type: a separate box pinned to the fader. */}
      <div className={`${css.bottom} ${fxCss.fill}`}>
        <div className={`${box.box} ${fxCss.typeList} ${fx.enabled ? "" : box.off}`}>
          <SynthSwitch label="Type" vertical led={false} value={fx.type}
            options={typeNames.map((name, i) => ({ value: i, label: name }))}
            onChange={(v) => onParam(0, v)} />
        </div>
        {/* Route chips share the slots above M/S with part and Comp+EQ strips; the hidden M/S row aligns them. */}
        <div className={css.toggles}>
          {routes.map((route) => (
            <div key={route.offset} className={css.muteRow}>
              <RouteChip value={route.value} names={route.names}
                onChange={(v) => onParam(route.offset, v)} title={route.title} />
            </div>
          ))}
          <div className={css.muteRow} style={{ visibility: "hidden" }}>
            <button className={css.muteButton}>M</button>
          </div>
        </div>
      </div>

      <div className={`${css.faderArea} ${fx.enabled ? "" : box.off}`}>
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
