import { useEffect, useState, useCallback, useMemo, type ReactNode } from "react";
import {
  sna_inst_page,
  sna_inst_params,
  sna_common_is_active,
  sna_common_spec,
  sna_common_address,
  sna_common_size,
  sna_mfx_address,
  mfx_block_size,
  SnaCommonField,
  ToneType,
  ParameterFamily,
} from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import {
  SynthKnob,
  SynthFader,
  SynthSwitch,
  SectionPanel,
  FaderGroup,
  ADSREnvelope,
  FaderGroupSep,
} from "./synth-ui";
import { MfxPanel } from "./MfxPanel";
import { knobProps, knobRange, switchOptions } from "./fieldSpec";
import { WavePicker } from "./WavePicker";
import { snaInstNumberOf, snaInstSource, snaInsts } from "./snaInstCatalog";
import { SignalArrows } from "./synth-ui/SignalArrows";
import { FilterScope } from "./synth-ui/FilterScope";
import { useMfxHandlers } from "./useMfxHandlers";
import type { MfxState } from "./MfxPanel";
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

/** Sets a Common field: `field` addresses it on the device, `key` names it
 *  in {@link SnAcousticCommon}. */
type CommonParamHandler = (field: SnaCommonField, key: CommonKey, value: number) => void;

/** A single instrument parameter definition from the Rust table. */
interface InstParamDef {
  index: number;
  name: string;
  min: number;
  max: number;
  defaultValue: number;
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

/** The family every Modify Parameter knob shows, from its Common field spec. */
function modifyParamFamily(): ParameterFamily {
  return sna_common_spec(SnaCommonField.ModifyParameter).family;
}

/** Parse the raw tuple array from WASM into structured InstParamDef[]. */
function parseInstParams(raw: unknown): InstParamDef[] {
  if (!Array.isArray(raw)) return [];
  return (raw as [number, string, number, number, number][]).map(
    ([index, name, min, max, defaultValue]) => ({ index, name, min, max, defaultValue }),
  );
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
    (field, key, value) => {
      service.device.setSnaCommonParam(partIndex, field, 0, value);
      setCommon((prev) => (prev ? { ...prev, [key]: value } : prev));
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
      const inst = snaInsts()[listNumber - 1];
      if (!inst) return; // the picker's "(unlisted)" entry
      setCommonParam(SnaCommonField.InstVariation, "instVariation", inst.variation);
      setCommonParam(SnaCommonField.InstNumber, "instNumber", inst.instNumber);
      // The Modify slots keep the previous instrument's values, which mean
      // nothing (or silence, for TW Organ's drawbars) to the new one: give it
      // its defaults.
      for (const def of parseInstParams(sna_inst_params(inst.variation, inst.instNumber))) {
        setModifyParam(def.index - 1, def.defaultValue);
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

  // ---------------------------------------------------------------------------
  // Instrument parameters
  // ---------------------------------------------------------------------------
  // The instrument's INST page ("Ac.Piano", "TW Organ", or its category) and
  // its Modify Parameters, both from core's per-instrument list.
  const instVariation = common?.instVariation ?? 0;
  const instNumber = common?.instNumber ?? 0;
  const categoryName = common ? (sna_inst_page(instVariation, instNumber) ?? null) : null;

  const instParamDefs = useMemo(
    () => parseInstParams(sna_inst_params(instVariation, instNumber)),
    [instVariation, instNumber],
  );

  const mfxHandlers = useMfxHandlers(service, partIndex, ToneType.SnAcoustic, setMfx);

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  if (loading) {
    return <div className={css.loading}>Loading SN-A tone data...</div>;
  }

  const boxes = common ? toneBoxes(common, setCommonParam) : null;
  const instPicker = common && (
    <WavePicker
      source={snaInstSource()}
      stereo={false}
      l={snaInstNumberOf(common.instVariation, common.instNumber)}
      onChange={(_side, n) => pickInstrument(n)}
    />
  );

  return (
    <div className={css.editor}>
      {/* Tone name */}
      {common && (
        <div className={css.toneName}>{common.toneName || "(unnamed)"}</div>
      )}

      {/* COMMON on top; below it the instrument, vibrato, filter and amp, which feeds the MFX */}
      {common && (
        <div className={css.mainArea}>
          <div className={css.commonCell}>{boxes!.common}</div>
          <div className={css.toneRow}>
            {categoryName ? (
              <InstrumentPanel
                header={instPicker}
                categoryName={categoryName}
                paramDefs={instParamDefs}
                modifyParams={common.modifyParams}
                onModify={setModifyParam}
              />
            ) : (
              <SectionPanel label="INSTRUMENT" family={ParameterFamily.Pitch}>{instPicker}</SectionPanel>
            )}
            {boxes!.vibrato}
            {boxes!.filter}
            <div className={css.ampCell} data-node="sna-amp">{boxes!.amp}</div>
          </div>
          {mfx && (
            <div className={css.mfxSidebar} data-node="mfx">
              <MfxPanel
                mfx={mfx}
                {...mfxHandlers}
              />
            </div>
          )}
          <SignalArrows edges={[{ from: "sna-amp", to: "mfx" }]} />
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Tone-wide boxes: COMMON, VIBRATO, FILTER, AMP, ENV
// ---------------------------------------------------------------------------

/** Offset controls, shown disabled when the instrument ignores them. */
function Offset({ enabled, children }: { enabled: boolean; children: ReactNode }) {
  return enabled ? <>{children}</> : (
    <div className={css.offsetDisabled} inert title="This instrument ignores these offsets">
      {children}
    </div>
  );
}

/** The tone-wide boxes, for the editor to lay out: COMMON, VIBRATO, FILTER, AMP. */
function toneBoxes(
  common: SnAcousticCommon,
  onChange: CommonParamHandler,
): Record<"common" | "vibrato" | "filter" | "amp", ReactNode> {
  const set =
    (field: SnaCommonField, key: Parameters<CommonParamHandler>[1]) =>
    (v: number) =>
      onChange(field, key, v);
  const resonanceCenter = sna_common_spec(SnaCommonField.ResonanceOffset).center ?? 0;
  // Some instruments ignore some offsets: core holds them inactive, shown disabled.
  const active = (field: SnaCommonField) => sna_common_is_active(field, common);
  return {
    common: (
        <SectionPanel label="COMMON" family={ParameterFamily.Common}>
          <div className={css.panelRow}>
            <SynthSwitch label="Mono/Poly" value={common.monoPoly} options={switchOptions(sna_common_spec(SnaCommonField.MonoPoly))}
              onChange={set(SnaCommonField.MonoPoly, "monoPoly")} />
            <SynthKnob label="Oct" value={common.octaveShift} {...knobProps(sna_common_spec(SnaCommonField.OctaveShift))}
              onChange={set(SnaCommonField.OctaveShift, "octaveShift")}
              title="Octave Shift" />
            <Offset enabled={active(SnaCommonField.PortamentoTimeOffset)}>
              <SynthKnob label="Port" value={common.portamentoTimeOffset} {...knobProps(sna_common_spec(SnaCommonField.PortamentoTimeOffset))}
                onChange={set(SnaCommonField.PortamentoTimeOffset, "portamentoTimeOffset")}
                title="Portamento Time Offset" />
            </Offset>
          </div>
        </SectionPanel>
    ),
    vibrato: (
        <SectionPanel label="VIB" family={ParameterFamily.Modulation}>
          <div className={css.knobColumn}>
            <SynthKnob label="Rate" value={common.vibratoRate} {...knobProps(sna_common_spec(SnaCommonField.VibratoRate))}
              onChange={set(SnaCommonField.VibratoRate, "vibratoRate")}
              title="Vibrato Rate (offset)" />
            <SynthKnob label="Depth" value={common.vibratoDepth} {...knobProps(sna_common_spec(SnaCommonField.VibratoDepth))}
              onChange={set(SnaCommonField.VibratoDepth, "vibratoDepth")}
              title="Vibrato Depth (offset)" />
            <SynthKnob label="Delay" value={common.vibratoDelay} {...knobProps(sna_common_spec(SnaCommonField.VibratoDelay))}
              onChange={set(SnaCommonField.VibratoDelay, "vibratoDelay")}
              title="Vibrato Delay (offset)" />
          </div>
        </SectionPanel>
    ),
    filter: (
        <SectionPanel label="FILTER" family={ParameterFamily.Filter}>
          {/* shortcut: SN-A never exposes its filter type; this draws a low-pass moved by the offsets, to show their direction, not the instrument's real filter. */}
          <FilterScope className={css.filterScope} type="LPF" cutoff={common.cutoffOffset}
            resonance={Math.max(0, common.resonanceOffset - resonanceCenter) * 2} />
          <div className={css.panelRow}>
            <Offset enabled={active(SnaCommonField.CutoffOffset)}>
              <SynthKnob label="Cutoff" value={common.cutoffOffset} {...knobProps(sna_common_spec(SnaCommonField.CutoffOffset))}
                onChange={set(SnaCommonField.CutoffOffset, "cutoffOffset")}
                title="Cutoff Offset" />
            </Offset>
            <Offset enabled={active(SnaCommonField.ResonanceOffset)}>
              <SynthKnob label="Reso" value={common.resonanceOffset} {...knobProps(sna_common_spec(SnaCommonField.ResonanceOffset))}
                onChange={set(SnaCommonField.ResonanceOffset, "resonanceOffset")}
                title="Resonance Offset" />
            </Offset>
          </div>
        </SectionPanel>
    ),
    amp: (
        <SectionPanel label="AMP" family={ParameterFamily.Amp}>
          <div className={css.ampRow}>
            {/* One envelope draws both offsets: it's disabled unless core holds both active. */}
            <Offset enabled={active(SnaCommonField.AttackTimeOffset) && active(SnaCommonField.ReleaseTimeOffset)}>
              <ADSREnvelope
                compact
                attack={{
                  label: "Atk", value: common.attackTimeOffset,
                  onChange: set(SnaCommonField.AttackTimeOffset, "attackTimeOffset"), ...knobRange(sna_common_spec(SnaCommonField.AttackTimeOffset)),
                  title: "Attack Time Offset",
                }}
                release={{
                  label: "Rel", value: common.releaseTimeOffset,
                  onChange: set(SnaCommonField.ReleaseTimeOffset, "releaseTimeOffset"), ...knobRange(sna_common_spec(SnaCommonField.ReleaseTimeOffset)),
                  title: "Release Time Offset",
                }}
              />
            </Offset>
            <FaderGroupSep />
            <FaderGroup>
              <SynthFader label="Level" value={common.toneLevel} {...knobRange(sna_common_spec(SnaCommonField.ToneLevel))}
                onChange={set(SnaCommonField.ToneLevel, "toneLevel")} compact
                title="Tone Level" />
            </FaderGroup>
          </div>
        </SectionPanel>
    ),
  };
}

// ---------------------------------------------------------------------------
// Instrument Panel — dispatches to custom or generic
// ---------------------------------------------------------------------------

function InstrumentPanel({
  header,
  categoryName,
  paramDefs,
  modifyParams,
  onModify,
}: {
  /** Shown first in the box: the instrument picker. */
  header?: ReactNode;
  categoryName: string;
  paramDefs: InstParamDef[];
  modifyParams: number[];
  /** Sets Modify Parameter `index + 1`. */
  onModify: (index: number, value: number) => void;
}) {
  const setModParam = (paramIndex: number, value: number) => {
    // paramIndex is 1-based
    onModify(paramIndex - 1, value);
  };

  switch (categoryName) {
    case "TW Organ":
      return (
        <TWOrganPanel header={header} paramDefs={paramDefs} modifyParams={modifyParams} onChangeParam={setModParam} />
      );
    case "Ac.Piano":
      return (
        <AcPianoPanel header={header} paramDefs={paramDefs} modifyParams={modifyParams} onChangeParam={setModParam} />
      );
    default:
      return (
        <GenericInstPanel header={header}
          categoryName={categoryName}
          paramDefs={paramDefs}
          modifyParams={modifyParams}
          onChangeParam={setModParam}
        />
      );
  }
}

// ---------------------------------------------------------------------------
// Generic Instrument Panel
// ---------------------------------------------------------------------------

function GenericInstPanel({
  header,
  categoryName,
  paramDefs,
  modifyParams,
  onChangeParam,
}: {
  /** Shown first in the box: the instrument picker. */
  header?: ReactNode;
  categoryName: string;
  paramDefs: InstParamDef[];
  modifyParams: number[];
  onChangeParam: (paramIndex: number, value: number) => void;
}) {
  return (
    <SectionPanel label={categoryName} family={ParameterFamily.Pitch}>
      {header}
      <div className={css.instParamGrid}>
        {paramDefs.map((def) => {
          const val = modifyParams[def.index - 1] ?? def.defaultValue;
          const range = def.max - def.min;
          if (range <= 1) {
            return (
              <SynthSwitch
                key={def.index}
                label={def.name}
                value={val}
                options={[
                  { value: def.min, label: "OFF" },
                  { value: def.max, label: "ON" },
                ]}
                onChange={(v) => onChangeParam(def.index, v)}
              />
            );
          }
          return (
            <SynthKnob
              key={def.index}
              label={def.name}
              value={val}
              min={def.min}
              max={def.max}
              defaultValue={def.defaultValue}
              onChange={(v) => onChangeParam(def.index, v)}
              formatValue={(v) => String(v)}
              family={modifyParamFamily()}
            />
          );
        })}
      </div>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// TW Organ Panel — 9 harmonic bars as faders + percussion controls
// ---------------------------------------------------------------------------

const HARMONIC_BAR_LABELS = [
  "16'", "5-1/3'", "8'", "4'", "2-2/3'", "2'", "1-3/5'", "1-1/3'", "1'",
];

function TWOrganPanel({
  header,
  paramDefs,
  modifyParams,
  onChangeParam,
}: {
  /** Shown first in the box: the instrument picker. */
  header?: ReactNode;
  paramDefs: InstParamDef[];
  modifyParams: number[];
  onChangeParam: (paramIndex: number, value: number) => void;
}) {
  // Params 1-9: harmonic bars, 10: leakage, 11-22: percussion + click
  const barDefs = paramDefs.filter((d) => d.index >= 1 && d.index <= 9);
  const percDefs = paramDefs.filter((d) => d.index >= 10);

  return (
    <SectionPanel label="TW Organ" family={ParameterFamily.Pitch}>
      {header}
      {/* Harmonic bars */}
      <FaderGroup>
        {barDefs.map((def, i) => (
          <SynthFader
            key={def.index}
            label={HARMONIC_BAR_LABELS[i] ?? String(def.index)}
            value={modifyParams[def.index - 1] ?? def.defaultValue}
            min={def.min}
            max={def.max}
            defaultValue={def.defaultValue}
            onChange={(v) => onChangeParam(def.index, v)}
            compact
          />
        ))}
      </FaderGroup>

      {/* Percussion + other controls */}
      <div className={css.instParamGrid}>
        {percDefs.map((def) => {
          const val = modifyParams[def.index - 1] ?? def.defaultValue;
          const range = def.max - def.min;
          if (range <= 1) {
            return (
              <SynthSwitch
                key={def.index}
                label={def.name}
                value={val}
                options={[
                  { value: def.min, label: "OFF" },
                  { value: def.max, label: "ON" },
                ]}
                onChange={(v) => onChangeParam(def.index, v)}
              />
            );
          }
          return (
            <SynthKnob
              key={def.index}
              label={def.name}
              value={val}
              min={def.min}
              max={def.max}
              defaultValue={def.defaultValue}
              onChange={(v) => onChangeParam(def.index, v)}
              formatValue={(v) => String(v)}
              family={modifyParamFamily()}
            />
          );
        })}
      </div>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Ac.Piano Panel
// ---------------------------------------------------------------------------

function AcPianoPanel({
  header,
  paramDefs,
  modifyParams,
  onChangeParam,
}: {
  /** Shown first in the box: the instrument picker. */
  header?: ReactNode;
  paramDefs: InstParamDef[];
  modifyParams: number[];
  onChangeParam: (paramIndex: number, value: number) => void;
}) {
  return (
    <SectionPanel label="Ac.Piano" family={ParameterFamily.Pitch}>
      {header}
      <div className={css.instParamGrid}>
        {paramDefs.map((def) => {
          const val = modifyParams[def.index - 1] ?? def.defaultValue;
          return (
            <SynthKnob
              key={def.index}
              label={def.name}
              value={val}
              min={def.min}
              max={def.max}
              defaultValue={def.defaultValue}
              onChange={(v) => onChangeParam(def.index, v)}
              formatValue={(v) => String(v)}
              family={modifyParamFamily()}
            />
          );
        })}
      </div>
    </SectionPanel>
  );
}
