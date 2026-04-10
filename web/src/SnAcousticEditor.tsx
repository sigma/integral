import { useEffect, useState, useCallback, useMemo } from "react";
import {
  mfx_type_names,
  mfx_type_param_count,
  mfx_param_def,
  sna_inst_params_by_category,
} from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import {
  SynthKnob,
  SynthFader,
  SynthSwitch,
  SectionPanel,
  FaderGroup,
  FaderGroupSep,
} from "./synth-ui";
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

interface MfxState {
  mfxType: number;
  chorusSend: number;
  reverbSend: number;
  controls: { source: number; sens: number; assign: number }[];
  params: number[];
}

/** A single instrument parameter definition from the Rust table. */
interface InstParamDef {
  index: number;
  name: string;
  min: number;
  max: number;
  defaultValue: number;
}

// ---------------------------------------------------------------------------
// Address helpers
// ---------------------------------------------------------------------------

function snaBaseAddress(part: number): [number, number] {
  const partTotal = part * 0x20;
  const toneBase0 = 0x19 + Math.floor(partTotal / 128);
  const toneBase1 = partTotal % 128;
  // Add SN-A type offset (02 00 00) to byte1
  const snaBase1 = toneBase1 + 2;
  const carry = Math.floor(snaBase1 / 128);
  return [toneBase0 + carry, snaBase1 % 128];
}

function snaCommonAddress(part: number): number[] {
  const [b0, b1] = snaBaseAddress(part);
  return [b0, b1, 0x00, 0x00];
}

