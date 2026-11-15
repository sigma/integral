import { Fragment, useMemo } from "react";
import { ToneType, is_editable_bank, tone_type_from_bank_msb } from "../pkg/integral_wasm.js";
import { SnSynthEditor } from "./SnSynthEditor";
import { SnAcousticEditor } from "./SnAcousticEditor";
import { SnDrumEditor } from "./SnDrumEditor";
import { PcmSynthEditor } from "./PcmSynthEditor";
import { PcmDrumEditor } from "./PcmDrumEditor";
import { InitGuard, InitItem, useInit } from "./InitFlow";
import type { InitScope } from "./init";
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
  /** Re-read the editor when this changes (a plugin refresh that adopted the
   * device's values, #562); the editor remounts as a tone change remounts it. */
  reread?: number;
  /** Whether the editor offers its Init… items: a browser's does, a plugin's not (web first). */
  initItems?: boolean;
}

export function ToneEditorPage({
  mixer,
  service,
  marking,
  onBack,
  part: bound,
  only,
  drum,
  reread,
  initItems,
}: Props) {
  // Preset, User and SRX tones load into one of the five Temporary Tone
  // types; core knows which from the bank, and which banks can't be edited.
  const partIndex = bound ?? mixer.state.selectedPart;
  const part = mixer.state.parts[partIndex]!;
  const init = useInit();
  const toneArea: InitScope = { kind: "tone", part: partIndex };
  // A Tone, Partial, Part or Studio Set Init re-reads the editor the same way.
  const initRereads = init.rereads(partIndex);
  // A kit's selected key, kept while the Part holds the same tone: the
  // editor an Init remounts opens on the key it was on. Another tone or
  // Part opens on the default key again.
  const toneId = `${partIndex}-${part.toneBankMsb}-${part.toneBankLsb}-${part.tonePC}`;
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const drumKey = useMemo(() => ({ current: undefined as number | undefined }), [toneId]);
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
          {/* Only the web page has this header: a plugin's editor has no Init (web first). */}
          {toneType !== undefined && <InitItem scope={toneArea} className={css.initButton} />}
        </div>
      )}
      <InitGuard area={toneArea}>
        {/* Keyed on tone identity so a new tone pick remounts and re-reads the editor. */}
        <Fragment key={`${part.toneBankMsb}-${part.toneBankLsb}-${part.tonePC}-${reread ?? 0}-${initRereads}`}>
          {toneType === ToneType.PcmDrum ? (
            <PcmDrumEditor
              partIndex={partIndex}
              service={service}
              marking={marking}
              scope={drum}
              initItems={initItems}
              lastKey={drumKey}
            />
          ) : toneType === ToneType.PcmSynth ? (
            <PcmSynthEditor partIndex={partIndex} service={service} marking={marking} initItems={initItems} />
          ) : toneType === ToneType.SnSynth ? (
            <SnSynthEditor partIndex={partIndex} service={service} initItems={initItems} />
          ) : toneType === ToneType.SnAcoustic ? (
            <SnAcousticEditor partIndex={partIndex} service={service} />
          ) : toneType === ToneType.SnDrum ? (
            <SnDrumEditor partIndex={partIndex} service={service} scope={drum} initItems={initItems} lastKey={drumKey} />
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
      </InitGuard>
    </div>
  );
}
