import { useState, useCallback, useMemo } from "react";
import { StudioSetSelector } from "./StudioSetSelector";
import { PartSelector } from "./PartSelector";
import { ToneSelector } from "./ToneSelector";
import { ToneCatalog } from "./toneCatalog";
import { SaveDialog, type SaveSlot } from "./SaveDialog";
import {
  findToneBank,
  toneSlotNumber,
  userToneBank,
  type ToneBank,
} from "./toneBanks";
import { useBankEntries } from "./useBankEntries";
import { deviceSpec } from "./deviceSpec";
import type { IntegraService } from "./integra";
import type { UseMixerResult } from "./useMixer";
import type { UseExpansionsResult } from "./useExpansions";
import type { Expansion } from "./expansionCatalog";
import { NotLoadedMarker } from "./NotLoadedMarker";
import type { PartState } from "./types";
import css from "./TopBar.module.css";

interface Props {
  mixer: UseMixerResult;
  service: IntegraService;
  expansions: UseExpansionsResult;
  /** Open the Expansions page with `expansion` staged. */
  onLoadExpansion: (expansion: Expansion) => void;
}

function toneDisplay(part: PartState): string {
  if (part.toneName) {
    return `${toneSourceLabel(part)} : ${part.tonePC + 1} : ${part.toneName}`;
  }
  return `Bank ${part.toneBankMsb}-${part.toneBankLsb} : PC ${part.tonePC + 1}`;
}

/** The part's tone bank group, or its Expansion for Expansion banks. */
function toneSourceLabel(part: PartState): string {
  const bank = findToneBank(part.toneBankMsb, part.toneBankLsb);
  if (!bank) return `Bank ${part.toneBankMsb}`;
  return bank.expansion?.name ?? bank.group;
}

interface ToneSlot extends SaveSlot {
  msb: number;
  lsb: number;
  pc: number;
}

interface StudioSetSlot extends SaveSlot {
  pc: number;
}

interface ToneSaveDialogProps {
  service: IntegraService;
  catalog: ToneCatalog;
  bank: ToneBank;
  part: number;
  toneName: string;
  onRename: (part: number, name: string) => void;
  onClose: () => void;
}

/** Save the part's tone into a slot of its type's user bank. */
function ToneSaveDialog({
  service,
  catalog,
  bank,
  part,
  toneName,
  onRename,
  onClose,
}: ToneSaveDialogProps) {
  const { entries, loading } = useBankEntries(catalog, bank);
  const slots: ToneSlot[] = useMemo(
    () =>
      entries.map((e) => ({
        key: `${e.lsb}:${e.pc}`,
        number: toneSlotNumber(bank, e.lsb, e.pc),
        name: e.name,
        msb: e.msb,
        lsb: e.lsb,
        pc: e.pc,
      })),
    [entries, bank],
  );
  const handleSave = useCallback(
    async (slot: ToneSlot, name: string) => {
      // Queued ahead of the save request, so the device stores the new name.
      if (name !== toneName) onRename(part, name);
      await service.saveToneToUser(part, slot.msb, slot.lsb, slot.pc);
      catalog.refresh(bank);
    },
    [service, catalog, bank, part, toneName, onRename],
  );
  return (
    <SaveDialog
      title={`Save Tone — Part ${part + 1}`}
      sourceName={toneName}
      maxNameLength={deviceSpec().tone_name_length}
      slots={slots}
      loading={loading}
      onSave={handleSave}
      onClose={onClose}
    />
  );
}

