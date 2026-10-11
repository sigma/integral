import { useEffect, useState, useCallback, useRef, useMemo } from "react";
import {
  pcmd_common_address,
  pcmd_common_size,
  pcmd_common2_address,
  pcmd_common2_size,
  pcmd_mfx_address,
  pcmd_partial_address,
  pcmd_partial_size,
  mfx_block_size,
  ToneType,
} from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import { deviceSpec } from "./deviceSpec";
import {
  SectionPanel,
} from "./synth-ui";
import { MfxPanel } from "./MfxPanel";
import { useMfxHandlers } from "./useMfxHandlers";
import type { MfxState } from "./MfxPanel";
import css from "./PcmDrumEditor.module.css";

import type {
  PcmDrumCommon,
  PcmDrumCommon2,
  PartialParamHandler,
  PcmDrumPartial,
  WmtParamHandler,
} from "./pcm-drum/types";
import { CommonPanel } from "./pcm-drum/CommonPanel";
import { DrumKeyGrid } from "./DrumKeyGrid";
import { useKeyPrefetch } from "./useKeyPrefetch";
import { NoteControls } from "./pcm-drum/NoteControls";

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface Props {
  partIndex: number;
  service: IntegraService;
}

export function PcmDrumEditor({ partIndex, service }: Props) {
  const keys = deviceSpec().pcm_drum_keys;
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
        // Common
        const commonData = await service.requestData(
          Array.from(pcmd_common_address(partIndex)),
          Array.from(pcmd_common_size()),
        );
        if (cancelled) return;
        const c = service.device.applyPcmdCommon(commonData) as PcmDrumCommon | null;
        if (c) setCommon(c);

        // Common2
        const c2Data = await service.requestData(
          Array.from(pcmd_common2_address(partIndex)),
          Array.from(pcmd_common2_size()),
        );
        if (cancelled) return;
        const c2 = service.device.applyPcmdCommon2(c2Data) as PcmDrumCommon2 | null;
        if (c2) setCommon2(c2);

        // MFX
        const mfxData = await service.requestData(
          Array.from(pcmd_mfx_address(partIndex)),
          Array.from(mfx_block_size()),
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
    const { first, last } = deviceSpec().pcm_drum_keys;
    if (selectedKey < first || selectedKey > last) return;

    const cached = noteCache.current.get(selectedKey);
    if (cached) {
      setNoteData(cached);
      return;
    }

    let cancelled = false;
    setNoteLoading(true);

    async function loadNote() {
      try {
        const data = await service.requestData(
          Array.from(pcmd_partial_address(partIndex, selectedKey)),
          Array.from(pcmd_partial_size()),
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

  // Prefetch every key so the grid shows routing and switched-off keys up front
  const fetchPartial = useCallback(
    async (k: number) =>
      service.device.applyPcmdPartial(
        await service.requestData(
          Array.from(pcmd_partial_address(partIndex, k)),
          Array.from(pcmd_partial_size()),
        ),
      ) as PcmDrumPartial | null,
    [partIndex, service],
  );
  const cacheVersion = useKeyPrefetch(!loading, keys.first, keys.last, noteCache, fetchPartial);

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
  // Per-key partial param setter
  // ---------------------------------------------------------------------------
  const setPartialParam = useCallback<PartialParamHandler>(
    (field, key, index, value) => {
      service.device.setPcmdPartialField(partIndex, selectedKey, field, index, value);
      setNoteData((prev) => {
        if (!prev) return prev;
        const cur = prev[key];
        const updated = Array.isArray(cur)
          ? cur.map((c, i) => (i === index ? value : c))
          : value;
        const next = { ...prev, [key]: updated };
        noteCache.current.set(selectedKey, next);
        return next;
      });
    },
    [partIndex, selectedKey, service],
  );

  // ---------------------------------------------------------------------------
  // Per-key WMT param setter
  // ---------------------------------------------------------------------------
  const setWmtParam = useCallback<WmtParamHandler>(
    (wmt, field, key, value) => {
      service.device.setPcmdWmtParam(partIndex, selectedKey, wmt, field, value);
      setNoteData((prev) => {
        if (!prev) return prev;
        const layers = [...prev.wmt];
        layers[wmt] = { ...layers[wmt]!, [key]: value };
        const next = { ...prev, wmt: layers };
        noteCache.current.set(selectedKey, next);
        return next;
      });
    },
    [partIndex, selectedKey, service],
  );

  const mfxHandlers = useMfxHandlers(service, partIndex, ToneType.PcmDrum, setMfx);

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  // Build key→outputAssign map and the set of silent keys (every WMT layer
  // switched off) from cached notes for key grid color coding.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const [keyOutputAssigns, offKeys] = useMemo(() => {
    const m = new Map<number, number>();
    const off = new Set<number>();
    for (const [k, n] of noteCache.current) {
      m.set(k, n.outputAssign);
      if (n.wmt.every((w) => w.wmtSwitch !== 1)) off.add(k);
    }
    return [m, off] as const;
  }, [noteData, selectedKey, cacheVersion]);

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
            <DrumKeyGrid
              firstKey={keys.first}
              lastKey={keys.last}
              selectedKey={selectedKey}
              onSelect={handleKeySelect}
              keyOutputAssigns={keyOutputAssigns}
              offKeys={offKeys}
            />
            {noteLoading ? (
              <div className={css.noteLoadingPlaceholder}>Loading key data...</div>
            ) : noteData ? (
              <NoteControls
                keyNumber={selectedKey}
                note={noteData}
                onParam={setPartialParam}
                onWmtParam={setWmtParam}
              />
            ) : (
              <div className={css.noteLoadingPlaceholder}>Select a key to edit</div>
            )}
          </SectionPanel>
        </div>

        {mfx && (
          <MfxPanel
            mfx={mfx}
            {...mfxHandlers}
          />
        )}
      </div>
    </div>
  );
}
