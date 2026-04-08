import type { PartState } from "./types";
import css from "./TopBar.module.css";

interface Props {
  studioSetName: string;
  selectedPart: PartState;
  selectedPartIndex: number;
  onPreview: () => void;
}

function toneDisplay(part: PartState): string {
  return `Bank ${part.toneBankMsb}-${part.toneBankLsb} : PC ${part.tonePC + 1}`;
}

export function TopBar({
  studioSetName,
  selectedPart,
  selectedPartIndex,
  onPreview,
}: Props) {
  return (
    <div className={css.bar}>
      <span className={css.studioSet}>
        {studioSetName || "---"}
      </span>
      <span className={css.toneInfo}>
        Part {selectedPartIndex + 1} : {toneDisplay(selectedPart)}
      </span>
      <button className={css.previewButton} onClick={onPreview}>
        PREVIEW
      </button>
    </div>
  );
}
