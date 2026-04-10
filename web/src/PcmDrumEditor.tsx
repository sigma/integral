import { useEffect, useState, useCallback, useRef } from "react";
import type { IntegraService } from "./integra";
import {
  SynthKnob,
  SynthFader,
  SynthSwitch,
  SynthSelect,
  SectionPanel,
  ADSREnvelope,
  FaderGroup,
  OutputStrip,
} from "./synth-ui";
import { MfxPanel } from "./MfxPanel";
import type { MfxState } from "./MfxPanel";
import css from "./PcmDrumEditor.module.css";

// ---------------------------------------------------------------------------
// Types mirroring the Rust serde output (PcmDrumCommon, PcmDrumPartial, etc.)
// ---------------------------------------------------------------------------

interface PcmDrumCommon {
  kitName: string;
  kitLevel: number;
}

interface PcmDrumWmt {
  wmtSwitch: number;
  waveGroupType: number;
  waveGroupId: number;
  waveNumberL: number;
  waveNumberR: number;
  waveGain: number;
  waveFxmSwitch: number;
  waveFxmColor: number;
  waveFxmDepth: number;
  waveTempoSync: number;
  coarseTune: number;
  fineTune: number;
  pan: number;
  randomPanSwitch: number;
  alternatePanSwitch: number;
  level: number;
  velocityRangeLower: number;
  velocityRangeUpper: number;
  velocityFadeLower: number;
  velocityFadeUpper: number;
}

interface PcmDrumPartial {
  partialName: string;
  assignType: number;
  muteGroup: number;
  level: number;
  coarseTune: number;
  fineTune: number;
  randomPitchDepth: number;
  pan: number;
  randomPanDepth: number;
  alternatePanDepth: number;
  envMode: number;
  outputLevel: number;
  chorusSend: number;
  reverbSend: number;
  outputAssign: number;
  pitchBendRange: number;
  receiveExpression: number;
  receiveHold1: number;
  wmtVelocityControl: number;
  wmt: PcmDrumWmt[];
  pitchEnvDepth: number;
  pitchEnvVelocitySens: number;
  pitchEnvT1VelocitySens: number;
  pitchEnvT4VelocitySens: number;
  pitchEnvTime: number[];
  pitchEnvLevel: number[];
  tvfFilterType: number;
  tvfCutoffFrequency: number;
  tvfCutoffVelocityCurve: number;
  tvfCutoffVelocitySens: number;
  tvfResonance: number;
  tvfResonanceVelocitySens: number;
  tvfEnvDepth: number;
  tvfEnvVelocityCurve: number;
  tvfEnvVelocitySens: number;
  tvfEnvT1VelocitySens: number;
  tvfEnvT4VelocitySens: number;
  tvfEnvTime: number[];
  tvfEnvLevel: number[];
  tvaLevelVelocityCurve: number;
  tvaLevelVelocitySens: number;
  tvaEnvT1VelocitySens: number;
  tvaEnvT4VelocitySens: number;
  tvaEnvTime: number[];
  tvaEnvLevel: number[];
  oneShotMode: number;
}

interface PcmDrumCommon2 {
  phraseNumber: number;
  tfxSwitch: number;
}

// ---------------------------------------------------------------------------
// Address helpers
// ---------------------------------------------------------------------------

function pcmdBaseAddress(part: number): [number, number] {
  const partTotal = part * 0x20;
  const toneBase0 = 0x19 + Math.floor(partTotal / 128);
  const toneBase1 = partTotal % 128;
  // PCM-D type offset = 10 00 00
  const b1 = toneBase1 + 0x10;
  const carry = Math.floor(b1 / 128);
  return [toneBase0 + carry, b1 % 128];
}

function pcmdCommonAddress(part: number): number[] {
  const [b0, b1] = pcmdBaseAddress(part);
  return [b0, b1, 0x00, 0x00];
}

function pcmdMfxAddress(part: number): number[] {
  const [b0, b1] = pcmdBaseAddress(part);
  return [b0, b1, 0x02, 0x00];
}

function pcmdCommon2Address(part: number): number[] {
  const [b0, b1] = pcmdBaseAddress(part);
  // Common2 is at offset 02 00 00 from PCM-D base
  const b1plus2 = b1 + 0x02;
  const carry = Math.floor(b1plus2 / 128);
  return [b0 + carry, b1plus2 % 128, 0x00, 0x00];
}

function pcmdPartialAddress(part: number, key: number): number[] {
  const [b0, b1] = pcmdBaseAddress(part);
  const keyOffset = key - 21;
  // Each key is spaced 00 02 00 apart, starting at byte2=0x10
  const byte2Total = 0x10 + keyOffset * 2;
  const byte1Add = Math.floor(byte2Total / 128);
  const byte2 = byte2Total % 128;
  const newB1 = b1 + byte1Add;
  const carry = Math.floor(newB1 / 128);
  return [b0 + carry, newB1 % 128, byte2, 0x00];
}

// ---------------------------------------------------------------------------
// Note name helpers
// ---------------------------------------------------------------------------

const NOTE_NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

function noteName(key: number): string {
  const name = NOTE_NAMES[key % 12]!;
  const octave = Math.floor(key / 12) - 1;
  return `${name}${octave}`;
}

