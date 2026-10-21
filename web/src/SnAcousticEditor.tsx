import { useEffect, useState, useCallback, useMemo, type ReactNode } from "react";
import {
  display_binding,
  sna_common_is_active,
  sna_common_spec,
  sna_common_address,
  sna_common_size,
  sna_mfx_address,
  mfx_block_size,
  DisplayKind,
  DisplayProp,
  SnaCommonField,
  ToneType,
  ParameterFamily,
} from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import { MfxPanel } from "./MfxPanel";
import { WavePicker } from "./WavePicker";
import { snaInstNumberOf, snaInstSource } from "./snaInstCatalog";
import { FilterScope } from "./synth-ui/FilterScope";
import { ScopeScreen } from "./synth-ui/ScopeScreen";
import { envelopePath } from "./synth-ui/ADSREnvelope";
import { useMfxHandlers } from "./useMfxHandlers";
import type { MfxState } from "./MfxPanel";
import { GridScreen, GridSection, variantLayout, type Binding, type SectionLayout } from "./layouts";
import { SNA_AMP_LAYOUT } from "./layouts/sn-acoustic/amp";
import { SNA_COMMON_LAYOUT } from "./layouts/sn-acoustic/common";
import { instOf, isTwoState, modifyDefs, modifyParams } from "./layouts/sn-acoustic/fields";
import { SNA_FILTER_LAYOUT } from "./layouts/sn-acoustic/filter";
import { SNA_INST_SET, instPage } from "./layouts/sn-acoustic/inst";
import { SNA_SCREEN, SNA_SECTION } from "./layouts/sn-acoustic/screen";
import { SNA_VIBRATO_LAYOUT } from "./layouts/sn-acoustic/vibrato";
import css from "./SnAcousticEditor.module.css";

// ---------------------------------------------------------------------------
// Types mirroring the Rust serde output
// ---------------------------------------------------------------------------

interface SnAcousticCommon {
  toneName: string;
  toneLevel: number;
  monoPoly: number;
  portamentoTimeOffset: number;
  cutoffOffset: number;
  resonanceOffset: number;
  attackTimeOffset: number;
  releaseTimeOffset: number;
  vibratoRate: number;
  vibratoDepth: number;
  vibratoDelay: number;
  octaveShift: number;
  category: number;
  phraseNumber: number;
  phraseOctaveShift: number;
  tfxSwitch: number;
  instVariation: number;
  instNumber: number;
  modifyParams: number[];
}

/** The keys of {@link SnAcousticCommon} holding a number. */
type CommonKey = {
  [K in keyof SnAcousticCommon]: SnAcousticCommon[K] extends number ? K : never;
}[keyof SnAcousticCommon];

/** A Common field other than the 32-slot Modify Parameter. */
type ScalarField = Exclude<SnaCommonField, SnaCommonField.ModifyParameter>;

/** Where each scalar Common field sits in {@link SnAcousticCommon} (core's serde names). */
const COMMON_KEY: Record<ScalarField, CommonKey> = {
  [SnaCommonField.ToneLevel]: "toneLevel",
  [SnaCommonField.MonoPoly]: "monoPoly",
  [SnaCommonField.PortamentoTimeOffset]: "portamentoTimeOffset",
  [SnaCommonField.CutoffOffset]: "cutoffOffset",
  [SnaCommonField.ResonanceOffset]: "resonanceOffset",
  [SnaCommonField.AttackTimeOffset]: "attackTimeOffset",
  [SnaCommonField.ReleaseTimeOffset]: "releaseTimeOffset",
  [SnaCommonField.VibratoRate]: "vibratoRate",
  [SnaCommonField.VibratoDepth]: "vibratoDepth",
  [SnaCommonField.VibratoDelay]: "vibratoDelay",
  [SnaCommonField.OctaveShift]: "octaveShift",
  [SnaCommonField.Category]: "category",
  [SnaCommonField.PhraseNumber]: "phraseNumber",
  [SnaCommonField.PhraseOctaveShift]: "phraseOctaveShift",
  [SnaCommonField.TfxSwitch]: "tfxSwitch",
  [SnaCommonField.InstVariation]: "instVariation",
  [SnaCommonField.InstNumber]: "instNumber",
};

/** Sets a scalar Common field. */
type CommonParamHandler = (field: ScalarField, value: number) => void;

/** A Common field's name, from its core enum member: `CutoffOffset` reads "Cutoff Offset". */
function commonName(field: SnaCommonField): string {
  const member = SnaCommonField[field] ?? String(field);
  let out = "";
  for (const [i, ch] of [...member].entries()) {
    const prev = member[i - 1];
    out += i > 0 && prev !== undefined && prev === prev.toLowerCase() && ch !== ch.toLowerCase() ? ` ${ch}` : ch;
  }
  return out;
}

