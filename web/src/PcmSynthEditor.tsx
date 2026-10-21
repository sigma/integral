import { useEffect, useState, useCallback } from "react";
import {
  pcms_common_address,
  pcms_common_size,
  pcms_pmt_address,
  pcms_pmt_size,
  pcms_common2_address,
  pcms_common2_size,
  pcms_mfx_address,
  mfx_block_size,
  pcms_partial_address,
  pcms_partial_size,
  ToneType,
} from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import type { ExpansionMarking } from "./WavePicker";
import { deviceSpec } from "./deviceSpec";
import { GridZoom } from "./layouts";
import { PCMS_SCREEN_CELLS } from "./layouts/pcm-synth/screen";
import { useMfxHandlers } from "./useMfxHandlers";
import type { MfxState } from "./MfxPanel";
import css from "./PcmSynthEditor.module.css";

import type {
  CommonParamHandler,
  PartialParamHandler,
  PcmSynthCommon,
  PcmSynthPmt,
  PcmSynthPartial,
  PcmSynthCommon2,
  PmtEntryParamHandler,
  PmtParamHandler,
} from "./pcm-synth/types";
import { PcmSynthScreen } from "./pcm-synth/PcmSynthScreen";

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface Props {
  partIndex: number;
  service: IntegraService;
  /** Loaded state of the Expansions behind SRX Wave Groups. */
  marking?: ExpansionMarking;
}

export function PcmSynthEditor({ partIndex, service, marking }: Props) {
  const [common, setCommon] = useState<PcmSynthCommon | null>(null);
  const [pmt, setPmt] = useState<PcmSynthPmt | null>(null);
  const [_common2, setCommon2] = useState<PcmSynthCommon2 | null>(null);
  const [partials, setPartials] = useState<(PcmSynthPartial | null)[]>(() =>
    Array.from({ length: deviceSpec().pcm_synth_partial_count }, () => null),
  );
  const [mfx, setMfx] = useState<MfxState | null>(null);
  const [loading, setLoading] = useState(true);

  // Load common data on mount / part change
  useEffect(() => {
    let cancelled = false;
    setLoading(true);

    async function load() {
      try {
        // Common
        const commonData = await service.requestData(
          Array.from(pcms_common_address(partIndex)),
          Array.from(pcms_common_size()),
        );
        if (cancelled) return;
        const c = service.device.applyPcmsCommon(commonData) as PcmSynthCommon | null;
        if (c) setCommon(c);

        // PMT
        const pmtData = await service.requestData(
          Array.from(pcms_pmt_address(partIndex)),
          Array.from(pcms_pmt_size()),
        );
        if (cancelled) return;
        const p = service.device.applyPcmsPmt(pmtData) as PcmSynthPmt | null;
        if (p) setPmt(p);

        // Common2
        const c2Data = await service.requestData(
          Array.from(pcms_common2_address(partIndex)),
          Array.from(pcms_common2_size()),
        );
        if (cancelled) return;
        const c2 = service.device.applyPcmsCommon2(c2Data) as PcmSynthCommon2 | null;
        if (c2) setCommon2(c2);

        // MFX
        const mfxData = await service.requestData(
          Array.from(pcms_mfx_address(partIndex)),
          Array.from(mfx_block_size()),
        );
        if (cancelled) return;
        const m = service.device.applyMfxBlock(partIndex, mfxData) as MfxState | null;
        if (m) setMfx(m);

        // Load all partials
        for (let idx = 0; idx < deviceSpec().pcm_synth_partial_count; idx++) {
          if (cancelled) return;
          const pData = await service.requestData(
            Array.from(pcms_partial_address(partIndex, idx)),
            Array.from(pcms_partial_size()),
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
  // Param setters: send to the device, then patch the local copy
  // ---------------------------------------------------------------------------
  const setCommonParam = useCallback<CommonParamHandler>(
    (field, key, value) => {
      service.device.setPcmsCommonParam(partIndex, field, 0, value);
      setCommon((prev) => (prev ? { ...prev, [key]: value } : prev));
    },
    [partIndex, service],
  );

  const setPmtParam = useCallback<PmtParamHandler>(
    (field, key, value) => {
      service.device.setPcmsPmtParam(partIndex, field, 0, value);
      setPmt((prev) => (prev ? { ...prev, [key]: value } : prev));
    },
    [partIndex, service],
  );

  const setPmtEntryParam = useCallback<PmtEntryParamHandler>(
    (partial, field, key, value) => {
      service.device.setPcmsPmtParam(partIndex, field, partial, value);
      setPmt((prev) => {
        if (!prev) return prev;
        const partialEntries = prev.partialEntries.map((e, i) =>
          i === partial ? { ...e, [key]: value } : e,
        );
        return { ...prev, partialEntries };
      });
    },
    [partIndex, service],
  );

  const setPartialParam = useCallback(
    (partial: number, ...[field, key, index, value]: Parameters<PartialParamHandler>) => {
      service.device.setPcmsPartialParam(partIndex, partial, field, index, value);
      setPartials((prev) => {
        const p = prev[partial];
        if (!p) return prev;
        const cur = p[key];
        const updated = Array.isArray(cur)
          ? cur.map((c, i) => (i === index ? value : c))
          : value;
        const next = [...prev];
        next[partial] = { ...p, [key]: updated };
        return next;
      });
    },
    [partIndex, service],
  );

  const mfxHandlers = useMfxHandlers(service, partIndex, ToneType.PcmSynth, setMfx);

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  if (loading) {
    return <div className={css.loading}>Loading PCM Synth tone data...</div>;
  }

  return (
    <div className={css.editor}>
      <GridZoom cells={PCMS_SCREEN_CELLS}>
        {common && <div className={css.toneName}>{common.toneName || "(unnamed)"}</div>}
        <PcmSynthScreen
          common={common}
          pmt={pmt}
          partials={partials}
          mfx={mfx}
          mfxHandlers={mfxHandlers}
          onCommon={setCommonParam}
          onPmt={setPmtParam}
          onPmtEntry={setPmtEntryParam}
          onPartial={setPartialParam}
          marking={marking}
        />
      </GridZoom>
    </div>
  );
}
