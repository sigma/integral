/**
 * useMixer — React hook for bidirectional mixer state with the INTEGRA-7.
 *
 * On mount, requests the current state from the device via RQ1.
 * Exposes setters that update state optimistically and send DT1.
 * Listens for incoming DT1 to reflect changes made on the hardware.
 */

import { useEffect, useState, useCallback, useRef } from "react";
import {
  part_level_address,
  part_pan_address,
  part_mute_address,
} from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import {
  defaultMixerState,
  defaultPartState,
  type MixerState,
  type PartState,
} from "./types";

/** Duration to suppress incoming DT1 echoes after a local send (ms). */
const ECHO_SUPPRESS_MS = 150;

/** Parse the 0x29-byte mixer dump for a single part. */
function parsePartDump(data: Uint8Array): Partial<PartState> {
  return {
    receiveChannel: data[0x00],
    toneBankMsb: data[0x06],
    toneBankLsb: data[0x07],
    tonePC: data[0x08],
    level: data[0x09],
    pan: data[0x0a],
    muted: data[0x25] === 1,
  };
}

/** Check if two 4-byte addresses match. */
function addressEquals(a: Uint8Array, b: Uint8Array): boolean {
  return a[0] === b[0] && a[1] === b[1] && a[2] === b[2] && a[3] === b[3];
}

export interface UseMixerResult {
  state: MixerState;
  setPartLevel: (part: number, value: number) => void;
  setPartPan: (part: number, value: number) => void;
  togglePartMute: (part: number) => void;
  setMasterLevel: (value: number) => void;
  selectPart: (part: number) => void;
  switchStudioSet: (pc: number) => void;
  loadStudioSetNames: () => void;
  preview: () => void;
}

