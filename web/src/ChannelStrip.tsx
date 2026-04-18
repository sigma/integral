import { memo } from "react";
import { VolumeFader } from "./VolumeFader";
import { PanKnob } from "./PanKnob";
import { EqKnob } from "./EqKnob";
import { EqSection } from "./EqSection";
import { defaultPartState, type PartState, type EqState, type CompEqUnit } from "./types";
import { paramMeta } from "./paramDefaults";
import { CATEGORIES, lookupToneCategory } from "./categories";
import { categoryIcon, stripIcon } from "./categoryIcons";
import css from "./ChannelStrip.module.css";

const noop = () => {};
const hide: React.CSSProperties = { visibility: "hidden", pointerEvents: "none" };

type Variant = "part" | "ext" | "master" | "comp-eq";

const ATTACK_VALUES = [
  "0.05", "0.06", "0.07", "0.08", "0.09", "0.10", "0.12", "0.14",
  "0.16", "0.18", "0.20", "0.25", "0.30", "0.35", "0.40", "0.50",
  "0.60", "0.70", "0.80", "0.90", "1.0", "2.0", "4.0", "6.0",
  "8.0", "10", "15", "20", "25", "30", "40", "50",
];
const RELEASE_VALUES = [
  "0.05", "0.07", "0.10", "0.50", "1", "5", "10", "17",
  "25", "50", "75", "100", "150", "200", "300", "400",
  "500", "600", "700", "800", "900", "1000", "1200", "2000",
];
const RATIO_VALUES = [
  "1:1", "1.5:1", "2:1", "3:1", "4:1", "5:1", "6:1", "7:1",
  "8:1", "9:1", "10:1", "12:1", "14:1", "16:1", "20:1", "24:1",
  "30:1", "40:1", "100:1", "inf:1",
];
const OUTPUT_ASSIGN_NAMES = ["PART", "A", "B", "C", "D", "1", "2", "3", "4", "5", "6", "7", "8"];

interface Props {
  variant?: Variant;
  label?: string;
  part?: PartState;
  partIndex?: number;
  eqExpanded: boolean;
  eq?: EqState;
  level?: number;
  muted?: boolean;
  // -- comp-eq variant props --
  compEqUnit?: CompEqUnit;
  compEqOutputAssign?: number;
  /** Whether this part is the Comp+EQ assigned part (shows active indicator). */
  compEqAssigned?: boolean;
  onCompEqParam?: (paramOffset: number, value: number) => void;
  onCompEqOutputAssign?: (value: number) => void;
  /** Toggle Comp+EQ assignment to this part. */
  onCompEqToggle?: () => void;
  // -- standard callbacks --
  onLevelChange?: (value: number) => void;
  onPanChange?: (value: number) => void;
  onMuteToggle?: () => void;
  soloed?: boolean;
  onSoloToggle?: () => void;
  onChorusSendChange?: (value: number) => void;
  onReverbSendChange?: (value: number) => void;
  onReceiveChannelChange?: (channel: number) => void;
  /** When surround is enabled, show AMB send instead of FX1/FX2. */
  surroundEnabled?: boolean;
  ambienceSend?: number;
  onAmbienceSendChange?: (value: number) => void;
  onEqToggle?: () => void;
  onEqParam?: (paramOffset: number, value: number) => void;
}

function toneLabel(part: PartState): string {
  return part.toneName || `${part.toneBankMsb}-${part.toneBankLsb}-${part.tonePC + 1}`;
}

/** Map CompEqUnit EQ fields to an EqState for the EqSection component. */
function compEqToEqState(u: CompEqUnit): EqState {
  return {
    enabled: u.eqSwitch,
    lowFreq: u.eqLowFreq,
    lowGain: u.eqLowGain,
    midFreq: u.eqMidFreq,
    midGain: u.eqMidGain,
    midQ: u.eqMidQ,
    highFreq: u.eqHighFreq,
    highGain: u.eqHighGain,
  };
}