function isBlackKey(key: number): boolean {
  const n = key % 12;
  return n === 1 || n === 3 || n === 6 || n === 8 || n === 10;
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

const FIRST_KEY = 21;
const LAST_KEY = 108;

function panFmt(raw: number): string {
  if (raw === 64) return "C";
  if (raw < 64) return `L${64 - raw}`;
  return `${raw - 64}R`;
}

function signedFmt(raw: number, center: number): string {
  const v = raw - center;
  return v > 0 ? `+${v}` : String(v);
}

const WAVE_GAIN_OPTIONS = [
  { value: 0, label: "-6dB" },
  { value: 1, label: "0dB" },
  { value: 2, label: "+6dB" },
  { value: 3, label: "+12dB" },
];

const WAVE_GROUP_OPTIONS = [
  { value: 0, label: "INT" },
  { value: 1, label: "SRX" },
  { value: 2, label: "SRX" },
  { value: 3, label: "SRX" },
];

const FILTER_TYPE_OPTIONS = [
  { value: 0, label: "OFF" },
  { value: 1, label: "LPF" },
  { value: 2, label: "BPF" },
  { value: 3, label: "HPF" },
  { value: 4, label: "PKG" },
  { value: 5, label: "LPF2" },
  { value: 6, label: "LPF3" },
];

const VELOCITY_CURVE_OPTIONS = [
  { value: 0, label: "FIXED" },
  { value: 1, label: "1" },
  { value: 2, label: "2" },
  { value: 3, label: "3" },
  { value: 4, label: "4" },
  { value: 5, label: "5" },
  { value: 6, label: "6" },
  { value: 7, label: "7" },
];

const ASSIGN_TYPE_OPTIONS = [
  { value: 0, label: "MULTI" },
  { value: 1, label: "SINGLE" },
];

const ENV_MODE_OPTIONS = [
  { value: 0, label: "NO-SUS" },
  { value: 1, label: "SUSTAIN" },
];

const OUTPUT_ASSIGN_OPTIONS = [
  { value: 0, label: "PART" },
  { value: 1, label: "C+EQ1" },
  { value: 2, label: "C+EQ2" },
  { value: 3, label: "C+EQ3" },
  { value: 4, label: "C+EQ4" },
  { value: 5, label: "C+EQ5" },
  { value: 6, label: "C+EQ6" },
];

const WMT_VELOCITY_CTRL_OPTIONS = [
  { value: 0, label: "OFF" },
  { value: 1, label: "ON" },
  { value: 2, label: "RANDOM" },
];

const ALT_PAN_OPTIONS = [
  { value: 0, label: "OFF" },
  { value: 1, label: "ON" },
  { value: 2, label: "REV" },
];

// ---------------------------------------------------------------------------
// SysEx offset helpers
// ---------------------------------------------------------------------------

/** Convert a linear byte index (0-194) to a 2-byte SysEx offset. */
function linearToSysex(linear: number): number {
  return Math.floor(linear / 128) * 256 + (linear % 128);
}

/** WMT layer linear start indices. */
const WMT_STARTS = [0x21, 0x3E, 0x5B, 0x78];

/** Compute the SysEx offset for a WMT field given the layer index and
 *  the field's byte position within the 29-byte layer. */
function wmtFieldOffset(layer: number, fieldPos: number): number {
  return linearToSysex(WMT_STARTS[layer]! + fieldPos);
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface Props {
  partIndex: number;
  service: IntegraService;
}

export function PcmDrumEditor({ partIndex, service }: Props) {
  const [common, setCommon] = useState<PcmDrumCommon | null>(null);
  const [common2, setCommon2] = useState<PcmDrumCommon2 | null>(null);
  const [mfx, setMfx] = useState<MfxState | null>(null);
  const [loading, setLoading] = useState(true);
  const [selectedKey, setSelectedKey] = useState(36); // C2 — kick drum
  const [noteData, setNoteData] = useState<PcmDrumPartial | null>(null);
  const [noteLoading, setNoteLoading] = useState(false);
  const noteCache = useRef<Map<number, PcmDrumPartial>>(new Map());

  // Load common + common2 + MFX on mount / part change
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    noteCache.current.clear();

    async function load() {
      try {
        // Common (0x12 = 18 bytes)
        const commonData = await service.requestData(
          pcmdCommonAddress(partIndex),
          [0x00, 0x00, 0x00, 0x12],
        );
        if (cancelled) return;
        const c = service.device.applyPcmdCommon(commonData) as PcmDrumCommon | null;
        if (c) setCommon(c);

        // Common2 (0x32 = 50 bytes)
        const c2Data = await service.requestData(
          pcmdCommon2Address(partIndex),
          [0x00, 0x00, 0x00, 0x32],
        );
        if (cancelled) return;
        const c2 = service.device.applyPcmdCommon2(c2Data) as PcmDrumCommon2 | null;
        if (c2) setCommon2(c2);

        // MFX (0x01 0x11 = 273 bytes)
        const mfxData = await service.requestData(
          pcmdMfxAddress(partIndex),
          [0x00, 0x00, 0x01, 0x11],
        );
        if (cancelled) return;
        const m = service.device.applyMfxBlock(partIndex, mfxData) as MfxState | null;
        if (m) setMfx(m);
      } catch {
        // Timeout -- partial data is OK
      }
      if (!cancelled) setLoading(false);
    }

    load();
    return () => { cancelled = true; };
  }, [partIndex, service]);

  // Load note data on key selection change
  useEffect(() => {
    if (selectedKey < FIRST_KEY || selectedKey > LAST_KEY) return;

    const cached = noteCache.current.get(selectedKey);
    if (cached) {
      setNoteData(cached);
      return;
    }

    let cancelled = false;
    setNoteLoading(true);

    async function loadNote() {
      try {
        // Partial block size: 01 43 = 195 bytes
        const data = await service.requestData(
          pcmdPartialAddress(partIndex, selectedKey),
          [0x00, 0x00, 0x01, 0x43],
        );
        if (cancelled) return;
        const n = service.device.applyPcmdPartial(data) as PcmDrumPartial | null;
        if (n) {
          noteCache.current.set(selectedKey, n);
          setNoteData(n);
        }
      } catch {
        // Timeout
      }
      if (!cancelled) setNoteLoading(false);
    }

    loadNote();
    return () => { cancelled = true; };
  }, [selectedKey, partIndex, service]);

  // ---------------------------------------------------------------------------
  // Key selection with note preview
  // ---------------------------------------------------------------------------
  const handleKeySelect = useCallback(
    (key: number) => {
      setSelectedKey(key);
      const rs = service.device.readState() as { parts?: { receiveChannel?: number }[] };
      const ch = rs?.parts?.[partIndex]?.receiveChannel ?? partIndex;
      service.sendNoteOn(ch, key, 100);
      setTimeout(() => {
        service.sendNoteOff(ch, key);
      }, 300);
    },
    [partIndex, service],
  );

  // ---------------------------------------------------------------------------
  // Common param setter
  // ---------------------------------------------------------------------------
  const setCommonParam = useCallback(
    (offset: number, value: number) => {
      service.device.setPcmdCommonParam(partIndex, offset, value);
      setCommon((prev) => {
        if (!prev) return prev;
        const next = { ...prev };
        if (offset === 0x0C) next.kitLevel = value;
        return next;
      });
    },
    [partIndex, service],
  );

  // ---------------------------------------------------------------------------
  // Per-key partial param setter (single byte)
  // ---------------------------------------------------------------------------
  const setPartialParam = useCallback(
    (offset: number, value: number) => {
      service.device.setPcmdPartialParam(partIndex, selectedKey, offset, value);
      setNoteData((prev) => {
        if (!prev) return prev;
        const next = { ...prev };
        applyPartialField(next, offset, value);
        noteCache.current.set(selectedKey, next);
        return next;
      });
    },
    [partIndex, selectedKey, service],
  );

  // ---------------------------------------------------------------------------
  // Per-key partial nibblized param setter (multi-byte)
  // ---------------------------------------------------------------------------
  const setPartialNibParam = useCallback(
    (offset: number, value: number) => {
      service.device.setPcmdPartialNibParam(partIndex, selectedKey, offset, value);
      setNoteData((prev) => {
        if (!prev) return prev;
        const next = { ...prev };
        applyPartialNibField(next, offset, value);
        noteCache.current.set(selectedKey, next);
        return next;
      });
    },
    [partIndex, selectedKey, service],
  );

  // ---------------------------------------------------------------------------
  // MFX callbacks (identical to SN-D)
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
  // Render
  // ---------------------------------------------------------------------------

  if (loading) {
    return <div className={css.loading}>Loading PCM drum kit data...</div>;
  }

  return (
    <div className={css.editor}>
      {common && (
        <div className={css.kitName}>{common.kitName || "(unnamed kit)"}</div>
      )}

      <div className={css.mainArea}>
        {common && (
          <CommonPanel common={common} common2={common2} onChange={setCommonParam} />
        )}

        <div className={css.keyEditorArea}>
          <SectionPanel label="DRUM KEY" accentColor="#f93">
            <KeyGrid
              selectedKey={selectedKey}
              onSelect={handleKeySelect}
            />
            {noteLoading ? (
              <div className={css.noteLoadingPlaceholder}>Loading key data...</div>
            ) : noteData ? (
              <NoteControls
                keyNumber={selectedKey}
                note={noteData}
                onParam={setPartialParam}
                onNibParam={setPartialNibParam}
              />
            ) : (
              <div className={css.noteLoadingPlaceholder}>Select a key to edit</div>
            )}
          </SectionPanel>
        </div>

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
  );
}

