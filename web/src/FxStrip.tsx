import { VolumeFader } from "./VolumeFader";
import type { FxState } from "./types";
import { useMemo } from "react";
import { stripIcon } from "./categoryIcons";
import css from "./ChannelStrip.module.css";
import fxCss from "./FxStrip.module.css";
import box from "./PanelBox.module.css";
import { RouteChip } from "./RouteChip";
import { SynthSwitch } from "./synth-ui";
import { CHORUS_SET, GridSection, REVERB_SET, variantLayout } from "./layouts";
import { useFxBind } from "./fxBinding";

/** Each FX strip's parameter Section, by block. */
const SETS = { chorus: CHORUS_SET, reverb: REVERB_SET } as const;

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
  /** Which FX block the strip edits: its params and their layouts come from core by type. */
  block: keyof typeof SETS;
  fx: FxState;
  typeNames: string[];
  routes: FxRoute[];
  onToggleSwitch: () => void;
  onParam: (offset: number, value: number) => void;
  onNibParam: (paramIndex: number, value: number) => void;
}

export function FxStrip({
  label,
  block,
  fx,
  typeNames,
  routes,
  onToggleSwitch,
  onParam,
  onNibParam,
}: Props) {
  // One footprint for every type, so the type list below never moves.
  const layout = useMemo(() => variantLayout(SETS[block], fx.type), [block, fx.type]);
  const bind = useFxBind(block, fx.type, fx.params, onNibParam);

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

        <GridSection layout={layout} bind={bind} />
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
