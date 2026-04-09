import { VolumeFader } from "./VolumeFader";
import { PanKnob } from "./PanKnob";
import { EqKnob } from "./EqKnob";
import { EqSection } from "./EqSection";
import { defaultPartState, type PartState, type EqState } from "./types";
import css from "./ChannelStrip.module.css";

const noop = () => {};
const hide: React.CSSProperties = { visibility: "hidden", pointerEvents: "none" };

type Variant = "part" | "ext" | "master";

interface Props {
  variant?: Variant;
  /** Label shown in the header (e.g. "EX", "Master"). For parts, shows "Ch N" select. */
  label?: string;
  part?: PartState;
  partIndex?: number;
  eqExpanded: boolean;
  /** EQ state override (Master uses its own EQ, not part EQ). */
  eq?: EqState;
  /** Level override (EX and Master don't use part.level). */
  level?: number;
  /** Muted override (EX doesn't use part.muted). */
  muted?: boolean;
  onLevelChange: (value: number) => void;
  onPanChange?: (value: number) => void;
  onMuteToggle?: () => void;
  onChorusSendChange?: (value: number) => void;
  onReverbSendChange?: (value: number) => void;
  onReceiveChannelChange?: (channel: number) => void;
  onEqToggle?: () => void;
  onEqParam?: (paramOffset: number, value: number) => void;
}

function toneLabel(part: PartState): string {
  return part.toneName || `${part.toneBankMsb}-${part.toneBankLsb}-${part.tonePC + 1}`;
}

export function ChannelStrip({
  variant = "part",
  label,
  part,
  partIndex = 0,
  eqExpanded,
  eq,
  level,
  muted,
  onLevelChange,
  onPanChange,
  onMuteToggle,
  onChorusSendChange,
  onReverbSendChange,
  onReceiveChannelChange,
  onEqToggle,
  onEqParam,
}: Props) {
  const p = part ?? defaultPartState();
  const eqState = eq ?? p.eq;
  const faderValue = level ?? p.level;
  const isMuted = muted ?? p.muted;
  const isPart = variant === "part";
  const showEq = variant !== "ext";
  const showPan = variant === "part";
  const showSends = variant === "part";
  const showMute = variant !== "master";
  const showToneName = variant === "part";

  const hideIf = (visible: boolean) => (visible ? undefined : hide);

  const stripClass = [
    css.strip,
    variant === "ext" ? css.extOverride : "",
    variant === "master" ? css.masterOverride : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={stripClass}>
      {/* Header: channel select for parts, static label for EX/Master */}
      {isPart ? (
        <select
          className={css.channelSelect}
          value={p.receiveChannel}
          onChange={(e) => onReceiveChannelChange?.(Number(e.target.value))}
          title={`Part ${partIndex + 1}`}
        >
          {Array.from({ length: 16 }, (_, i) => (
            <option key={i} value={i}>
              Ch {i + 1}
            </option>
          ))}
        </select>
      ) : (
        <div className={css.partNumber}>{label}</div>
      )}

      {/* EQ section — hidden for EX (no EQ on external input) */}
      {eqExpanded && (
        <EqSection
          eq={eqState}
          onToggleSwitch={onEqToggle ?? noop}
          onParam={onEqParam ?? noop}
          {...(variant === "master" ? { paramBase: 0 } : {})}
          style={hideIf(showEq)}
        />
      )}

      {/* Pan knob — hidden for EX and Master but still rendered */}
      <PanKnob
        value={p.pan}
        onChange={onPanChange ?? noop}
        style={hideIf(showPan)}
      />

      {/* Sends — hidden for EX and Master */}
      <div className={css.sends} style={hideIf(showSends)}>
        <EqKnob
          label="FX1"
          value={p.chorusSend}
          min={0}
          max={127}
          defaultValue={0}
          onChange={onChorusSendChange ?? noop}
          formatValue={(v) => String(v)}
          color="#668"
        />
        <EqKnob
          label="FX2"
          value={p.reverbSend}
          min={0}
          max={127}
          defaultValue={0}
          onChange={onReverbSendChange ?? noop}
          formatValue={(v) => String(v)}
          color="#686"
        />
      </div>

      {/* Mute — hidden for Master */}
      <span className={css.muteLabel} style={hideIf(showMute)}>MUTE</span>
      <button
        className={`${css.muteButton} ${isMuted ? css.muted : ""}`}
        onClick={onMuteToggle ?? noop}
        style={hideIf(showMute)}
      >
        M
      </button>

      {/* Fader area */}
      <div className={css.faderArea}>
        {showToneName && (
          <span className={css.toneName}>{toneLabel(p)}</span>
        )}
        <VolumeFader value={faderValue} onChange={onLevelChange} />
      </div>
    </div>
  );
}
