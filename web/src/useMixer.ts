/**
 * useMixer — React hook for bidirectional mixer state with the INTEGRA-7.
 *
 * State management, echo suppression, and send queuing are delegated to
 * the Rust `WasmDeviceState` (via `service.device`).  This hook:
 * - Orchestrates the initial device read sequence (RQ1 requests)
 * - Applies RQ1 responses to the Rust state
 * - Syncs Rust state into React state for rendering
 * - Forwards user actions to the Rust setters
 */

import { useEffect, useState, useCallback, useRef } from "react";
import type { IntegraService } from "./integra";
import { factoryToneName } from "../pkg/integral_wasm.js";
import {
  defaultMixerState,
  type MixerState,
  type PartState,
  type EqState,
  type FxState,
  type SurroundState,
  type DrumCompEqState,
} from "./types";

/**
 * Shape returned by WasmDeviceState.readState() — the Rust MixerState
 * serialized via serde with camelCase field names.
 *
 * This mirrors the Rust `state::MixerState` struct and lets us avoid
 * `as any` when reading the Rust state snapshot.
 */
interface RustMixerState {
  studioSetName: string;
  studioSetPC: number;
  masterLevel: number;
  soloPart: number;
  parts: PartState[];
  chorus: FxState;
  reverb: FxState;
  extLevel: number;
  extMuted: boolean;
  masterEq: EqState;
  surround: SurroundState;
  drumCompEq: DrumCompEqState;
  previewPart: number;
}

export interface UseMixerResult {
  state: MixerState;
  setPartLevel: (part: number, value: number) => void;
  setPartPan: (part: number, value: number) => void;
  togglePartMute: (part: number) => void;
  toggleSolo: (part: number) => void;
  changePartTone: (part: number, msb: number, lsb: number, pc: number) => void;
  setPartReceiveChannel: (part: number, channel: number) => void;
  setPartOutputAssign: (part: number, value: number) => void;
  setPartChorusSend: (part: number, value: number) => void;
  setPartReverbSend: (part: number, value: number) => void;
  setMasterLevel: (value: number) => void;
  setPartEqParam: (part: number, paramOffset: number, value: number) => void;
  togglePartEqSwitch: (part: number) => void;
  setMasterEqParam: (paramOffset: number, value: number) => void;
  toggleMasterEqSwitch: () => void;
  setChorusParam: (offset: number, value: number) => void;
  setChorusNibParam: (paramIndex: number, value: number) => void;
  toggleChorusSwitch: () => void;
  setReverbParam: (offset: number, value: number) => void;
  setReverbNibParam: (paramIndex: number, value: number) => void;
  toggleReverbSwitch: () => void;
  setExtLevel: (value: number) => void;
  toggleExtMute: () => void;
  setSurroundParam: (paramOffset: number, value: number) => void;
  setPartSurroundLr: (part: number, value: number) => void;
  setPartSurroundFb: (part: number, value: number) => void;
  setPartSurroundWidth: (part: number, value: number) => void;
  setPartSurroundAmbienceSend: (part: number, value: number) => void;
  setDrumCompEqSwitch: (enabled: boolean) => void;
  setDrumCompEqPart: (part: number) => void;
  setDrumCompEqOutputAssign: (unit: number, value: number) => void;
  setCompEqParam: (unit: number, paramOffset: number, value: number) => void;
  toggleEqExpanded: () => void;
  selectPart: (part: number) => void;
  switchStudioSet: (pc: number) => void;
  loadStudioSetNames: () => void;
  preview: () => void;
}

