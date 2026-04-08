import type { PartState } from "./types";
import css from "./TopBar.module.css";

interface Props {
  studioSetName: string;
  studioSetPC: number;
  selectedPart: PartState;
  selectedPartIndex: number;
  onStudioSetChange: (pc: number) => void;
  onPreview: () => void;
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
  studioSetName,
  studioSetPC,
  selectedPart,
  selectedPartIndex,
  onStudioSetChange,
  onPreview,
}: Props) {
  return (
    <div className={css.bar}>
      <div className={css.studioSetGroup}>
        <span className={css.studioSetLabel}>STUDIO SET</span>
        <select
          className={css.studioSetSelect}
          value={studioSetPC}
          onChange={(e) => onStudioSetChange(Number(e.target.value))}
        >
          {Array.from({ length: 64 }, (_, i) => (
            <option key={i} value={i}>
              {i === studioSetPC && studioSetName
                ? `${i + 1}: ${studioSetName}`
                : `${i + 1}`}
            </option>
          ))}
        </select>
      </div>
      <span className={css.toneInfo}>
        Part {selectedPartIndex + 1} : {toneDisplay(selectedPart)}
      </span>
      <button className={css.previewButton} onClick={onPreview}>
        PREVIEW
      </button>
    </div>
  );
}