export function useMixer(service: IntegraService | null): UseMixerResult {
  const [state, setState] = useState<MixerState>(defaultMixerState);
  const stateRef = useRef(state);
  stateRef.current = state;
  const recentSends = useRef(new Map<string, number>());

  // Track a recently sent value to suppress echoes
  const markSent = useCallback((addrBytes: Uint8Array) => {
    const key = Array.from(addrBytes)
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");
    recentSends.current.set(key, Date.now());
  }, []);

  const isSuppressed = useCallback((addrBytes: Uint8Array): boolean => {
    const key = Array.from(addrBytes)
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");
    const ts = recentSends.current.get(key);
    if (ts && Date.now() - ts < ECHO_SUPPRESS_MS) return true;
    recentSends.current.delete(key);
    return false;
  }, []);

  const loadGenRef = useRef(0);

  const loadState = useCallback(
    async (svc: IntegraService) => {
      const gen = ++loadGenRef.current;
      const isCurrent = () => loadGenRef.current === gen;

      setState((prev) => ({ ...prev, loading: true }));

      try {
        console.log("[mixer] Loading studio set name, PC, and master level...");
        const [name, studioSetPC, masterLevel] = await Promise.all([
          svc.requestStudioSetName(),
          svc.requestStudioSetPC(),
          svc.requestMasterLevel(),
        ]);
        console.log("[mixer] Studio set:", name, "PC:", studioSetPC, "Master:", masterLevel);

        if (!isCurrent()) return;

        const parts: Partial<PartState>[] = [];
        for (let i = 0; i < 16; i++) {
          try {
            const dump = await svc.requestPartMixerState(i);
            const parsed = parsePartDump(dump);
            console.log(`[mixer] Part ${i + 1}:`, parsed);
            parts.push(parsed);
          } catch (e) {
            console.warn(`[mixer] Part ${i + 1} failed:`, e);
            parts.push({});
          }
        }

        if (!isCurrent()) return;

        const initialNames = new Map<number, string>();
        initialNames.set(studioSetPC, name);

        setState((prev) => ({
          ...prev,
          studioSetName: name,
          studioSetPC,
          masterLevel,
          parts: prev.parts.map((_p, i) => ({
            ...defaultPartState(),
            ...parts[i],
          })),
          loading: false,
          studioSetNames: initialNames,
        }));

        // Load tone names (non-blocking)
        for (let i = 0; i < 16; i++) {
          const msb = parts[i]?.toneBankMsb;
          if (msb === undefined) continue;
          svc.requestToneName(i, msb).then((toneName) => {
            if (!isCurrent() || !toneName) return;
            setState((prev) => updatePart(prev, i, { toneName }));
          });
        }

        // Studio Set catalog is loaded lazily when the dropdown is opened.
      } catch {
        setState((prev) => ({ ...prev, loading: false }));
      }
    },
    [],
  );

  // Load initial state from device
  useEffect(() => {
    if (!service) return;
    loadState(service);
  }, [service, loadState]);

  // Load studio set names on demand (triggered when dropdown opens)
  const catalogLoaded = useRef(false);
  const loadStudioSetNames = useCallback(
    async () => {
      if (!service || catalogLoaded.current) return;
      catalogLoaded.current = true;

      const names = await service.requestStudioSetNames();
      console.log("[mixer] Studio Set catalog:", names.size, "names");
      setState((prev) => ({ ...prev, studioSetNames: names }));
    },
    [service],
  );

  // Listen for incoming DT1 messages
  useEffect(() => {
    if (!service) return;

    const unsub = service.onDt1((address, data) => {
      if (isSuppressed(address)) return;

      // Match incoming address to a part parameter
      for (let part = 0; part < 16; part++) {
        const levelAddr = new Uint8Array(part_level_address(part));
        if (addressEquals(address, levelAddr) && data.length >= 1) {
          setState((prev) => updatePart(prev, part, { level: data[0] }));
          return;
        }
        const panAddr = new Uint8Array(part_pan_address(part));
        if (addressEquals(address, panAddr) && data.length >= 1) {
          setState((prev) => updatePart(prev, part, { pan: data[0] }));
          return;
        }
        const muteAddr = new Uint8Array(part_mute_address(part));
        if (addressEquals(address, muteAddr) && data.length >= 1) {
          setState((prev) =>
            updatePart(prev, part, { muted: data[0] === 1 }),
          );
          return;
        }
      }
    });

    return unsub;
  }, [service, isSuppressed]);

  // --- Setters ---

  const setPartLevel = useCallback(
    (part: number, value: number) => {
      setState((prev) => updatePart(prev, part, { level: value }));
      markSent(new Uint8Array(part_level_address(part)));
      service?.setPartLevel(part, value);
    },
    [service, markSent],
  );

  const setPartPan = useCallback(
    (part: number, value: number) => {
      setState((prev) => updatePart(prev, part, { pan: value }));
      markSent(new Uint8Array(part_pan_address(part)));
      service?.setPartPan(part, value);
    },
    [service, markSent],
  );

  const togglePartMute = useCallback(
    (part: number) => {
      const muted = !stateRef.current.parts[part]!.muted;
      setState((prev) => updatePart(prev, part, { muted }));
      markSent(new Uint8Array(part_mute_address(part)));
      service?.setPartMute(part, muted);
    },
    [service, markSent],
  );

  const setMasterLevel = useCallback(
    (value: number) => {
      setState((prev) => ({ ...prev, masterLevel: value }));
      service?.setMasterLevel(value);
    },
    [service],
  );

  const selectPart = useCallback((part: number) => {
    setState((prev) => ({ ...prev, selectedPart: part }));
  }, []);

  const switchStudioSet = useCallback(
    (pc: number) => {
      if (!service) return;
      service.switchStudioSet(pc);
      // Wait a moment for the device to load the new set, then reload state
      setTimeout(() => {
        loadState(service);
      }, 500);
    },
    [service, loadState],
  );

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
    setMasterLevel,
    selectPart,
    switchStudioSet,
    loadStudioSetNames,
    preview,
  };
}

/** Immutably update a single part in the mixer state. */
function updatePart(
  state: MixerState,
  part: number,
  update: Partial<PartState>,
): MixerState {
  return {
    ...state,
    parts: state.parts.map((p, i) => (i === part ? { ...p, ...update } : p)),
  };
}
