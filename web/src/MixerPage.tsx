import { ChannelStrip } from "./ChannelStrip";
import { FxStrip } from "./FxStrip";
import { MixerLoadWarning } from "./MixerLoadWarning";
// CompEqPanel replaced by ChannelStrip variant="comp-eq"
import { chorus_params, reverb_params } from "../pkg/integral_wasm.js";
import { deviceSpec } from "./deviceSpec";
import type { UseMixerResult } from "./useMixer";
import type { UseExpansionsResult } from "./useExpansions";
import css from "./MixerPage.module.css";

interface Props {
  mixer: UseMixerResult;
  expansions: UseExpansionsResult;
}

export function MixerPage({ mixer, expansions }: Props) {
  const { state } = mixer;
  const spec = deviceSpec();

  return (
    <div className={css.page}>
      {state.loading ? (
        <div className={css.loading}>Loading mixer state from device...</div>
      ) : (
        <div className={css.mixerArea}>
          {state.failedBlocks.length > 0 && (
            <MixerLoadWarning failed={state.failedBlocks} onRetry={mixer.reload} />
          )}
          <div className={css.channels}>
            {state.parts.map((part, i) => (
              <ChannelStrip
                key={i}
                partIndex={i}
                part={part}
                unloadedExpansion={expansions.unloadedExpansionOf(
                  part.toneBankMsb,
                  part.toneBankLsb,
                )}
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
                onOutputAssignChange={(v) => mixer.setPartOutputAssign(i, v)}
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
              surroundEnabled={state.surround.enabled}
            />
          ))}
          <FxStrip
            label="FX1"
            fx={state.chorus}
            typeNames={spec.chorus_type_names}
            routes={[
              {
                offset: 2,
                names: spec.output_assign_names,
                value: state.chorus.outputAssign,
                title: "FX1 Output Assign",
              },
              {
                offset: 3,
                names: spec.chorus_output_names,
                value: state.chorus.output,
                title: "FX1 Output Select",
              },
            ]}
            paramDefs={chorus_params}
            onToggleSwitch={mixer.toggleChorusSwitch}
            onParam={mixer.setChorusParam}
            onNibParam={mixer.setChorusNibParam}
          />
          <FxStrip
            label="FX2"
            fx={state.reverb}
            typeNames={spec.reverb_type_names}
            routes={[
              {
                offset: 2,
                names: spec.output_assign_names,
                value: state.reverb.output,
                title: "FX2 Output Assign",
              },
            ]}
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
