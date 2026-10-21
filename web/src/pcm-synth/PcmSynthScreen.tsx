/**
 * The PCM Synth editor's Sections on the Grid, placed by its screen layout
 * (`layouts/pcm-synth`): each pair's Sections follow its Structure Type, and
 * the arrows between them run through the layout's Lanes.
 */

import { useMemo, type ReactNode } from "react";
import {
  ParameterFamily,
  PcmsPartialField,
  PcmsPmtField,
  pcms_pair_scope_is_active,
  pcms_partial_is_active,
  pcms_partial_scope_is_active,
  pcms_pmt_is_active,
  pcms_pmt_spec,
} from "../../pkg/integral_wasm.js";
import { GridScreen, GridSection, INSET, specBinding, type SectionPlacement } from "../layouts";
import { partialBoxOf, type PairSection } from "../layouts/pcm-synth/pair";
import { PCMS_IDS, parsePairSectionId, pcmsScreen } from "../layouts/pcm-synth/screen";
import {
  BOOST_12_LAYOUT,
  BOOST_34_LAYOUT,
  PARTIAL_LAYOUT,
  RING_12_LAYOUT,
  RING_34_LAYOUT,
} from "../layouts/pcm-synth/sections";
import { MfxPanel, type MfxState } from "../MfxPanel";
import { SectionPanel } from "../synth-ui";
import type { MfxHandlers } from "../useMfxHandlers";
import type { ExpansionMarking } from "../WavePicker";
import css from "../PcmSynthEditor.module.css";
import { CommonStrip } from "./CommonStrip";
import {
  ControlSection,
  Lfo1Section,
  Lfo2Section,
  OutputSection,
  PitchSection,
  StepLfoSection,
  TvaSection,
  TvfSection,
  WaveSection,
  setterFor,
} from "./PartialSections";
import { PmtStrip } from "./PmtStrip";
import { routingOf } from "./structureRouting";
import type {
  CommonParamHandler,
  PartialParamHandler,
  PcmSynthCommon,
  PcmSynthPartial,
  PcmSynthPmt,
  PmtEntryParamHandler,
  PmtParamHandler,
} from "./types";

/** Props of {@link PcmSynthScreen}. */
export interface PcmSynthScreenProps {
  common: PcmSynthCommon | null;
  /** The tone's PMT: the Structures, and what core reads to say which fields are active. */
  pmt: PcmSynthPmt | null;
  /** Each partial, `null` until loaded. */
  partials: readonly (PcmSynthPartial | null)[];
  mfx: MfxState | null;
  mfxHandlers: MfxHandlers;
  onCommon: CommonParamHandler;
  onPmt: PmtParamHandler;
  onPmtEntry: PmtEntryParamHandler;
  /** Sets a field of partial `partial` (0-based). */
  onPartial: (partial: number, ...args: Parameters<PartialParamHandler>) => void;
  /** Loaded state of the Expansions behind SRX Wave Groups. */
  marking?: ExpansionMarking;
}

/** Each pair's PMT fields and Section layouts, by pair (0: partials 1 & 2). */
const PAIRS = [
  { structure: "structureType12", booster: "booster12", boosterField: PcmsPmtField.Booster12, boost: BOOST_12_LAYOUT, ring: RING_12_LAYOUT },
  { structure: "structureType34", booster: "booster34", boosterField: PcmsPmtField.Booster34, boost: BOOST_34_LAYOUT, ring: RING_34_LAYOUT },
] as const;

/** The binding of a Section that places no Controls. */
function noControls(field: PcmsPmtField): never {
  throw new Error(`no Control binds ${PcmsPmtField[field]} here`);
}

/**
 * A box drawing its own CSS Controls, filling its Section's Cells inside the
 * border inset, as a Grid Section's border sits.
 */
function Hosted({ children, dimmed = false, title }: { children: ReactNode; dimmed?: boolean; title?: string }) {
  return (
    <div className={`${css.hosted} ${dimmed ? css.partialDimmed : ""}`} style={{ inset: INSET }} title={title}>
      {children}
    </div>
  );
}

