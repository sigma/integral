import { useEffect, useState, useCallback, useMemo, type ReactNode } from "react";
import {
  sna_inst_params_by_category,
  sna_has_portamento_offset,
  sna_has_tone_offsets,
  sna_common_address,
  sna_common_size,
  sna_mfx_address,
  mfx_block_size,
  SnaCommonField,
  ToneType,
} from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import {
  SynthKnob,
  SynthFader,
  SynthSwitch,
  SectionPanel,
  FaderGroup,
  ADSREnvelope,
} from "./synth-ui";
import { MfxPanel } from "./MfxPanel";
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

/** Category index to instrument type name mapping. */
const CATEGORY_NAMES: string[] = [
  "Ac.Piano",      // 0
  "E.Piano",       // 1
  "Clav",          // 2
  "Bell/Mallet",   // 3
  "TW Organ",      // 4
  "Accordion",     // 5
  "Harmonica",     // 6
  "Ac.Guitar",     // 7
  "Mandolin",      // 8
  "E.Guitar",      // 9
  "Ac.Bass",       // 10
  "E.Bass",        // 11
  "Strings Solo",  // 12
  "Strings Ensemble", // 13
  "Strings Erhu",  // 14
  "Strings Sarangi", // 15
  "Harp",          // 16
  "Sitar",         // 17
  "Shamisen",      // 18
  "Koto",          // 19
  "Taishou Koto",  // 20
  "Kalimba",       // 21
  "Vox/Choir",     // 22
  "Brass",         // 23
  "Wind",          // 24
  "Wind Pipes",    // 25
  "Flute",         // 26
  "Flute Ethnic",  // 27
  "Sax",           // 28
  "Recorder",      // 29
  "Ocarina",       // 30
  "Timpani",       // 31
  "Steel Drums",   // 32
];

const MONO_POLY_OPTIONS = [{ value: 0, label: "MONO" }, { value: 1, label: "POLY" }];

