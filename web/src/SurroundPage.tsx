import { useState } from "react";
import { DisplayKind, MixerField, MixerScopeKind } from "../pkg/integral_wasm.js";
import { GridScreen, GridSection, type Binding, type SectionPlacement } from "./layouts";
import { SURROUND_IDS, surroundPartIds, surroundScreen } from "./layouts/surround/screen";
import {
  SURROUND_AMBIENCE_LAYOUT,
  SURROUND_COMMON_LAYOUT,
  SURROUND_EXT_LAYOUT,
  SURROUND_PAD_LAYOUT,
  SURROUND_PART_LAYOUT,
} from "./layouts/surround/sections";
import { mixerBinding } from "./paramDefaults";
import { SurroundXYPad } from "./SurroundXYPad";
import type { SurroundPartState, SurroundState } from "./types";
import type { UseMixerResult } from "./useMixer";
import css from "./SurroundPage.module.css";

interface Props {
  mixer: UseMixerResult;
}

/** A mixer scope: its kind and its part index (0 where the kind has none). */
interface Scope {
  kind: MixerScopeKind;
  index: number;
}

const COMMON: Scope = { kind: MixerScopeKind.Common, index: 0 };
const EXT: Scope = { kind: MixerScopeKind.Ext, index: 0 };

/** The pad Section's binding: it places no Controls, only the pad Display. */
function noControls(field: MixerField): never {
  throw new Error(`the pad Section has no Control for ${MixerField[field]}`);
}

/** The state value each common surround field shows. */
const COMMON_VALUES: Partial<Record<MixerField, (s: SurroundState) => number>> = {
  [MixerField.SurroundSwitch]: (s) => Number(s.enabled),
  [MixerField.SurroundRoomType]: (s) => s.roomType,
  [MixerField.SurroundRoomSize]: (s) => s.roomSize,
  [MixerField.SurroundDepth]: (s) => s.depth,
  [MixerField.SurroundAmbienceLevel]: (s) => s.ambienceLevel,
  [MixerField.SurroundAmbienceTime]: (s) => s.ambienceTime,
  [MixerField.SurroundAmbienceDensity]: (s) => s.ambienceDensity,
  [MixerField.SurroundAmbienceHfDamp]: (s) => s.ambienceHfDamp,
};

/** The state value each part (or Ext Part) surround field shows. */
const POSITION_VALUES: Partial<Record<MixerField, (p: SurroundPartState) => number>> = {
  [MixerField.SurroundLr]: (p) => p.lr,
  [MixerField.SurroundFb]: (p) => p.fb,
  [MixerField.SurroundWidth]: (p) => p.width,
  [MixerField.SurroundAmbienceSend]: (p) => p.ambienceSend,
};

/** The position state of a part or Ext Part scope. */
function positionOf(surround: SurroundState, scope: Scope): SurroundPartState | undefined {
  return scope.kind === MixerScopeKind.Ext ? surround.ext : surround.parts[scope.index];
}

/** The current value of `field` in `scope`. */
function valueOf(surround: SurroundState, scope: Scope, field: MixerField): number {
  if (scope.kind === MixerScopeKind.Common) return COMMON_VALUES[field]?.(surround) ?? 0;
  const position = positionOf(surround, scope);
  return position ? (POSITION_VALUES[field]?.(position) ?? 0) : 0;
}

/** The Motional Surround page: its Sections on the Grid, the pad among them. */
export function SurroundPage({ mixer }: Props) {
  const { surround, parts: mixerParts } = mixer.state;
  const [selectedPart, setSelectedPart] = useState<number>(0);
  const extIndex = surroundPartIds().length;

  /** The scope a pad index (a part, or `extIndex` for the Ext Part) writes. */
  const padScope = (index: number): Scope =>
    index === extIndex ? EXT : { kind: MixerScopeKind.Part, index };

  const bindIn =
    (scope: Scope) =>
    (field: MixerField): Binding =>
      mixerBinding(field, valueOf(surround, scope, field), (v) =>
        mixer.setMixerField(scope.kind, scope.index, field, v),
      );

  const moveTo = (index: number, lr: number, fb: number) => {
    const { kind, index: i } = padScope(index);
    mixer.setMixerField(kind, i, MixerField.SurroundLr, lr);
    mixer.setMixerField(kind, i, MixerField.SurroundFb, fb);
  };

  const dots = [
    ...surround.parts.map((state, index) => ({ index, label: String(index + 1), state })),
    { index: extIndex, label: "EX", state: surround.ext },
  ];

  const renderDisplay = (display: DisplayKind) =>
    display === DisplayKind.SurroundPad ? (
      <SurroundXYPad parts={dots} selectedPart={selectedPart} onSelect={setSelectedPart} onMove={moveTo} />
    ) : null;

  /** A part's Section; selecting it picks the part a click on the pad moves. */
  const partSection = (index: number) => {
    const ext = index === extIndex;
    return (
      <div
        className={`${css.partSection} ${index === selectedPart ? css.selected : ""}`}
        title={ext ? "External Input" : (mixerParts[index]?.toneName ?? "")}
        onClick={() => setSelectedPart(index)}
      >
        <GridSection
          layout={ext ? SURROUND_EXT_LAYOUT : SURROUND_PART_LAYOUT}
          bind={bindIn(padScope(index))}
          scope={ext ? undefined : { part: index }}
        />
      </div>
    );
  };

  const placed = (p: SectionPlacement) => {
    switch (p.id) {
      case SURROUND_IDS.common:
        return <GridSection layout={SURROUND_COMMON_LAYOUT} bind={bindIn(COMMON)} />;
      case SURROUND_IDS.ambience:
        return <GridSection layout={SURROUND_AMBIENCE_LAYOUT} bind={bindIn(COMMON)} />;
      case SURROUND_IDS.pad:
        return <GridSection layout={SURROUND_PAD_LAYOUT} bind={noControls} renderDisplay={renderDisplay} />;
      case SURROUND_IDS.ext:
        return partSection(extIndex);
      default: {
        const part = surroundPartIds().indexOf(p.id);
        return part < 0 ? null : partSection(part);
      }
    }
  };

  return (
    <div className={css.page}>
      <GridScreen layout={surroundScreen()}>{placed}</GridScreen>
    </div>
  );
}
