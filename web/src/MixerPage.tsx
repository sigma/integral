import { TopBar } from "./TopBar";
import { PartSelector } from "./PartSelector";
import { ChannelStrip } from "./ChannelStrip";
import { FxStrip } from "./FxStrip";
// ExStrip and MasterStrip are now ChannelStrip variants.
import {
  CHORUS_PARAMS,
  CHORUS_TYPE_NAMES,
  CHORUS_OUTPUT_NAMES,
  REVERB_PARAMS,
  REVERB_TYPE_NAMES,
  REVERB_OUTPUT_NAMES,
} from "./fxParams";
import { ToneSelector } from "./ToneSelector";
import { ToneCatalog } from "./toneCatalog";
import type { IntegraService } from "./integra";
import type { UseMixerResult } from "./useMixer";
import { useState, useCallback, useMemo } from "react";
import css from "./MixerPage.module.css";

interface Props {
  mixer: UseMixerResult;
  service: IntegraService;
}

export function MixerPage({ mixer, service }: Props) {
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
    <div className={css.page}>
      <TopBar
        studioSetPC={state.studioSetPC}
        studioSetNames={state.studioSetNames}
        selectedPart={selectedPart}
        selectedPartIndex={state.selectedPart}
        onStudioSetChange={mixer.switchStudioSet}
        onLoadNames={mixer.loadStudioSetNames}
        onPreview={mixer.preview}
        onToneClick={() => setToneSelectorOpen(true)}
      />
      <div className={css.controls}>
        <PartSelector
          selectedPart={state.selectedPart}
          onSelect={mixer.selectPart}
        />
        <button
          className={css.eqToggle}
          onClick={mixer.toggleEqExpanded}
        >
          Details {state.eqExpanded ? "▲" : "▼"}
        </button>
      </div>
      {state.loading ? (
        <div className={css.loading}>Loading mixer state from device...</div>
      ) : (
        <div className={css.mixerArea}>
          <div className={css.channels}>
            {state.parts.map((part, i) => (
              <ChannelStrip
                key={i}
                partIndex={i}
                part={part}
                eqExpanded={state.eqExpanded}
                onLevelChange={(v) => mixer.setPartLevel(i, v)}
                onPanChange={(v) => mixer.setPartPan(i, v)}
                onMuteToggle={() => mixer.togglePartMute(i)}
                onChorusSendChange={(v) => mixer.setPartChorusSend(i, v)}
                onReverbSendChange={(v) => mixer.setPartReverbSend(i, v)}
                onReceiveChannelChange={(ch) => mixer.setPartReceiveChannel(i, ch)}
                onEqToggle={() => mixer.togglePartEqSwitch(i)}
                onEqParam={(offset, v) => mixer.setPartEqParam(i, offset, v)}
              />
            ))}
          </div>
          <ChannelStrip
            variant="ext"
            label="EX"
            level={state.extLevel}
            muted={state.extMuted}
            eqExpanded={state.eqExpanded}
            onLevelChange={mixer.setExtLevel}
            onMuteToggle={mixer.toggleExtMute}
          />
          <FxStrip
            label="FX1"
            fx={state.chorus}
            eqExpanded={state.eqExpanded}
            typeNames={CHORUS_TYPE_NAMES}
            outputNames={CHORUS_OUTPUT_NAMES}
            paramDefs={CHORUS_PARAMS}
            onToggleSwitch={mixer.toggleChorusSwitch}
            onParam={mixer.setChorusParam}
            onNibParam={mixer.setChorusNibParam}
          />
          <FxStrip
            label="FX2"
            fx={state.reverb}
            eqExpanded={state.eqExpanded}
            typeNames={REVERB_TYPE_NAMES}
            outputNames={REVERB_OUTPUT_NAMES}
            paramDefs={REVERB_PARAMS}
            onToggleSwitch={mixer.toggleReverbSwitch}
            onParam={mixer.setReverbParam}
            onNibParam={mixer.setReverbNibParam}
          />
          <ChannelStrip
            variant="master"
            label="Master"
            level={state.masterLevel}
            eq={state.masterEq}
            eqExpanded={state.eqExpanded}
            onLevelChange={mixer.setMasterLevel}
            onEqToggle={mixer.toggleMasterEqSwitch}
            onEqParam={mixer.setMasterEqParam}
          />
        </div>
      )}
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
