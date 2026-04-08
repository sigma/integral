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

/** Format tone info as a short display string. */
function toneLabel(part: PartState): string {
  return `${part.toneBankMsb}-${part.toneBankLsb}-${part.tonePC + 1}`;
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
      <span className={css.partNumber}>{partIndex + 1}</span>
      <button className={css.eqButton} disabled>
        EQ
      </button>
      <PanKnob value={part.pan} onChange={onPanChange} />
      <button
        className={`${css.muteButton} ${part.muted ? css.muted : ""}`}
        onClick={onMuteToggle}
      >
        M
      </button>
      <VolumeFader value={part.level} onChange={onLevelChange} />
      <span className={css.toneName}>{toneLabel(part)}</span>
    </div>
  );
}
