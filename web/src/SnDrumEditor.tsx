import { useCallback, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import {
  SndCommonField,
  SndNoteField,
  ToneType,
  mfx_block_size,
  snd_common_address,
  snd_common_size,
  snd_common_spec,
  snd_mfx_address,
  snd_note_address,
  snd_note_size,
  snd_note_spec,
  sn_drum_inst_supports_variation,
} from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import { deviceSpec } from "./deviceSpec";
import { switchOptions } from "./fieldSpec";
import { GridScreen, GridSection, loadedRelevance, specBinding, type Binding, type SectionPlacement } from "./layouts";
import { SND_FAMILIES, SND_KIT_LAYOUT, SND_OUTPUT_LAYOUT, SND_SCREEN, SND_TONE_LAYOUT } from "./layouts/drum/snDrum";
import { MfxPanel } from "./MfxPanel";
import type { MfxState } from "./MfxPanel";
import { useMfxHandlers } from "./useMfxHandlers";
import { DrumKeyGrid } from "./DrumKeyGrid";
import { useDrumKey } from "./useDrumKey";
import { SND_COMMON_ENTRIES, SND_NOTE_ENTRIES, type SnDrumCommon, type SnDrumNote } from "./sn-drum/types";
import css from "./SnDrumEditor.module.css";

/**
 * The Variations a key's Inst supports. A value the Inst doesn't support
 * (left over from another Inst) stays listed, marked, so it isn't hidden.
 */
function variationOptions(note: { instNumber: number; variation: number }) {
  const ok = (v: number) => sn_drum_inst_supports_variation(note.instNumber, v);
  return switchOptions(snd_note_spec(SndNoteField.Variation))
    .filter((o) => ok(o.value) || o.value === note.variation)
    .map((o) => (ok(o.value) ? o : { ...o, label: `${o.label} (n/a)` }));
}

interface Props {
  partIndex: number;
  service: IntegraService;
}

export function SnDrumEditor({ partIndex, service }: Props) {
  const keys = deviceSpec().sn_drum_keys;
  const [common, setCommon] = useState<SnDrumCommon | null>(null);
  const [mfx, setMfx] = useState<MfxState | null>(null);
  const [loading, setLoading] = useState(true);

  // Load common + MFX on mount / part change
  useEffect(() => {
    let cancelled = false;
    setLoading(true);

    async function load() {
      try {
        const commonData = await service.requestData(
          Array.from(snd_common_address(partIndex)),
          Array.from(snd_common_size()),
        );
        if (cancelled) return;
        const c = service.device.applySndCommon(commonData) as SnDrumCommon | null;
        if (c) setCommon(c);

        const mfxData = await service.requestData(
          Array.from(snd_mfx_address(partIndex)),
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

  const fetchNote = useCallback(
    async (k: number) =>
      service.device.applySndNote(
        await service.requestData(Array.from(snd_note_address(partIndex, k)), Array.from(snd_note_size())),
      ) as SnDrumNote | null,
    [partIndex, service],
  );
  const drumKey = useDrumKey(service, partIndex, keys, !loading, fetchNote);
  const { key: selectedKey, note, current, update } = drumKey;

  const setCommonParam = useCallback(
    (field: SndCommonField, value: number) => {
      service.device.setSndCommonParam(partIndex, field, value);
      setCommon((prev) => (prev ? { ...prev, [SND_COMMON_ENTRIES[field].key]: value } : prev));
    },
    [partIndex, service],
  );

  const setNoteParam = useCallback(
    (field: SndNoteField, value: number) => {
      if (!current) return;
      service.device.setSndNoteParam(partIndex, selectedKey, field, value);
      update((n) => ({ ...n, [SND_NOTE_ENTRIES[field].key]: value }));
    },
    [current, partIndex, selectedKey, service, update],
  );

  const mfxHandlers = useMfxHandlers(service, partIndex, ToneType.SnDrum, setMfx);

  const bindCommon = (field: SndCommonField): Binding => {
    const { key, name } = SND_COMMON_ENTRIES[field];
    const spec = snd_common_spec(field);
    return specBinding(name, spec, common?.[key] ?? spec.defaultValue, (v) => setCommonParam(field, v));
  };

  const bindNote = (field: SndNoteField): Binding => {
    const { key, name } = SND_NOTE_ENTRIES[field];
    const spec = snd_note_spec(field);
    const value = note?.[key] ?? spec.defaultValue;
    const options = field === SndNoteField.Variation && note ? variationOptions(note) : undefined;
    return specBinding(name, spec, value, (v) => setNoteParam(field, v), options);
  };

  const { cache, cacheVersion } = drumKey;
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const keyOutputAssigns = useMemo(() => {
    const m = new Map<number, number>();
    for (const [k, n] of cache.current) m.set(k, n.outputAssign);
    return m;
  }, [note, selectedKey, cacheVersion]);

  if (loading) {
    return <div className={css.loading}>Loading SN-D drum kit data...</div>;
  }

  const keyRelevance = () => loadedRelevance(current);
  const scope = { key: selectedKey };
  const section = (p: SectionPlacement): ReactNode => {
    switch (p.id) {
      case "mfx":
        return mfx && <MfxPanel mfx={mfx} {...mfxHandlers} />;
      case "tone":
        return (
          <GridSection
            layout={SND_TONE_LAYOUT}
            bind={bindNote}
            scope={scope}
            relevance={keyRelevance}
            family={SND_FAMILIES.tone}
          />
        );
      case "output":
        return (
          <GridSection
            layout={SND_OUTPUT_LAYOUT}
            bind={bindNote}
            scope={scope}
            relevance={keyRelevance}
            family={SND_FAMILIES.output}
          />
        );
      default:
        return null;
    }
  };

  return (
    <div className={css.editor}>
      {common && <div className={css.kitName}>{common.kitName || "(unnamed kit)"}</div>}

      {/* The key grid picks the Scope of the key Sections below it; the kit's levels sit beside it. */}
      <DrumKeyGrid
        firstKey={keys.first}
        lastKey={keys.last}
        selectedKey={selectedKey}
        onSelect={drumKey.select}
        keyOutputAssigns={keyOutputAssigns}
        kit={<GridSection layout={SND_KIT_LAYOUT} bind={bindCommon} relevance={() => loadedRelevance(common !== null)} />}
      />

      {/* The key Sections, then the MFX they all feed */}
      <GridScreen layout={SND_SCREEN}>{section}</GridScreen>
    </div>
  );
}
