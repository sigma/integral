import { ChannelStrip } from "./ChannelStrip";
import { FxStrip } from "./FxStrip";
// CompEqPanel replaced by ChannelStrip variant="comp-eq"
import { chorus_params, reverb_params } from "../pkg/integral_wasm.js";
import { deviceSpec } from "./deviceSpec";
import type { UseMixerResult } from "./useMixer";
import css from "./MixerPage.module.css";

interface Props {
  mixer: UseMixerResult;
}

export function MixerPage({ mixer }: Props) {
  const { state } = mixer;
  const spec = deviceSpec();

  return (
    <div className={css.page}>
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
                selected={state.selectedPart === i}
                onSelect={() => mixer.selectPart(i)}
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
            onLevelChange={mixer.setExtLevel}
            onMuteToggle={mixer.toggleExtMute}
          />
          {state.drumCompEq.enabled && state.drumCompEq.units.map((unit, i) => (
            <ChannelStrip
              key={`ceq-${i}`}
              variant="comp-eq"
              label={`C+E${i + 1}`}
              compEqUnit={unit}
              compEqUnitNumber={i + 1}
              compEqOutputAssign={state.drumCompEq.outputAssigns[i]}
              onCompEqParam={(offset, v) => mixer.setCompEqParam(i, offset, v)}
              onCompEqOutputAssign={(v) => mixer.setDrumCompEqOutputAssign(i, v)}
            />
          ))}
          <FxStrip
            label="FX1"
            fx={state.chorus}
            typeNames={spec.chorus_type_names}
            outputNames={spec.chorus_output_names}
            paramDefs={chorus_params}
            onToggleSwitch={mixer.toggleChorusSwitch}
            onParam={mixer.setChorusParam}
            onNibParam={mixer.setChorusNibParam}
          />
          <FxStrip
            label="FX2"
            fx={state.reverb}
            typeNames={spec.reverb_type_names}
            outputNames={spec.reverb_output_names}
            paramDefs={reverb_params}
            onToggleSwitch={mixer.toggleReverbSwitch}
            onParam={mixer.setReverbParam}
            onNibParam={mixer.setReverbNibParam}
          />
          <ChannelStrip
            variant="master"
            label="Master"
            level={state.masterLevel}
            eq={state.masterEq}
            onLevelChange={mixer.setMasterLevel}
            onEqToggle={mixer.toggleMasterEqSwitch}
            onEqParam={mixer.setMasterEqParam}
          />
        </div>
      )}
    </div>
  );
}
