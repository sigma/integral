import { VolumeFader } from "./VolumeFader";
import css from "./ChannelStrip.module.css";
import exCss from "./ExStrip.module.css";

interface Props {
  level: number;
  muted: boolean;
  onLevelChange: (value: number) => void;
  onMuteToggle: () => void;
}

export function ExStrip({
  level,
  muted,
  onLevelChange,
  onMuteToggle,
}: Props) {
  return (
    <div className={`${css.strip} ${exCss.override}`}>
      <div className={css.partNumber}>EX</div>
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
