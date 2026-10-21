import { useEffect, useState, useCallback } from "react";
import {
  sns_mfx_address,
  mfx_block_size,
  sns_partial_scope_is_active,
  ToneType,
} from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import { GridScreen, GridZoom } from "./layouts";
import { SNS_SECTION, rowSectionOf, snsScreen } from "./layouts/sn-synth/screen";
import { MfxPanel } from "./MfxPanel";
import { useMfxHandlers } from "./useMfxHandlers";
import type { MfxState } from "./MfxPanel";
import css from "./SnSynthEditor.module.css";
import type {
  CommonParamHandler,
  PartialParamHandler,
  SnSynthTone,
} from "./sn-synth/types";
import { boxOf } from "./layouts/sn-synth/partial";
import { CommonSection, PartialSwitch } from "./sn-synth/CommonSection";
import { PartialBox } from "./sn-synth/PartialBox";

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
  // Render
  // ---------------------------------------------------------------------------

  if (!tone) {
    return <div className={css.loading}>Loading SN-S tone data...</div>;
  }

  return (
    <div className={css.editor}>
      <GridZoom cells={snsScreen()}>
        <div className={css.toneName}>{tone.common.toneName || "(unnamed)"}</div>
        {/* COMMON over the partial rows; the MFX beside them, every row summed into it. */}
        <GridScreen layout={snsScreen()}>
          {({ id }) => {
            if (id === SNS_SECTION.common) return <CommonSection common={tone.common} onChange={setCommonParam} />;
            if (id === SNS_SECTION.mfx) {
              return <div className={css.mfx}>{mfx && <MfxPanel mfx={mfx} {...mfxHandlers} />}</div>;
            }
            const at = rowSectionOf(id);
            const partial = at && tone.partials[at.partial];
            if (!at || !partial) return null;
            const idx = at.partial;
            if (at.section === "switch") {
              return <PartialSwitch idx={idx} common={tone.common} onChange={setCommonParam} />;
            }
            const box = boxOf(at.section);
            const setP: PartialParamHandler = (field, value) => setPartialParam(idx, field, value);
            return (
              <PartialBox
                idx={idx}
                layout={box.layout}
                family={box.family}
                partial={partial}
                active={sns_partial_scope_is_active(idx, tone.common)}
                setP={setP}
              />
            );
          }}
        </GridScreen>
      </GridZoom>
    </div>
  );
}
