import { memo } from "react";
import { VolumeFader } from "./VolumeFader";
import { PanKnob } from "./PanKnob";
import { EqKnob } from "./EqKnob";
import { EqSection } from "./EqSection";
import box from "./PanelBox.module.css";
import { TransferCurve } from "./TransferCurve";
import { ADSREnvelope } from "./synth-ui";
import { defaultPartState, type PartState, type EqState, type CompEqUnit } from "./types";
import { paramMeta } from "./paramDefaults";
import { CATEGORIES, lookupToneCategory } from "./categories";
import { categoryIcon, stripIcon } from "./categoryIcons";
import { is_drum_kit_bank } from "../pkg/integral_wasm.js";
import css from "./ChannelStrip.module.css";
import { deviceSpec } from "./deviceSpec";
import { RouteChip } from "./RouteChip";
import { NotLoadedMarker } from "./NotLoadedMarker";

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
/** Numeric ratio for a RATIO_VALUES index ("inf:1" is a limiter). */
function ratioOf(i: number): number {
  const r = parseFloat(RATIO_VALUES[i] ?? "1");
  return Number.isNaN(r) ? Infinity : r;
}

interface Props {
  variant?: Variant;
  label?: string;
  part?: PartState;
  /** The Expansion of the part's tone, when known not to be loaded. */
  unloadedExpansion?: string | null;
  partIndex?: number;
  eq?: EqState;
  level?: number;
  muted?: boolean;
  // -- comp-eq variant props --
  compEqUnit?: CompEqUnit;
  /** 1-based Comp+EQ unit number, shown under the fader. */
  compEqUnitNumber?: number;
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
  /** Part is the one selected for editing. */
  selected?: boolean;
  /** Pointer pressed anywhere on the strip, including on its controls. */
  onSelect?: () => void;
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
  onOutputAssignChange?: (value: number) => void;
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
  unloadedExpansion,
  partIndex = 0,
  eq,
  level,
  muted,
  compEqUnit,
  compEqUnitNumber,
  compEqOutputAssign,
  compEqAssigned,
  onCompEqParam,
  onCompEqOutputAssign,
  onCompEqToggle,
  onLevelChange,
  onPanChange,
  onMuteToggle,
  selected,
  onSelect,
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
  onOutputAssignChange,
}: Props) {
  const p = part ?? defaultPartState();
  const isCompEq = variant === "comp-eq";
  const isPart = variant === "part";
  const showEq = variant !== "ext";
  const showMute = variant !== "master" && !isCompEq;
  const roleLabel = variant === "ext" ? "Ext In" : variant === "master" ? "Master" : `C+E${compEqUnitNumber}`;
  const faderLabel = isPart
    ? toneLabel(p)
    : isCompEq
      ? `Compressor ${compEqUnitNumber} Gain`
      : variant === "ext"
        ? "External Input"
        : roleLabel;

  const eqState = isCompEq && compEqUnit ? compEqToEqState(compEqUnit) : (eq ?? p.eq);
  const faderValue = level ?? p.level;
  const isMuted = muted ?? p.muted;

  const hideIf = (visible: boolean) => (visible ? undefined : hide);

  const stripClass = [
    css.strip,
    selected ? css.selected : "",
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
    <div className={stripClass} onPointerDown={onSelect}>
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
      ) : (
        <div className={css.partNumber}>{label}</div>
      )}

      {/* EQ section */}
      <EqSection
        eq={eqState}
        onToggleSwitch={isCompEq ? compEqOnEqToggle : (onEqToggle ?? noop)}
        onParam={isCompEq ? compEqOnEqParam : (onEqParam ?? noop)}
        {...(variant === "master" || isCompEq ? { paramBase: 0 } : {})}
        style={hideIf(showEq)}
      />

      {/* Pan/Sends and the toggles (or the comp box) sit on the fader; the free space opens above them. */}
      {(showMute || isCompEq) && (
        <div className={css.bottom}>
          {isCompEq && (
          // Compressor box: sits on its gain fader, which greys out with it.
            <div className={`${box.box} ${compEqUnit?.compSwitch ? "" : box.off}`}>
              <button
                className={`${box.switchButton} ${compEqUnit?.compSwitch ? box.switchOn : box.switchOff}`}
                onClick={() => onCompEqParam?.(0x00, compEqUnit?.compSwitch ? 0 : 1)}
              >
                {compEqUnit?.compSwitch ? "COMP ON" : "COMP OFF"}
              </button>
              {/* Curve plots table index, not ms: the tables are non-linear. */}
              <ADSREnvelope
                compact
                attack={{ label: "Atk", value: compEqUnit?.compAttack ?? 10, min: 0, max: 31, defaultValue: 10,
                  onChange: (v) => onCompEqParam?.(0x01, v), formatValue: (v) => ATTACK_VALUES[v] ?? String(v) }}
                decay={{ label: "Rel", value: compEqUnit?.compRelease ?? 10, min: 0, max: 23, defaultValue: 10,
                  onChange: (v) => onCompEqParam?.(0x02, v), formatValue: (v) => RELEASE_VALUES[v] ?? String(v) }}
              />
              <TransferCurve
                threshold={compEqUnit?.compThreshold ?? 127}
                ratio={ratioOf(compEqUnit?.compRatio ?? 0)}
                max={127}
              />
              <div className={css.sends}>
                <EqKnob label="Thr" value={compEqUnit?.compThreshold ?? 127} min={0} max={127} defaultValue={127}
                  onChange={(v) => onCompEqParam?.(0x03, v)}
                  formatValue={(v) => String(v)} color="#c96" />
                <EqKnob label="Ratio" value={compEqUnit?.compRatio ?? 0} min={0} max={19} defaultValue={0}
                  onChange={(v) => onCompEqParam?.(0x04, v)}
                  formatValue={(v) => RATIO_VALUES[v] ?? String(v)} color="#c96" />
              </div>
            </div>
          )}
          {isPart && (
            // Pan and sends boxes: same box as the EQ, at natural height.
            <>
              <div className={box.box}>
                <PanKnob value={p.pan} onChange={onPanChange ?? noop} />
              </div>
              <div className={box.box}>
                <div className={css.sends}>
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
              </div>
            </>
          )}

          {/* Toggles sit directly above the fader; M/S is the bottom row on every strip. */}
          {(showMute || isCompEq) && (
            <div className={css.toggles}>
              {(isPart || isCompEq) && (
                <div className={css.muteRow}>
                  {isPart ? (
                    <RouteChip value={p.outputAssign} names={deviceSpec().output_assigns}
                      onChange={onOutputAssignChange ?? noop} overridden={surroundEnabled}
                      title={`Part ${partIndex + 1} Output`} />
                  ) : (
                    <RouteChip value={compEqOutputAssign ?? 0} names={deviceSpec().comp_eq_output_assigns}
                      onChange={onCompEqOutputAssign ?? noop} overridden={surroundEnabled}
                      title={`${label} Output`} />
                  )}
                  {isPart && (
                    <button
                      className={`${css.compEqButton} ${compEqAssigned ? css.compOn : ""}`}
                      onClick={onCompEqToggle ?? noop}
                      // Only drum kits route keys to the Comp+EQ units; keep it enabled while assigned so it can be turned off.
                      disabled={!compEqAssigned && !is_drum_kit_bank(p.toneBankMsb)}
                      title={compEqAssigned || is_drum_kit_bank(p.toneBankMsb)
                        ? "Assign Drum Comp+EQ to this part"
                        : "Drum Comp+EQ only works on drum kit parts"}
                    >
                      C+EQ
                    </button>
                  )}
                </div>
              )}
              {/* M/S is the bottom row on every strip; Comp+EQ keeps an invisible one so its chip lines up. */}
              <div className={css.muteRow} style={isCompEq ? hide : undefined}>
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
            </div>
          )}
        </div>
      )}

      {/* Fader area: level, or makeup gain for Comp+EQ */}
      <div className={`${css.faderArea} ${isCompEq && !compEqUnit?.compSwitch ? box.off : ""}`}>
        <span className={css.faderLabel}>
          {/* Ahead of the tone name, so a long name can't clip it. */}
          {isPart && unloadedExpansion && (
            <NotLoadedMarker expansion={unloadedExpansion} named />
          )}
          {faderLabel}
        </span>
        {isCompEq ? (
          <VolumeFader value={compEqUnit?.compOutputGain ?? 0} onChange={(v) => onCompEqParam?.(0x05, v)}
            min={0} max={24} defaultValue={0} formatValue={(v) => `+${v}dB`} label="Gain" />
        ) : (
          <VolumeFader value={faderValue} onChange={onLevelChange ?? noop} />
        )}
      </div>

      {/* Category / role icon + label */}
      {(() => {
        if (isPart) {
          // The tone's own category covers user tones; the factory table fills in until it's read.
          const catId = p.toneCategory || lookupToneCategory(p.toneBankMsb, p.toneBankLsb, p.tonePC);
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
        const icon = stripIcon(variant);
        return (
          <div className={css.categoryArea}>
            {icon && (
              <div
                className={css.categoryIcon}
                dangerouslySetInnerHTML={{ __html: icon }}
              />
            )}
            <span className={css.categoryLabel}>{roleLabel}</span>
          </div>
        );
      })()}
    </div>
  );
}

export const ChannelStrip = memo(ChannelStripInner);