export function useMixer(service: IntegraService | null): UseMixerResult {
  const [state, setState] = useState<MixerState>(defaultMixerState);
  const stateRef = useRef(state);
  stateRef.current = state;
  const loadGenRef = useRef(0);
  const rafId = useRef(0);

  // Sync Rust DeviceState → React state.
  // Preserves UI-only fields (selectedPart, eqExpanded, loading, studioSetNames).
  // Debounced via requestAnimationFrame so multiple rapid calls within one
  // frame coalesce into a single state read.
  const syncFromRust = useCallback(() => {
    if (!service) return;
    cancelAnimationFrame(rafId.current);
    rafId.current = requestAnimationFrame(() => {
      // readState() returns a plain JS object matching the Rust MixerState
      // shape (camelCase fields via serde rename).
      const rs = service.device.readState() as RustMixerState;
      setState((prev) => ({
        studioSetName: rs.studioSetName ?? "",
        studioSetPC: rs.studioSetPC ?? 0,
        masterLevel: rs.masterLevel ?? 100,
        soloPart: rs.soloPart ?? 0,
        parts: (rs.parts ?? []).map((p: PartState) => p),
        chorus: rs.chorus ?? prev.chorus,
        reverb: rs.reverb ?? prev.reverb,
        extLevel: rs.extLevel ?? 100,
        extMuted: rs.extMuted ?? false,
        masterEq: rs.masterEq ?? prev.masterEq,
        surround: rs.surround ?? prev.surround,
        drumCompEq: rs.drumCompEq ?? prev.drumCompEq,
        previewPart: rs.previewPart ?? 0,
        // UI-only fields preserved from React state.
        selectedPart: prev.selectedPart,
        eqExpanded: prev.eqExpanded,
        loading: prev.loading,
        studioSetNames: prev.studioSetNames,
      }));
    });
  }, [service]);

  // Listen for incoming DT1 — Rust handles echo suppression and state update.
  // We just need to sync React state afterwards.
  useEffect(() => {
    if (!service) return;
    const unsub = service.onDt1(() => {
      syncFromRust();
    });
    return () => {
      unsub();
      cancelAnimationFrame(rafId.current);
    };
  }, [service, syncFromRust]);

  // -----------------------------------------------------------------------
  // Initial state load
  // -----------------------------------------------------------------------

  const loadState = useCallback(
    async (svc: IntegraService) => {
      const gen = ++loadGenRef.current;
      const isCurrent = () => loadGenRef.current === gen;
      const dev = svc.device;

      setState((prev) => ({ ...prev, loading: true }));

      try {
        const [name, studioSetPC, masterLevel, soloPart] = await Promise.all([
          svc.requestStudioSetName(),
          svc.requestStudioSetPC(),
          svc.requestMasterLevel(),
          svc.requestSoloPart(),
        ]);

        if (!isCurrent()) return;

        dev.setStudioSetName(name);
        dev.setStudioSetPc(studioSetPC);
        dev.applyMasterLevel(masterLevel);
        dev.setSoloPart(soloPart);

        for (let i = 0; i < 16; i++) {
          try {
            const dump = await svc.requestPartMixerState(i);
            dev.applyPartDump(i, dump);
          } catch (e) {
            console.warn(`[mixer] Part ${i + 1} failed:`, e);
          }
        }

        if (!isCurrent()) return;

        setState((prev) => ({
          ...prev,
          loading: false,
          studioSetNames: new Map([...prev.studioSetNames, [studioSetPC, name]]),
        }));
        syncFromRust();

        // Tone names: set factory names instantly, then request device names.
        const rs0 = dev.readState() as RustMixerState;
        for (let i = 0; i < 16; i++) {
          const p = rs0.parts?.[i];
          if (!p) continue;
          // Instant factory lookup — avoids blank names while MIDI loads.
          const fName = factoryToneName(p.toneBankMsb, p.toneBankLsb, p.tonePC);
          if (fName) {
            dev.setPartToneName(i, fName);
          }
        }
        syncFromRust();

        // Non-blocking: request actual device names (may differ for user tones).
        for (let i = 0; i < 16; i++) {
          const msb = rs0.parts?.[i]?.toneBankMsb;
          if (msb === undefined) continue;
          svc.requestToneName(i, msb).then((toneName) => {
            if (!isCurrent() || !toneName) return;
            dev.setPartToneName(i, toneName);
            syncFromRust();
          });
        }

        // Part EQ
        for (let i = 0; i < 16; i++) {
          svc.requestPartEq(i).then((eqData) => {
            if (!isCurrent()) return;
            dev.applyPartEqDump(i, eqData);
            syncFromRust();
          }).catch((e: unknown) => console.warn("[mixer] load error:", e));
        }

        // Master EQ
        Promise.all([svc.requestMasterEq(), svc.requestMasterEqSwitch()]).then(
          ([eqData, enabled]) => {
            if (!isCurrent()) return;
            dev.applyMasterEqDump(eqData);
            dev.setMasterEqEnabled(enabled);
            syncFromRust();
          },
        ).catch((e: unknown) => console.warn("[mixer] load error:", e));

        // Ext Part
        Promise.all([svc.requestExtPartLevel(), svc.requestExtPartMute()]).then(
          ([level, muted]) => {
            if (!isCurrent()) return;
            dev.applyExtLevel(level);
            dev.applyExtMuted(muted);
            syncFromRust();
          },
        ).catch((e: unknown) => console.warn("[mixer] load error:", e));

        // Chorus
        Promise.all([
          svc.requestChorusCore(),
          svc.requestChorusSwitch(),
          svc.requestChorusParams(),
        ]).then(([core, enabled, params]) => {
          if (!isCurrent()) return;
          dev.applyChorusCore(core);
          dev.setChorusEnabled(enabled);
          dev.applyChorusParams(Int32Array.from(params));
          syncFromRust();
        }).catch((e: unknown) => console.warn("[mixer] load error:", e));

        // Reverb
        Promise.all([
          svc.requestReverbCore(),
          svc.requestReverbSwitch(),
          svc.requestReverbParams(),
        ]).then(([core, enabled, params]) => {
          if (!isCurrent()) return;
          dev.applyReverbCore(core);
          dev.setReverbEnabled(enabled);
          dev.applyReverbParams(Int32Array.from(params));
          syncFromRust();
        }).catch((e: unknown) => console.warn("[mixer] load error:", e));

        // Motional Surround
        svc.requestSurroundCommon().then((data) => {
          if (!isCurrent()) return;
          dev.applySurroundCommon(data);
          // Load per-part surround positioning.
          for (let i = 0; i < 16; i++) {
            svc.requestPartSurround(i).then(({ lr, fb, width, ambienceSend }) => {
              if (!isCurrent()) return;
              dev.applyPartSurround(i, lr, fb, width, ambienceSend);
              syncFromRust();
            }).catch((e: unknown) => console.warn("[mixer] load error:", e));
          }
          syncFromRust();
        }).catch((e: unknown) => console.warn("[mixer] load error:", e));

        // Drum Comp+EQ
        svc.requestDrumCompEqCommon().then(({ enabled, part, outputAssigns }) => {
          if (!isCurrent()) return;
          dev.applyDrumCompEqCommon(enabled, part, new Uint8Array(outputAssigns));
          // Read the 6 units from the assigned part's tone block.
          svc.requestCompEqBlock(part).then((blockData) => {
            if (!isCurrent()) return;
            dev.applyCompEqBlock(blockData);
            syncFromRust();
          }).catch((e: unknown) => console.warn("[mixer] load error:", e));
          syncFromRust();
        }).catch((e: unknown) => console.warn("[mixer] load error:", e));

      } catch {
        setState((prev) => ({ ...prev, loading: false }));
      }
    },
    [syncFromRust],
  );

  useEffect(() => {
    if (!service) return;
    loadState(service);
  }, [service, loadState]);

  // -----------------------------------------------------------------------
  // Studio Set catalog (lazy)
  // -----------------------------------------------------------------------

  const catalogLoaded = useRef(false);
  const loadStudioSetNames = useCallback(async () => {
    if (!service || catalogLoaded.current) return;
    catalogLoaded.current = true;
    const names = await service.requestStudioSetNames();
    setState((prev) => ({ ...prev, studioSetNames: names }));
  }, [service]);

  // -----------------------------------------------------------------------
  // Setters — delegate to Rust DeviceState + sync React state
  // -----------------------------------------------------------------------

  const setPartLevel = useCallback(
    (part: number, value: number) => {
      service?.device.setPartLevel(part, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const setPartPan = useCallback(
    (part: number, value: number) => {
      service?.device.setPartPan(part, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const togglePartMute = useCallback(
    (part: number) => {
      service?.device.togglePartMute(part);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const toggleSolo = useCallback(
    (part: number) => {
      service?.device.toggleSolo(part);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const changePartTone = useCallback(
    (part: number, msb: number, lsb: number, pc: number) => {
      if (!service) return;
      service.device.changePartTone(part, msb, lsb, pc);

      // Set factory name instantly for responsive UI.
      const fName = factoryToneName(msb, lsb, pc);
      if (fName) {
        service.device.setPartToneName(part, fName);
      }
      syncFromRust();

      // Re-read actual device name (may differ for user tones).
      setTimeout(() => {
        service.requestToneName(part, msb).then((toneName) => {
          if (toneName) {
            service.device.setPartToneName(part, toneName);
            syncFromRust();
          }
        });
      }, 300);
    },
    [service, syncFromRust],
  );

  const setPartReceiveChannel = useCallback(
    (part: number, channel: number) => {
      service?.device.setPartReceiveChannel(part, channel);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const setPartOutputAssign = useCallback(
    (part: number, value: number) => {
      service?.device.setPartOutputAssign(part, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const setPartChorusSend = useCallback(
    (part: number, value: number) => {
      service?.device.setPartChorusSend(part, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const setPartReverbSend = useCallback(
    (part: number, value: number) => {
      service?.device.setPartReverbSend(part, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const setMasterLevel = useCallback(
    (value: number) => {
      service?.device.setMasterLevel(value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const selectPart = useCallback((part: number) => {
    if (service && service.device.previewPart() > 0) {
      service.device.previewStart(part + 1);
      syncFromRust();
    }
    setState((prev) => ({ ...prev, selectedPart: part }));
  }, [service, syncFromRust]);

  const switchStudioSet = useCallback(
    (pc: number) => {
      if (!service) return;
      service.device.switchStudioSet(pc);
      setTimeout(() => loadState(service), 500);
    },
    [service, loadState],
  );

  // --- EQ ---

  const setPartEqParam = useCallback(
    (part: number, paramOffset: number, value: number) => {
      service?.device.setPartEqParam(part, paramOffset, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const togglePartEqSwitch = useCallback(
    (part: number) => {
      const enabled = !stateRef.current.parts[part]!.eq.enabled;
      service?.device.setPartEqParam(part, 0, enabled ? 1 : 0);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const setMasterEqParam = useCallback(
    (paramOffset: number, value: number) => {
      service?.device.setMasterEqParam(paramOffset, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const toggleMasterEqSwitch = useCallback(() => {
    service?.device.toggleMasterEqSwitch();
    syncFromRust();
  }, [service, syncFromRust]);

  // --- FX ---

  const setChorusParam = useCallback(
    (offset: number, value: number) => {
      service?.device.setChorusParam(offset, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const setChorusNibParam = useCallback(
    (paramIndex: number, value: number) => {
      service?.device.setChorusNibParam(paramIndex, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const toggleChorusSwitch = useCallback(() => {
    service?.device.toggleChorusSwitch();
    syncFromRust();
  }, [service, syncFromRust]);

  const setReverbParam = useCallback(
    (offset: number, value: number) => {
      service?.device.setReverbParam(offset, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const setReverbNibParam = useCallback(
    (paramIndex: number, value: number) => {
      service?.device.setReverbNibParam(paramIndex, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const toggleReverbSwitch = useCallback(() => {
    service?.device.toggleReverbSwitch();
    syncFromRust();
  }, [service, syncFromRust]);

  // --- Ext Part ---

  const setExtLevel = useCallback(
    (value: number) => {
      service?.device.setExtLevel(value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const toggleExtMute = useCallback(() => {
    service?.device.toggleExtMute();
    syncFromRust();
  }, [service, syncFromRust]);

  // --- Motional Surround ---

  const setSurroundParam = useCallback(
    (paramOffset: number, value: number) => {
      service?.device.setSurroundParam(paramOffset, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const setPartSurroundLr = useCallback(
    (part: number, value: number) => {
      service?.device.setPartSurroundLr(part, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const setPartSurroundFb = useCallback(
    (part: number, value: number) => {
      service?.device.setPartSurroundFb(part, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const setPartSurroundWidth = useCallback(
    (part: number, value: number) => {
      service?.device.setPartSurroundWidth(part, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const setPartSurroundAmbienceSend = useCallback(
    (part: number, value: number) => {
      service?.device.setPartSurroundAmbienceSend(part, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  // --- Drum Comp+EQ ---

  const setDrumCompEqSwitch = useCallback(
    (enabled: boolean) => {
      service?.device.setDrumCompEqSwitch(enabled);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const setDrumCompEqPart = useCallback(
    (part: number) => {
      service?.device.setDrumCompEqPart(part);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const setDrumCompEqOutputAssign = useCallback(
    (unit: number, value: number) => {
      service?.device.setDrumCompEqOutputAssign(unit, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const setCompEqParam = useCallback(
    (unit: number, paramOffset: number, value: number) => {
      service?.device.setCompEqParam(unit, paramOffset, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  // --- UI-only state ---

  const toggleEqExpanded = useCallback(() => {
    setState((prev) => ({ ...prev, eqExpanded: !prev.eqExpanded }));
  }, []);

  const preview = useCallback(() => {
    if (!service) return;
    const current = stateRef.current;
    // Toggle phrase preview for the selected part (1-indexed).
    service.device.previewToggle(current.selectedPart + 1);
    syncFromRust();
  }, [service, syncFromRust]);

  return {
    state,
    setPartLevel,
    setPartPan,
    togglePartMute,
    toggleSolo,
    changePartTone,
    setPartReceiveChannel,
    setPartOutputAssign,
    setPartChorusSend,
    setPartReverbSend,
    setMasterLevel,
    setPartEqParam,
    togglePartEqSwitch,
    setMasterEqParam,
    toggleMasterEqSwitch,
    setChorusParam,
    setChorusNibParam,
    toggleChorusSwitch,
    setReverbParam,
    setReverbNibParam,
    toggleReverbSwitch,
    setExtLevel,
    toggleExtMute,
    setSurroundParam,
    setPartSurroundLr,
    setPartSurroundFb,
    setPartSurroundWidth,
    setPartSurroundAmbienceSend,
    setDrumCompEqSwitch,
    setDrumCompEqPart,
    setDrumCompEqOutputAssign,
    setCompEqParam,
    toggleEqExpanded,
    selectPart,
    switchStudioSet,
    loadStudioSetNames,
    preview,
  };
}
