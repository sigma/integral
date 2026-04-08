import { VolumeFader } from "./VolumeFader";
import { PanKnob } from "./PanKnob";
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
      {eqExpanded ? (
        <EqSection
          eq={eq}
          onToggleSwitch={onEqToggle}
          onParam={onEqParam}
        />
      ) : (
        /* Hidden replica for alignment when EQ is collapsed */
        <div className={masterCss.hidden}>
          <PanKnob value={64} onChange={noop} />
          <span className={css.muteLabel}>MUTE</span>
          <button className={css.muteButton}>M</button>
        </div>
      )}
      <div className={css.faderArea}>
        <VolumeFader value={value} onChange={onChange} />
      </div>
    </div>
  );
}