function ChannelStripInner({
  variant = "part",
  label,
  part,
  partIndex = 0,
  eqExpanded,
  eq,
  level,
  muted,
  compEqUnit,
  compEqOutputAssign,
  compEqAssigned,
  onCompEqParam,
  onCompEqOutputAssign,
  onCompEqToggle,
  onLevelChange,
  onPanChange,
  onMuteToggle,
  soloed,
  onSoloToggle,
  onChorusSendChange,
  onReverbSendChange,
  onReceiveChannelChange,
  surroundEnabled,
  ambienceSend,
  onAmbienceSendChange,
  onEqToggle,
  onEqParam,
}: Props) {
  const p = part ?? defaultPartState();
  const isCompEq = variant === "comp-eq";
  const isPart = variant === "part";
  const showEq = variant !== "ext";
  const showPan = isPart;
  const showSends = isPart;
  const showMute = variant !== "master" && !isCompEq;
  const showToneName = isPart;
  const showFader = !isCompEq;

  const eqState = isCompEq && compEqUnit ? compEqToEqState(compEqUnit) : (eq ?? p.eq);
  const faderValue = level ?? p.level;
  const isMuted = muted ?? p.muted;

  const hideIf = (visible: boolean) => (visible ? undefined : hide);

  const stripClass = [
    css.strip,
    variant === "ext" ? css.extOverride : "",
    variant === "master" ? css.masterOverride : "",
    isCompEq ? css.compEqOverride : "",
  ]
    .filter(Boolean)
    .join(" ");

  // Comp+EQ variant: EQ param callbacks route through onCompEqParam
  // with offsets 0x06–0x0D (the EQ portion of the 14-byte unit).
  const compEqOnEqToggle = () => {
    if (compEqUnit && onCompEqParam) {
      onCompEqParam(0x06, compEqUnit.eqSwitch ? 0 : 1);
    }
  };
  const compEqOnEqParam = (paramOffset: number, value: number) => {
    // EqSection uses paramBase=0, so offset 0=lowFreq, 1=lowGain, etc.
    // Map to Comp+EQ unit offsets: 0x07=lowFreq, 0x08=lowGain, ...
    onCompEqParam?.(0x07 + paramOffset, value);
  };

  return (
    <div className={stripClass}>
      {/* Header */}
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
      ) : isCompEq ? (
        <select
          className={css.channelSelect}
          value={compEqOutputAssign ?? 0}
          onChange={(e) => onCompEqOutputAssign?.(Number(e.target.value))}
          title={`${label} Output`}
        >
          {OUTPUT_ASSIGN_NAMES.map((name, v) => (
            <option key={v} value={v}>Out: {name}</option>
          ))}
        </select>
      ) : (
        <div className={css.partNumber}>{label}</div>
      )}

      {/* EQ section */}
      {eqExpanded && (
        <EqSection
          eq={eqState}
          onToggleSwitch={isCompEq ? compEqOnEqToggle : (onEqToggle ?? noop)}
          onParam={isCompEq ? compEqOnEqParam : (onEqParam ?? noop)}
          {...(variant === "master" || isCompEq ? { paramBase: 0 } : {})}
          style={hideIf(showEq)}
        />
      )}

      {/* Pan knob — hidden for non-part variants */}
      {isCompEq ? (
        // Compressor switch + knobs in place of pan+sends
        <button
          className={`${css.compSwitchButton} ${compEqUnit?.compSwitch ? css.compSwitchOn : css.compSwitchOff}`}
          onClick={() => onCompEqParam?.(0x00, compEqUnit?.compSwitch ? 0 : 1)}
        >
          {compEqUnit?.compSwitch ? "COMP ON" : "COMP OFF"}
        </button>
      ) : (
        <PanKnob
          value={p.pan}
          onChange={onPanChange ?? noop}
          style={hideIf(showPan)}
        />
      )}

      {/* Sends / Compressor knobs */}
      {isCompEq ? (
        <div className={css.sends}>
          <EqKnob label="Atk" value={compEqUnit?.compAttack ?? 10} min={0} max={31} defaultValue={10}
            onChange={(v) => onCompEqParam?.(0x01, v)}
            formatValue={(v) => ATTACK_VALUES[v] ?? String(v)} color="#c96" />
          <EqKnob label="Rel" value={compEqUnit?.compRelease ?? 10} min={0} max={23} defaultValue={10}
            onChange={(v) => onCompEqParam?.(0x02, v)}
            formatValue={(v) => RELEASE_VALUES[v] ?? String(v)} color="#c96" />
        </div>
      ) : (
        <div className={css.sends} style={hideIf(showSends)}>
          {surroundEnabled ? (
            <EqKnob label="AMB" value={ambienceSend ?? 0} min={0} max={127} defaultValue={0}
              onChange={onAmbienceSendChange ?? noop} formatValue={(v) => String(v)} color="#a6f" />
          ) : (
            <>
              {(() => { const m = paramMeta(`part.${partIndex}.chorus_send`); return (
              <EqKnob label={m.name} value={p.chorusSend} min={m.min} max={m.max} defaultValue={m.defaultValue}
                onChange={onChorusSendChange ?? noop} formatValue={(v) => String(v)} color="#668" />
              ); })()}
              {(() => { const m = paramMeta(`part.${partIndex}.reverb_send`); return (
              <EqKnob label={m.name} value={p.reverbSend} min={m.min} max={m.max} defaultValue={m.defaultValue}
                onChange={onReverbSendChange ?? noop} formatValue={(v) => String(v)} color="#686" />
              ); })()}
            </>
          )}
        </div>
      )}

      {/* Mute / More comp knobs */}
      {isCompEq ? (
        <>
          <div className={css.sends}>
            <EqKnob label="Thr" value={compEqUnit?.compThreshold ?? 127} min={0} max={127} defaultValue={127}
              onChange={(v) => onCompEqParam?.(0x03, v)}
              formatValue={(v) => String(v)} color="#c96" />
            <EqKnob label="Ratio" value={compEqUnit?.compRatio ?? 0} min={0} max={19} defaultValue={0}
              onChange={(v) => onCompEqParam?.(0x04, v)}
              formatValue={(v) => RATIO_VALUES[v] ?? String(v)} color="#c96" />
          </div>
          <div className={css.sends}>
            <EqKnob label="Gain" value={compEqUnit?.compOutputGain ?? 0} min={0} max={24} defaultValue={0}
              onChange={(v) => onCompEqParam?.(0x05, v)}
              formatValue={(v) => `+${v}dB`} color="#c96" />
          </div>
        </>
      ) : (
        <>
          <div className={css.muteRow} style={hideIf(showMute)}>
            <button
              className={`${css.muteButton} ${isMuted ? css.muted : ""}`}
              onClick={onMuteToggle ?? noop}
            >
              M
            </button>
            {isPart && (
              <button
                className={`${css.muteButton} ${soloed ? css.soloed : ""}`}
                onClick={onSoloToggle ?? noop}
              >
                S
              </button>
            )}
          </div>
          {isPart ? (
            <button
              className={`${css.compEqButton} ${compEqAssigned ? css.compOn : ""}`}
              onClick={onCompEqToggle ?? noop}
              title="Assign Drum Comp+EQ to this part"
            >
              C+EQ
            </button>
          ) : (
            <div className={css.compEqButton} style={hide} />
          )}
        </>
      )}

      {/* Fader area */}
      {showFader ? (
        <div className={css.faderArea}>
          {showToneName && <span className={css.toneName}>{toneLabel(p)}</span>}
          <VolumeFader value={faderValue} onChange={onLevelChange ?? noop} />
        </div>
      ) : (
        // Invisible fader to maintain alignment
        <div className={css.faderArea} style={hide}>
          <VolumeFader value={0} onChange={noop} />
        </div>
      )}

      {/* Category / role icon + label */}
      {(() => {
        if (isPart) {
          const catId = lookupToneCategory(p.toneBankMsb, p.toneBankLsb, p.tonePC);
          const icon = categoryIcon(catId);
          const name = CATEGORIES[catId];
          if (!name || catId === 0) return <div className={css.categoryArea} />;
          return (
            <div className={css.categoryArea}>
              {icon && (
                <div
                  className={css.categoryIcon}
                  dangerouslySetInnerHTML={{ __html: icon }}
                />
              )}
              <span className={css.categoryLabel}>{name}</span>
            </div>
          );
        }
        // Non-part strips: show role icon for alignment.
        const role = variant === "ext" ? "ext" : variant === "master" ? "master" : "";
        const icon = stripIcon(role);
        const roleLabel = variant === "ext" ? "Ext In" : variant === "master" ? "Master" : "";
        return (
          <div className={css.categoryArea}>
            {icon && (
              <div
                className={css.categoryIcon}
                dangerouslySetInnerHTML={{ __html: icon }}
              />
            )}
            {roleLabel && <span className={css.categoryLabel}>{roleLabel}</span>}
          </div>
        );
      })()}
    </div>
  );
}

export const ChannelStrip = memo(ChannelStripInner);
