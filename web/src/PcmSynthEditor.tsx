import { useCallback, useMemo } from "react";
import type { SetStateAction } from "react";
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
import { useToneBlocks, type ToneBlockRead } from "./useToneBlocks";
import css from "./PcmSynthEditor.module.css";

import {
  PCMS_COMMON_ENTRIES,
  PCMS_PARTIAL_ENTRIES,
  PCMS_PMT_ENTRIES,
  type CommonParamHandler,
  type PartialParamHandler,
  type PcmSynthCommon,
  type PcmSynthPmt,
  type PcmSynthPartial,
  type PcmSynthCommon2,
  type PmtEntryParamHandler,
  type PmtParamHandler,
} from "./pcm-synth/types";
import { PcmSynthScreen, type ScreenFailure } from "./pcm-synth/PcmSynthScreen";

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface Props {
  partIndex: number;
  service: IntegraService;
  /** Loaded state of the Expansions behind SRX Wave Groups. */
  marking?: ExpansionMarking;
}

/** Where each block sits in the read chain: Common, PMT, Common 2 (read for
 * core's copy only), MFX, then each Partial. */
const COMMON_I = 0;
const PMT_I = 1;
const MFX_I = 3;
const partialI = (i: number) => MFX_I + 1 + i;

export function PcmSynthEditor({ partIndex, service, marking }: Props) {
  const partialCount = deviceSpec().pcm_synth_partial_count;

  // The editor's blocks, in the order they are read. One block's failure
  // fails that block only; the rest of the chain still loads (#529).
  const reads = useMemo<readonly ToneBlockRead[]>(() => {
    /** What `apply` must answer with: the wasm decoders throw on a short or
     * malformed reply, and an empty answer fails the block visibly. */
    const answer = (value: unknown, name: string) => {
      if (!value) throw new Error(`${name} came back empty`);
      return value;
    };
    return [
      {
        name: "Tone Common",
        address: () => Array.from(pcms_common_address(partIndex)),
        size: () => Array.from(pcms_common_size()),
        apply: (d: Uint8Array) => answer(service.device.applyPcmsCommon(d), "Tone Common"),
      },
      {
        name: "PMT",
        address: () => Array.from(pcms_pmt_address(partIndex)),
        size: () => Array.from(pcms_pmt_size()),
        apply: (d: Uint8Array) => answer(service.device.applyPcmsPmt(d), "PMT"),
      },
      {
        // Read into core's copy of the tone; the editor shows none of it.
        name: "Common 2",
        address: () => Array.from(pcms_common2_address(partIndex)),
        size: () => Array.from(pcms_common2_size()),
        apply: (d: Uint8Array) => answer(service.device.applyPcmsCommon2(d) as PcmSynthCommon2 | null, "Common 2"),
      },
      {
        name: "MFX",
        address: () => Array.from(pcms_mfx_address(partIndex)),
        size: () => Array.from(mfx_block_size()),
        apply: (d: Uint8Array) => answer(service.device.applyMfxBlock(partIndex, d), "MFX"),
      },
      ...Array.from({ length: partialCount }, (_, i): ToneBlockRead => ({
        name: `Partial ${i + 1}`,
        address: () => Array.from(pcms_partial_address(partIndex, i)),
        size: () => Array.from(pcms_partial_size()),
        apply: (d: Uint8Array) => answer(service.device.applyPcmsPartial(d), `Partial ${i + 1}`),
      })),
    ];
  }, [partIndex, partialCount, service]);

  const { blocks, loading, failed, retry, setValue } = useToneBlocks(service, reads, [partIndex, service]);

  /** A block's data, or `null` until it has loaded. */
  const loaded = <T,>(i: number): T | null => {
    const b = blocks[i];
    return b?.status === "loaded" ? (b.value as T) : null;
  };
  const common = loaded<PcmSynthCommon>(COMMON_I);
  const pmt = loaded<PcmSynthPmt>(PMT_I);
  const mfx = loaded<MfxState>(MFX_I);
  const partials = Array.from({ length: partialCount }, (_, i) => loaded<PcmSynthPartial>(partialI(i)));

  // ---------------------------------------------------------------------------
  // Param setters: send to the device, then patch the local copy
  // ---------------------------------------------------------------------------
  const setCommonParam = useCallback<CommonParamHandler>(
    (field, value) => {
      service.device.setPcmsCommonParam(partIndex, field, 0, value);
      const key = PCMS_COMMON_ENTRIES[field]?.key;
      if (key) setValue(COMMON_I, (prev) => ({ ...(prev as PcmSynthCommon), [key]: value }));
    },
    [partIndex, service, setValue],
  );

  const setPmtParam = useCallback<PmtParamHandler>(
    (field, value) => {
      service.device.setPcmsPmtParam(partIndex, field, 0, value);
      const key = PCMS_PMT_ENTRIES[field]?.key;
      if (key) setValue(PMT_I, (prev) => ({ ...(prev as PcmSynthPmt), [key]: value }));
    },
    [partIndex, service, setValue],
  );

  const setPmtEntryParam = useCallback<PmtEntryParamHandler>(
    (partial, field, key, value) => {
      service.device.setPcmsPmtParam(partIndex, field, partial, value);
      setValue(PMT_I, (prev) => {
        const p = prev as PcmSynthPmt;
        const partialEntries = p.partialEntries.map((e, i) =>
          i === partial ? { ...e, [key]: value } : e,
        );
        return { ...p, partialEntries };
      });
    },
    [partIndex, service, setValue],
  );

  const setPartialParam = useCallback(
    (partial: number, ...[field, index, value]: Parameters<PartialParamHandler>) => {
      service.device.setPcmsPartialParam(partIndex, partial, field, index, value);
      const { key } = PCMS_PARTIAL_ENTRIES[field];
      setValue(partialI(partial), (prev) => {
        const p = prev as PcmSynthPartial;
        const cur = p[key];
        const updated = Array.isArray(cur)
          ? cur.map((c, i) => (i === index ? value : c))
          : value;
        return { ...p, [key]: updated };
      });
    },
    [partIndex, service, setValue],
  );

  const setMfx = useCallback(
    (action: SetStateAction<MfxState | null>) =>
      setValue(MFX_I, (prev) => {
        const next = typeof action === "function" ? action(prev as MfxState) : action;
        return next ?? prev;
      }),
    [setValue],
  );
  const mfxHandlers = useMfxHandlers(service, partIndex, ToneType.PcmSynth, setMfx);

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  if (loading) {
    return <div className={css.loading}>Loading PCM Synth tone data...</div>;
  }

  /** What a failed block shows, in place of its Sections. */
  const failure = (i: number): ScreenFailure | undefined => {
    const b = blocks[i];
    return b?.status === "error" ? { message: b.message ?? "its read failed", retry } : undefined;
  };
  const failures = failed
    ? {
        common: failure(COMMON_I),
        pmt: failure(PMT_I),
        mfx: failure(MFX_I),
        partials: Array.from({ length: partialCount }, (_, i) => failure(partialI(i))),
      }
    : undefined;

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
          failures={failures}
        />
      </GridZoom>
    </div>
  );
}
