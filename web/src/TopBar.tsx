import { useState, useCallback, useMemo } from "react";
import { StudioSetSelector } from "./StudioSetSelector";
import { PartSelector } from "./PartSelector";
import { ToneSelector } from "./ToneSelector";
import { ToneCatalog } from "./toneCatalog";
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

export function TopBar({ mixer, service }: Props) {
  const { state } = mixer;
  const selectedPart = state.parts[state.selectedPart]!;
  const [toneSelectorOpen, setToneSelectorOpen] = useState(false);
  const catalog = useMemo(() => new ToneCatalog(service), [service]);

  const handleToneSelect = useCallback(
    (msb: number, lsb: number, pc: number) => {
      mixer.changePartTone(state.selectedPart, msb, lsb, pc);
      setToneSelectorOpen(false);
    },
    [mixer, state.selectedPart],
  );

  return (
    <div className={css.bar}>
      <StudioSetSelector
        currentPC={state.studioSetPC}
        names={state.studioSetNames}
        onSelect={mixer.switchStudioSet}
        onLoadNames={mixer.loadStudioSetNames}
      />
      <PartSelector selectedPart={state.selectedPart} onSelect={mixer.selectPart} />
      <span className={css.toneInfo} onClick={() => setToneSelectorOpen(true)}>
        {toneDisplay(selectedPart)}
      </span>
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
    </div>
  );
}