export function TopBar({ mixer, service, expansions, onLoadExpansion }: Props) {
  const { state } = mixer;
  const selectedPart = state.parts[state.selectedPart]!;
  const [toneSelectorOpen, setToneSelectorOpen] = useState(false);
  // Save targets are snapshotted on open, so a tone or studio set change
  // arriving while the dialog is up can't redirect the save.
  const [toneSave, setToneSave] = useState<{
    part: number;
    bank: ToneBank;
    toneName: string;
  } | null>(null);
  const [studioSetSave, setStudioSetSave] = useState<{ name: string } | null>(
    null,
  );
  const [studioSetLoading, setStudioSetLoading] = useState(false);
  const unloadedExpansion = expansions.unloadedExpansionOf(
    selectedPart.toneBankMsb,
    selectedPart.toneBankLsb,
  );
  const catalog = useMemo(() => new ToneCatalog(service), [service]);
  const userBank = useMemo(
    () => userToneBank(selectedPart.toneBankMsb),
    [selectedPart.toneBankMsb],
  );

  const studioSetSlots: StudioSetSlot[] = useMemo(
    () =>
      [...state.studioSetNames.entries()]
        .sort(([a], [b]) => a - b)
        .map(([pc, name]) => ({ key: String(pc), number: String(pc + 1), name, pc })),
    [state.studioSetNames],
  );
  const { loadStudioSetNames, renameStudioSet } = mixer;
  const openStudioSetSave = useCallback(async () => {
    setStudioSetSave({ name: state.studioSetName });
    setStudioSetLoading(true);
    await loadStudioSetNames();
    setStudioSetLoading(false);
  }, [loadStudioSetNames, state.studioSetName]);
  const handleStudioSetSave = useCallback(
    async (slot: StudioSetSlot, name: string) => {
      // Queued ahead of the save request, so the device stores the new name.
      if (name !== studioSetSave?.name) renameStudioSet(name);
      await service.saveStudioSetToUser(slot.pc);
      await loadStudioSetNames(true);
    },
    [service, loadStudioSetNames, renameStudioSet, studioSetSave],
  );

  const handleToneSelect = useCallback(
    (msb: number, lsb: number, pc: number) => {
      mixer.changePartTone(state.selectedPart, msb, lsb, pc);
      setToneSelectorOpen(false);
    },
    [mixer, state.selectedPart],
  );

  return (
    <>
      <div className={css.toneInfo}>
        <span className={css.toneLabel}>TONE</span>
        <button
          className={css.toneButton}
          onClick={() => setToneSelectorOpen(true)}
          title="Choose tone"
        >
          <span className={css.toneName}>{toneDisplay(selectedPart)}</span>
          {unloadedExpansion && (
            <NotLoadedMarker expansion={unloadedExpansion} named />
          )}
          <span className={css.toneArrow}>&#x25BC;</span>
        </button>
        <button
          className={css.saveButton}
          onClick={() =>
            userBank &&
            setToneSave({
              part: state.selectedPart,
              bank: userBank,
              toneName: selectedPart.toneName,
            })
          }
          disabled={!userBank || !selectedPart.toneName}
          title="Save this part's tone to a user slot"
        >
          SAVE
        </button>
      </div>
      <div className={css.selectors}>
        <StudioSetSelector
          currentPC={state.studioSetPC}
          names={state.studioSetNames}
          onSelect={mixer.switchStudioSet}
          onLoadNames={mixer.loadStudioSetNames}
        />
        <button
          className={css.saveButton}
          onClick={openStudioSetSave}
          title="Save the studio set to a user slot"
        >
          SAVE
        </button>
        <PartSelector
          selectedPart={state.selectedPart}
          onSelect={mixer.selectPart}
        />
        <button
          className={`${css.previewButton} ${state.previewPart > 0 ? css.previewActive : ""}`}
          onClick={mixer.preview}
        >
          PREVIEW
        </button>
      </div>
      {toneSelectorOpen && (
        <ToneSelector
          partIndex={state.selectedPart}
          currentMsb={selectedPart.toneBankMsb}
          currentLsb={selectedPart.toneBankLsb}
          currentPC={selectedPart.tonePC}
          catalog={catalog}
          expansions={expansions}
          onSelect={handleToneSelect}
          onLoadExpansion={(expansion) => {
            setToneSelectorOpen(false);
            onLoadExpansion(expansion);
          }}
          onClose={() => setToneSelectorOpen(false)}
        />
      )}
      {toneSave && (
        <ToneSaveDialog
          service={service}
          catalog={catalog}
          bank={toneSave.bank}
          part={toneSave.part}
          toneName={toneSave.toneName}
          onRename={mixer.renameTone}
          onClose={() => setToneSave(null)}
        />
      )}
      {studioSetSave && (
        <SaveDialog
          title="Save Studio Set"
          sourceName={studioSetSave.name}
          maxNameLength={deviceSpec().studio_set_name_length}
          slots={studioSetSlots}
          loading={studioSetLoading}
          onSave={handleStudioSetSave}
          onClose={() => setStudioSetSave(null)}
        />
      )}
    </>
  );
}
