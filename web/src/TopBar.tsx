import { useState, useCallback, useMemo } from "react";
import { StudioSetSelector } from "./StudioSetSelector";
import { PartSelector } from "./PartSelector";
import { ToneSelector } from "./ToneSelector";
import { ToneCatalog } from "./toneCatalog";
import { SaveDialog, type SaveSlot } from "./SaveDialog";
import { toneSlotNumber, userToneBank, type ToneBank } from "./toneBanks";
import { useBankEntries } from "./useBankEntries";
import type { IntegraService } from "./integra";
import type { UseMixerResult } from "./useMixer";
import type { PartState } from "./types";
import css from "./TopBar.module.css";

interface Props {
  mixer: UseMixerResult;
  service: IntegraService;
}

function toneDisplay(part: PartState): string {
  if (part.toneName) {
    return `${toneTypeLabel(part.toneBankMsb)} : ${part.tonePC + 1} : ${part.toneName}`;
  }
  return `Bank ${part.toneBankMsb}-${part.toneBankLsb} : PC ${part.tonePC + 1}`;
}

function toneTypeLabel(bankMsb: number): string {
  switch (bankMsb) {
    case 87:
      return "PCM Synth";
    case 89:
      return "SN Acoustic";
    case 95:
      return "SN Synth";
    case 86:
      return "PCM Drum";
    case 88:
      return "SN Drum";
    case 93:
      return "Expansion PCM";
    case 121:
      return "GM2";
    default:
      return `Bank ${bankMsb}`;
  }
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
  onClose: () => void;
}

/** Save the part's tone into a slot of its type's user bank. */
function ToneSaveDialog({
  service,
  catalog,
  bank,
  part,
  toneName,
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
    async (slot: ToneSlot) => {
      await service.saveToneToUser(part, slot.msb, slot.lsb, slot.pc);
      catalog.refresh(bank);
    },
    [service, catalog, bank, part],
  );
  return (
    <SaveDialog
      title={`Save Tone — Part ${part + 1}`}
      sourceName={toneName}
      slots={slots}
      loading={loading}
      onSave={handleSave}
      onClose={onClose}
    />
  );
}

export function TopBar({ mixer, service }: Props) {
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
  const { loadStudioSetNames } = mixer;
  const openStudioSetSave = useCallback(async () => {
    setStudioSetSave({ name: state.studioSetName });
    setStudioSetLoading(true);
    await loadStudioSetNames();
    setStudioSetLoading(false);
  }, [loadStudioSetNames, state.studioSetName]);
  const handleStudioSetSave = useCallback(
    async (slot: StudioSetSlot) => {
      await service.saveStudioSetToUser(slot.pc);
      await loadStudioSetNames(true);
    },
    [service, loadStudioSetNames],
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
          onSelect={handleToneSelect}
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
          onClose={() => setToneSave(null)}
        />
      )}
      {studioSetSave && (
        <SaveDialog
          title="Save Studio Set"
          sourceName={studioSetSave.name}
          slots={studioSetSlots}
          loading={studioSetLoading}
          onSave={handleStudioSetSave}
          onClose={() => setStudioSetSave(null)}
        />
      )}
    </>
  );
}
