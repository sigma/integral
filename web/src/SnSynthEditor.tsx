import { useEffect, useState, useCallback } from "react";
import type { IntegraService } from "./integra";
import { MfxPanel } from "./MfxPanel";
import type { MfxState } from "./MfxPanel";
import css from "./SnSynthEditor.module.css";
import type { SnSynthCommon, SnSynthPartial } from "./sn-synth/types";
import { CommonStrip } from "./sn-synth/CommonStrip";
import { PartialRow } from "./sn-synth/PartialRow";

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
