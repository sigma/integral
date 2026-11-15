/**
 * The controls at the head of the navigation row's Studio Set, Part and Tone
 * groups (`NavRow`): each scope's selector, then what acts on that scope.
 */

import { useState, useCallback, useMemo } from "react";
import { MixerField } from "../pkg/integral_wasm.js";
import { paramFormatter } from "./paramDefaults";
import type { MixerState } from "./types";
import { StudioSetSelector } from "./StudioSetSelector";
import { PartSelector } from "./PartSelector";
import { ToneSelector } from "./ToneSelector";
import type { ToneCatalog } from "./toneCatalog";
import { SaveDialog } from "./SaveDialog";
import { toneDisplay, userToneBank, type ToneBank } from "./toneBanks";
import { studioSetUserSlots, toneUserSlots, type UserSlot } from "./userSlots";
import { studioSetType } from "./librarian";
import { useBankEntries } from "./useBankEntries";
import { deviceSpec } from "./deviceSpec";
import type { IntegraService } from "./integra";
import type { UseMixerResult } from "./useMixer";
import type { UseExpansionsResult } from "./useExpansions";
import type { Expansion } from "./expansionCatalog";
import { NotLoadedMarker } from "./NotLoadedMarker";
import { InitItem } from "./InitFlow";
import css from "./ScopeControls.module.css";

/** The Studio Set group's controls: the selector, SAVE and Init…. */
export function StudioSetControls({
  mixer,
  service,
}: {
  mixer: UseMixerResult;
  service: IntegraService;
}) {
  const { state, loadStudioSetNames, renameStudioSet } = mixer;
  // The save's name is snapshotted on open, so a Studio Set change arriving
  // while the dialog is up can't rename what is saved.
  const [save, setSave] = useState<{ name: string } | null>(null);
  const [loading, setLoading] = useState(false);
  const slots = useMemo(
    () => studioSetUserSlots(state.studioSetNames, studioSetType().bankMsb),
    [state.studioSetNames],
  );
  const openSave = useCallback(async () => {
    setSave({ name: state.studioSetName });
    setLoading(true);
    await loadStudioSetNames();
    setLoading(false);
  }, [loadStudioSetNames, state.studioSetName]);
  const handleSave = useCallback(
    async (slot: UserSlot, name: string) => {
      // Queued ahead of the save request, so the device stores the new name.
      if (name !== save?.name) renameStudioSet(name);
      await service.saveStudioSetToUser(slot.pc);
      await loadStudioSetNames(true);
    },
    [service, loadStudioSetNames, renameStudioSet, save],
  );
  return (
    <>
      <StudioSetSelector
        currentPC={state.studioSetPC}
        names={state.studioSetNames}
        onSelect={mixer.switchStudioSet}
        onLoadNames={mixer.loadStudioSetNames}
      />
      <button
        className={css.saveButton}
        onClick={openSave}
        title="Save the studio set to a user slot"
      >
        SAVE
      </button>
      <InitItem scope={{ kind: "studio-set" }} className={css.initButton} />
      {save && (
        <SaveDialog
          title="Save Studio Set"
          sourceName={save.name}
          maxNameLength={deviceSpec().studio_set_name_length}
          slots={slots}
          loading={loading}
          onSave={handleSave}
          onClose={() => setSave(null)}
        />
      )}
    </>
  );
}

/** A receive channel as core shows it ("1" for channel 0). */
const channelText = paramFormatter(MixerField.PartReceiveChannel);

/**
 * The selected Part's receive channel as core shows it; none before the
 * mixer is read, while the Parts hold defaults.
 */
export function selectedPartChannel(state: MixerState): string | undefined {
  const channel = state.loading ? undefined : state.parts[state.selectedPart]?.receiveChannel;
  return channel === undefined ? undefined : channelText(channel);
}