/** The family every Modify Parameter knob shows, from its Common field spec. */
function modifyParamFamily(): ParameterFamily {
  return sna_common_spec(SnaCommonField.ModifyParameter).family;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface Props {
  partIndex: number;
  service: IntegraService;
}

export function SnAcousticEditor({ partIndex, service }: Props) {
  const [common, setCommon] = useState<SnAcousticCommon | null>(null);
  const [mfx, setMfx] = useState<MfxState | null>(null);
  const [loading, setLoading] = useState(true);

  // Load all data on mount / part change
  useEffect(() => {
    let cancelled = false;
    setLoading(true);

    async function load() {
      try {
        // Common
        const commonData = await service.requestData(
          Array.from(sna_common_address(partIndex)),
          Array.from(sna_common_size()),
        );
        if (cancelled) return;
        const c = service.device.applySnaCommon(commonData) as SnAcousticCommon | null;
        if (c) setCommon(c);

        // MFX
        const mfxData = await service.requestData(
          Array.from(sna_mfx_address(partIndex)),
          Array.from(mfx_block_size()),
        );
        if (cancelled) return;
        const m = service.device.applyMfxBlock(partIndex, mfxData) as MfxState | null;
        if (m) setMfx(m);
      } catch {
        // Timeout -- partial data is OK
      }
      if (!cancelled) setLoading(false);
    }

    load();
    return () => { cancelled = true; };
  }, [partIndex, service]);

  // ---------------------------------------------------------------------------
  // Param setters: send to the device, then patch the local copy
  // ---------------------------------------------------------------------------
  const setCommonParam = useCallback<CommonParamHandler>(
    (field, value) => {
      service.device.setSnaCommonParam(partIndex, field, 0, value);
      setCommon((prev) => (prev ? { ...prev, [COMMON_KEY[field]]: value } : prev));
    },
    [partIndex, service],
  );

  const setModifyParam = useCallback(
    (index: number, value: number) => {
      service.device.setSnaCommonParam(partIndex, SnaCommonField.ModifyParameter, index, value);
      setCommon((prev) =>
        prev
          ? { ...prev, modifyParams: prev.modifyParams.map((v, i) => (i === index ? value : v)) }
          : prev,
      );
    },
    [partIndex, service],
  );

  /**
   * Switch the tone to another instrument, set its Modify Parameters to their
   * defaults, then read Common back to show what the device ended up with.
   */
  const pickInstrument = useCallback(
    (listNumber: number) => {
      const inst = instOf(listNumber);
      if (!inst) return; // the picker's "(unlisted)" entry
      setCommonParam(SnaCommonField.InstVariation, inst.variation);
      setCommonParam(SnaCommonField.InstNumber, inst.instNumber);
      // The Modify slots keep the previous instrument's values, which mean
      // nothing (or silence, for TW Organ's drawbars) to the new one: give it
      // its defaults.
      for (const def of modifyParams(inst.variation, inst.instNumber)) {
        setModifyParam(def.slot - 1, def.defaultValue);
      }
      service
        .requestData(Array.from(sna_common_address(partIndex)), Array.from(sna_common_size()))
        .then((data) => {
          const c = service.device.applySnaCommon(data) as SnAcousticCommon | null;
          if (c) setCommon(c);
        })
        .catch(() => {
          // Timeout: keep the local copy
        });
    },
    [partIndex, service, setCommonParam, setModifyParam],
  );

  const mfxHandlers = useMfxHandlers(service, partIndex, ToneType.SnAcoustic, setMfx);

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  if (loading) {
    return <div className={css.loading}>Loading SN-A tone data...</div>;
  }

  return (
    <div className={css.editor}>
      {common && (
        <div className={css.header}>
          <div className={css.toneName}>{common.toneName || "(unnamed)"}</div>
          {/* Picks the INST variant: it writes two fields, so it is no Grid Control. */}
          <WavePicker
            source={snaInstSource()}
            stereo={false}
            l={snaInstNumberOf(common.instVariation, common.instNumber)}
            onChange={(_side, n) => pickInstrument(n)}
          />
        </div>
      )}
      {common && (
        <SnaScreen
          common={common}
          onCommon={setCommonParam}
          onModify={setModifyParam}
          mfx={mfx && <MfxPanel mfx={mfx} {...mfxHandlers} />}
        />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// The screen
// ---------------------------------------------------------------------------

/** The tone-wide Sections, by placement id, with the family accenting each. */
const TONE_SECTIONS: Record<string, { layout: SectionLayout<SnaCommonField>; family: ParameterFamily }> = {
  [SNA_SECTION.common]: { layout: SNA_COMMON_LAYOUT, family: ParameterFamily.Common },
  [SNA_SECTION.vibrato]: { layout: SNA_VIBRATO_LAYOUT, family: ParameterFamily.Modulation },
  [SNA_SECTION.filter]: { layout: SNA_FILTER_LAYOUT, family: ParameterFamily.Filter },
  [SNA_SECTION.amp]: { layout: SNA_AMP_LAYOUT, family: ParameterFamily.Amp },
};

/** `value` as 0-1 across `field`'s range. */
function normalized(field: SnaCommonField, value: number): number {
  const { min, max } = sna_common_spec(field);
  return (value - min) / (max - min || 1);
}

/** The SN-A screen on the Grid: the tone-wide Sections, INST for the instrument, and the MFX host. */
function SnaScreen({
  common,
  onCommon,
  onModify,
  mfx,
}: {
  common: SnAcousticCommon;
  onCommon: CommonParamHandler;
  /** Sets Modify Parameter `index + 1`. */
  onModify: (index: number, value: number) => void;
  /** The MFX panel, drawn in its host column; none until it loads. */
  mfx: ReactNode;
}) {
  const instKey = snaInstNumberOf(common.instVariation, common.instNumber);
  const instLayout = useMemo(
    () => ({ ...variantLayout(SNA_INST_SET, instKey), title: instPage(instKey) ?? SNA_INST_SET.title }),
    [instKey],
  );
  const instDefs = useMemo(() => modifyDefs(instKey), [instKey]);

  const value = (field: SnaCommonField): number =>
    field === SnaCommonField.ModifyParameter ? 0 : common[COMMON_KEY[field]];
  // Some instruments ignore some offsets: core holds them inactive.
  const isActive = (field: SnaCommonField): boolean => sna_common_is_active(field, common);

  const bindCommon = (field: SnaCommonField): Binding => {
    const spec = sna_common_spec(field);
    return {
      name: commonName(field),
      value: value(field),
      min: spec.min,
      max: spec.max,
      defaultValue: spec.defaultValue,
      center: spec.center,
      family: spec.family,
      format: (v) => spec.formatValue(v),
      onChange: (v) => {
        if (field !== SnaCommonField.ModifyParameter) onCommon(field, v);
      },
    };
  };

  const bindModify = (slot: number): Binding => {
    // The variant places only the slots core lists for the instrument.
    const def = instDefs.get(slot);
    if (!def) throw new Error(`instrument ${instKey} has no Modify Parameter ${slot}`);
    return {
      name: def.name,
      value: common.modifyParams[slot - 1] ?? def.defaultValue,
      min: def.min,
      max: def.max,
      defaultValue: def.defaultValue,
      family: modifyParamFamily(),
      format: String,
      // shortcut: core gives Modify Parameters no value labels, so two-state ones read OFF/ON (Glide's Porta/Gliss included), as before the Grid.
      options: isTwoState(def)
        ? [
            { value: def.min, label: "OFF" },
            { value: def.max, label: "ON" },
          ]
        : undefined,
      onChange: (v) => onModify(slot - 1, v),
    };
  };

  /** Draws a tone-wide Display from the Common fields core binds it to, dimmed if any is inactive. */
  const renderDisplay = (display: DisplayKind): ReactNode => {
    const slots = display_binding(display, ToneType.SnAcoustic) ?? [];
    const read = (prop: DisplayProp): { field: SnaCommonField; value: number } | undefined => {
      const field = slots.find((s) => s.prop === prop)?.snaCommon;
      return field === undefined ? undefined : { field, value: value(field) };
    };
    const dim = slots.some((s) => s.snaCommon !== undefined && !isActive(s.snaCommon));
    let drawn: ReactNode = null;
    if (display === DisplayKind.FilterScope) {
      const cutoff = read(DisplayProp.Cutoff);
      const reso = read(DisplayProp.Resonance);
      const center = reso ? (sna_common_spec(reso.field).center ?? 0) : 0;
      // shortcut: SN-A never exposes its filter type; this draws a low-pass moved by the offsets, to show their direction, not the instrument's real filter.
      drawn = (
        <FilterScope
          className={css.display}
          type="LPF"
          cutoff={cutoff?.value ?? sna_common_spec(SnaCommonField.CutoffOffset).defaultValue}
          resonance={Math.max(0, (reso?.value ?? center) - center) * 2}
        />
      );
    } else if (display === DisplayKind.AmpEnvelope) {
      const attack = read(DisplayProp.Attack);
      const release = read(DisplayProp.Release);
      drawn = (
        <ScopeScreen className={css.display}>
          <path
            d={envelopePath({
              attack: attack ? normalized(attack.field, attack.value) : 0,
              release: release ? normalized(release.field, release.value) : 0,
            })}
            fill="none"
            stroke="#fc8"
            strokeWidth="2"
            vectorEffect="non-scaling-stroke"
          />
        </ScopeScreen>
      );
    }
    return dim ? <div className={`${css.display} ${css.inactive}`}>{drawn}</div> : drawn;
  };

  return (
    <div className={css.screen}>
      <GridScreen layout={SNA_SCREEN}>
        {(placement) => {
          if (placement.id === SNA_SECTION.mfx) return <div data-node="mfx">{mfx}</div>;
          if (placement.id === SNA_SECTION.inst) {
            return (
              <GridSection layout={instLayout} bind={bindModify} family={ParameterFamily.Pitch} />
            );
          }
          const tone = TONE_SECTIONS[placement.id];
          if (!tone) return null;
          return (
            <div data-node={placement.id}>
              <GridSection
                layout={tone.layout}
                bind={bindCommon}
                isActive={isActive}
                renderDisplay={renderDisplay}
                family={tone.family}
              />
            </div>
          );
        }}
      </GridScreen>
    </div>
  );
}
