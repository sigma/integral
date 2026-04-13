import { useEffect, useState, useCallback } from "react";
import type { IntegraService } from "./integra";
import { MfxPanel } from "./MfxPanel";
import type { MfxState } from "./MfxPanel";
import css from "./PcmSynthEditor.module.css";

import type {
  PcmSynthCommon,
  PcmSynthPmt,
  PcmSynthPartial,
  PcmSynthCommon2,
} from "./pcm-synth/types";
import { CommonStrip } from "./pcm-synth/CommonStrip";
import { PmtStrip } from "./pcm-synth/PmtStrip";
import { PartialRow } from "./pcm-synth/PartialRow";

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
