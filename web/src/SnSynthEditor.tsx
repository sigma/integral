import { useEffect, useState, useCallback } from "react";
import {
  sns_mfx_address,
  mfx_block_size,
  ToneType,
} from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import { MfxPanel } from "./MfxPanel";
import { useMfxHandlers } from "./useMfxHandlers";
import type { MfxState } from "./MfxPanel";
import css from "./SnSynthEditor.module.css";
import type { SnSynthTone } from "./sn-synth/types";
import { CommonStrip } from "./sn-synth/CommonStrip";
import { PartialRow } from "./sn-synth/PartialRow";

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface Props {
  partIndex: number;
  service: IntegraService;
}

export function SnSynthEditor({ partIndex, service }: Props) {
  const [tone, setTone] = useState<SnSynthTone | null>(null);
  const [mfx, setMfx] = useState<MfxState | null>(null);

  // The tone lives in the core cache, which reads it from the device (and
  // asks again for lost replies): show it, and show it again on each reply.
  const readTone = useCallback(() => {
    setTone(service.device.snsTone(partIndex) as SnSynthTone | null);
  }, [partIndex, service]);

  useEffect(() => {
    service.device.loadSnsTone(partIndex);
    readTone();
    const unsubscribe = service.onDt1(readTone);
    return () => {
      unsubscribe();
    };
  }, [partIndex, service, readTone]);

  // MFX
  useEffect(() => {
    let cancelled = false;
    service
      .requestData(Array.from(sns_mfx_address(partIndex)), Array.from(mfx_block_size()))
      .then((mfxData) => {
        if (cancelled) return;
        const m = service.device.applyMfxBlock(partIndex, mfxData) as MfxState | null;
        if (m) setMfx(m);
      })
      .catch(() => {
        // Timeout: no MFX panel
      });
    return () => {
      cancelled = true;
    };
  }, [partIndex, service]);

  // ---------------------------------------------------------------------------
  // Param setters: core updates its cached tone along with the device
  // ---------------------------------------------------------------------------
  const setCommonParam = useCallback(
    (offset: number, value: number) => {
      service.device.setSnsCommonParam(partIndex, offset, value);
      readTone();
    },
    [partIndex, service, readTone],
  );

  const setPartialParam = useCallback(
    (partial: number, offset: number, value: number) => {
      service.device.setSnsPartialParam(partIndex, partial, offset, value);
      readTone();
    },
    [partIndex, service, readTone],
  );

  const setPartialNibParam = useCallback(
    (partial: number, offset: number, value: number) => {
      service.device.setSnsPartialNibParam(partIndex, partial, offset, value);
      readTone();
    },
    [partIndex, service, readTone],
  );

  const mfxHandlers = useMfxHandlers(service, partIndex, ToneType.SnSynth, setMfx);

  // ---------------------------------------------------------------------------
  // Partial selector helpers
  // ---------------------------------------------------------------------------
  const common = tone?.common;
  const partialSwitches = common
    ? [common.partial1Switch, common.partial2Switch, common.partial3Switch]
    : [];

  const togglePartialSwitch = (idx: number) => {
    const offsets: Record<number, number> = { 0: 0x19, 1: 0x1B, 2: 0x1D };
    const current = partialSwitches[idx] ?? 0;
    setCommonParam(offsets[idx] ?? 0x19, current !== 0 ? 0 : 1);
  };

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  if (!tone) {
    return <div className={css.loading}>Loading SN-S tone data...</div>;
  }

  return (
    <div className={css.editor}>
      {/* Tone name */}
      <div className={css.toneName}>{tone.common.toneName || "(unnamed)"}</div>

      {/* Common Controls Strip */}
      <CommonStrip common={tone.common} onChange={setCommonParam} />

      {/* Partial rows + MFX sidebar */}
      <div className={css.mainArea}>
        <div className={css.partialRows}>
          {tone.partials.map((partial, idx) => {
            const isOn = (partialSwitches[idx] ?? 0) !== 0;
            const setP = (offset: number, value: number) => setPartialParam(idx, offset, value);
            const setNibP = (offset: number, value: number) =>
              setPartialNibParam(idx, offset, value);

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
              {...mfxHandlers}
            />
          )}
        </div>
      </div>
    </div>
  );
}
