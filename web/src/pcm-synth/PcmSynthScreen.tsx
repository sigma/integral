/**
 * The PCM Synth editor's Sections on the Grid, each drawn from its layout
 * (`layouts/pcm-synth`) and placed by the screen layout: each pair's
 * Sections follow its Structure Type, and the arrows between them run
 * through the layout's Lanes. Everything about the state comes from core:
 * ranges, relevance, Display bindings and tempo sync. A partial core holds
 * switched off (`pcms_partial_scope_relevance`) dims its boxes, which stay
 * editable. Within a box, each Control dims by `pcms_partial_relevance`.
 */

import { useMemo, type ReactNode } from "react";
import {
  ParameterFamily,
  PcmsCommonField,
  PcmsPartialField,
  PcmsPmtField,
  Relevance,
  pcms_common_spec,
  pcms_pair_scope_relevance,
  pcms_partial_relevance,
  pcms_partial_scope_relevance,
  pcms_pmt_relevance,
  pcms_pmt_spec,
} from "../../pkg/integral_wasm.js";
import { GridScreen, GridSection, ScopeGate, specBinding, type Binding, type SectionPlacement } from "../layouts";
import { partialBoxOf, type PairSection } from "../layouts/pcm-synth/pair";
import { PCMS_IDS, parsePairSectionId, pcmsScreen } from "../layouts/pcm-synth/screen";
import {
  BOOST_12_LAYOUT,
  BOOST_34_LAYOUT,
  PARTIAL_LAYOUT,
  PCMS_BOXES,
  PCMS_COMMON_LAYOUT,
  PCMS_PARTIAL_FIELDS,
  PCMS_PMT_LAYOUT,
  RING_12_LAYOUT,
  RING_34_LAYOUT,
  type PcmsBox,
} from "../layouts/pcm-synth/sections";
import { MfxPanel, type MfxState } from "../MfxPanel";
import type { MfxHandlers } from "../useMfxHandlers";
import type { ExpansionMarking } from "../WavePicker";
import css from "../PcmSynthEditor.module.css";
import { commonValue, partialBinding, partialValue, pmtValue } from "./binding";
import { PartialDisplay, PmtDisplay } from "./displays";
import { routingOf } from "./structureRouting";
import {
  PCMS_COMMON_ENTRIES,
  PCMS_PMT_ENTRIES,
  type CommonParamHandler,
  type PartialParamHandler,
  type PcmSynthCommon,
  type PcmSynthPartial,
  type PcmSynthPmt,
  type PmtEntryParamHandler,
  type PmtParamHandler,
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

/** Each pair's BOOST and RING layouts, by pair (0: partials 1 & 2). */
const PAIRS = [
  { boost: BOOST_12_LAYOUT, ring: RING_12_LAYOUT },
  { boost: BOOST_34_LAYOUT, ring: RING_34_LAYOUT },
] as const;

/** The binding of a Section that places no Controls. */
function noControls(field: PcmsPmtField): never {
  throw new Error(`no Control binds ${PcmsPmtField[field]} here`);
}

/** A PMT field's binding at its value in `pmt`, written through `onPmt`. */
function pmtBinding(field: PcmsPmtField, pmt: PcmSynthPmt, onPmt: PmtParamHandler): Binding {
  return specBinding(PCMS_PMT_ENTRIES[field]?.name ?? PcmsPmtField[field], pcms_pmt_spec(field), pmtValue(pmt, field), (v) =>
    onPmt(field, v),
  );
}

/** Props of {@link PartialBox}. */
interface PartialBoxProps {
  /** Which partial, 0-based: the box's Scope. */
  idx: number;
  /** Which box. */
  box: PcmsBox;
  /** Whether the pair is merged: the box then names its partial. */
  merged: boolean;
  partial: PcmSynthPartial;
  pmt: PcmSynthPmt | null;
  onPartial: PcmSynthScreenProps["onPartial"];
  marking?: ExpansionMarking;
}

/** One box of a partial's lane. */
function PartialBox({ idx, box, merged, partial, pmt, onPartial, marking }: PartialBoxProps) {
  const spec = PCMS_BOXES[box];
  // Relevance from core, per field: whether it is active, and whether it
  // would be for the pair's lower partial (a merged pair's upper partial
  // follows the lower one's pan settings).
  const relevance = useMemo(() => {
    const lower = idx - (idx % 2) + 1;
    const relevanceOf = (field: PcmsPartialField, partialIdx: number) =>
      pmt ? pcms_partial_relevance(field, partialIdx, pmt, partial) : Relevance.Active;
    return new Map(
      PCMS_PARTIAL_FIELDS.ids.map((f) => {
        const rel = relevanceOf(f, idx);
        const follows =
          rel !== Relevance.Active && lower !== idx && relevanceOf(f, lower) === Relevance.Active ? lower : undefined;
        return [f, { relevance: rel, follows }] as const;
      }),
    );
  }, [idx, pmt, partial]);
  const scopeRelevance = pmt ? pcms_partial_scope_relevance(idx, pmt) : Relevance.Active;

  const bind = (field: PcmsPartialField, index = 0): Binding => {
    const binding = partialBinding(field, index, partialValue(partial, field, index), (v) =>
      onPartial(idx, field, index, v),
    );
    const follows = relevance.get(field)?.follows;
    return follows === undefined
      ? binding
      : { ...binding, name: `${binding.name}. Follows partial ${follows + 1} in this Structure.` };
  };

  return (
    <ScopeGate relevance={scopeRelevance}>
      <GridSection
        layout={merged ? spec.numbered : spec.layout}
        scope={{ partial: idx }}
        bind={bind}
        family={spec.family}
        relevance={(f) => relevance.get(f)?.relevance ?? Relevance.Active}
        renderDisplay={(kind) => (
          <PartialDisplay
            kind={kind}
            partial={partial}
            onChange={(field, v) => onPartial(idx, field, 0, v)}
            marking={marking}
          />
        )}
      />
    </ScopeGate>
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
    const { boost, ring } = PAIRS[pair]!;
    const first = 2 * pair;
    if (section === "boost") {
      if (!pmt) return null;
      // The booster belongs to the pair; core holds it inactive once both partials are off.
      return (
        <GridSection
          layout={boost}
          bind={(f) => pmtBinding(f, pmt, onPmt)}
          relevance={(f) => pcms_pmt_relevance(f, pmt)}
          family={ParameterFamily.Filter}
        />
      );
    }
    if (section === "ring") {
      const scopeRelevance = pmt ? pcms_pair_scope_relevance(first, pmt) : Relevance.Irrelevant;
      return (
        <div
          title={`Ring modulator: multiplies partials ${first + 1} and ${first + 2}, adding new (often metallic) overtones. No settings; partial ${first + 1}'s AMP sets its depth.`}
        >
          <ScopeGate relevance={scopeRelevance}>
            <GridSection layout={ring} bind={noControls} family={ParameterFamily.Filter} />
          </ScopeGate>
        </div>
      );
    }

    const at = partialBoxOf(section);
    if (!at) return null;
    const idx = first + (at.side === "lower" ? 1 : 0);
    if (at.block === "partial") {
      const isOn = pmt?.partialEntries[idx]?.partialSwitch ?? 0;
      const binding = specBinding(`Partial ${idx + 1}`, pcms_pmt_spec(PcmsPmtField.PartialSwitch), isOn, (v) =>
        onPmtEntry(idx, PcmsPmtField.PartialSwitch, "partialSwitch", v),
      );
      return <GridSection layout={PARTIAL_LAYOUT} scope={{ partial: idx }} bind={() => binding} />;
    }
    const p = partials[idx];
    if (!p) return null;
    return (
      <PartialBox
        idx={idx}
        box={at.block}
        merged={!!routingOf(structures[pair]!).join}
        partial={p}
        pmt={pmt}
        onPartial={onPartial}
        marking={marking}
      />
    );
  };

  const placed = (placement: SectionPlacement): ReactNode => {
    if (placement.id === PCMS_IDS.common) {
      if (!common) return null;
      return (
        <GridSection
          layout={PCMS_COMMON_LAYOUT}
          family={ParameterFamily.Common}
          bind={(field) => {
            const name = PCMS_COMMON_ENTRIES[field]?.name ?? PcmsCommonField[field];
            return specBinding(name, pcms_common_spec(field), commonValue(common, field), (v) => onCommon(field, v));
          }}
        />
      );
    }
    if (placement.id === PCMS_IDS.pmt) {
      if (!pmt) return null;
      return (
        <GridSection
          layout={PCMS_PMT_LAYOUT}
          family={ParameterFamily.Common}
          bind={(field) => pmtBinding(field, pmt, onPmt)}
          renderDisplay={(kind) => <PmtDisplay kind={kind} pmt={pmt} />}
        />
      );
    }
    if (placement.id === PCMS_IDS.mfx) {
      return <div className={css.mfx}>{mfx && <MfxPanel mfx={mfx} {...mfxHandlers} />}</div>;
    }
    const at = parsePairSectionId(placement.id);
    return at ? pairSection(at.pair, at.section) : null;
  };

  return <GridScreen layout={layout}>{placed}</GridScreen>;
}