function signedFmt(raw: number, center: number): string {
  const v = raw - center;
  return v > 0 ? `+${v}` : String(v);
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

  // ---------------------------------------------------------------------------
  // Instrument parameters
  // ---------------------------------------------------------------------------
  const categoryName = common ? (CATEGORY_NAMES[common.category] ?? null) : null;

  const instParamDefs = useMemo(() => {
    if (!categoryName) return [];
    const raw = sna_inst_params_by_category(categoryName);
    return parseInstParams(raw);
  }, [categoryName]);

  const mfxHandlers = useMfxHandlers(service, partIndex, ToneType.SnAcoustic, setMfx);

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  if (loading) {
    return <div className={css.loading}>Loading SN-A tone data...</div>;
  }

  return (
    <div className={css.editor}>
      {/* Tone name */}
      {common && (
        <div className={css.toneName}>{common.toneName || "(unnamed)"}</div>
      )}

      {/* Side by side: the tone-wide boxes, the instrument, then the MFX */}
      <div className={css.mainArea}>
        {common && <CommonPanels common={common} onChange={setCommonParam} />}
        {common && categoryName && (
          <InstrumentPanel
            categoryName={categoryName}
            paramDefs={instParamDefs}
            modifyParams={common.modifyParams}
            onModify={setModifyParam}
          />
        )}
        {mfx && (
          <MfxPanel
            mfx={mfx}
            {...mfxHandlers}
          />
        )}
      </div>
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

function CommonPanels({
  common,
  onChange,
}: {
  common: SnAcousticCommon;
  onChange: CommonParamHandler;
}) {
  const set =
    (field: SnaCommonField, key: Parameters<CommonParamHandler>[1]) =>
    (v: number) =>
      onChange(field, key, v);
  const offset = (v: number) => signedFmt(v, 64);
  // Some instruments ignore some offsets (Parameter Guide): show them disabled.
  const toneOffsets = sna_has_tone_offsets(common.instVariation, common.instNumber);
  const portamento = sna_has_portamento_offset(common.instVariation, common.instNumber);
  return (
    <>
      <SectionPanel label="COMMON" accentColor="#8cf">
        <div className={css.panelRow}>
          <SynthSwitch label="Mono/Poly" value={common.monoPoly} options={MONO_POLY_OPTIONS}
            onChange={set(SnaCommonField.MonoPoly, "monoPoly")} />
          <SynthKnob label="Oct" value={common.octaveShift} min={61} max={67} defaultValue={64} center={64}
            onChange={set(SnaCommonField.OctaveShift, "octaveShift")} formatValue={offset} color="#8cf"
            title="Octave Shift" />
          <Offset enabled={portamento}>
            <SynthKnob label="Port" value={common.portamentoTimeOffset} min={0} max={127} defaultValue={64} center={64}
              onChange={set(SnaCommonField.PortamentoTimeOffset, "portamentoTimeOffset")} formatValue={offset} color="#8cf"
              title="Portamento Time Offset" />
          </Offset>
        </div>
      </SectionPanel>

      <SectionPanel label="VIBRATO" accentColor="#a6f">
        <div className={css.panelRow}>
          <SynthKnob label="Rate" value={common.vibratoRate} min={0} max={127} defaultValue={64} center={64}
            onChange={set(SnaCommonField.VibratoRate, "vibratoRate")} formatValue={offset} color="#a6f"
            title="Vibrato Rate (offset)" />
          <SynthKnob label="Depth" value={common.vibratoDepth} min={0} max={127} defaultValue={64} center={64}
            onChange={set(SnaCommonField.VibratoDepth, "vibratoDepth")} formatValue={offset} color="#a6f"
            title="Vibrato Depth (offset)" />
          <SynthKnob label="Delay" value={common.vibratoDelay} min={0} max={127} defaultValue={64} center={64}
            onChange={set(SnaCommonField.VibratoDelay, "vibratoDelay")} formatValue={offset} color="#a6f"
            title="Vibrato Delay (offset)" />
        </div>
      </SectionPanel>

      <SectionPanel label="FILTER" accentColor="#68c">
        <Offset enabled={toneOffsets}>
          <div className={css.panelRow}>
            <SynthKnob label="Cutoff" value={common.cutoffOffset} min={0} max={127} defaultValue={64} center={64}
              onChange={set(SnaCommonField.CutoffOffset, "cutoffOffset")} formatValue={offset} color="#68c"
              title="Cutoff Offset" />
            <SynthKnob label="Reso" value={common.resonanceOffset} min={0} max={127} defaultValue={64} center={64}
              onChange={set(SnaCommonField.ResonanceOffset, "resonanceOffset")} formatValue={offset} color="#68c"
              title="Resonance Offset" />
          </div>
        </Offset>
      </SectionPanel>

      <SectionPanel label="AMP" accentColor="#6c8">
        <div className={css.ampRow}>
          <FaderGroup>
            <SynthFader label="Level" value={common.toneLevel} min={0} max={127} defaultValue={127}
              onChange={set(SnaCommonField.ToneLevel, "toneLevel")} formatValue={(v) => String(v)} compact
              title="Tone Level" />
          </FaderGroup>
          <Offset enabled={toneOffsets}>
            <ADSREnvelope
              compact
              attack={{
                label: "Atk", value: common.attackTimeOffset, min: 0, max: 127, defaultValue: 64,
                onChange: set(SnaCommonField.AttackTimeOffset, "attackTimeOffset"), formatValue: offset,
                title: "Attack Time Offset",
              }}
              release={{
                label: "Rel", value: common.releaseTimeOffset, min: 0, max: 127, defaultValue: 64,
                onChange: set(SnaCommonField.ReleaseTimeOffset, "releaseTimeOffset"), formatValue: offset,
                title: "Release Time Offset",
              }}
            />
          </Offset>
        </div>
      </SectionPanel>

    </>
  );
}

// ---------------------------------------------------------------------------
// Instrument Panel — dispatches to custom or generic
// ---------------------------------------------------------------------------

function InstrumentPanel({
  categoryName,
  paramDefs,
  modifyParams,
  onModify,
}: {
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
        <TWOrganPanel paramDefs={paramDefs} modifyParams={modifyParams} onChangeParam={setModParam} />
      );
    case "Ac.Piano":
      return (
        <AcPianoPanel paramDefs={paramDefs} modifyParams={modifyParams} onChangeParam={setModParam} />
      );
    default:
      return (
        <GenericInstPanel
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
  categoryName,
  paramDefs,
  modifyParams,
  onChangeParam,
}: {
  categoryName: string;
  paramDefs: InstParamDef[];
  modifyParams: number[];
  onChangeParam: (paramIndex: number, value: number) => void;
}) {
  return (
    <SectionPanel label={categoryName} accentColor="#fc8">
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
              color="#fc8"
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
  paramDefs,
  modifyParams,
  onChangeParam,
}: {
  paramDefs: InstParamDef[];
  modifyParams: number[];
  onChangeParam: (paramIndex: number, value: number) => void;
}) {
  // Params 1-9: harmonic bars, 10: leakage, 11-22: percussion + click
  const barDefs = paramDefs.filter((d) => d.index >= 1 && d.index <= 9);
  const percDefs = paramDefs.filter((d) => d.index >= 10);

  return (
    <SectionPanel label="TW Organ" accentColor="#fc8">
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
              color="#fc8"
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
  paramDefs,
  modifyParams,
  onChangeParam,
}: {
  paramDefs: InstParamDef[];
  modifyParams: number[];
  onChangeParam: (paramIndex: number, value: number) => void;
}) {
  return (
    <SectionPanel label="Ac.Piano" accentColor="#8cf">
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
              color="#8cf"
            />
          );
        })}
      </div>
    </SectionPanel>
  );
}
