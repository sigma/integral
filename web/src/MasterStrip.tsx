import { VolumeFader } from "./VolumeFader";
import { PanKnob } from "./PanKnob";
import css from "./ChannelStrip.module.css";
import masterCss from "./MasterStrip.module.css";

interface Props {
  value: number;
  onChange: (value: number) => void;
}

const noop = () => {};

export function MasterStrip({ value, onChange }: Props) {
  return (
    <div className={`${css.strip} ${masterCss.override}`}>
      <div className={css.partNumber}>Master</div>
      {/* Exact same elements as ChannelStrip, visibility:hidden for alignment */}
      <div className={masterCss.hidden}>
        <button className={css.eqButton}>EQ</button>
        <PanKnob value={64} onChange={noop} />
        <span className={css.muteLabel}>MUTE</span>
        <button className={css.muteButton}>M</button>
      </div>
      <div className={css.faderArea}>
        <VolumeFader value={value} onChange={onChange} />
      </div>
    </div>
  );
}
