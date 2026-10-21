/**
 * useMixer — React hook for bidirectional mixer state with the INTEGRA-7.
 *
 * State management, echo suppression, and send queuing are delegated to
 * the Rust `WasmDeviceState` (via `service.device`).  This hook:
 * - Starts the Rust mixer load (RQ1 requests and their replies)
 * - Syncs Rust state into React state for rendering
 * - Forwards user actions to the Rust setters
 */

import { useEffect, useState, useCallback, useRef } from "react";
import type { IntegraService } from "./integra";
import type { MixerField, MixerScopeKind, SurroundField } from "../pkg/integral_wasm.js";
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
  /** Set a mixer field in a scope; `index` is the part or Comp+EQ unit, ignored for other kinds. */
  setMixerField: (kind: MixerScopeKind, index: number, field: MixerField, value: number) => void;
  setChorusParam: (offset: number, value: number) => void;
  setChorusNibParam: (paramIndex: number, value: number) => void;
  toggleChorusSwitch: () => void;
  setReverbParam: (offset: number, value: number) => void;
  setReverbNibParam: (paramIndex: number, value: number) => void;
  toggleReverbSwitch: () => void;
  setExtLevel: (value: number) => void;
  toggleExtMute: () => void;
  setSurroundParam: (field: SurroundField, value: number) => void;
  setPartSurroundLr: (part: number, value: number) => void;
  setPartSurroundFb: (part: number, value: number) => void;
  setPartSurroundWidth: (part: number, value: number) => void;
  setPartSurroundAmbienceSend: (part: number, value: number) => void;
  setDrumCompEqSwitch: (enabled: boolean) => void;
  setDrumCompEqPart: (part: number) => void;
  setDrumCompEqOutputAssign: (unit: number, value: number) => void;
  selectPart: (part: number) => void;
  switchStudioSet: (pc: number) => void;
  /** Rename a part's Temporary tone on the device. */
  renameTone: (part: number, name: string) => void;
  /** Rename the Temporary Studio Set on the device. */
  renameStudioSet: (name: string) => void;
  /**
   * Load Studio Set names once; `force` re-reads them (e.g. after a save).
   * A load that returns nothing keeps the previous names and is retried
   * on the next call. Resolves with the names known then.
   */
  loadStudioSetNames: (force?: boolean) => Promise<Map<number, string>>;
  /** Read the whole mixer state again, e.g. after a Pack import. */
  reload: () => void;
  preview: () => void;
}

/** How often a running mixer load is checked for reads given up on (ms). */
const LOAD_POLL_MS = 250;

export function useMixer(service: IntegraService | null): UseMixerResult {
  const [state, setState] = useState<MixerState>(defaultMixerState);
  const stateRef = useRef(state);
  stateRef.current = state;
  const loadPollRef = useRef(0);
  const rafId = useRef(0);

  // Sync Rust DeviceState → React state.
  // Preserves UI-only fields (selectedPart, studioSetNames).
  // Debounced via requestAnimationFrame so multiple rapid calls within one
  // frame coalesce into a single state read.
  const syncFromRust = useCallback(() => {
    if (!service) return;
    cancelAnimationFrame(rafId.current);
    rafId.current = requestAnimationFrame(() => {
      // readState() returns a plain JS object matching the Rust MixerState
      // shape (camelCase fields via serde rename).
      const rs = service.device.readState() as RustMixerState;
      const loading = service.device.mixerLoading();
      const failedBlocks = service.device.mixerFailedBlocks();
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
        loading,
        failedBlocks,
        // The current Studio Set's name is known once the load is done.
        studioSetNames:
          prev.loading && !loading && rs.studioSetName
            ? new Map([...prev.studioSetNames, [rs.studioSetPC, rs.studioSetName]])
            : prev.studioSetNames,
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
      clearInterval(loadPollRef.current);
    };
  }, [service, syncFromRust]);

  // -----------------------------------------------------------------------
  // Initial state load
  // -----------------------------------------------------------------------

  const loadState = useCallback(
    (svc: IntegraService) => {
      clearInterval(loadPollRef.current);
      svc.device.loadMixer();
      setState((prev) => ({ ...prev, loading: true }));
      // Replies sync through onDt1; this also notices reads given up on.
      loadPollRef.current = window.setInterval(() => {
        if (svc.device.mixerLoading()) return;
        clearInterval(loadPollRef.current);
        syncFromRust();
      }, LOAD_POLL_MS);
    },
    [syncFromRust],
  );

  useEffect(() => {
    if (!service) return;
    loadState(service);
  }, [service, loadState]);

  const reload = useCallback(() => {
    if (service) void loadState(service);
  }, [service, loadState]);

  // -----------------------------------------------------------------------
  // Studio Set catalog (lazy)
  // -----------------------------------------------------------------------

  const catalogLoaded = useRef(false);
  /** The names read in flight, shared by concurrent callers. */
  const namesRead = useRef<Promise<Map<number, string>> | null>(null);
  /** The current service, so a read from a previous connection is dropped. */
  const serviceRef = useRef(service);
  serviceRef.current = service;

  // Names belong to the device behind the connection: forget them on change.
  useEffect(() => {
    catalogLoaded.current = false;
    namesRead.current = null;
    setState((prev) => ({ ...prev, studioSetNames: new Map() }));
  }, [service]);

  const loadStudioSetNames = useCallback(async (force = false) => {
    if (!service) return new Map<number, string>();
    if (!force && namesRead.current) return namesRead.current;
    if (!force && catalogLoaded.current) return stateRef.current.studioSetNames;
    catalogLoaded.current = true;
    const read = service.requestStudioSetNames();
    namesRead.current = read;
    const names = await read.finally(() => {
      if (namesRead.current === read) namesRead.current = null;
    });
    if (serviceRef.current !== service) return names;
    if (names.size === 0) {
      catalogLoaded.current = false;
      return stateRef.current.studioSetNames;
    }
    setState((prev) => ({ ...prev, studioSetNames: names }));
    return names;
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

  const renameTone = useCallback(
    (part: number, name: string) => {
      service?.device.renameTone(part, name);
      syncFromRust();
    },
    [service, syncFromRust],
  );

  const renameStudioSet = useCallback(
    (name: string) => {
      service?.device.renameStudioSet(name);
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
      // Core reads the new tone's name and category back.
      service.device.changePartTone(part, msb, lsb, pc);
      syncFromRust();
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

  const setMixerField = useCallback(
    (kind: MixerScopeKind, index: number, field: MixerField, value: number) => {
      service?.device.setMixerField(kind, index, field, value);
      syncFromRust();
    },
    [service, syncFromRust],
  );

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
    (field: SurroundField, value: number) => {
      service?.device.setSurroundParam(field, value);
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

  // --- UI-only state ---

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
    setMixerField,
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
    selectPart,
    switchStudioSet,
    renameTone,
    renameStudioSet,
    loadStudioSetNames,
    reload,
    preview,
  };
}
