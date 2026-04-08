import { VolumeFader } from "./VolumeFader";
import { PanKnob } from "./PanKnob";
import { EqSection } from "./EqSection";
import { defaultEqState } from "./types";
import css from "./ChannelStrip.module.css";
import exCss from "./ExStrip.module.css";

interface Props {
  level: number;
  muted: boolean;
  eqExpanded: boolean;
  onLevelChange: (value: number) => void;
  onMuteToggle: () => void;
}

const noop = () => {};
const noopParam = (_offset: number, _value: number) => {};

export function ExStrip({
  level,
  muted,
  eqExpanded,
  onLevelChange,
  onMuteToggle,
}: Props) {
  return (
    <div className={`${css.strip} ${exCss.override}`}>
      <div className={css.partNumber}>EX</div>
      {/* Hidden spacers matching ChannelStrip layout for alignment */}
      <div className={exCss.hidden}>
        {eqExpanded && (
          <EqSection
            eq={defaultEqState()}
            onToggleSwitch={noop}
            onParam={noopParam}
          />
        )}
        <PanKnob value={64} onChange={noop} />
      </div>
      <span className={css.muteLabel}>MUTE</span>
      <button
        className={`${css.muteButton} ${muted ? css.muted : ""}`}
        onClick={onMuteToggle}
      >
        M
      </button>
      <div className={css.faderArea}>
        <VolumeFader value={level} onChange={onLevelChange} />
      </div>
    </div>
  );
}
