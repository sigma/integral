import { VolumeFader } from "./VolumeFader";
import { PanKnob } from "./PanKnob";
import type { PartState } from "./types";
import css from "./ChannelStrip.module.css";

interface Props {
  partIndex: number;
  part: PartState;
  onLevelChange: (value: number) => void;
  onPanChange: (value: number) => void;
  onMuteToggle: () => void;
}

function toneLabel(part: PartState): string {
  return part.toneName || `${part.toneBankMsb}-${part.toneBankLsb}-${part.tonePC + 1}`;
}

export function ChannelStrip({
  partIndex,
  part,
  onLevelChange,
  onPanChange,
  onMuteToggle,
}: Props) {
  return (
    <div className={css.strip}>
      <div className={css.partNumber}>{partIndex + 1}</div>
      <button className={css.eqButton} disabled>
        EQ
      </button>
      <PanKnob value={part.pan} onChange={onPanChange} />
      <span className={css.muteLabel}>MUTE</span>
      <button
        className={`${css.muteButton} ${part.muted ? css.muted : ""}`}
        onClick={onMuteToggle}
      >
        M
      </button>
      <div className={css.faderArea}>
        <VolumeFader value={part.level} onChange={onLevelChange} />
      </div>
      <span className={css.toneName}>{toneLabel(part)}</span>
    </div>
  );
}
