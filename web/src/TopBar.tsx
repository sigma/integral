import { StudioSetSelector } from "./StudioSetSelector";
import type { PartState } from "./types";
import css from "./TopBar.module.css";

interface Props {
  studioSetPC: number;
  studioSetNames: Map<number, string>;
  selectedPart: PartState;
  selectedPartIndex: number;
  previewActive: boolean;
  onStudioSetChange: (pc: number) => void;
  onLoadNames: () => void;
  onPreview: () => void;
  onToneClick: () => void;
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

export function TopBar({
  studioSetPC,
  studioSetNames,
  selectedPart,
  selectedPartIndex,
  onStudioSetChange,
  onLoadNames,
  onPreview,
  onToneClick,
  previewActive,
}: Props) {
  return (
    <div className={css.bar}>
      <StudioSetSelector
        currentPC={studioSetPC}
        names={studioSetNames}
        onSelect={onStudioSetChange}
        onLoadNames={onLoadNames}
      />
      <span className={css.toneInfo} onClick={onToneClick} style={{ cursor: "pointer" }}>
        Part {selectedPartIndex + 1} : {toneDisplay(selectedPart)}
      </span>
      <button
        className={`${css.previewButton} ${previewActive ? css.previewActive : ""}`}
        onClick={onPreview}
      >
        PREVIEW
      </button>
    </div>
  );
}
