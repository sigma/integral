import { TopBar } from "./TopBar";
import { PartSelector } from "./PartSelector";
import { ChannelStrip } from "./ChannelStrip";
import { FxStrip } from "./FxStrip";
// CompEqPanel replaced by ChannelStrip variant="comp-eq"
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
                compEqAssigned={state.drumCompEq.enabled && state.drumCompEq.part === i}
                onCompEqToggle={() => {
                  if (state.drumCompEq.enabled && state.drumCompEq.part === i) {
                    mixer.setDrumCompEqSwitch(false);
                  } else {
                    mixer.setDrumCompEqPart(i);
                    mixer.setDrumCompEqSwitch(true);
                  }
                }}
                onLevelChange={(v) => mixer.setPartLevel(i, v)}
                onPanChange={(v) => mixer.setPartPan(i, v)}
                onMuteToggle={() => mixer.togglePartMute(i)}
                soloed={state.soloPart === i + 1}
                onSoloToggle={() => mixer.toggleSolo(i)}
                onChorusSendChange={(v) => mixer.setPartChorusSend(i, v)}
                onReverbSendChange={(v) => mixer.setPartReverbSend(i, v)}
                surroundEnabled={state.surround.enabled}
                ambienceSend={state.surround.parts[i]?.ambienceSend}
                onAmbienceSendChange={(v) => mixer.setPartSurroundAmbienceSend(i, v)}
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
          {state.drumCompEq.enabled && state.eqExpanded && state.drumCompEq.units.map((unit, i) => (
            <ChannelStrip
              key={`ceq-${i}`}
              variant="comp-eq"
              label={`C+E${i + 1}`}
              eqExpanded={state.eqExpanded}
              compEqUnit={unit}
              compEqOutputAssign={state.drumCompEq.outputAssigns[i]}
              onCompEqParam={(offset, v) => mixer.setCompEqParam(i, offset, v)}
              onCompEqOutputAssign={(v) => mixer.setDrumCompEqOutputAssign(i, v)}
            />
          ))}
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
