import { useCallback, useMemo } from "react";
import type { Dispatch, ReactNode, SetStateAction } from "react";
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
import { lockedKey, type DrumScope } from "./pluginPages";
import { useKitBlocks, type KitBlockRead } from "./useKitBlocks";
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
  /** What of the kit to edit: all of it by default; a Kit plugin's Binding narrows it. */
  scope?: DrumScope;
}

export function SnDrumEditor({ partIndex, service, scope }: Props) {
  const keys = deviceSpec().sn_drum_keys;
  const reads = useMemo<KitBlockRead[]>(
    () => [
      {
        address: () => Array.from(snd_common_address(partIndex)),
        size: () => Array.from(snd_common_size()),
        apply: (data) => service.device.applySndCommon(data),
      },
      {
        address: () => Array.from(snd_mfx_address(partIndex)),
        size: () => Array.from(mfx_block_size()),
        apply: (data) => service.device.applyMfxBlock(partIndex, data),
      },
    ],
    [partIndex, service],
  );
  // A block the kit read missed (a set reopening races the recall push) is
  // an error to retry, not a silent placeholder.
  const { blocks, loading, failed, retry, setValue } = useKitBlocks(service, reads, [partIndex, scope]);
  const common = (blocks[0]?.value ?? null) as SnDrumCommon | null;
  const mfx = (blocks[1]?.value ?? null) as MfxState | null;

  const fetchNote = useCallback(
    async (k: number) =>
      service.device.applySndNote(
        await service.requestData(Array.from(snd_note_address(partIndex, k)), Array.from(snd_note_size())),
      ) as SnDrumNote | null,
    [partIndex, service],
  );
  // A Kit plugin edits the kit or one key, and prefetches none.
  const drumKey = useDrumKey(service, partIndex, keys, !loading && scope === undefined, fetchNote, lockedKey(scope));
  const { key: selectedKey, note, current, update } = drumKey;

  const setCommonParam = useCallback(
    (field: SndCommonField, value: number) => {
      service.device.setSndCommonParam(partIndex, field, value);
      setValue(0, (prev) => {
        const c = prev as SnDrumCommon | null;
        return c ? { ...c, [SND_COMMON_ENTRIES[field].key]: value } : c;
      });
    },
    [partIndex, service, setValue],
  );

  const setNoteParam = useCallback(
    (field: SndNoteField, value: number) => {
      if (!current) return;
      service.device.setSndNoteParam(partIndex, selectedKey, field, value);
      update((n) => ({ ...n, [SND_NOTE_ENTRIES[field].key]: value }));
    },
    [current, partIndex, selectedKey, service, update],
  );

  const setMfx = useCallback<Dispatch<SetStateAction<MfxState | null>>>(
    (action) =>
      setValue(1, (prev) => {
        const m = prev as MfxState | null;
        return typeof action === "function" ? action(m) : action;
      }),
    [setValue],
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
  const keyScope = { key: selectedKey };
  const kitSection = (
    <GridSection layout={SND_KIT_LAYOUT} bind={bindCommon} relevance={() => loadedRelevance(common !== null)} />
  );
  const section = (p: SectionPlacement): ReactNode => {
    switch (p.id) {
      case "mfx":
        // The kit's, which a key does not own.
        return scope === undefined ? mfx && <MfxPanel mfx={mfx} {...mfxHandlers} /> : null;
      case "tone":
        return (
          <GridSection
            layout={SND_TONE_LAYOUT}
            bind={bindNote}
            scope={keyScope}
            relevance={keyRelevance}
            family={SND_FAMILIES.tone}
          />
        );
      case "output":
        return (
          <GridSection
            layout={SND_OUTPUT_LAYOUT}
            bind={bindNote}
            scope={keyScope}
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

      {/* A block the kit read missed: say so, and offer to read it again,
          rather than sitting on placeholders that look like device values. */}
      {failed && (
        <div className={css.readError} role="alert">
          Couldn't read the SN-D drum kit from the device.
          <button type="button" onClick={retry}>
            Retry
          </button>
        </div>
      )}

      {/* The key grid picks the Scope of the key Sections below it; the kit's levels sit beside it. */}
      {scope === undefined ? (
        <DrumKeyGrid
          firstKey={keys.first}
          lastKey={keys.last}
          selectedKey={selectedKey}
          onSelect={drumKey.select}
          keyOutputAssigns={keyOutputAssigns}
          kit={kitSection}
        />
      ) : (
        scope === "kit" && kitSection
      )}

      {/* The key Sections, then the MFX they all feed; a kit's own are just the MFX */}
      {scope === "kit" ? (
        mfx && <MfxPanel mfx={mfx} {...mfxHandlers} />
      ) : (
        <GridScreen layout={SND_SCREEN}>{section}</GridScreen>
      )}
    </div>
  );
}
