import { useEffect, useState, useCallback } from "react";
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
import css from "./PcmSynthEditor.module.css";

// ---------------------------------------------------------------------------
// Types mirroring the Rust serde output
// ---------------------------------------------------------------------------

interface PcmSynthCommon {
  toneName: string;
  toneLevel: number;
  tonePan: number;
  tonePriority: number;
  coarseTune: number;
  fineTune: number;
  octaveShift: number;
  stretchTuneDepth: number;
  analogFeel: number;
  monoPoly: number;
  legatoSwitch: number;
  legatoRetrigger: number;
  portamentoSwitch: number;
  portamentoMode: number;
  portamentoType: number;
  portamentoStart: number;
  portamentoTime: number;
  cutoffOffset: number;
  resonanceOffset: number;
  attackTimeOffset: number;
  releaseTimeOffset: number;
  velocitySensOffset: number;
  pmtControlSwitch: number;
  pitchBendRangeUp: number;
  pitchBendRangeDown: number;
}

interface PmtPartialEntry {
  partialSwitch: number;
  keyRangeLower: number;
  keyRangeUpper: number;
  keyFadeLower: number;
  keyFadeUpper: number;
  velocityRangeLower: number;
  velocityRangeUpper: number;
  velocityFadeLower: number;
  velocityFadeUpper: number;
}

interface PcmSynthPmt {
  structureType12: number;
  booster12: number;
  structureType34: number;
  booster34: number;
  pmtVelocityControl: number;
  partialEntries: PmtPartialEntry[];
}

interface PcmSynthPartial {
  level: number;
  coarseTune: number;
  fineTune: number;
  randomPitchDepth: number;
  pan: number;
  panKeyfollow: number;
  randomPanDepth: number;
  alternatePanDepth: number;
  envMode: number;
  delayMode: number;
  delayTime: number;
  outputLevel: number;
  chorusSend: number;
  reverbSend: number;
  receiveBender: number;
  receiveExpression: number;
  receiveHold1: number;
  redamperSwitch: number;
  control1Switches: number[];
  control2Switches: number[];
  control3Switches: number[];
  control4Switches: number[];
  waveGroupType: number;
  waveGroupId: number;
  waveNumberL: number;
  waveNumberR: number;
  waveGain: number;
  waveFxmSwitch: number;
  waveFxmColor: number;
  waveFxmDepth: number;
  waveTempoSync: number;
  wavePitchKeyfollow: number;
  pitchEnvDepth: number;
  pitchEnvVelocitySens: number;
  pitchEnvT1VelocitySens: number;
  pitchEnvT4VelocitySens: number;
  pitchEnvTimeKeyfollow: number;
  pitchEnvTime: number[];
  pitchEnvLevel: number[];
  tvfFilterType: number;
  tvfCutoffFrequency: number;
  tvfCutoffKeyfollow: number;
  tvfCutoffVelocityCurve: number;
  tvfCutoffVelocitySens: number;
  tvfResonance: number;
  tvfResonanceVelocitySens: number;
  tvfEnvDepth: number;
  tvfEnvVelocityCurve: number;
  tvfEnvVelocitySens: number;
  tvfEnvT1VelocitySens: number;
  tvfEnvT4VelocitySens: number;
  tvfEnvTimeKeyfollow: number;
  tvfEnvTime: number[];
  tvfEnvLevel: number[];
  tvaBiasLevel: number;
  tvaBiasPosition: number;
  tvaBiasDirection: number;
  tvaLevelVelocityCurve: number;
  tvaLevelVelocitySens: number;
  tvaEnvT1VelocitySens: number;
  tvaEnvT4VelocitySens: number;
  tvaEnvTimeKeyfollow: number;
  tvaEnvTime: number[];
  tvaEnvLevel: number[];
  lfo1Waveform: number;
  lfo1Rate: number;
  lfo1Offset: number;
  lfo1RateDetune: number;
  lfo1DelayTime: number;
  lfo1DelayTimeKeyfollow: number;
  lfo1FadeMode: number;
  lfo1FadeTime: number;
  lfo1KeyTrigger: number;
  lfo1PitchDepth: number;
  lfo1TvfDepth: number;
  lfo1TvaDepth: number;
  lfo1PanDepth: number;
  lfo2Waveform: number;
  lfo2Rate: number;
  lfo2Offset: number;
  lfo2RateDetune: number;
  lfo2DelayTime: number;
  lfo2DelayTimeKeyfollow: number;
  lfo2FadeMode: number;
  lfo2FadeTime: number;
  lfo2KeyTrigger: number;
  lfo2PitchDepth: number;
  lfo2TvfDepth: number;
  lfo2TvaDepth: number;
  lfo2PanDepth: number;
  lfoStepType: number;
  lfoStepValues: number[];
}

interface PcmSynthCommon2 {
  toneCategory: number;
  phraseOctaveShift: number;
  tfxSwitch: number;
  phraseNumber: number;
}

// ---------------------------------------------------------------------------
// Address helpers
// ---------------------------------------------------------------------------

/** Compute PCM Synth base address bytes for a part (same as temporary_tone_base). */
function pcmsBaseAddress(part: number): [number, number] {
  const partTotal = part * 0x20;
  const byte0 = 0x19 + Math.floor(partTotal / 128);
  const byte1 = partTotal % 128;
  return [byte0, byte1];
}

function pcmsCommonAddress(part: number): number[] {
  const [b0, b1] = pcmsBaseAddress(part);
  return [b0, b1, 0x00, 0x00];
}

function pcmsPmtAddress(part: number): number[] {
  const [b0, b1] = pcmsBaseAddress(part);
  return [b0, b1, 0x10, 0x00];
}

function pcmsPartialAddress(part: number, partial: number): number[] {
  const [b0, b1] = pcmsBaseAddress(part);
  return [b0, b1, 0x20 + partial * 2, 0x00];
}

function pcmsCommon2Address(part: number): number[] {
  const [b0, b1] = pcmsBaseAddress(part);
  return [b0, b1, 0x30, 0x00];
}

