import { Fragment } from "react";
import { ToneType, is_editable_bank, tone_type_from_bank_msb } from "../pkg/integral_wasm.js";
import { SnSynthEditor } from "./SnSynthEditor";
import { SnAcousticEditor } from "./SnAcousticEditor";
import { SnDrumEditor } from "./SnDrumEditor";
import { PcmSynthEditor } from "./PcmSynthEditor";
import { PcmDrumEditor } from "./PcmDrumEditor";
import type { IntegraService } from "./integra";
import type { UseMixerResult } from "./useMixer";
import type { ExpansionMarking } from "./WavePicker";
import type { DrumScope } from "./pluginPages";
import css from "./ToneEditorPage.module.css";

interface Props {
  mixer: UseMixerResult;
  service: IntegraService;
  /** Loaded state of the Expansions behind SRX Wave Groups. */
  marking: ExpansionMarking;
  /** Back to the Mixer; absent where there is no Mixer page (a plugin's editor). */
  onBack?: () => void;
  /** The Part to edit, if not the mixer's selected one (a plugin's Binding). */
  part?: number;
  /** The only tone type to edit (a plugin's): the tone of another type is left alone. */
  only?: ToneType;
  /** What of a drum kit to edit (a Kit plugin's Binding); all of it by default. */
  drum?: DrumScope;
}

export function ToneEditorPage({ mixer, service, marking, onBack, part: bound, only, drum }: Props) {
  const partIndex = bound ?? mixer.state.selectedPart;
  const part = mixer.state.parts[partIndex]!;
  // Preset, User and SRX tones load into one of the five Temporary Tone
  // types; core knows which from the bank, and which banks can't be edited.
  const msb = part.toneBankMsb;
  const holds = is_editable_bank(msb) ? tone_type_from_bank_msb(msb) : undefined;
  // A plugin edits its own type only: another type is inert (ADR 0007).
  const toneType = only === undefined || holds === only ? holds : undefined;
  const mismatch = only !== undefined && holds !== undefined && holds !== only;

  return (
    <div className={css.page}>
      {onBack && (
        <div className={css.header}>
          <button className={css.backButton} onClick={onBack}>
            Back to Mixer
          </button>
        </div>
      )}
      {/* Keyed on tone identity so a new tone pick remounts and re-reads the editor. */}
      <Fragment key={`${part.toneBankMsb}-${part.toneBankLsb}-${part.tonePC}`}>
        {toneType === ToneType.PcmDrum ? (
          <PcmDrumEditor partIndex={partIndex} service={service} marking={marking} scope={drum} />
        ) : toneType === ToneType.PcmSynth ? (
          <PcmSynthEditor partIndex={partIndex} service={service} marking={marking} />
        ) : toneType === ToneType.SnSynth ? (
          <SnSynthEditor partIndex={partIndex} service={service} />
        ) : toneType === ToneType.SnAcoustic ? (
          <SnAcousticEditor partIndex={partIndex} service={service} />
        ) : toneType === ToneType.SnDrum ? (
          <SnDrumEditor partIndex={partIndex} service={service} scope={drum} />
        ) : (
          <div className={css.placeholder}>
            {mismatch
              ? "This Part holds another type of tone than this plugin edits, so the plugin stays inert."
              : tone_type_from_bank_msb(msb) === undefined
              ? `Bank MSB ${msb} isn't a tone bank, so there's no tone to edit.`
              : "The INTEGRA-7 can't edit GM2 or ExPCM tones. Pick a Preset, User or SRX tone to edit it."}
          </div>
        )}
      </Fragment>
    </div>
  );
}
