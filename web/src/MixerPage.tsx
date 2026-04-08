import { TopBar } from "./TopBar";
import { PartSelector } from "./PartSelector";
import { ChannelStrip } from "./ChannelStrip";
import { FxStrip } from "./FxStrip";
import { ExStrip } from "./ExStrip";
import { MasterStrip } from "./MasterStrip";
import {
  CHORUS_PARAMS,
  CHORUS_TYPE_NAMES,
  CHORUS_OUTPUT_NAMES,
  REVERB_PARAMS,
  REVERB_TYPE_NAMES,
  REVERB_OUTPUT_NAMES,
} from "./fxParams";
import type { UseMixerResult } from "./useMixer";
import css from "./MixerPage.module.css";

interface Props {
  mixer: UseMixerResult;
}

export function MixerPage({ mixer }: Props) {
  const { state } = mixer;
  const selectedPart = state.parts[state.selectedPart]!;

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
          EQ {state.eqExpanded ? "▲" : "▼"}
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
          <ExStrip
            level={state.extLevel}
            muted={state.extMuted}
            eqExpanded={state.eqExpanded}
            onLevelChange={mixer.setExtLevel}
            onMuteToggle={mixer.toggleExtMute}
          />
          <MasterStrip
            value={state.masterLevel}
            onChange={mixer.setMasterLevel}
            eq={state.masterEq}
            eqExpanded={state.eqExpanded}
            onEqToggle={mixer.toggleMasterEqSwitch}
            onEqParam={mixer.setMasterEqParam}
          />
        </div>
      )}
    </div>
  );
}