function pcmsMfxAddress(part: number): number[] {
  const [b0, b1] = pcmsBaseAddress(part);
  return [b0, b1, 0x02, 0x00];
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

const ON_OFF_OPTIONS = [{ value: 0, label: "OFF" }, { value: 1, label: "ON" }];

const WAVE_GAIN_NAMES = ["-6dB", "0dB", "+6dB", "+12dB"];

const FILTER_TYPE_NAMES = ["OFF", "LPF", "BPF", "HPF", "PKG", "LPF2", "LPF3"];

const LFO_WAVEFORM_NAMES = [
  "SIN", "TRI", "SAW-UP", "SAW-DW", "SQR", "RND",
  "BEND-UP", "BEND-DW", "TRP", "S&H", "CHS", "VSIN", "STEP",
];
const LFO_WAVEFORM_STEP = LFO_WAVEFORM_NAMES.indexOf("STEP");

const LFO_OFFSET_NAMES = ["-100", "-50", "0", "+50", "+100"];

const LFO_FADE_MODE_NAMES = ["ON-IN", "ON-OUT", "OFF-IN", "OFF-OUT"];

const VELOCITY_CURVE_NAMES = ["FIXED", "1", "2", "3", "4", "5", "6", "7"];

const STRUCTURE_TYPE_NAMES = [
  "1: Independent",
  "2: Stacked Filt",
  "3: Boost → Filt",
  "4: Boost + Bal",
  "5: Ring → Filt",
  "6: Ring + Bal",
  "7: Filt → Ring",
  "8: Filt+Ring+Mix",
  "9: Ring+Mix → Filt",
  "10: Mix → Boost",
];

const BOOSTER_NAMES = ["0", "+6", "+12", "+18"];

const PMT_VEL_CTRL_NAMES = ["OFF", "ON", "RANDOM", "CYCLE"];

const BIAS_DIR_NAMES = ["LOWER", "UPPER", "LOWER&UPPER", "ALL"];

const ENV_MODE_NAMES = ["NO-SUS", "SUSTAIN"];

const DELAY_MODE_NAMES = ["NORMAL", "HOLD", "KEY-OFF-NORMAL", "KEY-OFF-DECAY"];

function signedFmt(raw: number, center: number): string {
  const v = raw - center;
  return v > 0 ? `+${v}` : String(v);
}

function panFmt(v: number): string {
  if (v === 64) return "C";
  if (v < 64) return `L${64 - v}`;
  return `${v - 64}R`;
}

function noteName(midi: number): string {
  const names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
  const oct = Math.floor(midi / 12) - 1;
  return `${names[midi % 12]}${oct}`;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface Props {
  partIndex: number;
  service: IntegraService;
}

export function PcmSynthEditor({ partIndex, service }: Props) {
  const [common, setCommon] = useState<PcmSynthCommon | null>(null);
  const [pmt, setPmt] = useState<PcmSynthPmt | null>(null);
  const [_common2, setCommon2] = useState<PcmSynthCommon2 | null>(null);
  const [partials, setPartials] = useState<(PcmSynthPartial | null)[]>([null, null, null, null]);
  const [mfx, setMfx] = useState<MfxState | null>(null);
  const [loading, setLoading] = useState(true);

  // Load common data on mount / part change
  useEffect(() => {
    let cancelled = false;
    setLoading(true);

    async function load() {
      try {
        // Common (0x50 bytes)
        const commonData = await service.requestData(
          pcmsCommonAddress(partIndex),
          [0x00, 0x00, 0x00, 0x50],
        );
        if (cancelled) return;
        const c = service.device.applyPcmsCommon(commonData) as PcmSynthCommon | null;
        if (c) setCommon(c);

        // PMT (0x29 bytes)
        const pmtData = await service.requestData(
          pcmsPmtAddress(partIndex),
          [0x00, 0x00, 0x00, 0x29],
        );
        if (cancelled) return;
        const p = service.device.applyPcmsPmt(pmtData) as PcmSynthPmt | null;
        if (p) setPmt(p);

        // Common2 (0x3C bytes)
        const c2Data = await service.requestData(
          pcmsCommon2Address(partIndex),
          [0x00, 0x00, 0x00, 0x3C],
        );
        if (cancelled) return;
        const c2 = service.device.applyPcmsCommon2(c2Data) as PcmSynthCommon2 | null;
        if (c2) setCommon2(c2);

        // MFX (0x111 bytes)
        const mfxData = await service.requestData(
          pcmsMfxAddress(partIndex),
          [0x00, 0x00, 0x01, 0x11],
        );
        if (cancelled) return;
        const m = service.device.applyMfxBlock(partIndex, mfxData) as MfxState | null;
        if (m) setMfx(m);

        // Load all 4 partials
        for (let idx = 0; idx < 4; idx++) {
          if (cancelled) return;
          const pData = await service.requestData(
            pcmsPartialAddress(partIndex, idx),
            [0x00, 0x00, 0x01, 0x1A],
          );
          if (cancelled) return;
          const parsed = service.device.applyPcmsPartial(pData) as PcmSynthPartial | null;
          if (parsed) {
            setPartials((prev) => {
              const next = [...prev];
              next[idx] = parsed;
              return next;
            });
          }
        }
      } catch (err) {
        console.error("[PcmSynthEditor] load error:", err);
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
      service.device.setPcmsCommonParam(partIndex, offset, value);
      setCommon((prev) => {
        if (!prev) return prev;
        const next = { ...prev };
        switch (offset) {
          case 0x0C: next.toneName = prev.toneName; break; // no-op placeholder
          case 0x0E: next.toneLevel = value; break;
          case 0x0F: next.tonePan = value; break;
          case 0x10: next.tonePriority = value; break;
          case 0x11: next.coarseTune = value; break;
          case 0x12: next.fineTune = value; break;
          case 0x13: next.octaveShift = value; break;
          case 0x14: next.stretchTuneDepth = value; break;
          case 0x15: next.analogFeel = value; break;
          case 0x16: next.monoPoly = value; break;
          case 0x17: next.legatoSwitch = value; break;
          case 0x18: next.legatoRetrigger = value; break;
          case 0x19: next.portamentoSwitch = value; break;
          case 0x1A: next.portamentoMode = value; break;
          case 0x1B: next.portamentoType = value; break;
          case 0x1C: next.portamentoStart = value; break;
          case 0x1D: next.portamentoTime = value; break;
          case 0x1E: next.cutoffOffset = value; break;
          case 0x1F: next.resonanceOffset = value; break;
          case 0x20: next.attackTimeOffset = value; break;
          case 0x21: next.releaseTimeOffset = value; break;
          case 0x22: next.velocitySensOffset = value; break;
          case 0x23: next.pmtControlSwitch = value; break;
          case 0x24: next.pitchBendRangeUp = value; break;
          case 0x25: next.pitchBendRangeDown = value; break;
        }
        return next;
      });
    },
    [partIndex, service],
  );

  // ---------------------------------------------------------------------------
  // PMT param setter
  // ---------------------------------------------------------------------------
  const setPmtParam = useCallback(
    (offset: number, value: number) => {
      service.device.setPcmsPmtParam(partIndex, offset, value);
      setPmt((prev) => {
        if (!prev) return prev;
        const next = { ...prev, partialEntries: prev.partialEntries.map((p) => ({ ...p })) };
        switch (offset) {
          case 0x00: next.structureType12 = value; break;
          case 0x01: next.booster12 = value; break;
          case 0x02: next.structureType34 = value; break;
          case 0x03: next.booster34 = value; break;
          case 0x04: next.pmtVelocityControl = value; break;
          default: {
            // Per-partial params: offsets 0x05..0x28 (9 params per partial)
            const relOff = offset - 0x05;
            if (relOff >= 0 && relOff < 36) {
              const pi = Math.floor(relOff / 9);
              const field = relOff % 9;
              const entry = next.partialEntries[pi];
              if (entry) {
                switch (field) {
                  case 0: entry.partialSwitch = value; break;
                  case 1: entry.keyRangeLower = value; break;
                  case 2: entry.keyRangeUpper = value; break;
                  case 3: entry.keyFadeLower = value; break;
                  case 4: entry.keyFadeUpper = value; break;
                  case 5: entry.velocityRangeLower = value; break;
                  case 6: entry.velocityRangeUpper = value; break;
                  case 7: entry.velocityFadeLower = value; break;
                  case 8: entry.velocityFadeUpper = value; break;
                }
              }
            }
          }
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
      service.device.setPcmsPartialParam(partIndex, partial, offset, value);
      updatePartialField(partial, offset, value);
    },
    [partIndex, service],
  );

  const setPartialNib2Param = useCallback(
    (partial: number, offset: number, value: number) => {
      service.device.setPcmsPartialNib2Param(partIndex, partial, offset, value);
      updatePartialField(partial, offset, value);
    },
    [partIndex, service],
  );

  const setPartialNibParam = useCallback(
    (partial: number, offset: number, value: number) => {
      service.device.setPcmsPartialNibParam(partIndex, partial, offset, value);
      updatePartialField(partial, offset, value);
    },
    [partIndex, service],
  );

  function updatePartialField(partial: number, offset: number, value: number) {
    setPartials((prev) => {
      const next = [...prev];
      const p = next[partial];
      if (!p) return prev;
      const u = { ...p };
      switch (offset) {
        case 0x00: u.level = value; break;
        case 0x01: u.coarseTune = value; break;
        case 0x02: u.fineTune = value; break;
        case 0x03: u.randomPitchDepth = value; break;
        case 0x04: u.pan = value; break;
        case 0x05: u.panKeyfollow = value; break;
        case 0x06: u.randomPanDepth = value; break;
        case 0x07: u.alternatePanDepth = value; break;
        case 0x08: u.envMode = value; break;
        case 0x09: u.delayMode = value; break;
        case 0x0A: u.delayTime = value; break; // nib2
        case 0x0C: u.outputLevel = value; break;
        case 0x0D: u.chorusSend = value; break;
        case 0x0E: u.reverbSend = value; break;
        case 0x0F: u.receiveBender = value; break;
        case 0x10: u.receiveExpression = value; break;
        case 0x11: u.receiveHold1 = value; break;
        case 0x12: u.redamperSwitch = value; break;
        // Skipping control switch arrays for now
        case 0x27: u.waveGroupType = value; break;
        case 0x28: u.waveGroupId = value; break; // nib4
        case 0x2C: u.waveNumberL = value; break; // nib4
        case 0x30: u.waveNumberR = value; break; // nib4
        case 0x34: u.waveGain = value; break;
        case 0x35: u.waveFxmSwitch = value; break;
        case 0x36: u.waveFxmColor = value; break;
        case 0x37: u.waveFxmDepth = value; break;
        case 0x38: u.waveTempoSync = value; break;
        case 0x39: u.wavePitchKeyfollow = value; break;
        case 0x3A: u.pitchEnvDepth = value; break;
        case 0x3B: u.pitchEnvVelocitySens = value; break;
        case 0x3C: u.pitchEnvT1VelocitySens = value; break;
        case 0x3D: u.pitchEnvT4VelocitySens = value; break;
        case 0x3E: u.pitchEnvTimeKeyfollow = value; break;
        case 0x3F: u.pitchEnvTime = [...p.pitchEnvTime]; u.pitchEnvTime[0] = value; break;
        case 0x40: u.pitchEnvTime = [...p.pitchEnvTime]; u.pitchEnvTime[1] = value; break;
        case 0x41: u.pitchEnvTime = [...p.pitchEnvTime]; u.pitchEnvTime[2] = value; break;
        case 0x42: u.pitchEnvTime = [...p.pitchEnvTime]; u.pitchEnvTime[3] = value; break;
        case 0x43: u.pitchEnvLevel = [...p.pitchEnvLevel]; u.pitchEnvLevel[0] = value; break;
        case 0x44: u.pitchEnvLevel = [...p.pitchEnvLevel]; u.pitchEnvLevel[1] = value; break;
        case 0x45: u.pitchEnvLevel = [...p.pitchEnvLevel]; u.pitchEnvLevel[2] = value; break;
        case 0x46: u.pitchEnvLevel = [...p.pitchEnvLevel]; u.pitchEnvLevel[3] = value; break;
        case 0x47: u.pitchEnvLevel = [...p.pitchEnvLevel]; u.pitchEnvLevel[4] = value; break;
        case 0x48: u.tvfFilterType = value; break;
        case 0x49: u.tvfCutoffFrequency = value; break;
        case 0x4A: u.tvfCutoffKeyfollow = value; break;
        case 0x4B: u.tvfCutoffVelocityCurve = value; break;
        case 0x4C: u.tvfCutoffVelocitySens = value; break;
        case 0x4D: u.tvfResonance = value; break;
        case 0x4E: u.tvfResonanceVelocitySens = value; break;
        case 0x4F: u.tvfEnvDepth = value; break;
        case 0x50: u.tvfEnvVelocityCurve = value; break;
        case 0x51: u.tvfEnvVelocitySens = value; break;
        case 0x52: u.tvfEnvT1VelocitySens = value; break;
        case 0x53: u.tvfEnvT4VelocitySens = value; break;
        case 0x54: u.tvfEnvTimeKeyfollow = value; break;
        case 0x55: u.tvfEnvTime = [...p.tvfEnvTime]; u.tvfEnvTime[0] = value; break;
        case 0x56: u.tvfEnvTime = [...p.tvfEnvTime]; u.tvfEnvTime[1] = value; break;
        case 0x57: u.tvfEnvTime = [...p.tvfEnvTime]; u.tvfEnvTime[2] = value; break;
        case 0x58: u.tvfEnvTime = [...p.tvfEnvTime]; u.tvfEnvTime[3] = value; break;
        case 0x59: u.tvfEnvLevel = [...p.tvfEnvLevel]; u.tvfEnvLevel[0] = value; break;
        case 0x5A: u.tvfEnvLevel = [...p.tvfEnvLevel]; u.tvfEnvLevel[1] = value; break;
        case 0x5B: u.tvfEnvLevel = [...p.tvfEnvLevel]; u.tvfEnvLevel[2] = value; break;
        case 0x5C: u.tvfEnvLevel = [...p.tvfEnvLevel]; u.tvfEnvLevel[3] = value; break;
        case 0x5D: u.tvfEnvLevel = [...p.tvfEnvLevel]; u.tvfEnvLevel[4] = value; break;
        case 0x5E: u.tvaBiasLevel = value; break;
        case 0x5F: u.tvaBiasPosition = value; break;
        case 0x60: u.tvaBiasDirection = value; break;
        case 0x61: u.tvaLevelVelocityCurve = value; break;
        case 0x62: u.tvaLevelVelocitySens = value; break;
        case 0x63: u.tvaEnvT1VelocitySens = value; break;
        case 0x64: u.tvaEnvT4VelocitySens = value; break;
        case 0x65: u.tvaEnvTimeKeyfollow = value; break;
        case 0x66: u.tvaEnvTime = [...p.tvaEnvTime]; u.tvaEnvTime[0] = value; break;
        case 0x67: u.tvaEnvTime = [...p.tvaEnvTime]; u.tvaEnvTime[1] = value; break;
        case 0x68: u.tvaEnvTime = [...p.tvaEnvTime]; u.tvaEnvTime[2] = value; break;
        case 0x69: u.tvaEnvTime = [...p.tvaEnvTime]; u.tvaEnvTime[3] = value; break;
        case 0x6A: u.tvaEnvLevel = [...p.tvaEnvLevel]; u.tvaEnvLevel[0] = value; break;
        case 0x6B: u.tvaEnvLevel = [...p.tvaEnvLevel]; u.tvaEnvLevel[1] = value; break;
        case 0x6C: u.tvaEnvLevel = [...p.tvaEnvLevel]; u.tvaEnvLevel[2] = value; break;
        case 0x6D: u.lfo1Waveform = value; break;
        case 0x6E: u.lfo1Rate = value; break; // nib2
        case 0x70: u.lfo1Offset = value; break;
        case 0x71: u.lfo1RateDetune = value; break;
        case 0x72: u.lfo1DelayTime = value; break;
        case 0x73: u.lfo1DelayTimeKeyfollow = value; break;
        case 0x74: u.lfo1FadeMode = value; break;
        case 0x75: u.lfo1FadeTime = value; break;
        case 0x76: u.lfo1KeyTrigger = value; break;
        case 0x77: u.lfo1PitchDepth = value; break;
        case 0x78: u.lfo1TvfDepth = value; break;
        case 0x79: u.lfo1TvaDepth = value; break;
        case 0x7A: u.lfo1PanDepth = value; break;
        case 0x7B: u.lfo2Waveform = value; break;
        case 0x7C: u.lfo2Rate = value; break; // nib2
        case 0x7E: u.lfo2Offset = value; break;
        case 0x7F: u.lfo2RateDetune = value; break;
        // Past 0x7F boundary, use extended offsets
        case 0x0100: u.lfo2DelayTime = value; break;
        case 0x0101: u.lfo2DelayTimeKeyfollow = value; break;
        case 0x0102: u.lfo2FadeMode = value; break;
        case 0x0103: u.lfo2FadeTime = value; break;
        case 0x0104: u.lfo2KeyTrigger = value; break;
        case 0x0105: u.lfo2PitchDepth = value; break;
        case 0x0106: u.lfo2TvfDepth = value; break;
        case 0x0107: u.lfo2TvaDepth = value; break;
        case 0x0108: u.lfo2PanDepth = value; break;
        case 0x0109: u.lfoStepType = value; break;
        // Step LFO steps: 0x010A..0x0119
        default: {
          if (offset >= 0x010A && offset <= 0x0119) {
            const stepIdx = offset - 0x010A;
            u.lfoStepValues = [...p.lfoStepValues];
            u.lfoStepValues[stepIdx] = value;
          }
        }
      }
      next[partial] = u;
      return next;
    });
  }

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
    return <div className={css.loading}>Loading PCM Synth tone data...</div>;
  }

  const partialSwitches = pmt
    ? pmt.partialEntries.map((p) => p.partialSwitch)
    : [0, 0, 0, 0];

  return (
    <div className={css.editor}>
      {/* Tone name */}
      {common && (
        <div className={css.toneName}>{common.toneName || "(unnamed)"}</div>
      )}

      {/* Common + PMT strips */}
      {common && <CommonStrip common={common} onChange={setCommonParam} />}
      {pmt && <PmtStrip pmt={pmt} onChange={setPmtParam} />}

      {/* 4 partial rows + MFX sidebar */}
      <div className={css.mainArea}>
        <div className={css.partialRows}>
          {[0, 1, 2, 3].map((idx) => {
            const p = partials[idx] ?? null;
            const isOn = (partialSwitches[idx] ?? 0) !== 0;
            return (
              <PartialRow
                key={idx}
                idx={idx}
                partial={p}
                isOn={isOn}
                setP={(o, v) => setPartialParam(idx, o, v)}
                setNibP={(o, v) => setPartialNibParam(idx, o, v)}
                setNib2P={(o, v) => setPartialNib2Param(idx, o, v)}
                setPmtParam={setPmtParam}
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
// Common Controls Strip
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Partial Row — renders all section panels for one partial in a horizontal row
// ---------------------------------------------------------------------------

function PartialRow({
  idx,
  partial,
  isOn,
  setP,
  setNibP,
  setNib2P,
  setPmtParam,
}: {
  idx: number;
  partial: PcmSynthPartial | null;
  isOn: boolean;
  setP: (offset: number, value: number) => void;
  setNibP: (offset: number, value: number) => void;
  setNib2P: (offset: number, value: number) => void;
  setPmtParam: (offset: number, value: number) => void;
}) {
  const dimClass = isOn ? "" : css.partialDimmed;
  const pmtSwOffset = 0x05 + idx * 9;

  return (
    <div className={`${css.partialRow} ${dimClass}`}>
      {/* Partial switch + label */}
      <div className={css.partialSwitch}>
        <SynthSwitch
          label={`P${idx + 1}`}
          value={isOn ? 1 : 0}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={(v) => setPmtParam(pmtSwOffset, v)}
        />
      </div>
      {partial ? (
        <>
          <WaveSection partial={partial} partialIdx={idx} setP={setP} setNibP={setNibP} />
          <PitchSection partial={partial} setP={setP} />
          <TvfSection partial={partial} setP={setP} />
          <TvaSection partial={partial} setP={setP} />
          <Lfo1Section partial={partial} setP={setP} setNib2P={setNib2P} />
          <Lfo2Section partial={partial} setP={setP} setNib2P={setNib2P} />
          <StepLfoSection partial={partial} setP={setP}
            dimmed={partial.lfo1Waveform !== LFO_WAVEFORM_STEP && partial.lfo2Waveform !== LFO_WAVEFORM_STEP} />
          <OutputSection partial={partial} setP={setP} setNib2P={setNib2P} />
        </>
      ) : (
        <div className={css.loading}>Loading...</div>
      )}
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
  common: PcmSynthCommon;
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <div className={css.commonStrip}>
      {/* Mono/Poly */}
      <div className={css.commonGroup}>
        <SynthSwitch label="MONO" value={common.monoPoly}
          options={[{ value: 0, label: "MONO" }, { value: 1, label: "POLY" }, { value: 2, label: "MONO2" }]}
          onChange={(v) => onChange(0x16, v)} vertical title="Voice mode: monophonic, polyphonic, or mono with legato" />
      </div>

      <div className={css.commonDivider} />

      {/* Portamento */}
      <div className={css.commonGroup}>
        <span className={css.groupLabel}>PORTAMENTO</span>
        <div className={css.groupRow}>
          <SynthSwitch label="SW" value={common.portamentoSwitch} options={ON_OFF_OPTIONS}
            onChange={(v) => onChange(0x19, v)} title="Portamento on/off — smooth pitch glide between notes" />
          <SynthKnob label="Time" value={common.portamentoTime} min={0} max={127} defaultValue={0}
            onChange={(v) => onChange(0x1D, v)} formatValue={(v) => String(v)} color="#8cf"
            title="Portamento time — speed of pitch glide" />
        </div>
      </div>

      <div className={css.commonDivider} />

      {/* Bend Range */}
      <div className={css.commonGroup}>
        <span className={css.groupLabel}>BEND RANGE</span>
        <div className={css.groupRow}>
          <SynthKnob label="Down" value={common.pitchBendRangeDown} min={0} max={24} defaultValue={2}
            onChange={(v) => onChange(0x25, v)} formatValue={(v) => String(v)} color="#8cf"
            title="Pitch bend range down (semitones)" />
          <SynthKnob label="Up" value={common.pitchBendRangeUp} min={0} max={24} defaultValue={2}
            onChange={(v) => onChange(0x24, v)} formatValue={(v) => String(v)} color="#8cf"
            title="Pitch bend range up (semitones)" />
        </div>
      </div>

      <div className={css.commonDivider} />

      {/* Standalone params */}
      <SynthKnob label="Level" value={common.toneLevel} min={0} max={127} defaultValue={127}
        onChange={(v) => onChange(0x0E, v)} formatValue={(v) => String(v)} color="#8cf"
        title="Overall tone volume" />
      <SynthKnob label="Pan" value={common.tonePan} min={0} max={127} defaultValue={64}
        onChange={(v) => onChange(0x0F, v)} formatValue={panFmt} color="#8cf"
        title="Stereo panning position" />
      <SynthKnob label="Coarse" value={common.coarseTune} min={16} max={112} defaultValue={64}
        onChange={(v) => onChange(0x11, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cf"
        title="Coarse tuning (semitones)" />
      <SynthKnob label="Fine" value={common.fineTune} min={14} max={114} defaultValue={64}
        onChange={(v) => onChange(0x12, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cf"
        title="Fine tuning (cents)" />

      <div className={css.commonDivider} />

      <SynthKnob label="Octave" value={common.octaveShift} min={61} max={67} defaultValue={64}
        onChange={(v) => onChange(0x13, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cf"
        title="Octave shift (-3 to +3)" />
      <SynthKnob label="Analog" value={common.analogFeel} min={0} max={127} defaultValue={0}
        onChange={(v) => onChange(0x15, v)} formatValue={(v) => String(v)} color="#8cf"
        title="Analog Feel — adds subtle pitch/timing randomness like vintage synths" />

      <div className={css.commonDivider} />

      <SynthSwitch label="LEGATO" value={common.legatoSwitch} options={ON_OFF_OPTIONS}
        onChange={(v) => onChange(0x17, v)} title="Legato — overlapping notes share the same voice" />

      <div className={css.commonDivider} />

      {/* Offsets */}
      <div className={css.commonGroup}>
        <span className={css.groupLabel}>OFFSETS</span>
        <div className={css.groupRow}>
          <SynthKnob label="Cutoff" value={common.cutoffOffset} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x1E, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
            title="Filter cutoff offset — shifts cutoff across all partials" />
          <SynthKnob label="Reso" value={common.resonanceOffset} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x1F, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
            title="Resonance offset — shifts resonance across all partials" />
          <SynthKnob label="Attack" value={common.attackTimeOffset} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x20, v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
            title="Attack time offset — shifts envelope attack across all partials" />
          <SynthKnob label="Release" value={common.releaseTimeOffset} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x21, v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
            title="Release time offset — shifts envelope release across all partials" />
          <SynthKnob label="Velocity" value={common.velocitySensOffset} min={1} max={127} defaultValue={64}
            onChange={(v) => onChange(0x22, v)} formatValue={(v) => signedFmt(v, 64)} color="#c8a"
            title="Velocity sensitivity offset — shifts velocity response across all partials" />
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// PMT Strip
// ---------------------------------------------------------------------------

function PmtStrip({
  pmt,
  onChange,
}: {
  pmt: PcmSynthPmt;
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <div className={css.pmtStrip}>
      <div className={css.commonGroup}>
        <span className={css.groupLabel}>PARTIAL MIX 1&2</span>
        <div className={css.groupRow}>
          <SynthSelect label="Structure" value={pmt.structureType12}
            options={STRUCTURE_TYPE_NAMES.map((l, i) => ({ value: i, label: l }))}
            onChange={(v) => onChange(0x00, v)}
            title="How partials 1 & 2 are combined: 1=Independent, 2=Stacked filters, 3-4=Booster, 5-6=Ring mod, 7-9=Filter+Ring combos, 10=Mix+Boost" />
          <SynthSwitch label="Boost" value={pmt.booster12} vertical
            options={BOOSTER_NAMES.map((l, i) => ({ value: i, label: l }))}
            onChange={(v) => onChange(0x01, v)} led={false} title="Output boost for partials 1 & 2" />
        </div>
      </div>

      <div className={css.commonDivider} />

      <div className={css.commonGroup}>
        <span className={css.groupLabel}>PARTIAL MIX 3&4</span>
        <div className={css.groupRow}>
          <SynthSelect label="Structure" value={pmt.structureType34}
            options={STRUCTURE_TYPE_NAMES.map((l, i) => ({ value: i, label: l }))}
            onChange={(v) => onChange(0x02, v)}
            title="How partials 3 & 4 are combined: 1=Independent, 2=Stacked filters, 3-4=Booster, 5-6=Ring mod, 7-9=Filter+Ring combos, 10=Mix+Boost" />
          <SynthSwitch label="Boost" value={pmt.booster34} vertical
            options={BOOSTER_NAMES.map((l, i) => ({ value: i, label: l }))}
            onChange={(v) => onChange(0x03, v)} led={false} title="Output boost for partials 3 & 4" />
        </div>
      </div>

      <div className={css.commonDivider} />

      <SynthSwitch label="Vel Ctrl" value={pmt.pmtVelocityControl} vertical
        options={PMT_VEL_CTRL_NAMES.map((l, i) => ({ value: i, label: l }))}
        onChange={(v) => onChange(0x04, v)} led={false}
        title="Velocity control — how key velocity selects partials" />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Wave Section
// ---------------------------------------------------------------------------

function WaveSection({
  partial,
  partialIdx: _partialIdx,
  setP,
  setNibP,
}: {
  partial: PcmSynthPartial;
  partialIdx: number;
  setP: (offset: number, value: number) => void;
  setNibP: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="WAVE" accentColor="#fc8">
      <SynthSwitch label="Source" value={partial.waveGroupType}
        options={[{ value: 0, label: "INT" }, { value: 1, label: "SRX" }]}
        onChange={(v) => setP(0x27, v)} title="Wave source: Internal or SRX expansion" />
      <label className={css.selectLabel} title="Left (mono) waveform number">
        Wave L <input type="number" className={css.waveNumberInput}
          value={partial.waveNumberL} min={0} max={16384}
          onChange={(e) => setNibP(0x2C, Math.max(0, Math.min(16384, Number(e.target.value))))} />
      </label>
      <label className={css.selectLabel} title="Right waveform number (stereo pairs)">
        Wave R <input type="number" className={css.waveNumberInput}
          value={partial.waveNumberR} min={0} max={16384}
          onChange={(e) => setNibP(0x30, Math.max(0, Math.min(16384, Number(e.target.value))))} />
      </label>
      <SynthSwitch label="Gain" value={partial.waveGain}
        options={WAVE_GAIN_NAMES.map((l, i) => ({ value: i, label: l }))}
        onChange={(v) => setP(0x34, v)} title="Wave output gain" />
      <SynthKnob label="Key Trk" value={partial.wavePitchKeyfollow} min={44} max={84} defaultValue={64}
        onChange={(v) => setP(0x39, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
        title="Pitch keyboard tracking" />
      <SynthSwitch label="Tempo" value={partial.waveTempoSync} options={ON_OFF_OPTIONS}
        onChange={(v) => setP(0x38, v)} title="Sync wave playback to song tempo" />
      <SynthSwitch label="FXM" value={partial.waveFxmSwitch} options={ON_OFF_OPTIONS}
        onChange={(v) => setP(0x35, v)} title="Frequency Cross Modulation" />
      <SynthKnob label="FXM Col" value={partial.waveFxmColor} min={0} max={3} defaultValue={0}
        onChange={(v) => setP(0x36, v)} formatValue={(v) => String(v + 1)} color="#fc8"
        title="FXM Color — tonal character" />
      <SynthKnob label="FXM Dpt" value={partial.waveFxmDepth} min={0} max={16} defaultValue={0}
        onChange={(v) => setP(0x37, v)} formatValue={(v) => String(v)} color="#fc8"
        title="FXM Depth — intensity" />
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Pitch + Envelope Section
// ---------------------------------------------------------------------------

function PitchSection({
  partial,
  setP,
}: {
  partial: PcmSynthPartial;
  setP: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="PITCH ENV" accentColor="#fc8">
      <div className={css.panelRow}>
        <SynthKnob label="Coarse" value={partial.coarseTune} min={16} max={112} defaultValue={64}
          onChange={(v) => setP(0x01, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Coarse tuning (semitones)" />
        <SynthKnob label="Fine" value={partial.fineTune} min={14} max={114} defaultValue={64}
          onChange={(v) => setP(0x02, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Fine tuning (cents)" />
        <SynthKnob label="Random" value={partial.randomPitchDepth} min={0} max={127} defaultValue={0}
          onChange={(v) => setP(0x03, v)} formatValue={(v) => String(v)} color="#fc8"
          title="Random pitch variation on each note" />
      </div>
      <div className={css.panelRow}>
        <SynthKnob label="Depth" value={partial.pitchEnvDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x3A, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Pitch envelope depth — how much the envelope affects pitch" />
        <SynthKnob label="Vel Sns" value={partial.pitchEnvVelocitySens} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x3B, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Velocity sensitivity — how key velocity affects envelope depth" />
        <SynthKnob label="Key Trk" value={partial.pitchEnvTimeKeyfollow} min={54} max={74} defaultValue={64}
          onChange={(v) => setP(0x3E, v)} formatValue={(v) => signedFmt(v, 64)} color="#fc8"
          title="Envelope time key tracking — higher keys = faster envelope" />
      </div>
      <ADSREnvelope
        compact
        attack={{
          label: "Atk", value: partial.pitchEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x3F, v),
        }}
        decay={{
          label: "Dec", value: partial.pitchEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x40, v),
        }}
        sustain={{
          label: "Sus", value: partial.pitchEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x41, v),
        }}
        release={{
          label: "Rel", value: partial.pitchEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x42, v),
        }}
        levels={[
          { label: "Start", value: partial.pitchEnvLevel[0] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => setP(0x43, v), formatValue: (v) => signedFmt(v, 64) },
          { label: "Atk", value: partial.pitchEnvLevel[1] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => setP(0x44, v), formatValue: (v) => signedFmt(v, 64) },
          { label: "Dec", value: partial.pitchEnvLevel[2] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => setP(0x45, v), formatValue: (v) => signedFmt(v, 64) },
          { label: "Sus", value: partial.pitchEnvLevel[3] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => setP(0x46, v), formatValue: (v) => signedFmt(v, 64) },
          { label: "End", value: partial.pitchEnvLevel[4] ?? 64, min: 1, max: 127, defaultValue: 64,
            onChange: (v) => setP(0x47, v), formatValue: (v) => signedFmt(v, 64) },
        ]}
      />
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// TVF (Filter) Section
// ---------------------------------------------------------------------------

function TvfSection({
  partial,
  setP,
}: {
  partial: PcmSynthPartial;
  setP: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="FILTER" accentColor="#68c">
      <div className={css.panelRow}>
        <SynthSwitch label="Type" value={partial.tvfFilterType} vertical
          options={FILTER_TYPE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x48, v)} title="Filter type" />
        <div className={`${css.knobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Cutoff" value={partial.tvfCutoffFrequency} min={0} max={127} defaultValue={127}
            onChange={(v) => setP(0x49, v)} formatValue={(v) => String(v)} color="#68c"
            title="Filter cutoff frequency" />
          <SynthKnob label="Reso" value={partial.tvfResonance} min={0} max={127} defaultValue={0}
            onChange={(v) => setP(0x4D, v)} formatValue={(v) => String(v)} color="#68c"
            title="Filter resonance — emphasis at the cutoff frequency" />
          <SynthKnob label="Key Trk" value={partial.tvfCutoffKeyfollow} min={44} max={84} defaultValue={64}
            onChange={(v) => setP(0x4A, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
            title="Cutoff key tracking — higher keys = higher cutoff" />
          <SynthKnob label="Vel Sns" value={partial.tvfCutoffVelocitySens} min={1} max={127} defaultValue={64}
            onChange={(v) => setP(0x4C, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
            title="Cutoff velocity sensitivity — harder keys = higher cutoff" />
        </div>
      </div>
      <div className={css.panelRow}>
        <SynthKnob label="Depth" value={partial.tvfEnvDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x4F, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Filter envelope depth — how much the envelope sweeps the cutoff" />
        <SynthKnob label="Vel Sns" value={partial.tvfEnvVelocitySens} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x51, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Envelope velocity sensitivity — harder keys = deeper sweep" />
        <SynthKnob label="Key Trk" value={partial.tvfEnvTimeKeyfollow} min={54} max={74} defaultValue={64}
          onChange={(v) => setP(0x54, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Envelope time key tracking — higher keys = faster envelope" />
        <SynthSelect label="Vel Curve" value={partial.tvfCutoffVelocityCurve}
          options={VELOCITY_CURVE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x4B, v)} title="Velocity response curve shape" />
      </div>
      <ADSREnvelope
        compact
        attack={{
          label: "Atk", value: partial.tvfEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x55, v),
        }}
        decay={{
          label: "Dec", value: partial.tvfEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x56, v),
        }}
        sustain={{
          label: "Sus", value: partial.tvfEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x57, v),
        }}
        release={{
          label: "Rel", value: partial.tvfEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x58, v),
        }}
        levels={[
          { label: "Start", value: partial.tvfEnvLevel[0] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => setP(0x59, v) },
          { label: "Atk", value: partial.tvfEnvLevel[1] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: (v) => setP(0x5A, v) },
          { label: "Dec", value: partial.tvfEnvLevel[2] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: (v) => setP(0x5B, v) },
          { label: "Sus", value: partial.tvfEnvLevel[3] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: (v) => setP(0x5C, v) },
          { label: "End", value: partial.tvfEnvLevel[4] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => setP(0x5D, v) },
        ]}
      />
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// TVA (Amp) Section
// ---------------------------------------------------------------------------

function TvaSection({
  partial,
  setP,
}: {
  partial: PcmSynthPartial;
  setP: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="AMP" accentColor="#6c8">
      <div className={css.panelRow}>
        <div className={`${css.knobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Level" value={partial.level} min={0} max={127} defaultValue={127}
            onChange={(v) => setP(0x00, v)} formatValue={(v) => String(v)} color="#6c8"
            title="Partial volume level" />
          <SynthKnob label="Pan" value={partial.pan} min={0} max={127} defaultValue={64}
            onChange={(v) => setP(0x04, v)} formatValue={panFmt} color="#6c8"
            title="Partial stereo panning" />
          <SynthKnob label="Vel Sns" value={partial.tvaLevelVelocitySens} min={1} max={127} defaultValue={64}
            onChange={(v) => setP(0x62, v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
            title="Volume velocity sensitivity — harder keys = louder" />
          <SynthKnob label="Key Trk" value={partial.tvaEnvTimeKeyfollow} min={54} max={74} defaultValue={64}
            onChange={(v) => setP(0x65, v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
            title="Envelope time key tracking — higher keys = faster envelope" />
        </div>
      </div>
      <div className={css.panelRow}>
        <SynthSelect label="Vel Curve" value={partial.tvaLevelVelocityCurve}
          options={VELOCITY_CURVE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x61, v)} title="Velocity response curve shape" />
        <SynthKnob label="Bias Lvl" value={partial.tvaBiasLevel} min={0} max={127} defaultValue={0}
          onChange={(v) => setP(0x5E, v)} formatValue={(v) => signedFmt(v, 64)} color="#6c8"
          title="Level bias amount — attenuates volume away from the bias key" />
        <SynthKnob label="Bias Key" value={partial.tvaBiasPosition} min={0} max={127} defaultValue={64}
          onChange={(v) => setP(0x5F, v)} formatValue={noteName} color="#6c8"
          title="Bias center key — volume is loudest here, attenuated away" />
        <SynthSwitch label="Bias Dir" value={partial.tvaBiasDirection} vertical
          options={BIAS_DIR_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x60, v)} led={false}
          title="Bias direction — which side of the center key is attenuated" />
      </div>
      <ADSREnvelope
        compact
        attack={{
          label: "Atk", value: partial.tvaEnvTime[0] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x66, v),
        }}
        decay={{
          label: "Dec", value: partial.tvaEnvTime[1] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x67, v),
        }}
        sustain={{
          label: "Sus", value: partial.tvaEnvTime[2] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x68, v),
        }}
        release={{
          label: "Rel", value: partial.tvaEnvTime[3] ?? 0, min: 0, max: 127, defaultValue: 0,
          onChange: (v) => setP(0x69, v),
        }}
        levels={[
          { label: "", value: 127, min: 0, max: 127, defaultValue: 127, onChange: () => {}, hidden: true },
          { label: "Atk", value: partial.tvaEnvLevel[0] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: (v) => setP(0x6A, v) },
          { label: "Dec", value: partial.tvaEnvLevel[1] ?? 127, min: 0, max: 127, defaultValue: 127,
            onChange: (v) => setP(0x6B, v) },
          { label: "Sus", value: partial.tvaEnvLevel[2] ?? 0, min: 0, max: 127, defaultValue: 0,
            onChange: (v) => setP(0x6C, v) },
          { label: "", value: 0, min: 0, max: 127, defaultValue: 0, onChange: () => {}, hidden: true },
        ]}
      />
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// LFO1 Section
// ---------------------------------------------------------------------------

function Lfo1Section({
  partial,
  setP,
  setNib2P,
}: {
  partial: PcmSynthPartial;
  setP: (offset: number, value: number) => void;
  setNib2P: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="LFO1" accentColor="#a6f">
      <div className={css.panelRow}>
        <SynthSwitch label="Wave" value={partial.lfo1Waveform} vertical
          options={LFO_WAVEFORM_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x6D, v)} title="LFO waveform shape" />
        <div className={`${css.knobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Rate" value={partial.lfo1Rate} min={0} max={149} defaultValue={0}
            onChange={(v) => setNib2P(0x6E, v)} formatValue={(v) => String(v)} color="#a6f"
            title="LFO speed" />
          <SynthKnob label="Detune" value={partial.lfo1RateDetune} min={0} max={127} defaultValue={0}
            onChange={(v) => setP(0x71, v)} formatValue={(v) => String(v)} color="#a6f"
            title="Rate randomization — adds organic variation" />
          <SynthKnob label="Delay" value={partial.lfo1DelayTime} min={0} max={127} defaultValue={0}
            onChange={(v) => setP(0x72, v)} formatValue={(v) => String(v)} color="#a6f"
            title="Time before LFO starts after note-on" />
          <SynthKnob label="Fade In" value={partial.lfo1FadeTime} min={0} max={127} defaultValue={0}
            onChange={(v) => setP(0x75, v)} formatValue={(v) => String(v)} color="#a6f"
            title="Time for LFO to reach full depth after delay" />
        </div>
      </div>
      <div className={css.panelRow}>
        <SynthSwitch label="Offset" value={partial.lfo1Offset} vertical
          options={LFO_OFFSET_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x70, v)} led={false}
          title="DC offset of the LFO waveform" />
        <SynthSwitch label="Dly KF" value={partial.lfo1DelayTimeKeyfollow}
          options={ON_OFF_OPTIONS}
          onChange={(v) => setP(0x73, v)}
          title="Delay time key tracking — higher keys = shorter delay" />
        <SynthSwitch label="Fade" value={partial.lfo1FadeMode} vertical
          options={LFO_FADE_MODE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x74, v)} led={false}
          title="Fade behavior: fade in on note-on, fade out, or on key-off" />
        <SynthSwitch label="Key Trig" value={partial.lfo1KeyTrigger} options={ON_OFF_OPTIONS}
          onChange={(v) => setP(0x76, v)}
          title="Reset LFO phase on each new note" />
      </div>
      <FaderGroup>
        <SynthFader label="Pitch" value={partial.lfo1PitchDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x77, v)} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on pitch (vibrato)" />
        <SynthFader label="Filter" value={partial.lfo1TvfDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x78, v)} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on filter cutoff (wah)" />
        <SynthFader label="Amp" value={partial.lfo1TvaDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x79, v)} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on amplitude (tremolo)" />
        <SynthFader label="Pan" value={partial.lfo1PanDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x7A, v)} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on panning (auto-pan)" />
      </FaderGroup>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// LFO2 Section
// ---------------------------------------------------------------------------

function Lfo2Section({
  partial,
  setP,
  setNib2P,
}: {
  partial: PcmSynthPartial;
  setP: (offset: number, value: number) => void;
  setNib2P: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="LFO2" accentColor="#a6f">
      <div className={css.panelRow}>
        <SynthSwitch label="Wave" value={partial.lfo2Waveform} vertical
          options={LFO_WAVEFORM_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x7B, v)} title="LFO waveform shape" />
        <div className={`${css.knobGrid} ${css.knobsRight}`}>
          <SynthKnob label="Rate" value={partial.lfo2Rate} min={0} max={149} defaultValue={0}
            onChange={(v) => setNib2P(0x7C, v)} formatValue={(v) => String(v)} color="#a6f"
            title="LFO speed" />
          <SynthKnob label="Detune" value={partial.lfo2RateDetune} min={0} max={127} defaultValue={0}
            onChange={(v) => setP(0x7F, v)} formatValue={(v) => String(v)} color="#a6f"
            title="Rate randomization — adds organic variation" />
          <SynthKnob label="Delay" value={partial.lfo2DelayTime} min={0} max={127} defaultValue={0}
            onChange={(v) => setP(0x0100, v)} formatValue={(v) => String(v)} color="#a6f"
            title="Time before LFO starts after note-on" />
          <SynthKnob label="Fade In" value={partial.lfo2FadeTime} min={0} max={127} defaultValue={0}
            onChange={(v) => setP(0x0103, v)} formatValue={(v) => String(v)} color="#a6f"
            title="Time for LFO to reach full depth after delay" />
        </div>
      </div>
      <div className={css.panelRow}>
        <SynthSwitch label="Offset" value={partial.lfo2Offset} vertical
          options={LFO_OFFSET_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x7E, v)} led={false}
          title="DC offset of the LFO waveform" />
        <SynthSwitch label="Dly KF" value={partial.lfo2DelayTimeKeyfollow}
          options={ON_OFF_OPTIONS}
          onChange={(v) => setP(0x0101, v)}
          title="Delay time key tracking — higher keys = shorter delay" />
        <SynthSwitch label="Fade" value={partial.lfo2FadeMode} vertical
          options={LFO_FADE_MODE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x0102, v)} led={false}
          title="Fade behavior: fade in on note-on, fade out, or on key-off" />
        <SynthSwitch label="Key Trig" value={partial.lfo2KeyTrigger} options={ON_OFF_OPTIONS}
          onChange={(v) => setP(0x0104, v)}
          title="Reset LFO phase on each new note" />
      </div>
      <FaderGroup>
        <SynthFader label="Pitch" value={partial.lfo2PitchDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x0105, v)} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on pitch (vibrato)" />
        <SynthFader label="Filter" value={partial.lfo2TvfDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x0106, v)} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on filter cutoff (wah)" />
        <SynthFader label="Amp" value={partial.lfo2TvaDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x0107, v)} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on amplitude (tremolo)" />
        <SynthFader label="Pan" value={partial.lfo2PanDepth} min={1} max={127} defaultValue={64}
          onChange={(v) => setP(0x0108, v)} formatValue={(v) => signedFmt(v, 64)} compact
          title="LFO modulation depth on panning (auto-pan)" />
      </FaderGroup>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Step LFO Section
// ---------------------------------------------------------------------------

function StepLfoSection({
  partial,
  setP,
  dimmed,
}: {
  partial: PcmSynthPartial;
  setP: (offset: number, value: number) => void;
  dimmed?: boolean;
}) {
  const steps = partial.lfoStepValues ?? [];
  const smooth = partial.lfoStepType === 1;
  const dimStyle = dimmed ? { opacity: 0.35 } : undefined;

  // Build SVG path for the step visualizer
  const vizW = 160;
  const vizH = 40;
  const pad = 2;
  const stepW = (vizW - pad * 2) / 16;
  const yRange = vizH - pad * 2;
  const yFor = (v: number) => pad + (1 - (((v ?? 64) - 1) / 126)) * yRange;

  let vizPath = "";
  if (smooth) {
    // Smooth: line through center of each step
    vizPath = steps.map((v, i) => {
      const x = pad + (i + 0.5) * stepW;
      const y = yFor(v);
      return i === 0 ? `M ${x} ${y}` : `L ${x} ${y}`;
    }).join(" ");
  } else {
    // Staircase: horizontal bars per step
    vizPath = steps.map((v, i) => {
      const x1 = pad + i * stepW;
      const x2 = x1 + stepW;
      const y = yFor(v);
      return `${i === 0 ? "M" : "L"} ${x1} ${y} L ${x2} ${y}`;
    }).join(" ");
  }

  return (
    <div style={dimStyle}>
    <SectionPanel label="STEP LFO" accentColor="#a6f">
      <div className={css.panelRow}>
        <SynthSwitch label="Shape" value={partial.lfoStepType}
          options={[{ value: 0, label: "STEP" }, { value: 1, label: "SMOOTH" }]}
          onChange={(v) => setP(0x0109, v)}
          title="Step shape: STEP = hard jumps, SMOOTH = interpolated transitions. Set LFO1 or LFO2 waveform to STEP to use this pattern." />
      </div>
      {/* Step pattern visualizer */}
      <svg width={vizW} height={vizH} className={css.stepViz}>
        {/* Center line (zero) */}
        <line x1={pad} y1={vizH / 2} x2={vizW - pad} y2={vizH / 2}
          stroke="#333" strokeWidth="1" strokeDasharray="2 2" />
        {/* Pattern curve */}
        <path d={vizPath} fill="none" stroke="#a6f" strokeWidth="1.5" />
      </svg>
      <div className={css.stepGrid}>
        {Array.from({ length: 16 }, (_, i) => (
          <SynthFader
            key={i}
            label={`${i + 1}`}
            value={steps[i] ?? 64}
            min={1}
            max={127}
            defaultValue={64}
            onChange={(v) => setP(0x010A + i, v)}
            formatValue={(v) => signedFmt(v, 64)}
            compact
          />
        ))}
      </div>
    </SectionPanel>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Output Section
// ---------------------------------------------------------------------------

function OutputSection({
  partial,
  setP,
  setNib2P,
}: {
  partial: PcmSynthPartial;
  setP: (offset: number, value: number) => void;
  setNib2P: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="OUTPUT" accentColor="#6c8">
      <div className={css.panelRow}>
        <SynthSwitch label="Env Mode" value={partial.envMode} vertical
          options={ENV_MODE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x08, v)} led={false}
          title="Envelope mode: NO-SUS plays once, SUSTAIN holds at sustain level" />
        <SynthSwitch label="Delay" value={partial.delayMode} vertical
          options={DELAY_MODE_NAMES.map((l, i) => ({ value: i, label: l }))}
          onChange={(v) => setP(0x09, v)} led={false}
          title="Partial delay mode — when sound starts relative to note-on" />
        <SynthKnob label="Delay" value={partial.delayTime} min={0} max={149} defaultValue={0}
          onChange={(v) => setNib2P(0x0A, v)} formatValue={(v) => String(v)} color="#6c8"
          title="Delay time before this partial sounds" />
      </div>
      <div className={css.panelRow}>
        <SynthSwitch label="Bender" value={partial.receiveBender} options={ON_OFF_OPTIONS}
          onChange={(v) => setP(0x0F, v)}
          title="Respond to pitch bend" />
        <SynthSwitch label="Express" value={partial.receiveExpression} options={ON_OFF_OPTIONS}
          onChange={(v) => setP(0x10, v)}
          title="Respond to expression pedal (CC11)" />
        <SynthSwitch label="Hold" value={partial.receiveHold1} options={ON_OFF_OPTIONS}
          onChange={(v) => setP(0x11, v)}
          title="Respond to sustain pedal (CC64)" />
      </div>
      <OutputStrip
        fx1={partial.chorusSend}
        fx2={partial.reverbSend}
        level={partial.outputLevel}
        onFx1Change={(v) => setP(0x0D, v)}
        onFx2Change={(v) => setP(0x0E, v)}
        onLevelChange={(v) => setP(0x0C, v)}
        trackHeight={150}
      />
    </SectionPanel>
  );
}