// ---------------------------------------------------------------------------
// Field update helpers — map SysEx offset to the TS field
// ---------------------------------------------------------------------------

function applyPartialField(p: PcmDrumPartial, offset: number, value: number): void {
  switch (offset) {
    case 0x0C: p.assignType = value; break;
    case 0x0D: p.muteGroup = value; break;
    case 0x0E: p.level = value; break;
    case 0x0F: p.coarseTune = value; break;
    case 0x10: p.fineTune = value; break;
    case 0x11: p.randomPitchDepth = value; break;
    case 0x12: p.pan = value; break;
    case 0x13: p.randomPanDepth = value; break;
    case 0x14: p.alternatePanDepth = value; break;
    case 0x15: p.envMode = value; break;
    case 0x16: p.outputLevel = value; break;
    case 0x19: p.chorusSend = value; break;
    case 0x1A: p.reverbSend = value; break;
    case 0x1B: p.outputAssign = value; break;
    case 0x1C: p.pitchBendRange = value; break;
    case 0x1D: p.receiveExpression = value; break;
    case 0x1E: p.receiveHold1 = value; break;
    case 0x20: p.wmtVelocityControl = value; break;
    default:
      // WMT layer single-byte fields
      for (let i = 0; i < 4; i++) {
        const base = WMT_STARTS[i]!;
        applyWmtField(p.wmt[i]!, offset, base, value);
      }
      // Pitch env
      applyPitchEnvField(p, offset, value);
      // TVF
      applyTvfField(p, offset, value);
      // TVA
      applyTvaField(p, offset, value);
      // One-shot
      if (offset === linearToSysex(193)) p.oneShotMode = value;
      break;
  }
}

