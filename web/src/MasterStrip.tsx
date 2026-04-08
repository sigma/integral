import { VolumeFader } from "./VolumeFader";
import { PanKnob } from "./PanKnob";
import { EqKnob } from "./EqKnob";
import { EqSection } from "./EqSection";
import type { EqState } from "./types";
import css from "./ChannelStrip.module.css";
import masterCss from "./MasterStrip.module.css";

interface Props {
  value: number;
  onChange: (value: number) => void;
  eq: EqState;
  eqExpanded: boolean;
  onEqToggle: () => void;
  onEqParam: (paramOffset: number, value: number) => void;
}

const noop = () => {};

export function MasterStrip({
  value,
  onChange,
  eq,
  eqExpanded,
  onEqToggle,
  onEqParam,
}: Props) {
  return (
    <div className={`${css.strip} ${masterCss.override}`}>
      <div className={css.partNumber}>Master</div>
      {eqExpanded && (
        <EqSection
          eq={eq}
          onToggleSwitch={onEqToggle}
          onParam={onEqParam}
          paramBase={0}
        />
      )}
      {/* Hidden replica of PAN + sends + MUTE for fader alignment */}
      <div className={masterCss.hidden}>
        <PanKnob value={64} onChange={noop} />
        <div className={css.sends}>
          <EqKnob label="CHO" value={0} min={0} max={127} defaultValue={0}
            onChange={noop} formatValue={(v) => String(v)} />
          <EqKnob label="REV" value={0} min={0} max={127} defaultValue={0}
            onChange={noop} formatValue={(v) => String(v)} />
        </div>
        <span className={css.muteLabel}>MUTE</span>
        <button className={css.muteButton}>M</button>
      </div>
      <div className={css.faderArea}>
        <VolumeFader value={value} onChange={onChange} />
      </div>
    </div>
  );
}
