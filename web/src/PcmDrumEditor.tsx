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
      // Pitch env
      applyPitchEnvField(p, offset, value);
      // TVF
      applyTvfField(p, offset, value);
      // TVA
      applyTvaField(p, offset, value);
      // One-shot
      if (offset === 0x0141) p.oneShotMode = value;
      break;
  }
}

function applyPitchEnvField(p: PcmDrumPartial, offset: number, value: number): void {
  // Pitch env fields at 0x0115-0x0121
  const pitchBase = 0x0115; // Pitch Env Depth
  const rel = offset - pitchBase;
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
  // TVF fields at 0x0122-0x0135
  const tvfBase = 0x0122; // TVF Filter Type
  const rel = offset - tvfBase;
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
  // TVA fields at 0x0136-0x0141
  const tvaBase = 0x0136; // TVA Level Velocity Curve
  const rel = offset - tvaBase;
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
