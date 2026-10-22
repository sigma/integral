import { useCallback, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import {
  DisplayKind,
  DisplayProp,
  PcmdCommonField,
  PcmdPartialField,
  ToneType,
  mfx_block_size,
  pcmd_common2_address,
  pcmd_common2_size,
  pcmd_common_address,
  pcmd_common_size,
  pcmd_common_spec,
  pcmd_mfx_address,
  pcmd_partial_address,
  pcmd_partial_count,
  pcmd_partial_size,
  pcmd_partial_spec,
  pcmd_wmt_spec,
  type PcmdWmtField,
  type WasmDisplaySlot,
} from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import type { ExpansionMarking } from "./WavePicker";
import { deviceSpec } from "./deviceSpec";
import {
  BreakpointDisplay,
  GridScreen,
  GridSection,
  slotField,
  slotOf,
  specBinding,
  type Binding,
  type SectionPlacement,
  type SlotValue,
} from "./layouts";
import {
  PCMD_AMP_ENV_LAYOUT,
  PCMD_FAMILIES,
  PCMD_FILTER_ENV_LAYOUT,
  PCMD_FILTER_LAYOUT,
  PCMD_KIT_LAYOUT,
  PCMD_MISC_LAYOUT,
  PCMD_OUTPUT_LAYOUT,
  PCMD_PITCH_ENV_LAYOUT,
  PCMD_SCREEN,
  PCMD_WMT_LAYOUT,
} from "./layouts/drum/pcmDrum";
import { MfxPanel } from "./MfxPanel";
import type { MfxState } from "./MfxPanel";
import { FilterScope } from "./synth-ui/FilterScope";
import { useMfxHandlers } from "./useMfxHandlers";
import { DrumKeyGrid } from "./DrumKeyGrid";
import { useDrumKey } from "./useDrumKey";
import { WaveDisplay } from "./WaveDisplay";
import {
  PCMD_COMMON_ENTRIES,
  PCMD_PARTIAL_ENTRIES,
  PCMD_WMT_ENTRIES,
  noteName,
  type PcmDrumCommon,
  type PcmDrumPartial,
} from "./pcm-drum/types";
import css from "./PcmDrumEditor.module.css";

/** The Partial Sections other than WMT, by placement id: they all bind the key's Partial. */
const PARTIAL_SECTIONS = {
  pitchEnv: PCMD_PITCH_ENV_LAYOUT,
  filter: PCMD_FILTER_LAYOUT,
  filterEnv: PCMD_FILTER_ENV_LAYOUT,
  ampEnv: PCMD_AMP_ENV_LAYOUT,
  misc: PCMD_MISC_LAYOUT,
  output: PCMD_OUTPUT_LAYOUT,
} as const;

/** The PCM Drum Partial field a Display slot reads. */
const partialField = (slot: WasmDisplaySlot): PcmdPartialField => slotField(slot, slot.pcmdPartial);

interface Props {
  partIndex: number;
  service: IntegraService;
  /** Loaded state of the Expansions behind SRX Wave Groups. */
  marking?: ExpansionMarking;
}

export function PcmDrumEditor({ partIndex, service, marking }: Props) {
  const keys = deviceSpec().pcm_drum_keys;
  const layers = deviceSpec().pcm_drum_wmt_count;
  const [common, setCommon] = useState<PcmDrumCommon | null>(null);
  const [mfx, setMfx] = useState<MfxState | null>(null);
  const [loading, setLoading] = useState(true);
  const [layer, setLayer] = useState(0);

  // Load common + common2 + MFX on mount / part change
  useEffect(() => {
    let cancelled = false;
    setLoading(true);

    async function load() {
      try {
        const commonData = await service.requestData(
          Array.from(pcmd_common_address(partIndex)),
          Array.from(pcmd_common_size()),
        );
        if (cancelled) return;
        const c = service.device.applyPcmdCommon(commonData) as PcmDrumCommon | null;
        if (c) setCommon(c);

        // Common2: read into core's copy of the kit (the editor shows none of it)
        const c2Data = await service.requestData(
          Array.from(pcmd_common2_address(partIndex)),
          Array.from(pcmd_common2_size()),
        );
        if (cancelled) return;
        service.device.applyPcmdCommon2(c2Data);

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
    return () => {
      cancelled = true;
    };
  }, [partIndex, service]);

  const fetchPartial = useCallback(
    async (k: number) =>
      service.device.applyPcmdPartial(
        await service.requestData(Array.from(pcmd_partial_address(partIndex, k)), Array.from(pcmd_partial_size())),
      ) as PcmDrumPartial | null,
    [partIndex, service],
  );
  const drumKey = useDrumKey(service, partIndex, keys, !loading, fetchPartial);
  const { key: selectedKey, note, current, update } = drumKey;

  const setKitLevel = useCallback(
    (value: number) => {
      service.device.setPcmdKitLevel(partIndex, value);
      setCommon((prev) => (prev ? { ...prev, kitLevel: value } : prev));
    },
    [partIndex, service],
  );

  const setPartialParam = useCallback(
    (field: PcmdPartialField, index: number, value: number) => {
      if (!current) return;
      service.device.setPcmdPartialField(partIndex, selectedKey, field, index, value);
      const { key } = PCMD_PARTIAL_ENTRIES[field];
      update((n) => {
        const cur = n[key];
        return { ...n, [key]: Array.isArray(cur) ? cur.map((c, i) => (i === index ? value : c)) : value };
      });
    },
    [current, partIndex, selectedKey, service, update],
  );

  const setWmtParam = useCallback(
    (field: PcmdWmtField, value: number) => {
      if (!current) return;
      service.device.setPcmdWmtParam(partIndex, selectedKey, layer, field, value);
      const { key } = PCMD_WMT_ENTRIES[field];
      update((n) => ({ ...n, wmt: n.wmt.map((w, i) => (i === layer ? { ...w, [key]: value } : w)) }));
    },
    [current, layer, partIndex, selectedKey, service, update],
  );

  const mfxHandlers = useMfxHandlers(service, partIndex, ToneType.PcmDrum, setMfx);

  // ---------------------------------------------------------------------------
  // Bindings for the Scope: the selected key, and its selected layer
  // ---------------------------------------------------------------------------

  const bindKit = (field: PcmdCommonField): Binding => {
    const { key, name } = PCMD_COMMON_ENTRIES[field];
    const spec = pcmd_common_spec(field);
    return specBinding(name, spec, common?.[key] ?? spec.defaultValue, setKitLevel);
  };

  /** Value `index` of a Partial field, for the selected key. */
  const partialValue = (field: PcmdPartialField, index: number): number => {
    const v = note?.[PCMD_PARTIAL_ENTRIES[field].key];
    const value = Array.isArray(v) ? v[index] : v;
    return value ?? pcmd_partial_spec(field).defaultValue;
  };

  const bindPartial = (field: PcmdPartialField, index = 0): Binding => {
    const spec = pcmd_partial_spec(field);
    const { name } = PCMD_PARTIAL_ENTRIES[field];
    const count = pcmd_partial_count(field);
    const full = count > 1 ? `${name} (${index + 1} of ${count})` : name;
    return specBinding(full, spec, partialValue(field, index), (v) => setPartialParam(field, index, v));
  };

  /** A WMT field of the selected layer. */
  const wmtValue = (field: PcmdWmtField): number =>
    note?.wmt[layer]?.[PCMD_WMT_ENTRIES[field].key] ?? pcmd_wmt_spec(field).defaultValue;

  const bindWmt = (field: PcmdWmtField): Binding =>
    specBinding(PCMD_WMT_ENTRIES[field].name, pcmd_wmt_spec(field), wmtValue(field), (v) => setWmtParam(field, v));

  const readPartial = (slot: WasmDisplaySlot): SlotValue => {
    const field = partialField(slot);
    const { min, max } = pcmd_partial_spec(field);
    return { value: partialValue(field, slot.index), min, max };
  };

  const renderPartialDisplay = (display: DisplayKind): ReactNode => {
    if (display === DisplayKind.FilterScope) {
      const at = (prop: DisplayProp) => {
        const slot = slotOf(display, ToneType.PcmDrum, prop);
        return { field: partialField(slot), value: readPartial(slot).value };
      };
      const type = at(DisplayProp.FilterType);
      return (
        <FilterScope
          className={css.fill}
          type={pcmd_partial_spec(type.field).formatValue(type.value)}
          cutoff={at(DisplayProp.Cutoff).value}
          resonance={at(DisplayProp.Resonance).value}
        />
      );
    }
    return <BreakpointDisplay kind={display} toneType={ToneType.PcmDrum} read={readPartial} />;
  };

  const renderWmtDisplay = (display: DisplayKind): ReactNode =>
    display === DisplayKind.WavePicker && note ? (
      <WaveDisplay
        toneType={ToneType.PcmDrum}
        fieldOf={(s) => s.pcmdWmt}
        value={wmtValue}
        onChange={setWmtParam}
        marking={marking}
      />
    ) : null;

  // Build key→outputAssign map and the set of silent keys (every WMT layer
  // switched off) from cached notes for key grid color coding.
  const { cache, cacheVersion } = drumKey;
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const [keyOutputAssigns, offKeys] = useMemo(() => {
    const m = new Map<number, number>();
    const off = new Set<number>();
    for (const [k, n] of cache.current) {
      m.set(k, n.outputAssign);
      if (n.wmt.every((w) => w.wmtSwitch !== 1)) off.add(k);
    }
    return [m, off] as const;
  }, [note, selectedKey, cacheVersion]);

  if (loading) {
    return <div className={css.loading}>Loading PCM drum kit data...</div>;
  }

  const isCurrent = () => current;
  const section = (p: SectionPlacement): ReactNode => {
    if (p.id === "mfx") return mfx && <MfxPanel mfx={mfx} {...mfxHandlers} />;
    if (p.id === "wmt") {
      return (
        <GridSection
          layout={PCMD_WMT_LAYOUT}
          bind={bindWmt}
          scope={{ key: selectedKey, layer }}
          isActive={isCurrent}
          renderDisplay={renderWmtDisplay}
          family={PCMD_FAMILIES.wmt}
        />
      );
    }
    if (!(p.id in PARTIAL_SECTIONS)) return null;
    return (
      <GridSection
        layout={PARTIAL_SECTIONS[p.id as keyof typeof PARTIAL_SECTIONS]}
        bind={bindPartial}
        scope={{ key: selectedKey }}
        isActive={isCurrent}
        renderDisplay={renderPartialDisplay}
        family={PCMD_FAMILIES[p.id]}
      />
    );
  };

  return (
    <div className={css.editor}>
      {common && <div className={css.kitName}>{common.kitName || "(unnamed kit)"}</div>}

      {/* The key grid and the layer tabs pick the Scope of the key Sections below them; the kit's level sits beside the grid. */}
      <DrumKeyGrid
        firstKey={keys.first}
        lastKey={keys.last}
        selectedKey={selectedKey}
        onSelect={drumKey.select}
        keyOutputAssigns={keyOutputAssigns}
        offKeys={offKeys}
        kit={<GridSection layout={PCMD_KIT_LAYOUT} bind={bindKit} isActive={() => common !== null} />}
      />
      <div className={css.selection}>
        <span className={css.partialName}>
          {noteName(selectedKey)}
          {note?.partialName ? `: ${note.partialName}` : ""}
        </span>
        <div className={css.wmtTabs} role="tablist" aria-label="WMT layer">
          {Array.from({ length: layers }, (_, i) => {
            const on = note?.wmt[i]?.wmtSwitch === 1;
            const tabClass = i === layer ? css.wmtTabActive : on ? css.wmtTab : css.wmtTabOff;
            return (
              <button
                key={i}
                type="button"
                role="tab"
                aria-selected={i === layer}
                className={tabClass}
                onClick={() => setLayer(i)}
              >
                WMT{i + 1}
              </button>
            );
          })}
        </div>
      </div>

      {/* The key Sections, then the MFX they all feed */}
      <GridScreen layout={PCMD_SCREEN}>{section}</GridScreen>
    </div>
  );
}
