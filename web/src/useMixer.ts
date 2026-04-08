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
  type EqState,
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
    chorusSend: data[0x27],
    reverbSend: data[0x28],
    muted: data[0x25] === 1,
  };
}

/** Parse 8-byte Part EQ dump. */
function parsePartEqDump(data: Uint8Array): EqState {
  return {
    enabled: data[0] === 1,
    lowFreq: data[1] ?? 0,
    lowGain: data[2] ?? 15,
    midFreq: data[3] ?? 0,
    midGain: data[4] ?? 15,
    midQ: data[5] ?? 0,
    highFreq: data[6] ?? 0,
    highGain: data[7] ?? 15,
  };
}

/** Parse 7-byte Master EQ dump (no switch byte). */
function parseMasterEqDump(data: Uint8Array): Omit<EqState, "enabled"> {
  return {
    lowFreq: data[0] ?? 0,
    lowGain: data[1] ?? 15,
    midFreq: data[2] ?? 0,
    midGain: data[3] ?? 15,
    midQ: data[4] ?? 0,
    highFreq: data[5] ?? 0,
    highGain: data[6] ?? 15,
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

        // Load Part EQ state (non-blocking)
        for (let i = 0; i < 16; i++) {
          svc.requestPartEq(i).then((eqData) => {
            if (!isCurrent()) return;
            setState((prev) => updatePart(prev, i, { eq: parsePartEqDump(eqData) }));
          }).catch(() => {});
        }

        // Load Master EQ (non-blocking)
        Promise.all([svc.requestMasterEq(), svc.requestMasterEqSwitch()]).then(
          ([eqData, enabled]) => {
            if (!isCurrent()) return;
            setState((prev) => ({
              ...prev,
              masterEq: { ...parseMasterEqDump(eqData), enabled },
            }));
          },
        ).catch(() => {});

        // Load Ext Part state (non-blocking)
        Promise.all([svc.requestExtPartLevel(), svc.requestExtPartMute()]).then(
          ([level, muted]) => {
            if (!isCurrent()) return;
            setState((prev) => ({ ...prev, extLevel: level, extMuted: muted }));
          },
        ).catch(() => {});

        // Load Chorus (FX1) state (non-blocking)
        Promise.all([
          svc.requestChorusCore(),
          svc.requestChorusSwitch(),
          svc.requestChorusParams(),
        ]).then(([core, enabled, params]) => {
          if (!isCurrent()) return;
          setState((prev) => ({
            ...prev,
            chorus: {
              enabled,
              type: core[0] ?? 0,
              level: core[1] ?? 0,
              output: core[3] ?? 0,
              params,
            },
          }));
        }).catch(() => {});

        // Load Reverb (FX2) state (non-blocking)
        Promise.all([
          svc.requestReverbCore(),
          svc.requestReverbSwitch(),
          svc.requestReverbParams(),
        ]).then(([core, enabled, params]) => {
          if (!isCurrent()) return;
          setState((prev) => ({
            ...prev,
            reverb: {
              enabled,
              type: core[0] ?? 0,
              level: core[1] ?? 0,
              output: core[2] ?? 0,
              params,
            },
          }));
        }).catch(() => {});

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

  const changePartTone = useCallback(
    (part: number, msb: number, lsb: number, pc: number) => {
      if (!service) return;
      // Update state optimistically
      setState((prev) =>
        updatePart(prev, part, {
          toneBankMsb: msb,
          toneBankLsb: lsb,
          tonePC: pc,
          toneName: "", // clear until re-read
        }),
      );
      service.setPartTone(part, msb, lsb, pc);
      // Re-read tone name after the device loads the new tone
      setTimeout(() => {
        service.requestToneName(part, msb).then((toneName) => {
          if (toneName) {
            setState((prev) => updatePart(prev, part, { toneName }));
          }
        });
      }, 300);
    },
    [service],
  );

  const setPartReceiveChannel = useCallback(
    (part: number, channel: number) => {
      setState((prev) => updatePart(prev, part, { receiveChannel: channel }));
      service?.setPartReceiveChannel(part, channel);
    },
    [service],
  );

  const setPartChorusSend = useCallback(
    (part: number, value: number) => {
      setState((prev) => updatePart(prev, part, { chorusSend: value }));
      service?.setPartChorusSend(part, value);
    },
    [service],
  );

  const setPartReverbSend = useCallback(
    (part: number, value: number) => {
      setState((prev) => updatePart(prev, part, { reverbSend: value }));
      service?.setPartReverbSend(part, value);
    },
    [service],
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

  // --- EQ setters ---

  const setPartEqParam = useCallback(
    (part: number, paramOffset: number, value: number) => {
      setState((prev) => {
        const eq = { ...prev.parts[part]!.eq };
        switch (paramOffset) {
          case 0: eq.enabled = value === 1; break;
          case 1: eq.lowFreq = value; break;
          case 2: eq.lowGain = value; break;
          case 3: eq.midFreq = value; break;
          case 4: eq.midGain = value; break;
          case 5: eq.midQ = value; break;
          case 6: eq.highFreq = value; break;
          case 7: eq.highGain = value; break;
        }
        return updatePart(prev, part, { eq });
      });
      service?.setPartEqParam(part, paramOffset, value);
    },
    [service],
  );

  const togglePartEqSwitch = useCallback(
    (part: number) => {
      const enabled = !stateRef.current.parts[part]!.eq.enabled;
      setPartEqParam(part, 0, enabled ? 1 : 0);
    },
    [setPartEqParam],
  );

  const setMasterEqParam = useCallback(
    (paramOffset: number, value: number) => {
      setState((prev) => {
        const eq = { ...prev.masterEq };
        switch (paramOffset) {
          case 0: eq.lowFreq = value; break;
          case 1: eq.lowGain = value; break;
          case 2: eq.midFreq = value; break;
          case 3: eq.midGain = value; break;
          case 4: eq.midQ = value; break;
          case 5: eq.highFreq = value; break;
          case 6: eq.highGain = value; break;
        }
        return { ...prev, masterEq: eq };
      });
      service?.setMasterEqParam(paramOffset, value);
    },
    [service],
  );

  const toggleMasterEqSwitch = useCallback(() => {
    const enabled = !stateRef.current.masterEq.enabled;
    setState((prev) => ({
      ...prev,
      masterEq: { ...prev.masterEq, enabled },
    }));
    service?.setMasterEqSwitch(enabled);
  }, [service]);

  // --- FX setters ---

  const setChorusParam = useCallback(
    (offset: number, value: number) => {
      setState((prev) => {
        const chorus = { ...prev.chorus };
        if (offset === 0) chorus.type = value;
        else if (offset === 1) chorus.level = value;
        else if (offset === 3) chorus.output = value;
        return { ...prev, chorus };
      });
      service?.setChorusParam(offset, value);
    },
    [service],
  );

  const setChorusNibParam = useCallback(
    (paramIndex: number, value: number) => {
      setState((prev) => {
        const params = [...prev.chorus.params];
        params[paramIndex] = value;
        return { ...prev, chorus: { ...prev.chorus, params } };
      });
      service?.setChorusNibParam(paramIndex, value);
    },
    [service],
  );

  const toggleChorusSwitch = useCallback(() => {
    const enabled = !stateRef.current.chorus.enabled;
    setState((prev) => ({ ...prev, chorus: { ...prev.chorus, enabled } }));
    service?.setChorusSwitch(enabled);
  }, [service]);

  const setReverbParam = useCallback(
    (offset: number, value: number) => {
      setState((prev) => {
        const reverb = { ...prev.reverb };
        if (offset === 0) reverb.type = value;
        else if (offset === 1) reverb.level = value;
        else if (offset === 2) reverb.output = value;
        return { ...prev, reverb };
      });
      service?.setReverbParam(offset, value);
    },
    [service],
  );

  const setReverbNibParam = useCallback(
    (paramIndex: number, value: number) => {
      setState((prev) => {
        const params = [...prev.reverb.params];
        params[paramIndex] = value;
        return { ...prev, reverb: { ...prev.reverb, params } };
      });
      service?.setReverbNibParam(paramIndex, value);
    },
    [service],
  );

  const toggleReverbSwitch = useCallback(() => {
    const enabled = !stateRef.current.reverb.enabled;
    setState((prev) => ({ ...prev, reverb: { ...prev.reverb, enabled } }));
    service?.setReverbSwitch(enabled);
  }, [service]);

  const setExtLevel = useCallback(
    (value: number) => {
      setState((prev) => ({ ...prev, extLevel: value }));
      service?.setExtPartLevel(value);
    },
    [service],
  );

  const toggleExtMute = useCallback(() => {
    const muted = !stateRef.current.extMuted;
    setState((prev) => ({ ...prev, extMuted: muted }));
    service?.setExtPartMute(muted);
  }, [service]);

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
