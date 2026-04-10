import { useEffect, useState, useCallback } from "react";
import type { IntegraService } from "./integra";
import {
  SynthKnob,
  SynthFader,
  SynthSwitch,
  SynthSelect,
  SectionPanel,
  ADSREnvelope,
  ADEnvelope,
  FaderGroup,
  FaderGroupSep,
} from "./synth-ui";
import { MfxPanel } from "./MfxPanel";
import type { MfxState } from "./MfxPanel";
import css from "./SnSynthEditor.module.css";

// ---------------------------------------------------------------------------
// Types mirroring the Rust serde output
// ---------------------------------------------------------------------------

interface SnSynthCommon {
  toneName: string;
  toneLevel: number;
  portamentoSwitch: number;
  portamentoTime: number;
  monoSwitch: number;
  octaveShift: number;
  pitchBendRangeUp: number;
  pitchBendRangeDown: number;
  partial1Switch: number;
  partial1Select: number;
  partial2Switch: number;
  partial2Select: number;
  partial3Switch: number;
  partial3Select: number;
  ringSwitch: number;
  tfxSwitch: number;
  unisonSwitch: number;
  portamentoMode: number;
  legatoSwitch: number;
  analogFeel: number;
  waveShape: number;
  toneCategory: number;
  phraseNumber: number;
  phraseOctaveShift: number;
  unisonSize: number;
}

interface SnSynthPartial {
  oscWave: number;
  oscWaveVariation: number;
  oscPitch: number;
  oscDetune: number;
  oscPwModDepth: number;
  oscPulseWidth: number;
  oscPitchEnvAttack: number;
  oscPitchEnvDecay: number;
  oscPitchEnvDepth: number;
  filterMode: number;
  filterSlope: number;
  filterCutoff: number;
  filterKeyfollow: number;
  filterEnvVelSens: number;
  filterResonance: number;
  filterEnvAttack: number;
  filterEnvDecay: number;
  filterEnvSustain: number;
  filterEnvRelease: number;
  filterEnvDepth: number;
  ampLevel: number;
  ampVelSens: number;
  ampEnvAttack: number;
  ampEnvDecay: number;
  ampEnvSustain: number;
  ampEnvRelease: number;
  ampPan: number;
  lfoShape: number;
  lfoRate: number;
  lfoTempoSync: number;
  lfoTempoSyncNote: number;
  lfoFadeTime: number;
  lfoKeyTrigger: number;
  lfoPitchDepth: number;
  lfoFilterDepth: number;
  lfoAmpDepth: number;
  lfoPanDepth: number;
  modLfoShape: number;
  modLfoRate: number;
  modLfoTempoSync: number;
  modLfoTempoSyncNote: number;
  pwShift: number;
  modLfoPitchDepth: number;
  modLfoFilterDepth: number;
  modLfoAmpDepth: number;
  modLfoPanDepth: number;
  aftertouchCutoff: number;
  aftertouchLevel: number;
  waveGain: number;
  waveNumber: number;
  hpfCutoff: number;
  superSawDetune: number;
  modLfoRateControl: number;
  ampLevelKeyfollow: number;
}

// ---------------------------------------------------------------------------
// Address helpers
// ---------------------------------------------------------------------------

/** Compute SN-S base address bytes for a part. */
function snsBaseAddress(part: number): [number, number] {
  const partTotal = part * 0x20;
  const toneBase0 = 0x19 + Math.floor(partTotal / 128);
  const toneBase1 = partTotal % 128;
  // Add SN-S type offset (01 00 00) to byte1
  const snsBase1 = toneBase1 + 1;
  const carry = Math.floor(snsBase1 / 128);
  return [toneBase0 + carry, snsBase1 % 128];
}

function snsCommonAddress(part: number): number[] {
  const [b0, b1] = snsBaseAddress(part);
  return [b0, b1, 0x00, 0x00];
}

function snsPartialAddress(part: number, partial: number): number[] {
  const [b0, b1] = snsBaseAddress(part);
  return [b0, b1, 0x20 + partial, 0x00];
}

