import { VolumeFader } from "./VolumeFader";
import { PanKnob } from "./PanKnob";
import { EqKnob } from "./EqKnob";
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
  onChorusSendChange: (value: number) => void;
  onReverbSendChange: (value: number) => void;
  onReceiveChannelChange: (channel: number) => void;
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
  onChorusSendChange,
  onReverbSendChange,
  onReceiveChannelChange,
  onEqToggle,
  onEqParam,
}: Props) {
  return (
    <div className={css.strip}>
      <select
        className={css.channelSelect}
        value={part.receiveChannel}
        onChange={(e) => onReceiveChannelChange(Number(e.target.value))}
        title={`Part ${partIndex + 1}`}
      >
        {Array.from({ length: 16 }, (_, i) => (
          <option key={i} value={i}>
            Ch {i + 1}
          </option>
        ))}
      </select>
      {eqExpanded && (
        <EqSection
          eq={part.eq}
          onToggleSwitch={onEqToggle}
          onParam={onEqParam}
        />
      )}
      <PanKnob value={part.pan} onChange={onPanChange} />
      <div className={css.sends}>
        <EqKnob
          label="FX1"
          value={part.chorusSend}
          min={0}
          max={127}
          defaultValue={0}
          onChange={onChorusSendChange}
          formatValue={(v) => String(v)}
          color="#668"
        />
        <EqKnob
          label="FX2"
          value={part.reverbSend}
          min={0}
          max={127}
          defaultValue={0}
          onChange={onReverbSendChange}
          formatValue={(v) => String(v)}
          color="#686"
        />
      </div>
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
