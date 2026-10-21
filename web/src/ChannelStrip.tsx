import { memo } from "react";
import { VolumeFader } from "./VolumeFader";
import { PanKnob } from "./PanKnob";
import { EqKnob } from "./EqKnob";
import { GridSection, type Binding } from "./layouts";
import { EQ_LAYOUT } from "./layouts/mixer/eq";
import box from "./PanelBox.module.css";
import { TransferCurve } from "./TransferCurve";
import { ADSREnvelope } from "./synth-ui";
import { defaultPartState, type PartState, type EqState, type CompEqUnit } from "./types";
import { paramFormatter, paramMeta } from "./paramDefaults";
import { CATEGORIES, lookupToneCategory } from "./categories";
import { categoryIcon, stripIcon } from "./categoryIcons";
import { MixerField } from "../pkg/integral_wasm.js";
import css from "./ChannelStrip.module.css";
import { deviceSpec } from "./deviceSpec";
import { RouteChip } from "./RouteChip";
import { NotLoadedMarker } from "./NotLoadedMarker";

const noop = () => {};
const hide: React.CSSProperties = { visibility: "hidden", pointerEvents: "none" };

type Variant = "part" | "ext" | "master" | "comp-eq";

const formatCompRatio = paramFormatter(MixerField.CompRatio);

/** Numeric ratio for a `CompRatio` value ("inf:1" is a limiter). */
function ratioOf(i: number): number {
  const r = parseFloat(formatCompRatio(i));
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
  /** Whether core holds this part's Comp+EQ assignment active (a drum kit, or assigned). */
  compEqActive?: boolean;
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
  /** Set a mixer field (EQ, and the compressor on a Comp+EQ strip) in this strip's scope. */
  onField?: (field: MixerField, value: number) => void;
  onOutputAssignChange?: (value: number) => void;
}

function toneLabel(part: PartState): string {
  return part.toneName || `${part.toneBankMsb}-${part.toneBankLsb}-${part.tonePC + 1}`;
}

/** The `EqState` member holding each EQ knob's value. */
const EQ_STATE_KEY: Partial<Record<MixerField, Exclude<keyof EqState, "enabled">>> = {
  [MixerField.EqLowFreq]: "lowFreq",
  [MixerField.EqLowGain]: "lowGain",
  [MixerField.EqMidFreq]: "midFreq",
  [MixerField.EqMidGain]: "midGain",
  [MixerField.EqMidQ]: "midQ",
  [MixerField.EqHighFreq]: "highFreq",
  [MixerField.EqHighGain]: "highGain",
};

/** Gains read in dB; core's labels carry the sign only. */
const EQ_GAINS: ReadonlySet<MixerField> = new Set([MixerField.EqLowGain, MixerField.EqMidGain, MixerField.EqHighGain]);

/** `eq`'s raw value of the EQ field `field`. */
function eqValue(eq: EqState, field: MixerField): number {
  if (field === MixerField.EqSwitch) return Number(eq.enabled);
  const key = EQ_STATE_KEY[field];
  if (key === undefined) throw new Error(`${MixerField[field]} is not an EQ field`);
  return eq[key];
}

/** Binds the EQ layout's fields to `eq`, writing through `onField` in the strip's Scope. */
function eqBinding(eq: EqState, onField: (field: MixerField, value: number) => void) {
  return (field: MixerField): Binding => {
    const meta = paramMeta(field);
    const fmt = paramFormatter(field);
    const unit = EQ_GAINS.has(field) ? "dB" : "";
    return {
      ...meta,
      value: eqValue(eq, field),
      format: (v) => `${fmt(v)}${unit}`,
      onChange: (v) => onField(field, v),
    };
  };
}

/** The EQ Section's accent: the family of its gain knobs. */
const eqFamily = () => paramMeta(MixerField.EqMidGain).family;

/** Map CompEqUnit EQ fields to an EqState for the EQ Section. */
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
  compEqActive,
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
  onField,
  onOutputAssignChange,
}: Props) {
  const p = part ?? defaultPartState();
  // Drum Comp+EQ compressor ranges, shared by every unit.
  const attack = paramMeta(MixerField.CompAttack);
  const release = paramMeta(MixerField.CompRelease);
  const threshold = paramMeta(MixerField.CompThreshold);
  const ratio = paramMeta(MixerField.CompRatio);
  const outputGain = paramMeta(MixerField.CompOutputGain);
  const chorusSend = paramMeta(MixerField.PartChorusSend);
  const reverbSend = paramMeta(MixerField.PartReverbSend);
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
      <GridSection
        layout={EQ_LAYOUT}
        bind={eqBinding(eqState, onField ?? noop)}
        family={eqFamily()}
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
                onClick={() => onField?.(MixerField.CompSwitch, compEqUnit?.compSwitch ? 0 : 1)}
              >
                {compEqUnit?.compSwitch ? "COMP ON" : "COMP OFF"}
              </button>
              {/* Curve plots table index, not ms: the tables are non-linear. */}
              <ADSREnvelope
                compact
                attack={{ label: "Atk", value: compEqUnit?.compAttack ?? attack.defaultValue, min: attack.min, max: attack.max,
                  defaultValue: attack.defaultValue, onChange: (v) => onField?.(MixerField.CompAttack, v),
                  formatValue: paramFormatter(MixerField.CompAttack) }}
                decay={{ label: "Rel", value: compEqUnit?.compRelease ?? release.defaultValue, min: release.min, max: release.max,
                  defaultValue: release.defaultValue, onChange: (v) => onField?.(MixerField.CompRelease, v),
                  formatValue: paramFormatter(MixerField.CompRelease) }}
              />
              <TransferCurve
                threshold={compEqUnit?.compThreshold ?? threshold.defaultValue}
                ratio={ratioOf(compEqUnit?.compRatio ?? ratio.defaultValue)}
                max={threshold.max}
              />
              <div className={css.sends}>
                <EqKnob label="Thr" value={compEqUnit?.compThreshold ?? threshold.defaultValue} min={threshold.min}
                  max={threshold.max} defaultValue={threshold.defaultValue}
                  onChange={(v) => onField?.(MixerField.CompThreshold, v)}
                  formatValue={paramFormatter(MixerField.CompThreshold)} family={threshold.family} />
                <EqKnob label="Ratio" value={compEqUnit?.compRatio ?? ratio.defaultValue} min={ratio.min} max={ratio.max}
                  defaultValue={ratio.defaultValue} onChange={(v) => onField?.(MixerField.CompRatio, v)}
                  formatValue={formatCompRatio} family={ratio.family} />
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
                      <EqKnob label={chorusSend.name} value={p.chorusSend} min={chorusSend.min} max={chorusSend.max}
                        defaultValue={chorusSend.defaultValue} onChange={onChorusSendChange ?? noop}
                        formatValue={(v) => String(v)} color="#668" />
                      <EqKnob label={reverbSend.name} value={p.reverbSend} min={reverbSend.min} max={reverbSend.max}
                        defaultValue={reverbSend.defaultValue} onChange={onReverbSendChange ?? noop}
                        formatValue={(v) => String(v)} color="#686" />
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
                      disabled={!compEqActive}
                      title={compEqActive
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
          <VolumeFader value={compEqUnit?.compOutputGain ?? outputGain.defaultValue} onChange={(v) => onField?.(MixerField.CompOutputGain, v)}
            min={outputGain.min} max={outputGain.max} defaultValue={outputGain.defaultValue}
            formatValue={(v) => `+${v}dB`} label="Gain" />
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
