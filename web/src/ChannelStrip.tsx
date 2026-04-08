import { VolumeFader } from "./VolumeFader";
import { PanKnob } from "./PanKnob";
import { EqSection } from "./EqSection";
import type { PartState } from "./types";
import css from "./ChannelStrip.module.css";

interface Props {
  partIndex: number;
  part: PartState;
  eqExpanded: boolean;
  onLevelChange: (value: number) => void;
  onPanChange: (value: number) => void;
  onMuteToggle: () => void;
  onEqToggle: () => void;
  onEqParam: (paramOffset: number, value: number) => void;
}

function toneLabel(part: PartState): string {
  return part.toneName || `${part.toneBankMsb}-${part.toneBankLsb}-${part.tonePC + 1}`;
}

export function ChannelStrip({
  partIndex,
  part,
  eqExpanded,
  onLevelChange,
  onPanChange,
  onMuteToggle,
  onEqToggle,
  onEqParam,
}: Props) {
  return (
    <div className={css.strip}>
      <div className={css.partNumber}>{partIndex + 1}</div>
      {eqExpanded && (
        <EqSection
          eq={part.eq}
          onToggleSwitch={onEqToggle}
          onParam={onEqParam}
        />
      )}
      <PanKnob value={part.pan} onChange={onPanChange} />
      <span className={css.muteLabel}>MUTE</span>
      <button
        className={`${css.muteButton} ${part.muted ? css.muted : ""}`}
        onClick={onMuteToggle}
      >
        M
      </button>
      <div className={css.faderArea}>
        <span className={css.toneName}>{toneLabel(part)}</span>
        <VolumeFader value={part.level} onChange={onLevelChange} />
      </div>
    </div>
  );
}
