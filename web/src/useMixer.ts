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
import {
  defaultMixerState,
  type MixerState,
  type PartState,
} from "./types";

export interface UseMixerResult {
  state: MixerState;
  setPartLevel: (part: number, value: number) => void;
  setPartPan: (part: number, value: number) => void;
  togglePartMute: (part: number) => void;
  changePartTone: (part: number, msb: number, lsb: number, pc: number) => void;
  setPartReceiveChannel: (part: number, channel: number) => void;
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

  // Sync Rust DeviceState → React state.
  // Preserves UI-only fields (selectedPart, eqExpanded, loading, studioSetNames).
  const syncFromRust = useCallback(() => {
    if (!service) return;
    // readState() returns a plain JS object matching the Rust MixerState
    // shape (camelCase fields via serde rename).
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const rs = service.device.readState() as any;
    setState((prev) => ({
      studioSetName: rs.studioSetName ?? "",
      studioSetPC: rs.studioSetPC ?? 0,
      masterLevel: rs.masterLevel ?? 100,
      parts: (rs.parts ?? []).map((p: PartState) => p),
      chorus: rs.chorus ?? prev.chorus,
      reverb: rs.reverb ?? prev.reverb,
      extLevel: rs.extLevel ?? 100,
      extMuted: rs.extMuted ?? false,
      masterEq: rs.masterEq ?? prev.masterEq,
      // UI-only fields preserved from React state.
      selectedPart: prev.selectedPart,
      eqExpanded: prev.eqExpanded,
      loading: prev.loading,
      studioSetNames: prev.studioSetNames,
    }));
  }, [service]);

  // Listen for incoming DT1 — Rust handles echo suppression and state update.
  // We just need to sync React state afterwards.
  useEffect(() => {
    if (!service) return;
    const unsub = service.onDt1(() => {
      syncFromRust();
    });
    return unsub;
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
        const [name, studioSetPC, masterLevel] = await Promise.all([
          svc.requestStudioSetName(),
          svc.requestStudioSetPC(),
          svc.requestMasterLevel(),
        ]);

        if (!isCurrent()) return;

        dev.setStudioSetName(name);
        dev.setStudioSetPc(studioSetPC);
        dev.applyMasterLevel(masterLevel);

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
          studioSetNames: new Map([[studioSetPC, name]]),
        }));
        syncFromRust();

        // Non-blocking loads: tone names
        for (let i = 0; i < 16; i++) {
          const msb = dev.readState().parts[i]?.tone_bank_msb;
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
          }).catch(() => {});
        }

        // Master EQ
        Promise.all([svc.requestMasterEq(), svc.requestMasterEqSwitch()]).then(
          ([eqData, enabled]) => {
            if (!isCurrent()) return;
            dev.applyMasterEqDump(eqData);
            dev.setMasterEqEnabled(enabled);
            syncFromRust();
          },
        ).catch(() => {});

        // Ext Part
        Promise.all([svc.requestExtPartLevel(), svc.requestExtPartMute()]).then(
          ([level, muted]) => {
            if (!isCurrent()) return;
            dev.applyExtLevel(level);
            dev.applyExtMuted(muted);
            syncFromRust();
          },
        ).catch(() => {});

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
        }).catch(() => {});

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
        }).catch(() => {});

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

  const changePartTone = useCallback(
    (part: number, msb: number, lsb: number, pc: number) => {
      if (!service) return;
      service.device.changePartTone(part, msb, lsb, pc);
      syncFromRust();
      // Re-read tone name after device loads the new tone.
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
    setState((prev) => ({ ...prev, selectedPart: part }));
  }, []);

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

  // --- UI-only state ---

  const toggleEqExpanded = useCallback(() => {
    setState((prev) => ({ ...prev, eqExpanded: !prev.eqExpanded }));
  }, []);

  const preview = useCallback(() => {
    if (!service) return;
    const current = stateRef.current;
    const part = current.parts[current.selectedPart];
    if (!part) return;
    const ch = part.receiveChannel;
    service.sendNoteOn(ch, 60, 100);
    setTimeout(() => {
      service.sendNoteOff(ch, 60);
    }, 500);
  }, [service]);

  return {
    state,
    setPartLevel,
    setPartPan,
    togglePartMute,
    changePartTone,
    setPartReceiveChannel,
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
    toggleEqExpanded,
    selectPart,
    switchStudioSet,
    loadStudioSetNames,
    preview,
  };
}