/** The PCM Synth tone's Sections, on its screen layout. */
export function PcmSynthScreen({
  common,
  pmt,
  partials,
  mfx,
  mfxHandlers,
  onCommon,
  onPmt,
  onPmtEntry,
  onPartial,
  marking,
}: PcmSynthScreenProps) {
  const structures = [pmt?.structureType12 ?? 0, pmt?.structureType34 ?? 0] as const;
  const [s12, s34] = structures;
  const layout = useMemo(() => pcmsScreen([s12, s34]), [s12, s34]);

  /** A Section of pair `pair` (0 or 1). */
  const pairSection = (pair: number, section: PairSection): ReactNode => {
    const { booster, boosterField, boost, ring } = PAIRS[pair]!;
    const first = 2 * pair;
    const numbers = `${first + 1} & ${first + 2}`;
    if (section === "boost") {
      const binding = specBinding(`Booster ${numbers}`, pcms_pmt_spec(boosterField), pmt?.[booster] ?? 0, (v) =>
        onPmt(boosterField, booster, v),
      );
      // The booster belongs to the pair; core holds it inactive once both partials are off.
      return (
        <GridSection
          layout={boost}
          bind={() => binding}
          isActive={() => !!pmt && pcms_pmt_is_active(boosterField, pmt)}
          family={ParameterFamily.Filter}
        />
      );
    }
    if (section === "ring") {
      const active = !!pmt && pcms_pair_scope_is_active(first, pmt);
      return (
        <div
          className={active ? "" : css.partialDimmed}
          title={`Ring modulator: multiplies partials ${first + 1} and ${first + 2}, adding new (often metallic) overtones. No settings; partial ${first + 1}'s AMP sets its depth.`}
        >
          <GridSection layout={ring} bind={noControls} family={ParameterFamily.Filter} />
        </div>
      );
    }

    const box = partialBoxOf(section);
    if (!box) return null;
    const { side, block } = box;
    const idx = first + (side === "lower" ? 1 : 0);
    if (block === "partial") {
      const isOn = pmt?.partialEntries[idx]?.partialSwitch ?? 0;
      const binding = specBinding(`Partial ${idx + 1}`, pcms_pmt_spec(PcmsPmtField.PartialSwitch), isOn, (v) =>
        onPmtEntry(idx, PcmsPmtField.PartialSwitch, "partialSwitch", v),
      );
      return <GridSection layout={PARTIAL_LAYOUT} scope={{ partial: idx }} bind={() => binding} />;
    }
    const p = partials[idx];
    if (!p) return null;
    const set = setterFor((...args) => onPartial(idx, ...args));
    // Boxes carry their partial's number once the pair is merged.
    const n = routingOf(structures[pair]!).join ? idx + 1 : undefined;
    /** Whether core holds `field` of this partial active. */
    const active = (field: PcmsPartialField) => !pmt || pcms_partial_is_active(field, idx, pmt, p);
    /** The partial an inactive pan setting follows: core makes only a merged pair's upper one follow the lower. */
    const followed = (field: PcmsPartialField) => (active(field) ? undefined : first + 2);
    const hosted = (content: ReactNode) => (
      <Hosted dimmed={!pmt || !pcms_partial_scope_is_active(idx, pmt)}>{content}</Hosted>
    );
    switch (block) {
      case "lfo1":
        return hosted(<Lfo1Section partial={p} set={set} n={n} followed={followed} />);
      case "lfo2":
        return hosted(<Lfo2Section partial={p} set={set} n={n} followed={followed} />);
      case "step":
        return hosted(<StepLfoSection partial={p} set={set} n={n} dimmed={!active(PcmsPartialField.LfoStep)} />);
      case "wave":
        return hosted(<WaveSection partial={p} set={set} n={n} marking={marking} />);
      case "pitch":
        return hosted(<PitchSection partial={p} set={set} n={n} />);
      case "ctrl":
        return hosted(<ControlSection partial={p} set={set} n={n} />);
      case "tvf":
        return hosted(<TvfSection partial={p} set={set} n={n} />);
      case "tva":
        return hosted(<TvaSection partial={p} set={set} n={n} followed={followed} />);
      case "out":
        return hosted(<OutputSection partial={p} set={set} n={n} />);
    }
  };

  const placed = (placement: SectionPlacement): ReactNode => {
    if (placement.id === PCMS_IDS.common) {
      return (
        (common || pmt) && (
          <Hosted>
            <SectionPanel label="COMMON" family={ParameterFamily.Common}>
              <div className={css.commonStrip}>
                {common && <CommonStrip common={common} onChange={onCommon} />}
                {common && pmt && <div className={css.commonDivider} />}
                {pmt && <PmtStrip pmt={pmt} onChange={onPmt} />}
              </div>
            </SectionPanel>
          </Hosted>
        )
      );
    }
    if (placement.id === PCMS_IDS.mfx) {
      return mfx && <Hosted><MfxPanel mfx={mfx} {...mfxHandlers} /></Hosted>;
    }
    const at = parsePairSectionId(placement.id);
    return at ? pairSection(at.pair, at.section) : null;
  };

  return <GridScreen layout={layout}>{placed}</GridScreen>;
}