// MFX block is at sns_base + 00 02 00
function mfxAddress(part: number): number[] {
  const [b0, b1] = snsBaseAddress(part);
  return [b0, b1, 0x02, 0x00];
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

const OSC_WAVE_NAMES = ["SAW", "SQR", "PW-SQR", "TRI", "SINE", "NOISE", "SP-SAW", "PCM"];
const OSC_VARIATION_NAMES = ["A", "B", "C"];
const FILTER_MODE_NAMES = ["BYPASS", "LPF", "HPF", "BPF", "PKG", "LPF2", "LPF3", "LPF4"];
const FILTER_SLOPE_NAMES = ["-12dB", "-24dB"];
const LFO_SHAPE_NAMES = ["TRI", "SIN", "SAW", "SQR", "S&H", "RND"];
const UNISON_SIZE_NAMES = ["2", "4", "6", "8"];
const WAVE_GAIN_NAMES = ["-6dB", "0dB", "+6dB", "+12dB"];
const TEMPO_SYNC_NOTE_NAMES = [
  "16", "12", "8", "4", "2", "1", "3/4", "2/3", "1/2", "3/8",
  "1/3", "1/4", "3/16", "1/6", "1/8", "3/32", "1/12", "1/16", "1/24", "1/32",
];

const ON_OFF_OPTIONS = [{ value: 0, label: "OFF" }, { value: 1, label: "ON" }];

function signedFmt(raw: number, center: number): string {
  const v = raw - center;
  return v > 0 ? `+${v}` : String(v);
}

function panFmt(v: number): string {
  if (v === 64) return "C";
  if (v < 64) return `L${64 - v}`;
  return `${v - 64}R`;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface Props {
  partIndex: number;
  service: IntegraService;
}

export function SnSynthEditor({ partIndex, service }: Props) {
  const [common, setCommon] = useState<SnSynthCommon | null>(null);
  const [partials, setPartials] = useState<(SnSynthPartial | null)[]>([null, null, null]);
  const [mfx, setMfx] = useState<MfxState | null>(null);
  const [loading, setLoading] = useState(true);

  // Load all data on mount / part change
  useEffect(() => {
    let cancelled = false;
    setLoading(true);

    async function load() {
      try {
        // Common
        const commonData = await service.requestData(
          snsCommonAddress(partIndex),
          [0x00, 0x00, 0x00, 0x40],
        );
        if (cancelled) return;
        const c = service.device.applySnsCommon(commonData) as SnSynthCommon | null;
        if (c) setCommon(c);

        // Partials
        for (let i = 0; i < 3; i++) {
          const pData = await service.requestData(
            snsPartialAddress(partIndex, i),
            [0x00, 0x00, 0x00, 0x3D],
          );
          if (cancelled) return;
          const p = service.device.applySnsPartial(pData) as SnSynthPartial | null;
          if (p) {
            setPartials((prev) => {
              const next = [...prev];
              next[i] = p;
              return next;
            });
          }
        }

        // MFX
        const mfxData = await service.requestData(
          mfxAddress(partIndex),
          [0x00, 0x00, 0x01, 0x11],
        );
        if (cancelled) return;
        const m = service.device.applyMfxBlock(partIndex, mfxData) as MfxState | null;
        if (m) setMfx(m);
      } catch {
        // Timeout — partial data is OK
      }
      if (!cancelled) setLoading(false);
    }

    load();
    return () => { cancelled = true; };
  }, [partIndex, service]);

  // ---------------------------------------------------------------------------
  // Common param setter
  // ---------------------------------------------------------------------------
  const setCommonParam = useCallback(
    (offset: number, value: number) => {
      service.device.setSnsCommonParam(partIndex, offset, value);
      setCommon((prev) => {
        if (!prev) return prev;
        const next = { ...prev };
        switch (offset) {
          case 0x0C: next.toneLevel = value; break;
          case 0x12: next.portamentoSwitch = value; break;
          case 0x13: next.portamentoTime = value; break;
          case 0x14: next.monoSwitch = value; break;
          case 0x15: next.octaveShift = value; break;
          case 0x16: next.pitchBendRangeUp = value; break;
          case 0x17: next.pitchBendRangeDown = value; break;
          case 0x19: next.partial1Switch = value; break;
          case 0x1A: next.partial1Select = value; break;
          case 0x1B: next.partial2Switch = value; break;
          case 0x1C: next.partial2Select = value; break;
          case 0x1D: next.partial3Switch = value; break;
          case 0x1E: next.partial3Select = value; break;
          case 0x1F: next.ringSwitch = value; break;
          case 0x2E: next.unisonSwitch = value; break;
          case 0x31: next.portamentoMode = value; break;
          case 0x32: next.legatoSwitch = value; break;
          case 0x34: next.analogFeel = value; break;
          case 0x35: next.waveShape = value; break;
          case 0x3C: next.unisonSize = value; break;
        }
        return next;
      });
    },
    [partIndex, service],
  );

  // ---------------------------------------------------------------------------
  // Partial param setter
  // ---------------------------------------------------------------------------
  const setPartialParam = useCallback(
    (partial: number, offset: number, value: number) => {
      console.log(`[sns] setPartialParam part=${partIndex} partial=${partial} offset=0x${offset.toString(16)} value=${value}`);
      service.device.setSnsPartialParam(partIndex, partial, offset, value);
      setPartials((prev) => {
        const next = [...prev];
        const p = next[partial];
        if (!p) return prev;
        const u = { ...p };
        switch (offset) {
          case 0x00: u.oscWave = value; break;
          case 0x01: u.oscWaveVariation = value; break;
          case 0x03: u.oscPitch = value; break;
          case 0x04: u.oscDetune = value; break;
          case 0x05: u.oscPwModDepth = value; break;
          case 0x06: u.oscPulseWidth = value; break;
          case 0x07: u.oscPitchEnvAttack = value; break;
          case 0x08: u.oscPitchEnvDecay = value; break;
          case 0x09: u.oscPitchEnvDepth = value; break;
          case 0x0A: u.filterMode = value; break;
          case 0x0B: u.filterSlope = value; break;
          case 0x0C: u.filterCutoff = value; break;
          case 0x0D: u.filterKeyfollow = value; break;
          case 0x0E: u.filterEnvVelSens = value; break;
          case 0x0F: u.filterResonance = value; break;
          case 0x10: u.filterEnvAttack = value; break;
          case 0x11: u.filterEnvDecay = value; break;
          case 0x12: u.filterEnvSustain = value; break;
          case 0x13: u.filterEnvRelease = value; break;
          case 0x14: u.filterEnvDepth = value; break;
          case 0x15: u.ampLevel = value; break;
          case 0x16: u.ampVelSens = value; break;
          case 0x17: u.ampEnvAttack = value; break;
          case 0x18: u.ampEnvDecay = value; break;
          case 0x19: u.ampEnvSustain = value; break;
          case 0x1A: u.ampEnvRelease = value; break;
          case 0x1B: u.ampPan = value; break;
          case 0x1C: u.lfoShape = value; break;
          case 0x1D: u.lfoRate = value; break;
          case 0x1E: u.lfoTempoSync = value; break;
          case 0x1F: u.lfoTempoSyncNote = value; break;
          case 0x20: u.lfoFadeTime = value; break;
          case 0x21: u.lfoKeyTrigger = value; break;
          case 0x22: u.lfoPitchDepth = value; break;
          case 0x23: u.lfoFilterDepth = value; break;
          case 0x24: u.lfoAmpDepth = value; break;
          case 0x25: u.lfoPanDepth = value; break;
          case 0x26: u.modLfoShape = value; break;
          case 0x27: u.modLfoRate = value; break;
          case 0x28: u.modLfoTempoSync = value; break;
          case 0x29: u.modLfoTempoSyncNote = value; break;
          case 0x2A: u.pwShift = value; break;
          case 0x2C: u.modLfoPitchDepth = value; break;
          case 0x2D: u.modLfoFilterDepth = value; break;
          case 0x2E: u.modLfoAmpDepth = value; break;
          case 0x2F: u.modLfoPanDepth = value; break;
          case 0x30: u.aftertouchCutoff = value; break;
          case 0x31: u.aftertouchLevel = value; break;
          case 0x34: u.waveGain = value; break;
          case 0x39: u.hpfCutoff = value; break;
          case 0x3A: u.superSawDetune = value; break;
          case 0x3B: u.modLfoRateControl = value; break;
          case 0x3C: u.ampLevelKeyfollow = value; break;
        }
        next[partial] = u;
        return next;
      });
    },
    [partIndex, service],
  );

  // ---------------------------------------------------------------------------
  // MFX callbacks
  // ---------------------------------------------------------------------------
  const handleMfxTypeChange = useCallback(
    (type_: number) => {
      service.device.setMfxParam(partIndex, 0x00, type_);
      setMfx((prev) => prev ? { ...prev, mfxType: type_, params: [] } : prev);
    },
    [partIndex, service],
  );

  const handleMfxHeaderParam = useCallback(
    (offset: number, value: number) => {
      service.device.setMfxParam(partIndex, offset, value);
      setMfx((prev) => {
        if (!prev) return prev;
        const next = { ...prev, controls: [...prev.controls] };
        if (offset === 0x02) next.chorusSend = value;
        else if (offset === 0x03) next.reverbSend = value;
        // Control source/sens/assign updates
        for (let s = 0; s < 4; s++) {
          if (offset === 0x05 + s * 2) {
            next.controls[s] = { ...next.controls[s]!, source: value };
          } else if (offset === 0x06 + s * 2) {
            next.controls[s] = { ...next.controls[s]!, sens: value };
          } else if (offset === 0x0D + s) {
            next.controls[s] = { ...next.controls[s]!, assign: value };
          }
        }
        return next;
      });
    },
    [partIndex, service],
  );

  const handleMfxNibParam = useCallback(
    (paramIndex: number, value: number) => {
      service.device.setMfxNibParam(partIndex, paramIndex, value);
      setMfx((prev) => {
        if (!prev) return prev;
        const params = [...prev.params];
        params[paramIndex] = value;
        return { ...prev, params };
      });
    },
    [partIndex, service],
  );

  // ---------------------------------------------------------------------------
  // Partial selector helpers
  // ---------------------------------------------------------------------------
  const partialSwitches = common
    ? [common.partial1Switch, common.partial2Switch, common.partial3Switch]
    : [0, 0, 0];

  const togglePartialSwitch = (idx: number) => {
    const offsets: Record<number, number> = { 0: 0x19, 1: 0x1B, 2: 0x1D };
    const current = partialSwitches[idx] ?? 0;
    setCommonParam(offsets[idx] ?? 0x19, current !== 0 ? 0 : 1);
  };

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  if (loading) {
    return <div className={css.loading}>Loading SN-S tone data...</div>;
  }

  return (
    <div className={css.editor}>
      {/* Tone name */}
      {common && (
        <div className={css.toneName}>{common.toneName || "(unnamed)"}</div>
      )}

      {/* Common Controls Strip */}
      {common && <CommonStrip common={common} onChange={setCommonParam} />}

      {/* 3 partial rows + MFX sidebar */}
      <div className={css.mainArea}>
        <div className={css.partialRows}>
          {[0, 1, 2].map((idx) => {
            const partial = partials[idx] ?? null;
            const isOn = (partialSwitches[idx] ?? 0) !== 0;
            const setP = (offset: number, value: number) => setPartialParam(idx, offset, value);
            const setNibP = (offset: number, value: number) => {
              service.device.setSnsPartialNibParam(partIndex, idx, offset, value);
              setPartials((prev) => {
                const next = [...prev];
                const p = next[idx];
                if (!p) return prev;
                const u = { ...p };
                if (offset === 0x35) u.waveNumber = value;
                next[idx] = u;
                return next;
              });
            };

            return (
              <PartialRow
                key={idx}
                idx={idx}
                partial={partial}
                isOn={isOn}
                onToggle={() => togglePartialSwitch(idx)}
                setP={setP}
                setNibP={setNibP}
              />
            );
          })}
        </div>
        <div className={css.mfxSidebar}>
          {mfx && (
            <MfxPanel
              mfx={mfx}
              onTypeChange={handleMfxTypeChange}
              onHeaderParam={handleMfxHeaderParam}
              onNibParam={handleMfxNibParam}
            />
          )}
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Partial Row — renders SW + 5 section panels for one partial
// ---------------------------------------------------------------------------

function PartialRow({
  idx,
  partial,
  isOn,
  onToggle,
  setP,
  setNibP,
}: {
  idx: number;
  partial: SnSynthPartial | null;
  isOn: boolean;
  onToggle: () => void;
  setP: (offset: number, value: number) => void;
  setNibP: (offset: number, value: number) => void;
}) {
  const dimClass = isOn ? "" : css.partialRowDimmed;

  return (
    <div className={css.partialRow}>
      {/* Partial switch */}
      <div className={css.partialSwitch}>
        <button
          className={`${css.partialSwitchBtn} ${isOn ? css.partialSwitchBtnOn : ""}`}
          onClick={onToggle}
        >
          <span className={css.partialNum}>{idx + 1}</span>
          <span className={css.partialSwLabel}>{isOn ? "ON" : "OFF"}</span>
        </button>
      </div>

      {/* OSC */}
      <div className={dimClass}>
        {partial && <OscPanel partial={partial} onChange={setP} onNibChange={setNibP} />}
      </div>

      {/* Filter */}
      <div className={dimClass}>
        {partial && <FilterPanel partial={partial} onChange={setP} />}
      </div>

      {/* Amp */}
      <div className={dimClass}>
        {partial && <AmpPanel partial={partial} onChange={setP} />}
      </div>

      {/* Mod LFO */}
      <div className={dimClass}>
        {partial && <ModLfoPanel partial={partial} onChange={setP} />}
      </div>

      {/* LFO + Aftertouch */}
      <div className={dimClass}>
        {partial && <LfoPanel partial={partial} onChange={setP} />}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Common Controls Strip
// ---------------------------------------------------------------------------

function CommonStrip({
  common,
  onChange,
}: {
  common: SnSynthCommon;
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <div className={css.commonStrip}>
      {/* Unison group */}
      <div className={css.commonGroup}>
        <span className={css.groupLabel}>UNISON</span>
        <div className={css.groupRow}>
          <SynthSwitch label="SW" value={common.unisonSwitch} options={ON_OFF_OPTIONS}
            onChange={(v) => onChange(0x2E, v)} />
          <SynthKnob label="Size" value={common.unisonSize} min={0} max={3} defaultValue={0}
            onChange={(v) => onChange(0x3C, v)} formatValue={(v) => UNISON_SIZE_NAMES[v] ?? String(v)} color="#8cf" />
        </div>
      </div>

      <div className={css.commonDivider} />

      {/* Mono */}
      <div className={css.commonGroup}>
        <SynthSwitch label="MONO" value={common.monoSwitch} options={ON_OFF_OPTIONS}
          onChange={(v) => onChange(0x14, v)} />
      </div>

      <div className={css.commonDivider} />

      {/* Portamento group */}
      <div className={css.commonGroup}>
        <span className={css.groupLabel}>PORTAMENTO</span>
        <div className={css.groupRow}>
          <SynthSwitch label="SW" value={common.portamentoSwitch} options={ON_OFF_OPTIONS}
            onChange={(v) => onChange(0x12, v)} />
          <SynthKnob label="Time" value={common.portamentoTime} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x13, v)} formatValue={(v) => String(v)} color="#8cf" />
          <SynthSelect label="Mode" value={common.portamentoMode}
            options={[{ value: 0, label: "NORMAL" }, { value: 1, label: "LEGATO" }]}
            onChange={(v) => onChange(0x31, v)} />
        </div>
      </div>

      <div className={css.commonDivider} />

      {/* Bend Range */}
      <div className={css.commonGroup}>
        <span className={css.groupLabel}>BEND RANGE</span>
        <div className={css.groupRow}>
          <SynthKnob label="Down" value={common.pitchBendRangeDown} min={0} max={24} defaultValue={2}
            onChange={(v) => onChange(0x17, v)} formatValue={(v) => String(v)} color="#8cf" />
          <SynthKnob label="Up" value={common.pitchBendRangeUp} min={0} max={24} defaultValue={2}
            onChange={(v) => onChange(0x16, v)} formatValue={(v) => String(v)} color="#8cf" />
        </div>
      </div>

      <div className={css.commonDivider} />

      {/* Standalone params */}
      <SynthKnob label="Wave Shape" value={common.waveShape} min={0} max={127} defaultValue={0}
        onChange={(v) => onChange(0x35, v)} formatValue={(v) => String(v)} color="#8cf" />
      <SynthKnob label="Tone Level" value={common.toneLevel} min={0} max={127} defaultValue={127}
        onChange={(v) => onChange(0x0C, v)} formatValue={(v) => String(v)} color="#8cf" />
      <SynthKnob label="Analog Feel" value={common.analogFeel} min={0} max={127} defaultValue={0}
        onChange={(v) => onChange(0x34, v)} formatValue={(v) => String(v)} color="#8cf" />

      <div className={css.commonDivider} />

      <SynthSwitch label="LEGATO" value={common.legatoSwitch} options={ON_OFF_OPTIONS}
        onChange={(v) => onChange(0x32, v)} />

      <SynthKnob label="Oct Shift" value={common.octaveShift} min={61} max={67} defaultValue={64}
        onChange={(v) => onChange(0x15, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cf" />
    </div>
  );
}

// ---------------------------------------------------------------------------
// OSC Panel
// ---------------------------------------------------------------------------

function OscPanel({
  partial,
  onChange,
  onNibChange,
}: {
  partial: SnSynthPartial;
  onChange: (offset: number, value: number) => void;
  onNibChange: (offset: number, value: number) => void;
}) {
  const hasVariation = partial.oscWave <= 5; // SAW..NOISE
  const isPcm = partial.oscWave === 7;
  const isSuperSaw = partial.oscWave === 6;

  return (
    <SectionPanel label="OSC" accentColor="#fc8">
      <div className={css.panelRow}>
        <div className={css.oscSelectors}>
          <SynthSwitch label="Wave" value={partial.oscWave} vertical
            options={OSC_WAVE_NAMES.map((l, i) => ({ value: i, label: l }))}
            onChange={(v) => onChange(0x00, v)} led={false} />
          {hasVariation && (
            <SynthSwitch label="Var" value={partial.oscWaveVariation} vertical
              options={OSC_VARIATION_NAMES.map((l, i) => ({ value: i, label: l }))}
              onChange={(v) => onChange(0x01, v)} led={false} />
          )}
          {isPcm && (
            <label className={css.selectLabel}>
              PCM #
              <input
                type="number"
                className={css.waveNumberInput}
                value={partial.waveNumber}
                min={0}
                max={16384}
                onChange={(e) => {
                  const v = Math.max(0, Math.min(16384, Number(e.target.value)));
                  onNibChange(0x35, v);
                }}
              />
            </label>
          )}
        </div>
        <div className={css.oscKnobGrid}>
          <SynthKnob label="Pitch" value={partial.oscPitch} min={40} max={88} defaultValue={64}
            onChange={(v) => onChange(0x03, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8" />
          <SynthKnob label="HPF" value={partial.hpfCutoff} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x39, v)} formatValue={(v) => String(v)} color="#fc8" />
          <SynthKnob label="Detune" value={partial.oscDetune} min={14} max={114} defaultValue={64}
            onChange={(v) => onChange(0x04, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8" />
          {isSuperSaw && (
            <SynthKnob label="S-Saw" value={partial.superSawDetune} min={0} max={127} defaultValue={0}
              onChange={(v) => onChange(0x3A, v)} formatValue={(v) => String(v)} color="#fc8" />
          )}
        </div>
      </div>
      <FaderGroup>
        <div className={css.faderWithCurveSpace}>
          <SynthFader label="PWM" value={partial.oscPwModDepth} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x05, v)} compact />
        </div>
        <div className={css.faderWithCurveSpace}>
          <SynthFader label="PW" value={partial.oscPulseWidth} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x06, v)} compact />
        </div>
        <FaderGroupSep />
        <ADEnvelope
          compact
          attack={{
            label: "A", value: partial.oscPitchEnvAttack, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onChange(0x07, v),
          }}
          decay={{
            label: "D", value: partial.oscPitchEnvDecay, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onChange(0x08, v),
          }}
          extra={{
            label: "Dep", value: partial.oscPitchEnvDepth, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => onChange(0x09, v), formatValue: (v) => signedFmt(v, 64),
          }}
        />
      </FaderGroup>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Filter Panel
// ---------------------------------------------------------------------------

function FilterPanel({
  partial,
  onChange,
}: {
  partial: SnSynthPartial;
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="FILTER" accentColor="#68c">
      <div className={css.panelRow}>
        <SynthSwitch label="Mode" value={partial.filterMode} vertical
          options={FILTER_MODE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => onChange(0x0A, v)} />
        <SynthSwitch label="Slope" value={partial.filterSlope} vertical
          options={FILTER_SLOPE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => onChange(0x0B, v)} led={false} />
        <div className={`${css.oscKnobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Cutoff" value={partial.filterCutoff} min={0} max={127} defaultValue={127}
            onChange={(v) => onChange(0x0C, v)} formatValue={(v) => String(v)} color="#68c" />
          <SynthKnob label="Reso" value={partial.filterResonance} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x0F, v)} formatValue={(v) => String(v)} color="#68c" />
          <SynthKnob label="KeyF" value={partial.filterKeyfollow} min={54} max={74} defaultValue={64}
            onChange={(v) => onChange(0x0D, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c" />
          <SynthKnob label="Vel Sns" value={partial.filterEnvVelSens} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x0E, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c" />
        </div>
      </div>
      <ADSREnvelope
        compact
        attack={{
          label: "A", value: partial.filterEnvAttack, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onChange(0x10, v),
        }}
        decay={{
          label: "D", value: partial.filterEnvDecay, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onChange(0x11, v),
        }}
        sustain={{
          label: "S", value: partial.filterEnvSustain, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onChange(0x12, v),
        }}
        release={{
          label: "R", value: partial.filterEnvRelease, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onChange(0x13, v),
        }}
        extra={{
          label: "Dep", value: partial.filterEnvDepth, min: 1, max: 127, defaultValue: 64,
          onChange: (v) => onChange(0x14, v), formatValue: (v) => signedFmt(v, 64),
        }}
      />
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Amp Panel
// ---------------------------------------------------------------------------

function AmpPanel({
  partial,
  onChange,
}: {
  partial: SnSynthPartial;
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="AMP" accentColor="#6c8">
      <div className={css.panelRow}>
        <SynthSwitch label="Gain" value={partial.waveGain} vertical
          options={WAVE_GAIN_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => onChange(0x34, v)} led={false} />
        <div className={`${css.oscKnobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Level" value={partial.ampLevel} min={0} max={127} defaultValue={127}
            onChange={(v) => onChange(0x15, v)} formatValue={(v) => String(v)} color="#6c8" />
          <SynthKnob label="Pan" value={partial.ampPan} min={0} max={127} defaultValue={64}
            onChange={(v) => onChange(0x1B, v)} formatValue={panFmt} color="#6c8" />
          <SynthKnob label="Vel Sns" value={partial.ampVelSens} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x16, v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8" />
          <SynthKnob label="KeyF" value={partial.ampLevelKeyfollow} min={54} max={74} defaultValue={64}
            onChange={(v) => onChange(0x3C, v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8" />
        </div>
      </div>
      <ADSREnvelope
        compact
        attack={{
          label: "A", value: partial.ampEnvAttack, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onChange(0x17, v),
        }}
        decay={{
          label: "D", value: partial.ampEnvDecay, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onChange(0x18, v),
        }}
        sustain={{
          label: "S", value: partial.ampEnvSustain, min: 0, max: 127, defaultValue: 127,
          onChange: (v) => onChange(0x19, v),
        }}
        release={{
          label: "R", value: partial.ampEnvRelease, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onChange(0x1A, v),
        }}
      />
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Mod LFO Panel
// ---------------------------------------------------------------------------

function ModLfoPanel({
  partial,
  onChange,
}: {
  partial: SnSynthPartial;
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="MOD LFO" accentColor="#a6f">
      <div className={css.panelRow}>
        <SynthSwitch label="Shape" value={partial.modLfoShape} vertical
          options={LFO_SHAPE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => onChange(0x26, v)} led={false} />
        <div>
          <SynthSwitch label="T.Sync" value={partial.modLfoTempoSync} options={ON_OFF_OPTIONS}
            onChange={(v) => onChange(0x28, v)} />
          {partial.modLfoTempoSync !== 0 && (
            <SynthSelect label="Note" value={partial.modLfoTempoSyncNote}
              options={TEMPO_SYNC_NOTE_NAMES.map((l, i) => ({ value: i, label: l }))}
              onChange={(v) => onChange(0x29, v)} />
          )}
        </div>
        <div className={`${css.oscKnobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Rate" value={partial.modLfoRate} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x27, v)} formatValue={(v) => String(v)} color="#a6f" />
          <SynthKnob label="PW Shft" value={partial.pwShift} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x2A, v)} formatValue={(v) => String(v)} color="#a6f" />
          <SynthKnob label="Rate Ctrl" value={partial.modLfoRateControl} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x3B, v)} formatValue={(v) => signedFmt(v, 64)} color="#a6f" />
        </div>
      </div>
      <FaderGroup>
        <SynthFader label="Pit" value={partial.modLfoPitchDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => onChange(0x2C, v)} formatValue={(v) => signedFmt(v, 64)} compact />
        <SynthFader label="Flt" value={partial.modLfoFilterDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => onChange(0x2D, v)} formatValue={(v) => signedFmt(v, 64)} compact />
        <SynthFader label="Amp" value={partial.modLfoAmpDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => onChange(0x2E, v)} formatValue={(v) => signedFmt(v, 64)} compact />
        <SynthFader label="Pan" value={partial.modLfoPanDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => onChange(0x2F, v)} formatValue={(v) => signedFmt(v, 64)} compact />
      </FaderGroup>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// LFO Panel (includes Aftertouch at bottom)
// ---------------------------------------------------------------------------

function LfoPanel({
  partial,
  onChange,
}: {
  partial: SnSynthPartial;
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="LFO" accentColor="#a6f">
      <div className={css.panelRow}>
        <SynthSwitch label="Shape" value={partial.lfoShape} vertical
          options={LFO_SHAPE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => onChange(0x1C, v)} led={false} />
        <div>
          <SynthSwitch label="T.Sync" value={partial.lfoTempoSync} options={ON_OFF_OPTIONS}
            onChange={(v) => onChange(0x1E, v)} />
          {partial.lfoTempoSync !== 0 && (
            <SynthSelect label="Note" value={partial.lfoTempoSyncNote}
              options={TEMPO_SYNC_NOTE_NAMES.map((l, i) => ({ value: i, label: l }))}
              onChange={(v) => onChange(0x1F, v)} />
          )}
          <SynthSwitch label="KeyTrig" value={partial.lfoKeyTrigger} options={ON_OFF_OPTIONS}
            onChange={(v) => onChange(0x21, v)} />
        </div>
        <div className={`${css.oscKnobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Rate" value={partial.lfoRate} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x1D, v)} formatValue={(v) => String(v)} color="#a6f" />
          <SynthKnob label="Fade" value={partial.lfoFadeTime} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x20, v)} formatValue={(v) => String(v)} color="#a6f" />
          <SynthKnob label="Aft Cut" value={partial.aftertouchCutoff} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x30, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cc" />
          <SynthKnob label="Aft Lvl" value={partial.aftertouchLevel} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x31, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cc" />
        </div>
      </div>
      <FaderGroup>
          <SynthFader label="Pit" value={partial.lfoPitchDepth} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x22, v)} formatValue={(v) => signedFmt(v, 64)} compact />
          <SynthFader label="Flt" value={partial.lfoFilterDepth} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x23, v)} formatValue={(v) => signedFmt(v, 64)} compact />
          <SynthFader label="Amp" value={partial.lfoAmpDepth} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x24, v)} formatValue={(v) => signedFmt(v, 64)} compact />
          <SynthFader label="Pan" value={partial.lfoPanDepth} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x25, v)} formatValue={(v) => signedFmt(v, 64)} compact />
      </FaderGroup>
    </SectionPanel>
  );
}
