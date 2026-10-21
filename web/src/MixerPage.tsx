import type { ReactNode } from "react";
import { ChannelStrip, StripCategory, StripEq } from "./ChannelStrip";
import { FxStrip } from "./FxStrip";
import { MixerLoadWarning } from "./MixerLoadWarning";
import { MixerScopeKind } from "../pkg/integral_wasm.js";
import { deviceSpec } from "./deviceSpec";
import { GridScreen, GridZoom } from "./layouts";
import { mixerScreen, type MixerSlot } from "./layouts/mixer/screen";
import type { UseMixerResult } from "./useMixer";
import type { UseExpansionsResult } from "./useExpansions";
import css from "./MixerPage.module.css";

interface Props {
  mixer: UseMixerResult;
  expansions: UseExpansionsResult;
}

/** The mixer: a screen of strips on the Grid, the Drum Comp+EQ units shown while it is on. */
export function MixerPage({ mixer, expansions }: Props) {
  const { state } = mixer;
  const spec = deviceSpec();
  const screen = mixerScreen(state.drumCompEq.enabled);
  const { w, h } = screen.layout;

  /**
   * The Chorus or Reverb strip: a CSS Section around its parameters' Grid
   * Section; its switch and routes still write raw block offsets.
   */
  const fxStrip = (fx: "chorus" | "reverb"): ReactNode =>
    fx === "chorus" ? (
      <FxStrip
        label="FX1"
        block="chorus"
        fx={state.chorus}
        typeNames={spec.chorus_type_names}
        routes={[
          { offset: 2, names: spec.output_assign_names, value: state.chorus.outputAssign, title: "FX1 Output Assign" },
          { offset: 3, names: spec.chorus_output_names, value: state.chorus.output, title: "FX1 Output Select" },
        ]}
        onToggleSwitch={mixer.toggleChorusSwitch}
        onParam={mixer.setChorusParam}
        onNibParam={mixer.setChorusNibParam}
      />
    ) : (
      <FxStrip
        label="FX2"
        block="reverb"
        fx={state.reverb}
        typeNames={spec.reverb_type_names}
        routes={[{ offset: 2, names: spec.output_assign_names, value: state.reverb.output, title: "FX2 Output Assign" }]}
        onToggleSwitch={mixer.toggleReverbSwitch}
        onParam={mixer.setReverbParam}
        onNibParam={mixer.setReverbNibParam}
      />
    );

  /** What one placement draws. */
  const draw = (slot: MixerSlot): ReactNode => {
    if (slot.role === "fx") return <div className={css.fxHost}>{fxStrip(slot.fx)}</div>;
    const { kind, index } = slot;
    const strip = { state, kind, index, setField: mixer.setMixerField };
    const unloadedExpansion =
      kind === MixerScopeKind.Part
        ? expansions.unloadedExpansionOf(state.parts[index]!.toneBankMsb, state.parts[index]!.toneBankLsb)
        : null;
    const body =
      slot.role === "eq" ? (
        <StripEq {...strip} />
      ) : slot.role === "strip" ? (
        <ChannelStrip {...strip} unloadedExpansion={unloadedExpansion} />
      ) : (
        <StripCategory state={state} kind={kind} index={index} />
      );
    if (kind !== MixerScopeKind.Part) return body;
    // Pressing anywhere on a part's column, its controls included, selects the part.
    return (
      <div
        className={`${css.column} ${state.selectedPart === index ? css.selected : ""}`}
        onPointerDown={() => mixer.selectPart(index)}
      >
        {body}
      </div>
    );
  };

  // The Grid page root: everything on the page zooms as one, inside the scrolling page.
  return (
    <div className={css.page}>
      <GridZoom cells={{ w, h }}>
        {state.loading ? (
          <div className={css.loading}>Loading mixer state from device...</div>
        ) : (
          <>
            {state.failedBlocks.length > 0 && (
              <MixerLoadWarning failed={state.failedBlocks} onRetry={mixer.reload} />
            )}
            <GridScreen layout={screen.layout}>
              {(p) => {
                const slot = screen.slots.get(p.id);
                return slot ? draw(slot) : null;
              }}
            </GridScreen>
          </>
        )}
      </GridZoom>
    </div>
  );
}