function snaMfxAddress(part: number): number[] {
  const [b0, b1] = snaBaseAddress(part);
  return [b0, b1, 0x02, 0x00];
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

// MFX control source names (same as SN-S editor)
const CTRL_SOURCE_NAMES: string[] = (() => {
  const names = ["OFF"];
  for (let i = 1; i <= 31; i++) names.push(`CC${String(i).padStart(2, "0")}`);
  for (let i = 33; i <= 95; i++) names.push(`CC${String(i).padStart(2, "0")}`);
  names.push("BEND", "AFT", "SYS1", "SYS2", "SYS3", "SYS4");
  return names;
})();

let _typeNamesCache: string[] | null = null;
function getMfxTypeNames(): string[] {
  if (!_typeNamesCache) _typeNamesCache = mfx_type_names();
  return _typeNamesCache;
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
          snaCommonAddress(partIndex),
          [0x00, 0x00, 0x00, 0x46],
        );
        if (cancelled) return;
        const c = service.device.applySnaCommon(commonData) as SnAcousticCommon | null;
        if (c) setCommon(c);

        // MFX
        const mfxData = await service.requestData(
          snaMfxAddress(partIndex),
          [0x00, 0x00, 0x01, 0x11],
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
  // Common param setter
  // ---------------------------------------------------------------------------
  const setCommonParam = useCallback(
    (offset: number, value: number) => {
      service.device.setSnaCommonParam(partIndex, offset, value);
      setCommon((prev) => {
        if (!prev) return prev;
        const next = { ...prev };
        switch (offset) {
          case 0x10: next.toneLevel = value; break;
          case 0x11: next.monoPoly = value; break;
          case 0x12: next.portamentoTimeOffset = value; break;
          case 0x13: next.cutoffOffset = value; break;
          case 0x14: next.resonanceOffset = value; break;
          case 0x15: next.attackTimeOffset = value; break;
          case 0x16: next.releaseTimeOffset = value; break;
          case 0x17: next.vibratoRate = value; break;
          case 0x18: next.vibratoDepth = value; break;
          case 0x19: next.vibratoDelay = value; break;
          case 0x1A: next.octaveShift = value; break;
          default: {
            // Modify parameters: offsets 0x22..0x41
            if (offset >= 0x22 && offset <= 0x41) {
              const mp = [...next.modifyParams];
              mp[offset - 0x22] = value;
              next.modifyParams = mp;
            }
            break;
          }
        }
        return next;
      });
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

  // ---------------------------------------------------------------------------
  // MFX callbacks
  // ---------------------------------------------------------------------------
  const handleMfxTypeChange = useCallback(
    (type_: number) => {
      service.device.setMfxParam(partIndex, 0x00, type_);
      setMfx((prev) => prev ? { ...prev, mfxType: type_, params: [] } : prev);
    },
    [partIndex, service],
  );

  const handleMfxHeaderParam = useCallback(
    (offset: number, value: number) => {
      service.device.setMfxParam(partIndex, offset, value);
      setMfx((prev) => {
        if (!prev) return prev;
        const next = { ...prev, controls: [...prev.controls] };
        if (offset === 0x02) next.chorusSend = value;
        else if (offset === 0x03) next.reverbSend = value;
        for (let s = 0; s < 4; s++) {
          if (offset === 0x05 + s * 2) {
            next.controls[s] = { ...next.controls[s]!, source: value };
          } else if (offset === 0x06 + s * 2) {
            next.controls[s] = { ...next.controls[s]!, sens: value };
          } else if (offset === 0x0D + s) {
            next.controls[s] = { ...next.controls[s]!, assign: value };
          }
        }
        return next;
      });
    },
    [partIndex, service],
  );

  const handleMfxNibParam = useCallback(
    (paramIndex: number, value: number) => {
      service.device.setMfxNibParam(partIndex, paramIndex, value);
      setMfx((prev) => {
        if (!prev) return prev;
        const params = [...prev.params];
        params[paramIndex] = value;
        return { ...prev, params };
      });
    },
    [partIndex, service],
  );

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

      {/* Three boxes side by side: Common | Instrument | MFX */}
      <div className={css.mainArea}>
        {common && <CommonPanel common={common} onChange={setCommonParam} />}
        {common && categoryName && (
          <InstrumentPanel
            categoryName={categoryName}
            paramDefs={instParamDefs}
            modifyParams={common.modifyParams}
            onChange={setCommonParam}
          />
        )}
        {mfx && (
          <MfxPanel
            mfx={mfx}
            onTypeChange={handleMfxTypeChange}
            onHeaderParam={handleMfxHeaderParam}
            onNibParam={handleMfxNibParam}
          />
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Common Controls Strip
// ---------------------------------------------------------------------------

function CommonPanel({
  common,
  onChange,
}: {
  common: SnAcousticCommon;
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="COMMON" accentColor="#8cf">
      <div className={css.panelRow}>
        <SynthSwitch label="Mono/Poly" value={common.monoPoly} options={MONO_POLY_OPTIONS}
          onChange={(v) => onChange(0x11, v)} />
        <SynthKnob label="Oct" value={common.octaveShift} min={61} max={67} defaultValue={64}
          onChange={(v) => onChange(0x1A, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cf"
          title="Octave Shift" />
        <SynthKnob label="Cutoff" value={common.cutoffOffset} min={0} max={127} defaultValue={64}
          onChange={(v) => onChange(0x13, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Cutoff Offset" />
        <SynthKnob label="Reso" value={common.resonanceOffset} min={0} max={127} defaultValue={64}
          onChange={(v) => onChange(0x14, v)} formatValue={(v) => signedFmt(v, 64)} color="#68c"
          title="Resonance Offset" />
      </div>
      <div className={css.panelRow}>
        <SynthKnob label="Port" value={common.portamentoTimeOffset} min={0} max={127} defaultValue={64}
          onChange={(v) => onChange(0x12, v)} formatValue={(v) => signedFmt(v, 64)} color="#8cf"
          title="Portamento Time Offset" />
        <SynthKnob label="Vib Rt" value={common.vibratoRate} min={0} max={127} defaultValue={64}
          onChange={(v) => onChange(0x17, v)} formatValue={(v) => signedFmt(v, 64)} color="#a6f"
          title="Vibrato Rate" />
        <SynthKnob label="Vib Dp" value={common.vibratoDepth} min={0} max={127} defaultValue={64}
          onChange={(v) => onChange(0x18, v)} formatValue={(v) => signedFmt(v, 64)} color="#a6f"
          title="Vibrato Depth" />
        <SynthKnob label="Vib Dl" value={common.vibratoDelay} min={0} max={127} defaultValue={64}
          onChange={(v) => onChange(0x19, v)} formatValue={(v) => signedFmt(v, 64)} color="#a6f"
          title="Vibrato Delay" />
      </div>
      <FaderGroup>
        <SynthFader label="Level" value={common.toneLevel} min={0} max={127} defaultValue={127}
          onChange={(v) => onChange(0x10, v)} formatValue={(v) => String(v)} compact
          title="Tone Level" />
        <FaderGroupSep />
        <SynthFader label="Atk" value={common.attackTimeOffset} min={0} max={127} defaultValue={64}
          onChange={(v) => onChange(0x15, v)} formatValue={(v) => signedFmt(v, 64)} compact
          title="Attack Time Offset" />
        <SynthFader label="Rel" value={common.releaseTimeOffset} min={0} max={127} defaultValue={64}
          onChange={(v) => onChange(0x16, v)} formatValue={(v) => signedFmt(v, 64)} compact
          title="Release Time Offset" />
      </FaderGroup>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Instrument Panel — dispatches to custom or generic
// ---------------------------------------------------------------------------

function InstrumentPanel({
  categoryName,
  paramDefs,
  modifyParams,
  onChange,
}: {
  categoryName: string;
  paramDefs: InstParamDef[];
  modifyParams: number[];
  onChange: (offset: number, value: number) => void;
}) {
  const setModParam = (paramIndex: number, value: number) => {
    // paramIndex is 1-based; offset = 0x22 + (paramIndex - 1)
    onChange(0x22 + paramIndex - 1, value);
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

// ---------------------------------------------------------------------------
// MFX Panel (same pattern as SN-S editor)
// ---------------------------------------------------------------------------

function MfxPanel({
  mfx,
  onTypeChange,
  onHeaderParam,
  onNibParam,
}: {
  mfx: MfxState;
  onTypeChange: (type_: number) => void;
  onHeaderParam: (offset: number, value: number) => void;
  onNibParam: (paramIndex: number, value: number) => void;
}) {
  const paramDefs = useMemo(() => {
    const count = mfx_type_param_count(mfx.mfxType);
    const defs = [];
    for (let i = 0; i < count; i++) {
      const d = mfx_param_def(mfx.mfxType, i);
      if (d) {
        defs.push({ index: d.index, name: d.name, min: d.min, max: d.max, defaultValue: d.defaultValue });
        d.free();
      }
    }
    return defs;
  }, [mfx.mfxType]);

  return (
    <div className={css.mfxPanel}>
      <div className={css.panelHeader}>MFX</div>
      <div className={css.panelBody}>
        {/* Type selector */}
        <div className={css.mfxTypeRow}>
          <span className={css.mfxTypeLabel}>Type</span>
          <select
            className={css.mfxTypeSelect}
            value={mfx.mfxType}
            onChange={(e) => onTypeChange(Number(e.target.value))}
          >
            {getMfxTypeNames().map((name, i) => (
              <option key={i} value={i}>{i}: {name}</option>
            ))}
          </select>
        </div>

        {/* Chorus / Reverb sends */}
        <div className={css.mfxSendRow}>
          <SynthKnob label="Cho Send" value={mfx.chorusSend} min={0} max={127} defaultValue={0}
            onChange={(v) => onHeaderParam(0x02, v)} formatValue={(v) => String(v)} color="#668" />
          <SynthKnob label="Rev Send" value={mfx.reverbSend} min={0} max={127} defaultValue={0}
            onChange={(v) => onHeaderParam(0x03, v)} formatValue={(v) => String(v)} color="#686" />
        </div>

        {/* Dynamic parameters */}
        {paramDefs.length > 0 && (
          <div className={css.mfxParamGrid}>
            {paramDefs.map((def, i) => {
              const val = mfx.params[i] ?? def.defaultValue;
              const range = def.max - def.min;
              if (range <= 1) {
                return (
                  <SynthSwitch
                    key={`${mfx.mfxType}-${def.index}`}
                    label={def.name}
                    value={val}
                    options={[
                      { value: def.min, label: "OFF" },
                      { value: def.max, label: "ON" },
                    ]}
                    onChange={(v) => onNibParam(i, v)}
                  />
                );
              }
              return (
                <SynthKnob
                  key={`${mfx.mfxType}-${def.index}`}
                  label={def.name}
                  value={val}
                  min={def.min}
                  max={def.max}
                  defaultValue={def.defaultValue}
                  onChange={(v) => onNibParam(i, v)}
                  formatValue={(v) => String(v)}
                  color="#c8a"
                />
              );
            })}
          </div>
        )}

        {/* MFX Control (4 slots) */}
        <div className={css.mfxCtrlSection}>
          <span className={css.mfxCtrlTitle}>MFX Control</span>
          <div className={css.mfxCtrlSlots}>
            {[0, 1, 2, 3].map((slot) => {
              const ctrl = mfx.controls[slot] ?? { source: 0, sens: 64, assign: 0 };
              return (
                <div key={slot} className={css.mfxCtrlSlot}>
                  <span className={css.mfxCtrlSlotLabel}>{slot + 1}</span>
                  <select
                    className={css.mfxCtrlSelect}
                    value={ctrl.source}
                    onChange={(e) => onHeaderParam(0x05 + slot * 2, Number(e.target.value))}
                    title={`Source ${slot + 1}`}
                  >
                    {CTRL_SOURCE_NAMES.map((name, i) => (
                      <option key={i} value={i}>{name}</option>
                    ))}
                  </select>
                  <SynthKnob
                    label="Depth"
                    value={ctrl.sens}
                    min={1}
                    max={127}
                    defaultValue={64}
                    onChange={(v) => onHeaderParam(0x06 + slot * 2, v)}
                    formatValue={(v) => String(v - 64)}
                    color="#8ac"
                  />
                  <select
                    className={css.mfxCtrlSelect}
                    value={ctrl.assign}
                    onChange={(e) => onHeaderParam(0x0D + slot, Number(e.target.value))}
                    title={`Assign ${slot + 1}`}
                  >
                    <option value={0}>OFF</option>
                    {paramDefs.map((def) => (
                      <option key={def.index} value={def.index}>{def.name}</option>
                    ))}
                  </select>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
