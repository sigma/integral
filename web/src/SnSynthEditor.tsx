import { useEffect, useState, useCallback } from "react";
import { EqKnob } from "./EqKnob";
import { MfxEditor } from "./MfxEditor";
import type { IntegraService } from "./integra";
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

interface MfxState {
  mfxType: number;
  chorusSend: number;
  reverbSend: number;
  controls: { source: number; sens: number; assign: number }[];
  params: number[];
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
const FILTER_MODE_NAMES = ["BYPASS", "LPF", "HPF", "BPF", "PKG", "LPF2", "LPF3", "LPF4"];
const FILTER_SLOPE_NAMES = ["-12dB", "-24dB"];
const LFO_SHAPE_NAMES = ["TRI", "SIN", "SAW", "SQR", "S&H", "RND"];
const RING_NAMES = ["OFF", "", "ON"]; // 0=OFF, 2=ON
const UNISON_SIZE_NAMES = ["2", "4", "6", "8"];
const WAVE_GAIN_NAMES = ["-6dB", "0dB", "+6dB", "+12dB"];

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
// Sub-tab type
// ---------------------------------------------------------------------------

type SubTab = "common" | "partial0" | "partial1" | "partial2" | "mfx";

const SUB_TABS: { key: SubTab; label: string }[] = [
  { key: "common", label: "Common" },
  { key: "partial0", label: "Partial 1" },
  { key: "partial1", label: "Partial 2" },
  { key: "partial2", label: "Partial 3" },
  { key: "mfx", label: "MFX" },
];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface Props {
  partIndex: number;
  service: IntegraService;
}

export function SnSynthEditor({ partIndex, service }: Props) {
  const [subTab, setSubTab] = useState<SubTab>("common");
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
          case 0x20: u.lfoFadeTime = value; break;
          case 0x21: u.lfoKeyTrigger = value; break;
          case 0x22: u.lfoPitchDepth = value; break;
          case 0x23: u.lfoFilterDepth = value; break;
          case 0x24: u.lfoAmpDepth = value; break;
          case 0x25: u.lfoPanDepth = value; break;
          case 0x26: u.modLfoShape = value; break;
          case 0x27: u.modLfoRate = value; break;
          case 0x28: u.modLfoTempoSync = value; break;
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
        const next = { ...prev };
        if (offset === 0x02) next.chorusSend = value;
        else if (offset === 0x03) next.reverbSend = value;
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

      {/* Sub-tab bar */}
      <div className={css.tabBar}>
        {SUB_TABS.map((t) => (
          <button
            key={t.key}
            className={`${css.tab} ${subTab === t.key ? css.tabActive : ""}`}
            onClick={() => setSubTab(t.key)}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Common tab */}
      {subTab === "common" && common && (
        <CommonTab common={common} onChange={setCommonParam} />
      )}

      {/* Partial tabs */}
      {subTab === "partial0" && partials[0] && (
        <PartialTab partial={partials[0]} index={0} onChange={setPartialParam} />
      )}
      {subTab === "partial1" && partials[1] && (
        <PartialTab partial={partials[1]} index={1} onChange={setPartialParam} />
      )}
      {subTab === "partial2" && partials[2] && (
        <PartialTab partial={partials[2]} index={2} onChange={setPartialParam} />
      )}

      {/* MFX tab */}
      {subTab === "mfx" && mfx && (
        <MfxEditor
          partIndex={partIndex}
          mfxType={mfx.mfxType}
          chorusSend={mfx.chorusSend}
          reverbSend={mfx.reverbSend}
          controls={mfx.controls}
          params={mfx.params}
          onTypeChange={handleMfxTypeChange}
          onHeaderParam={handleMfxHeaderParam}
          onNibParam={handleMfxNibParam}
        />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Common Tab
// ---------------------------------------------------------------------------

function CommonTab({
  common,
  onChange,
}: {
  common: SnSynthCommon;
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <div className={css.paramSection}>
      <div className={css.paramGrid}>
        <EqKnob label="Level" value={common.toneLevel} min={0} max={127} defaultValue={127}
          onChange={(v) => onChange(0x0C, v)} formatValue={(v) => String(v)} color="#8cf" />
        <EqKnob label="Wave Shape" value={common.waveShape} min={0} max={127} defaultValue={0}
          onChange={(v) => onChange(0x35, v)} formatValue={(v) => String(v)} color="#8cf" />
        <EqKnob label="Analog Feel" value={common.analogFeel} min={0} max={127} defaultValue={0}
          onChange={(v) => onChange(0x34, v)} formatValue={(v) => String(v)} color="#8cf" />
        <EqKnob label="Oct Shift" value={common.octaveShift} min={61} max={67} defaultValue={64}
          onChange={(v) => onChange(0x15, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cf" />
        <EqKnob label="PB Up" value={common.pitchBendRangeUp} min={0} max={24} defaultValue={2}
          onChange={(v) => onChange(0x16, v)} formatValue={(v) => String(v)} color="#8cf" />
        <EqKnob label="PB Down" value={common.pitchBendRangeDown} min={0} max={24} defaultValue={2}
          onChange={(v) => onChange(0x17, v)} formatValue={(v) => String(v)} color="#8cf" />
        <EqKnob label="Porta Time" value={common.portamentoTime} min={0} max={127} defaultValue={0}
          onChange={(v) => onChange(0x13, v)} formatValue={(v) => String(v)} color="#8cf" />
      </div>

      <div className={css.sectionLabel}>Switches</div>
      <div className={css.switchRow}>
        <SelectParam label="Ring" value={common.ringSwitch}
          options={[{ v: 0, l: "OFF" }, { v: 2, l: "ON" }]}
          onChange={(v) => onChange(0x1F, v)} />
        <SelectParam label="Unison" value={common.unisonSwitch}
          options={[{ v: 0, l: "OFF" }, { v: 1, l: "ON" }]}
          onChange={(v) => onChange(0x2E, v)} />
        <SelectParam label="Uni Size" value={common.unisonSize}
          options={UNISON_SIZE_NAMES.map((l, i) => ({ v: i, l }))}
          onChange={(v) => onChange(0x3C, v)} />
        <SelectParam label="Porta" value={common.portamentoSwitch}
          options={[{ v: 0, l: "OFF" }, { v: 1, l: "ON" }]}
          onChange={(v) => onChange(0x12, v)} />
        <SelectParam label="Porta Mode" value={common.portamentoMode}
          options={[{ v: 0, l: "NORMAL" }, { v: 1, l: "LEGATO" }]}
          onChange={(v) => onChange(0x31, v)} />
        <SelectParam label="Mono" value={common.monoSwitch}
          options={[{ v: 0, l: "OFF" }, { v: 1, l: "ON" }]}
          onChange={(v) => onChange(0x14, v)} />
        <SelectParam label="Legato" value={common.legatoSwitch}
          options={[{ v: 0, l: "OFF" }, { v: 1, l: "ON" }]}
          onChange={(v) => onChange(0x32, v)} />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Partial Tab
// ---------------------------------------------------------------------------

function PartialTab({
  partial,
  index,
  onChange,
}: {
  partial: SnSynthPartial;
  index: number;
  onChange: (partial: number, offset: number, value: number) => void;
}) {
  const set = (offset: number, value: number) => onChange(index, offset, value);

  return (
    <div className={css.paramSection}>
      {/* OSC */}
      <div className={css.sectionLabel}>OSC</div>
      <div className={css.paramGrid}>
        <SelectParam label="Wave" value={partial.oscWave}
          options={OSC_WAVE_NAMES.map((l, i) => ({ v: i, l }))}
          onChange={(v) => set(0x00, v)} />
        <EqKnob label="Pitch" value={partial.oscPitch} min={40} max={88} defaultValue={64}
          onChange={(v) => set(0x03, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8" />
        <EqKnob label="Detune" value={partial.oscDetune} min={14} max={114} defaultValue={64}
          onChange={(v) => set(0x04, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8" />
        <EqKnob label="PW Mod" value={partial.oscPwModDepth} min={0} max={127} defaultValue={0}
          onChange={(v) => set(0x05, v)} formatValue={(v) => String(v)} color="#fc8" />
        <EqKnob label="Pulse W" value={partial.oscPulseWidth} min={0} max={127} defaultValue={0}
          onChange={(v) => set(0x06, v)} formatValue={(v) => String(v)} color="#fc8" />
        <EqKnob label="PE Atk" value={partial.oscPitchEnvAttack} min={0} max={127} defaultValue={0}
          onChange={(v) => set(0x07, v)} formatValue={(v) => String(v)} color="#fc8" />
        <EqKnob label="PE Dcy" value={partial.oscPitchEnvDecay} min={0} max={127} defaultValue={0}
          onChange={(v) => set(0x08, v)} formatValue={(v) => String(v)} color="#fc8" />
        <EqKnob label="PE Depth" value={partial.oscPitchEnvDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => set(0x09, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8" />
        <EqKnob label="S-Saw Det" value={partial.superSawDetune} min={0} max={127} defaultValue={0}
          onChange={(v) => set(0x3A, v)} formatValue={(v) => String(v)} color="#fc8" />
        <EqKnob label="HPF Cut" value={partial.hpfCutoff} min={0} max={127} defaultValue={0}
          onChange={(v) => set(0x39, v)} formatValue={(v) => String(v)} color="#fc8" />
        <SelectParam label="Wav Gain" value={partial.waveGain}
          options={WAVE_GAIN_NAMES.map((l, i) => ({ v: i, l }))}
          onChange={(v) => set(0x34, v)} />
      </div>

      {/* Filter */}
      <div className={css.sectionLabel}>Filter</div>
      <div className={css.paramGrid}>
        <SelectParam label="Mode" value={partial.filterMode}
          options={FILTER_MODE_NAMES.map((l, i) => ({ v: i, l }))}
          onChange={(v) => set(0x0A, v)} />
        <SelectParam label="Slope" value={partial.filterSlope}
          options={FILTER_SLOPE_NAMES.map((l, i) => ({ v: i, l }))}
          onChange={(v) => set(0x0B, v)} />
        <EqKnob label="Cutoff" value={partial.filterCutoff} min={0} max={127} defaultValue={127}
          onChange={(v) => set(0x0C, v)} formatValue={(v) => String(v)} color="#8fc" />
        <EqKnob label="Keyflw" value={partial.filterKeyfollow} min={54} max={74} defaultValue={64}
          onChange={(v) => set(0x0D, v)} formatValue={(v) => signedFmt(v, 64)} color="#8fc" />
        <EqKnob label="Env Vel" value={partial.filterEnvVelSens} min={1} max={127} defaultValue={64}
          onChange={(v) => set(0x0E, v)} formatValue={(v) => signedFmt(v, 64)} color="#8fc" />
        <EqKnob label="Reso" value={partial.filterResonance} min={0} max={127} defaultValue={0}
          onChange={(v) => set(0x0F, v)} formatValue={(v) => String(v)} color="#8fc" />
        <EqKnob label="FE Atk" value={partial.filterEnvAttack} min={0} max={127} defaultValue={0}
          onChange={(v) => set(0x10, v)} formatValue={(v) => String(v)} color="#8fc" />
        <EqKnob label="FE Dcy" value={partial.filterEnvDecay} min={0} max={127} defaultValue={0}
          onChange={(v) => set(0x11, v)} formatValue={(v) => String(v)} color="#8fc" />
        <EqKnob label="FE Sus" value={partial.filterEnvSustain} min={0} max={127} defaultValue={0}
          onChange={(v) => set(0x12, v)} formatValue={(v) => String(v)} color="#8fc" />
        <EqKnob label="FE Rel" value={partial.filterEnvRelease} min={0} max={127} defaultValue={0}
          onChange={(v) => set(0x13, v)} formatValue={(v) => String(v)} color="#8fc" />
        <EqKnob label="FE Depth" value={partial.filterEnvDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => set(0x14, v)} formatValue={(v) => signedFmt(v, 64)} color="#8fc" />
      </div>

      {/* Amp */}
      <div className={css.sectionLabel}>Amp</div>
      <div className={css.paramGrid}>
        <EqKnob label="Level" value={partial.ampLevel} min={0} max={127} defaultValue={127}
          onChange={(v) => set(0x15, v)} formatValue={(v) => String(v)} color="#f8c" />
        <EqKnob label="Vel Sens" value={partial.ampVelSens} min={1} max={127} defaultValue={64}
          onChange={(v) => set(0x16, v)} formatValue={(v) => signedFmt(v, 64)} color="#f8c" />
        <EqKnob label="AE Atk" value={partial.ampEnvAttack} min={0} max={127} defaultValue={0}
          onChange={(v) => set(0x17, v)} formatValue={(v) => String(v)} color="#f8c" />
        <EqKnob label="AE Dcy" value={partial.ampEnvDecay} min={0} max={127} defaultValue={0}
          onChange={(v) => set(0x18, v)} formatValue={(v) => String(v)} color="#f8c" />
        <EqKnob label="AE Sus" value={partial.ampEnvSustain} min={0} max={127} defaultValue={127}
          onChange={(v) => set(0x19, v)} formatValue={(v) => String(v)} color="#f8c" />
        <EqKnob label="AE Rel" value={partial.ampEnvRelease} min={0} max={127} defaultValue={0}
          onChange={(v) => set(0x1A, v)} formatValue={(v) => String(v)} color="#f8c" />
        <EqKnob label="Pan" value={partial.ampPan} min={0} max={127} defaultValue={64}
          onChange={(v) => set(0x1B, v)} formatValue={panFmt} color="#f8c" />
        <EqKnob label="Lvl Keyflw" value={partial.ampLevelKeyfollow} min={54} max={74} defaultValue={64}
          onChange={(v) => set(0x3C, v)} formatValue={(v) => signedFmt(v, 64)} color="#f8c" />
      </div>

      {/* LFO */}
      <div className={css.sectionLabel}>LFO</div>
      <div className={css.paramGrid}>
        <SelectParam label="Shape" value={partial.lfoShape}
          options={LFO_SHAPE_NAMES.map((l, i) => ({ v: i, l }))}
          onChange={(v) => set(0x1C, v)} />
        <EqKnob label="Rate" value={partial.lfoRate} min={0} max={127} defaultValue={0}
          onChange={(v) => set(0x1D, v)} formatValue={(v) => String(v)} color="#c8f" />
        <SelectParam label="Tempo Sync" value={partial.lfoTempoSync}
          options={[{ v: 0, l: "OFF" }, { v: 1, l: "ON" }]}
          onChange={(v) => set(0x1E, v)} />
        <EqKnob label="Fade" value={partial.lfoFadeTime} min={0} max={127} defaultValue={0}
          onChange={(v) => set(0x20, v)} formatValue={(v) => String(v)} color="#c8f" />
        <SelectParam label="Key Trig" value={partial.lfoKeyTrigger}
          options={[{ v: 0, l: "OFF" }, { v: 1, l: "ON" }]}
          onChange={(v) => set(0x21, v)} />
        <EqKnob label="Pitch D" value={partial.lfoPitchDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => set(0x22, v)} formatValue={(v) => signedFmt(v, 64)} color="#c8f" />
        <EqKnob label="Filter D" value={partial.lfoFilterDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => set(0x23, v)} formatValue={(v) => signedFmt(v, 64)} color="#c8f" />
        <EqKnob label="Amp D" value={partial.lfoAmpDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => set(0x24, v)} formatValue={(v) => signedFmt(v, 64)} color="#c8f" />
        <EqKnob label="Pan D" value={partial.lfoPanDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => set(0x25, v)} formatValue={(v) => signedFmt(v, 64)} color="#c8f" />
      </div>

      {/* Mod LFO */}
      <div className={css.sectionLabel}>Mod LFO</div>
      <div className={css.paramGrid}>
        <SelectParam label="Shape" value={partial.modLfoShape}
          options={LFO_SHAPE_NAMES.map((l, i) => ({ v: i, l }))}
          onChange={(v) => set(0x26, v)} />
        <EqKnob label="Rate" value={partial.modLfoRate} min={0} max={127} defaultValue={0}
          onChange={(v) => set(0x27, v)} formatValue={(v) => String(v)} color="#ca8" />
        <SelectParam label="Tempo Sync" value={partial.modLfoTempoSync}
          options={[{ v: 0, l: "OFF" }, { v: 1, l: "ON" }]}
          onChange={(v) => set(0x28, v)} />
        <EqKnob label="PW Shift" value={partial.pwShift} min={0} max={127} defaultValue={0}
          onChange={(v) => set(0x2A, v)} formatValue={(v) => String(v)} color="#ca8" />
        <EqKnob label="Pitch D" value={partial.modLfoPitchDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => set(0x2C, v)} formatValue={(v) => signedFmt(v, 64)} color="#ca8" />
        <EqKnob label="Filter D" value={partial.modLfoFilterDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => set(0x2D, v)} formatValue={(v) => signedFmt(v, 64)} color="#ca8" />
        <EqKnob label="Amp D" value={partial.modLfoAmpDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => set(0x2E, v)} formatValue={(v) => signedFmt(v, 64)} color="#ca8" />
        <EqKnob label="Pan D" value={partial.modLfoPanDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => set(0x2F, v)} formatValue={(v) => signedFmt(v, 64)} color="#ca8" />
        <EqKnob label="Rate Ctrl" value={partial.modLfoRateControl} min={1} max={127} defaultValue={64}
          onChange={(v) => set(0x3B, v)} formatValue={(v) => signedFmt(v, 64)} color="#ca8" />
      </div>

      {/* Aftertouch */}
      <div className={css.sectionLabel}>Aftertouch</div>
      <div className={css.paramGrid}>
        <EqKnob label="Cutoff" value={partial.aftertouchCutoff} min={1} max={127} defaultValue={64}
          onChange={(v) => set(0x30, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cc" />
        <EqKnob label="Level" value={partial.aftertouchLevel} min={1} max={127} defaultValue={64}
          onChange={(v) => set(0x31, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cc" />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Select dropdown helper
// ---------------------------------------------------------------------------

function SelectParam({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: number;
  options: { v: number; l: string }[];
  onChange: (v: number) => void;
}) {
  return (
    <label className={css.selectLabel}>
      {label}
      <select className={css.select} value={value}
        onChange={(e) => onChange(Number(e.target.value))}>
        {options.map((o) => (
          <option key={o.v} value={o.v}>{o.l}</option>
        ))}
      </select>
    </label>
  );
}

// Suppress unused-variable warnings for display helpers referenced in types
void RING_NAMES;
