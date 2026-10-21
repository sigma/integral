import type { ReactNode } from "react";
import { ChannelStrip, StripCategory, StripEq } from "./ChannelStrip";
import { FxCategory, FxParams, FxStrip } from "./FxStrip";
import { MixerLoadWarning } from "./MixerLoadWarning";
import { MixerScopeKind } from "../pkg/integral_wasm.js";
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
  const screen = mixerScreen(state.drumCompEq.enabled);
  const { w, h } = screen.layout;

  /** Each FX block's state and parameter writer. */
  const fxBlocks = {
    chorus: { state: state.chorus, onNib: mixer.setChorusNibParam },
    reverb: { state: state.reverb, onNib: mixer.setReverbNibParam },
  } as const;

  /** What one placement draws. */
  const draw = (slot: MixerSlot): ReactNode => {
    if (slot.role === "fx") {
      const { state: fx, onNib } = fxBlocks[slot.fx];
      if (slot.section === "params") return <FxParams block={slot.fx} fx={fx} onNibParam={onNib} />;
      if (slot.section === "strip") return <FxStrip block={slot.fx} fx={fx} setField={mixer.setMixerField} />;
      return <FxCategory block={slot.fx} />;
    }
    const { kind, index } = slot;
    const strip = { state, kind, index, setField: mixer.setMixerField };
    const unloadedExpansion = () =>
      kind === MixerScopeKind.Part
        ? expansions.unloadedExpansionOf(state.parts[index]!.toneBankMsb, state.parts[index]!.toneBankLsb)
        : null;
    const body =
      slot.role === "eq" ? (
        <StripEq {...strip} />
      ) : slot.role === "strip" ? (
        <ChannelStrip {...strip} unloadedExpansion={unloadedExpansion()} />
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
