import { SnSynthEditor } from "./SnSynthEditor";
import type { IntegraService } from "./integra";
import type { UseMixerResult } from "./useMixer";
import css from "./ToneEditorPage.module.css";

// Bank MSB to tone type mapping
const TONE_TYPE_LABELS: Record<number, string> = {
  86: "PCM Drum",
  87: "PCM Synth",
  88: "SN-D (Drums)",
  89: "SN-A (Acoustic)",
  95: "SN-S (Synth)",
  96: "ExPCM",
  97: "ExPCM",
};

interface Props {
  mixer: UseMixerResult;
  service: IntegraService;
  onBack: () => void;
}

export function ToneEditorPage({ mixer, service, onBack }: Props) {
  const partIndex = mixer.state.selectedPart;
  const part = mixer.state.parts[partIndex]!;
  const bankMsb = part.toneBankMsb;
  const toneType = TONE_TYPE_LABELS[bankMsb] ?? `Unknown (MSB ${bankMsb})`;
  const isSns = bankMsb === 95;

  return (
    <div className={css.page}>
      <div className={css.header}>
        <button className={css.backButton} onClick={onBack}>
          Back to Mixer
        </button>
        <span className={css.partLabel}>Part {partIndex + 1}</span>
        <span className={css.toneType}>{toneType}</span>
        <span className={css.toneLabel}>
          {part.toneName || `${bankMsb}-${part.toneBankLsb}-${part.tonePC + 1}`}
        </span>
      </div>
      {isSns ? (
        <SnSynthEditor partIndex={partIndex} service={service} />
      ) : (
        <div className={css.placeholder}>
          Tone editor for {toneType} is not yet implemented.
        </div>
      )}
    </div>
  );
}
