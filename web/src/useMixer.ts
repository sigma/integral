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
import { defaultMixerState, type MixerState, type PartState } from "./types";

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

  // Load initial state from device
  useEffect(() => {
    if (!service) return;

    let cancelled = false;

    async function loadState() {
      if (!service) return;

      try {
        const [name, masterLevel] = await Promise.all([
          service.requestStudioSetName(),
          service.requestMasterLevel(),
        ]);

        if (cancelled) return;

        // Load parts sequentially (throttle queue handles timing)
        const parts: Partial<PartState>[] = [];
        for (let i = 0; i < 16; i++) {
          try {
            const dump = await service.requestPartMixerState(i);
            parts.push(parsePartDump(dump));
          } catch {
            parts.push({});
          }
        }

        if (cancelled) return;

        setState((prev) => ({
          ...prev,
          studioSetName: name,
          masterLevel,
          parts: prev.parts.map((p, i) => ({ ...p, ...parts[i] })),
          loading: false,
        }));

        // Load tone names (non-blocking, after mixer state is loaded)
        for (let i = 0; i < 16; i++) {
          const partData = parts[i];
          const msb = partData?.toneBankMsb;
          if (msb === undefined) continue;
          service.requestToneName(i, msb).then((toneName) => {
            if (cancelled || !toneName) return;
            setState((prev) => updatePart(prev, i, { toneName }));
          });
        }
      } catch {
        if (!cancelled) {
          setState((prev) => ({ ...prev, loading: false }));
        }
      }
    }

    loadState();
    return () => {
      cancelled = true;
    };
  }, [service]);

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