function applyWmtField(w: PcmDrumWmt, offset: number, base: number, value: number): void {
  const sBase = linearToSysex(base);
  // Single-byte fields within a WMT layer
  const rel = offset - sBase;
  // Handle 7-bit boundary crossing for WMT4
  if (rel < 0) return;
  // The WMT layer occupies 29 contiguous linear bytes starting at `base`.
  // For sysex offsets, bytes that cross the 0x80 boundary jump to 0x01xx.
  // Check each field by linear position.
  const linearOff = base + fieldForSysex(sBase, offset);
  if (linearOff < base || linearOff >= base + 29) return;
  const pos = linearOff - base;
  switch (pos) {
    case 0: w.wmtSwitch = value; break;
    case 1: w.waveGroupType = value; break;
    case 14: w.waveGain = value; break;
    case 15: w.waveFxmSwitch = value; break;
    case 16: w.waveFxmColor = value; break;
    case 17: w.waveFxmDepth = value; break;
    case 18: w.waveTempoSync = value; break;
    case 19: w.coarseTune = value; break;
    case 20: w.fineTune = value; break;
    case 21: w.pan = value; break;
    case 22: w.randomPanSwitch = value; break;
    case 23: w.alternatePanSwitch = value; break;
    case 24: w.level = value; break;
    case 25: w.velocityRangeLower = value; break;
    case 26: w.velocityRangeUpper = value; break;
    case 27: w.velocityFadeLower = value; break;
    case 28: w.velocityFadeUpper = value; break;
  }
}

/** Convert a sysex offset back to a linear position relative to a known base. */
function fieldForSysex(sysexBase: number, sysexOffset: number): number {
  // Sysex offset high byte = linear / 128, low byte = linear % 128
  const baseLinear = (sysexBase >> 8) * 128 + (sysexBase & 0xFF);
  const offLinear = (sysexOffset >> 8) * 128 + (sysexOffset & 0xFF);
  return offLinear - baseLinear;
}

function applyPartialNibField(p: PcmDrumPartial, offset: number, value: number): void {
  // Nibblized fields in WMT layers
  for (let i = 0; i < 4; i++) {
    const base = WMT_STARTS[i]!;
    const sBase = linearToSysex(base);
    const lin = fieldForSysex(sBase, offset);
    if (lin >= 0 && lin < 29) {
      const w = p.wmt[i]!;
      // Nibblized fields: waveGroupId(2-5), waveNumberL(6-9), waveNumberR(10-13)
      if (lin === 2) w.waveGroupId = value;
      else if (lin === 6) w.waveNumberL = value;
      else if (lin === 10) w.waveNumberR = value;
    }
  }
}

function applyPitchEnvField(p: PcmDrumPartial, offset: number, value: number): void {
  // Pitch env fields at linear 149-161 → sysex 0x0115-0x0121
  const pitchBase = linearToSysex(149); // 0x0115
  const rel = fieldForSysex(pitchBase, offset);
  if (rel < 0 || rel > 12) return;
  switch (rel) {
    case 0: p.pitchEnvDepth = value; break;
    case 1: p.pitchEnvVelocitySens = value; break;
    case 2: p.pitchEnvT1VelocitySens = value; break;
    case 3: p.pitchEnvT4VelocitySens = value; break;
    case 4: case 5: case 6: case 7:
      p.pitchEnvTime = [...p.pitchEnvTime]; p.pitchEnvTime[rel - 4] = value; break;
    case 8: case 9: case 10: case 11: case 12:
      p.pitchEnvLevel = [...p.pitchEnvLevel]; p.pitchEnvLevel[rel - 8] = value; break;
  }
}

function applyTvfField(p: PcmDrumPartial, offset: number, value: number): void {
  // TVF fields at linear 162-181 → sysex 0x0122-0x0135
  const tvfBase = linearToSysex(162); // 0x0122
  const rel = fieldForSysex(tvfBase, offset);
  if (rel < 0 || rel > 19) return;
  switch (rel) {
    case 0: p.tvfFilterType = value; break;
    case 1: p.tvfCutoffFrequency = value; break;
    case 2: p.tvfCutoffVelocityCurve = value; break;
    case 3: p.tvfCutoffVelocitySens = value; break;
    case 4: p.tvfResonance = value; break;
    case 5: p.tvfResonanceVelocitySens = value; break;
    // TVF Env
    case 6: p.tvfEnvDepth = value; break;
    case 7: p.tvfEnvVelocityCurve = value; break;
    case 8: p.tvfEnvVelocitySens = value; break;
    case 9: p.tvfEnvT1VelocitySens = value; break;
    case 10: p.tvfEnvT4VelocitySens = value; break;
    case 11: case 12: case 13: case 14:
      p.tvfEnvTime = [...p.tvfEnvTime]; p.tvfEnvTime[rel - 11] = value; break;
    case 15: case 16: case 17: case 18: case 19:
      p.tvfEnvLevel = [...p.tvfEnvLevel]; p.tvfEnvLevel[rel - 15] = value; break;
  }
}

