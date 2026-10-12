import { useEffect, useState, useCallback } from "react";
import {
  sns_mfx_address,
  mfx_block_size,
  sns_partial_switch_field,
  ToneType,
} from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import { MfxPanel } from "./MfxPanel";
import { SignalArrows } from "./synth-ui/SignalArrows";
import { useMfxHandlers } from "./useMfxHandlers";
import type { MfxState } from "./MfxPanel";
import css from "./SnSynthEditor.module.css";
import type {
  CommonParamHandler,
  PartialParamHandler,
  SnSynthTone,
} from "./sn-synth/types";
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
  // asks again for lost replies): read it afresh on open, in case it was
  // edited on the device, and show it again on each reply.
  const readTone = useCallback(() => {
    setTone(service.device.snsTone(partIndex) as SnSynthTone | null);
  }, [partIndex, service]);

  useEffect(() => {
    service.device.reloadSnsTone(partIndex);
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
  const setCommonParam = useCallback<CommonParamHandler>(
    (field, value) => {
      service.device.setSnsCommonParam(partIndex, field, value);
      readTone();
    },
    [partIndex, service, readTone],
  );

  const setPartialParam = useCallback(
    (partial: number, ...[field, value]: Parameters<PartialParamHandler>) => {
      service.device.setSnsPartialParam(partIndex, partial, field, value);
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
    const current = partialSwitches[idx] ?? 0;
    setCommonParam(sns_partial_switch_field(idx), current !== 0 ? 0 : 1);
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

      {/* Common strip + partial rows, MFX sidebar */}
      <div className={css.mainArea}>
        <div className={css.partialRows}>
          <CommonStrip common={tone.common} onChange={setCommonParam} />
          {tone.partials.map((partial, idx) => {
            const isOn = (partialSwitches[idx] ?? 0) !== 0;
            const setP: PartialParamHandler = (field, value) =>
              setPartialParam(idx, field, value);

            return (
              <PartialRow
                key={idx}
                idx={idx}
                partial={partial}
                isOn={isOn}
                onToggle={() => togglePartialSwitch(idx)}
                setP={setP}
              />
            );
          })}
        </div>
        <div className={css.mfxSidebar} data-node="mfx">
          {mfx && (
            <MfxPanel
              mfx={mfx}
              {...mfxHandlers}
            />
          )}
        </div>
        {/* Every partial is summed into the MFX. */}
        <SignalArrows edges={tone.partials.map((_, i) => ({ from: `out-${i}`, to: "mfx", glyph: "+" }))} />
      </div>
    </div>
  );
}