/** The Part settings drawer's toggle in the Part group: whether it is open, and what opens or closes it. */
export interface PartSettingsToggle {
  open: boolean;
  onToggle: () => void;
}

/** The Part group's controls: the selector, the Part's receive channel, ⚙ (the Part settings drawer) and PREVIEW. */
export function PartControls({ mixer, settings }: { mixer: UseMixerResult; settings?: PartSettingsToggle }) {
  const { state } = mixer;
  const part = state.selectedPart;
  const channel = selectedPartChannel(state);
  return (
    <>
      <PartSelector selectedPart={part} onSelect={mixer.selectPart} />
      {channel !== undefined && (
        <span className={css.channel} title="The Part's receive channel">
          ch {channel}
        </span>
      )}
      {settings && (
        <button
          className={`${css.settingsButton} ${settings.open ? css.settingsOpen : ""}`}
          onClick={settings.onToggle}
          aria-pressed={settings.open}
          aria-label="Part settings"
          title={`${settings.open ? "Close" : "Open"} Part ${part + 1}'s settings`}
        >
          ⚙
        </button>
      )}
      <button
        className={`${css.previewButton} ${state.previewPart > 0 ? css.previewActive : ""}`}
        onClick={mixer.preview}
        title={`Play Part ${part + 1}'s preview phrase`}
      >
        PREVIEW
      </button>
    </>
  );
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
  const slots = useMemo(() => toneUserSlots(entries, bank), [entries, bank]);
  const handleSave = useCallback(
    async (slot: UserSlot, name: string) => {
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

interface ToneControlsProps {
  mixer: UseMixerResult;
  service: IntegraService;
  /** The tone catalog, shared with the other pages. */
  catalog: ToneCatalog;
  expansions: UseExpansionsResult;
  /** Open the Expansions page with `expansion` staged. */
  onLoadExpansion: (expansion: Expansion) => void;
}

/** The Tone group's controls: the selected Part's tone selector and SAVE. */
export function ToneControls({
  mixer,
  service,
  catalog,
  expansions,
  onLoadExpansion,
}: ToneControlsProps) {
  const { state } = mixer;
  const selectedPart = state.parts[state.selectedPart]!;
  const [selectorOpen, setSelectorOpen] = useState(false);
  // The save target is snapshotted on open, so a tone change arriving while
  // the dialog is up can't redirect the save.
  const [save, setSave] = useState<{
    part: number;
    bank: ToneBank;
    toneName: string;
  } | null>(null);
  const unloadedExpansion = expansions.unloadedExpansionOf(
    selectedPart.toneBankMsb,
    selectedPart.toneBankLsb,
  );
  const userBank = useMemo(
    () => userToneBank(selectedPart.toneBankMsb),
    [selectedPart.toneBankMsb],
  );

  const handleSelect = useCallback(
    (msb: number, lsb: number, pc: number) => {
      mixer.changePartTone(state.selectedPart, msb, lsb, pc);
      setSelectorOpen(false);
    },
    [mixer, state.selectedPart],
  );

  return (
    <>
      <button
        className={css.toneButton}
        onClick={() => setSelectorOpen(true)}
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
          setSave({
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
      {selectorOpen && (
        <ToneSelector
          partIndex={state.selectedPart}
          currentMsb={selectedPart.toneBankMsb}
          currentLsb={selectedPart.toneBankLsb}
          currentPC={selectedPart.tonePC}
          catalog={catalog}
          expansions={expansions}
          onSelect={handleSelect}
          onLoadExpansion={(expansion) => {
            setSelectorOpen(false);
            onLoadExpansion(expansion);
          }}
          onClose={() => setSelectorOpen(false)}
        />
      )}
      {save && (
        <ToneSaveDialog
          service={service}
          catalog={catalog}
          bank={save.bank}
          part={save.part}
          toneName={save.toneName}
          onRename={mixer.renameTone}
          onClose={() => setSave(null)}
        />
      )}
    </>
  );
}