function applyTvaField(p: PcmDrumPartial, offset: number, value: number): void {
  // TVA fields at linear 182-193 → sysex 0x0136-0x0141
  const tvaBase = linearToSysex(182); // 0x0136
  const rel = fieldForSysex(tvaBase, offset);
  if (rel < 0 || rel > 11) return;
  switch (rel) {
    case 0: p.tvaLevelVelocityCurve = value; break;
    case 1: p.tvaLevelVelocitySens = value; break;
    case 2: p.tvaEnvT1VelocitySens = value; break;
    case 3: p.tvaEnvT4VelocitySens = value; break;
    case 4: case 5: case 6: case 7:
      p.tvaEnvTime = [...p.tvaEnvTime]; p.tvaEnvTime[rel - 4] = value; break;
    case 8: case 9: case 10:
      p.tvaEnvLevel = [...p.tvaEnvLevel]; p.tvaEnvLevel[rel - 8] = value; break;
    case 11: p.oneShotMode = value; break;
  }
}

// ---------------------------------------------------------------------------
// Common Controls Panel
// ---------------------------------------------------------------------------

function CommonPanel({
  common,
  common2: _common2,
  onChange,
}: {
  common: PcmDrumCommon;
  common2: PcmDrumCommon2 | null;
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="COMMON" accentColor="#8cf">
      <FaderGroup>
        <SynthFader label="Kit Lv" value={common.kitLevel} min={0} max={127} defaultValue={127}
          onChange={(v) => onChange(0x0C, v)} formatValue={(v) => String(v)} compact
          title="Kit Level" />
      </FaderGroup>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Key selector grid (88 keys: 21-108)
// ---------------------------------------------------------------------------

function KeyGrid({
  selectedKey,
  onSelect,
}: {
  selectedKey: number;
  onSelect: (key: number) => void;
}) {
  const keys: number[] = [];
  for (let k = FIRST_KEY; k <= LAST_KEY; k++) {
    keys.push(k);
  }

  return (
    <div className={css.keyGrid}>
      {keys.map((k) => {
        const selected = k === selectedKey;
        const black = isBlackKey(k);
        const className = [
          selected ? css.keyCellSelected : css.keyCell,
          black ? css.keyCellBlack : "",
        ].filter(Boolean).join(" ");
        return (
          <div key={k} className={className} onClick={() => onSelect(k)}>
            <span className={css.keyNoteName}>{noteName(k)}</span>
            <span className={css.keyNumber}>{k}</span>
          </div>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Per-key note controls
// ---------------------------------------------------------------------------

function NoteControls({
  keyNumber,
  note,
  onParam,
  onNibParam,
}: {
  keyNumber: number;
  note: PcmDrumPartial;
  onParam: (offset: number, value: number) => void;
  onNibParam: (offset: number, value: number) => void;
}) {
  const [wmtTab, setWmtTab] = useState(0);

  return (
    <>
      <div className={css.keyHeader}>
        <span className={css.keyLabel}>
          Key {keyNumber} ({noteName(keyNumber)})
        </span>
        {note.partialName && (
          <span className={css.partialName}>{note.partialName}</span>
        )}
      </div>

      <div className={css.noteControls}>
        {/* WAVE (WMT) */}
        <WmtSection
          note={note}
          wmtTab={wmtTab}
          onWmtTab={setWmtTab}
          onParam={onParam}
          onNibParam={onNibParam}
        />

        {/* PITCH ENV */}
        <PitchEnvSection note={note} onParam={onParam} />

        {/* FILTER (TVF) */}
        <TvfSection note={note} onParam={onParam} />

        {/* AMP (TVA) */}
        <TvaSection note={note} onParam={onParam} />

        {/* MISC */}
        <MiscSection note={note} onParam={onParam} />

        {/* LEVEL / PAN */}
        <SectionPanel label="LEVEL / PAN" accentColor="#8cf">
          <OutputStrip
            pan={note.pan} onPanChange={(v) => onParam(0x12, v)} panFormat={panFmt}
            fx1={note.chorusSend} fx2={note.reverbSend}
            level={note.outputLevel}
            onFx1Change={(v) => onParam(0x19, v)}
            onFx2Change={(v) => onParam(0x1A, v)}
            onLevelChange={(v) => onParam(0x16, v)}
            trackHeight={200}
          />
        </SectionPanel>
      </div>
    </>
  );
}

// ---------------------------------------------------------------------------
// WMT Section (tabbed, 4 layers)
// ---------------------------------------------------------------------------

function WmtSection({
  note,
  wmtTab,
  onWmtTab,
  onParam,
  onNibParam,
}: {
  note: PcmDrumPartial;
  wmtTab: number;
  onWmtTab: (tab: number) => void;
  onParam: (offset: number, value: number) => void;
  onNibParam: (offset: number, value: number) => void;
}) {
  const wmt = note.wmt[wmtTab]!;
  const layerOffset = (fieldPos: number) => wmtFieldOffset(wmtTab, fieldPos);

  return (
    <SectionPanel label="WAVE (WMT)" accentColor="#fc8">
      {/* Velocity Control */}
      <div className={css.knobRow}>
        <SynthSelect label="Vel Ctrl" value={note.wmtVelocityControl}
          options={WMT_VELOCITY_CTRL_OPTIONS}
          onChange={(v) => onParam(0x20, v)} />
      </div>

      {/* Layer tabs */}
      <div className={css.wmtTabs}>
        {[0, 1, 2, 3].map((i) => {
          const w = note.wmt[i]!;
          const isActive = i === wmtTab;
          const isOn = w.wmtSwitch === 1;
          const tabClass = isActive
            ? css.wmtTabActive
            : isOn ? css.wmtTab : css.wmtTabOff;
          return (
            <div key={i} className={tabClass} onClick={() => onWmtTab(i)}>
              WMT{i + 1}
            </div>
          );
        })}
      </div>

      {/* Layer controls */}
      <div className={css.wmtLayer}>
        <SynthSwitch label="Switch" value={wmt.wmtSwitch}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={(v) => onParam(layerOffset(0), v)}
          title="Wave layer switch" />
        <SynthSelect label="Group" value={wmt.waveGroupType}
          options={WAVE_GROUP_OPTIONS}
          onChange={(v) => onParam(layerOffset(1), v)} />
        <SynthSelect label="Gain" value={wmt.waveGain}
          options={WAVE_GAIN_OPTIONS}
          onChange={(v) => onParam(layerOffset(14), v)} />
      </div>

      <div className={css.wmtLayer}>
        <SynthKnob label="Wave L" value={wmt.waveNumberL} min={0} max={16384} defaultValue={0}
          onChange={(v) => onNibParam(layerOffset(6), v)}
          formatValue={(v) => String(v)} color="#fc8"
          title="Wave Number L / Mono" />
        <SynthKnob label="Wave R" value={wmt.waveNumberR} min={0} max={16384} defaultValue={0}
          onChange={(v) => onNibParam(layerOffset(10), v)}
          formatValue={(v) => String(v)} color="#fc8"
          title="Wave Number R" />
      </div>

      <div className={css.wmtLayer}>
        <SynthSwitch label="FXM" value={wmt.waveFxmSwitch}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={(v) => onParam(layerOffset(15), v)}
          title="FXM Switch" />
        <SynthKnob label="Color" value={wmt.waveFxmColor} min={0} max={3} defaultValue={0}
          onChange={(v) => onParam(layerOffset(16), v)}
          formatValue={(v) => String(v + 1)} color="#ea6"
          title="FXM Color (1-4)" />
        <SynthKnob label="Depth" value={wmt.waveFxmDepth} min={0} max={16} defaultValue={0}
          onChange={(v) => onParam(layerOffset(17), v)}
          formatValue={(v) => String(v)} color="#ea6"
          title="FXM Depth" />
      </div>

      <div className={css.wmtLayer}>
        <SynthKnob label="Coarse" value={wmt.coarseTune} min={16} max={112} defaultValue={64}
          onChange={(v) => onParam(layerOffset(19), v)}
          formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Coarse Tune (-48 to +48)" />
        <SynthKnob label="Fine" value={wmt.fineTune} min={14} max={114} defaultValue={64}
          onChange={(v) => onParam(layerOffset(20), v)}
          formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Fine Tune (-50 to +50)" />
        <SynthKnob label="Pan" value={wmt.pan} min={0} max={127} defaultValue={64}
          onChange={(v) => onParam(layerOffset(21), v)}
          formatValue={panFmt} color="#8cf"
          title="Wave Pan" />
        <SynthKnob label="Level" value={wmt.level} min={0} max={127} defaultValue={127}
          onChange={(v) => onParam(layerOffset(24), v)}
          formatValue={(v) => String(v)} color="#8cf"
          title="Wave Level" />
      </div>

      <div className={css.wmtLayer}>
        <SynthSwitch label="Rnd Pan" value={wmt.randomPanSwitch}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={(v) => onParam(layerOffset(22), v)}
          title="Random Pan Switch" />
        <SynthSelect label="Alt Pan" value={wmt.alternatePanSwitch}
          options={ALT_PAN_OPTIONS}
          onChange={(v) => onParam(layerOffset(23), v)} />
        <SynthSwitch label="T.Sync" value={wmt.waveTempoSync}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={(v) => onParam(layerOffset(18), v)}
          title="Tempo Sync" />
      </div>

      <div className={css.wmtLayer}>
        <SynthKnob label="Vel Lo" value={wmt.velocityRangeLower} min={1} max={127} defaultValue={1}
          onChange={(v) => onParam(layerOffset(25), v)}
          formatValue={(v) => String(v)} color="#a6f"
          title="Velocity Range Lower" />
        <SynthKnob label="Vel Hi" value={wmt.velocityRangeUpper} min={1} max={127} defaultValue={127}
          onChange={(v) => onParam(layerOffset(26), v)}
          formatValue={(v) => String(v)} color="#a6f"
          title="Velocity Range Upper" />
        <SynthKnob label="Fade Lo" value={wmt.velocityFadeLower} min={0} max={127} defaultValue={0}
          onChange={(v) => onParam(layerOffset(27), v)}
          formatValue={(v) => String(v)} color="#a6f"
          title="Velocity Fade Width Lower" />
        <SynthKnob label="Fade Hi" value={wmt.velocityFadeUpper} min={0} max={127} defaultValue={0}
          onChange={(v) => onParam(layerOffset(28), v)}
          formatValue={(v) => String(v)} color="#a6f"
          title="Velocity Fade Width Upper" />
      </div>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Pitch Envelope Section
// ---------------------------------------------------------------------------

function PitchEnvSection({
  note,
  onParam,
}: {
  note: PcmDrumPartial;
  onParam: (offset: number, value: number) => void;
}) {
  const base = linearToSysex(149); // 0x0115
  const off = (rel: number) => base + rel;

  return (
    <SectionPanel label="PITCH ENV" accentColor="#fc8">
      <div className={css.knobRow}>
        <SynthKnob label="Depth" value={note.pitchEnvDepth} min={52} max={76} defaultValue={64}
          onChange={(v) => onParam(off(0), v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Pitch envelope depth (-12 to +12)" />
        <SynthKnob label="Vel Sns" value={note.pitchEnvVelocitySens} min={1} max={127} defaultValue={64}
          onChange={(v) => onParam(off(1), v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Pitch envelope velocity sensitivity" />
      </div>
      <ADSREnvelope
        compact
        attack={{
          label: "Atk", value: note.pitchEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(4), v),
        }}
        decay={{
          label: "Dec", value: note.pitchEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(5), v),
        }}
        sustain={{
          label: "Sus", value: note.pitchEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(6), v),
        }}
        release={{
          label: "Rel", value: note.pitchEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(7), v),
        }}
        levels={[
          { label: "Start", value: note.pitchEnvLevel[0] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => onParam(off(8), v), formatValue: (v) => signedFmt(v, 64) },
          { label: "Atk", value: note.pitchEnvLevel[1] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => onParam(off(9), v), formatValue: (v) => signedFmt(v, 64) },
          { label: "Dec", value: note.pitchEnvLevel[2] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => onParam(off(10), v), formatValue: (v) => signedFmt(v, 64) },
          { label: "Sus", value: note.pitchEnvLevel[3] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => onParam(off(11), v), formatValue: (v) => signedFmt(v, 64) },
          { label: "End", value: note.pitchEnvLevel[4] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => onParam(off(12), v), formatValue: (v) => signedFmt(v, 64) },
        ]}
      />
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// TVF (Filter) Section
// ---------------------------------------------------------------------------

function TvfSection({
  note,
  onParam,
}: {
  note: PcmDrumPartial;
  onParam: (offset: number, value: number) => void;
}) {
  const base = linearToSysex(162); // 0x0122
  const off = (rel: number) => base + rel;

  return (
    <SectionPanel label="FILTER" accentColor="#68c">
      <div className={css.panelRow}>
        <SynthSwitch label="Type" value={note.tvfFilterType} vertical
          options={FILTER_TYPE_OPTIONS}
          onChange={(v) => onParam(off(0), v)} title="Filter type" />
        <SynthKnob label="Cutoff" value={note.tvfCutoffFrequency} min={0} max={127} defaultValue={127}
          onChange={(v) => onParam(off(1), v)} formatValue={(v) => String(v)} color="#68c"
          title="Filter cutoff frequency" />
        <SynthKnob label="Reso" value={note.tvfResonance} min={0} max={127} defaultValue={0}
          onChange={(v) => onParam(off(4), v)} formatValue={(v) => String(v)} color="#68c"
          title="Filter resonance" />
      </div>
      <div className={css.knobRow}>
        <SynthKnob label="Vel Sns" value={note.tvfCutoffVelocitySens} min={1} max={127} defaultValue={64}
          onChange={(v) => onParam(off(3), v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Cutoff velocity sensitivity" />
        <SynthSelect label="Vel Crv" value={note.tvfCutoffVelocityCurve}
          options={VELOCITY_CURVE_OPTIONS}
          onChange={(v) => onParam(off(2), v)} />
        <SynthKnob label="Res Vel" value={note.tvfResonanceVelocitySens} min={1} max={127} defaultValue={64}
          onChange={(v) => onParam(off(5), v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Resonance velocity sensitivity" />
      </div>
      <div className={css.knobRow}>
        <SynthKnob label="Depth" value={note.tvfEnvDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => onParam(off(6), v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Filter envelope depth" />
        <SynthKnob label="Env Vel" value={note.tvfEnvVelocitySens} min={1} max={127} defaultValue={64}
          onChange={(v) => onParam(off(8), v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Envelope velocity sensitivity" />
        <SynthSelect label="Env Crv" value={note.tvfEnvVelocityCurve}
          options={VELOCITY_CURVE_OPTIONS}
          onChange={(v) => onParam(off(7), v)} />
      </div>
      <ADSREnvelope
        compact
        attack={{
          label: "Atk", value: note.tvfEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(11), v),
        }}
        decay={{
          label: "Dec", value: note.tvfEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(12), v),
        }}
        sustain={{
          label: "Sus", value: note.tvfEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(13), v),
        }}
        release={{
          label: "Rel", value: note.tvfEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(14), v),
        }}
        levels={[
          { label: "Start", value: note.tvfEnvLevel[0] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onParam(off(15), v) },
          { label: "Atk", value: note.tvfEnvLevel[1] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onParam(off(16), v) },
          { label: "Dec", value: note.tvfEnvLevel[2] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onParam(off(17), v) },
          { label: "Sus", value: note.tvfEnvLevel[3] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onParam(off(18), v) },
          { label: "End", value: note.tvfEnvLevel[4] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onParam(off(19), v) },
        ]}
      />
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// TVA (Amplifier) Section
// ---------------------------------------------------------------------------

function TvaSection({
  note,
  onParam,
}: {
  note: PcmDrumPartial;
  onParam: (offset: number, value: number) => void;
}) {
  const base = linearToSysex(182); // 0x0136
  const off = (rel: number) => base + rel;

  return (
    <SectionPanel label="AMP" accentColor="#6c8">
      <div className={css.knobRow}>
        <SynthSelect label="Vel Crv" value={note.tvaLevelVelocityCurve}
          options={VELOCITY_CURVE_OPTIONS}
          onChange={(v) => onParam(off(0), v)} />
        <SynthKnob label="Vel Sns" value={note.tvaLevelVelocitySens} min={1} max={127} defaultValue={64}
          onChange={(v) => onParam(off(1), v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
          title="Amplitude velocity sensitivity" />
      </div>
      <ADSREnvelope
        compact
        attack={{
          label: "Atk", value: note.tvaEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(4), v),
        }}
        decay={{
          label: "Dec", value: note.tvaEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(5), v),
        }}
        sustain={{
          label: "Sus", value: note.tvaEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(6), v),
        }}
        release={{
          label: "Rel", value: note.tvaEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => onParam(off(7), v),
        }}
        levels={[
          { label: "", value: 127, min: 0, max: 127, defaultValue: 127, onChange: () => {}, hidden: true },
          { label: "Atk", value: note.tvaEnvLevel[0] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: (v) => onParam(off(8), v) },
          { label: "Dec", value: note.tvaEnvLevel[1] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: (v) => onParam(off(9), v) },
          { label: "Sus", value: note.tvaEnvLevel[2] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => onParam(off(10), v) },
          { label: "", value: 0, min: 0, max: 127, defaultValue: 0, onChange: () => {}, hidden: true },
        ]}
      />
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Misc Section
// ---------------------------------------------------------------------------

function MiscSection({
  note,
  onParam,
}: {
  note: PcmDrumPartial;
  onParam: (offset: number, value: number) => void;
}) {
  const muteGroupOptions = [{ value: 0, label: "OFF" }];
  for (let i = 1; i <= 31; i++) {
    muteGroupOptions.push({ value: i, label: String(i) });
  }

  return (
    <SectionPanel label="MISC" accentColor="#a6f">
      <div className={css.noteSelects}>
        <SynthSelect label="Assign" value={note.assignType}
          options={ASSIGN_TYPE_OPTIONS}
          onChange={(v) => onParam(0x0C, v)} />
        <SynthSelect label="Mute Grp" value={note.muteGroup}
          options={muteGroupOptions}
          onChange={(v) => onParam(0x0D, v)} />
        <SynthSelect label="Env Mode" value={note.envMode}
          options={ENV_MODE_OPTIONS}
          onChange={(v) => onParam(0x15, v)} />
        <SynthSelect label="Output" value={note.outputAssign}
          options={OUTPUT_ASSIGN_OPTIONS}
          onChange={(v) => onParam(0x1B, v)} />
        <SynthSwitch label="1-Shot" value={note.oneShotMode}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={(v) => onParam(linearToSysex(193), v)}
          title="One Shot Mode" />
      </div>
      <div className={css.knobRow}>
        <SynthKnob label="PB Rng" value={note.pitchBendRange} min={0} max={48} defaultValue={0}
          onChange={(v) => onParam(0x1C, v)} formatValue={(v) => String(v)} color="#a6f"
          title="Pitch Bend Range" />
        <SynthKnob label="Level" value={note.level} min={0} max={127} defaultValue={127}
          onChange={(v) => onParam(0x0E, v)} formatValue={(v) => String(v)} color="#a6f"
          title="Partial Level" />
      </div>
      <div className={css.knobRow}>
        <SynthKnob label="Coarse" value={note.coarseTune} min={0} max={127} defaultValue={64}
          onChange={(v) => onParam(0x0F, v)} formatValue={(v) => noteName(v)} color="#fc8"
          title="Partial Coarse Tune (C-1 to G9)" />
        <SynthKnob label="Fine" value={note.fineTune} min={14} max={114} defaultValue={64}
          onChange={(v) => onParam(0x10, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Partial Fine Tune (-50 to +50)" />
        <SynthKnob label="Rnd Pit" value={note.randomPitchDepth} min={0} max={30} defaultValue={0}
          onChange={(v) => onParam(0x11, v)} formatValue={(v) => String(v)} color="#fc8"
          title="Random Pitch Depth" />
      </div>
      <div className={css.miscRow}>
        <SynthSwitch label="RxExpr" value={note.receiveExpression}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={(v) => onParam(0x1D, v)}
          title="Receive Expression" />
        <SynthSwitch label="RxHold" value={note.receiveHold1}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={(v) => onParam(0x1E, v)}
          title="Receive Hold-1" />
      </div>
    </SectionPanel>
  );
}
