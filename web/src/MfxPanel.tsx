import { useMemo } from "react";
import {
  mfx_type_names,
  mfx_type_param_count,
  mfx_param_def,
} from "../pkg/integral_wasm.js";
import {
  SynthKnob,
  SynthFader,
  SynthSwitch,
} from "./synth-ui";
import css from "./MfxPanel.module.css";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export interface MfxCtrl {
  source: number;
  sens: number;
  assign: number;
}

export interface MfxState {
  mfxType: number;
  chorusSend: number;
  reverbSend: number;
  controls: MfxCtrl[];
  params: number[];
}

interface Props {
  mfx: MfxState;
  onTypeChange: (type_: number) => void;
  onHeaderParam: (offset: number, value: number) => void;
  onNibParam: (paramIndex: number, value: number) => void;
}

// ---------------------------------------------------------------------------
// Cached MFX type names
// ---------------------------------------------------------------------------

let _typeNamesCache: string[] | null = null;
function getMfxTypeNames(): string[] {
  if (!_typeNamesCache) _typeNamesCache = mfx_type_names();
  return _typeNamesCache;
}

// ---------------------------------------------------------------------------
// MFX control source names
// ---------------------------------------------------------------------------

const CTRL_SOURCE_NAMES: string[] = (() => {
  const names = ["OFF"];
  for (let i = 1; i <= 31; i++) names.push(`CC${String(i).padStart(2, "0")}`);
  for (let i = 33; i <= 95; i++) names.push(`CC${String(i).padStart(2, "0")}`);
  names.push("BEND", "AFT", "SYS1", "SYS2", "SYS3", "SYS4");
  return names;
})();

// ---------------------------------------------------------------------------
// Param definition helper
// ---------------------------------------------------------------------------

interface ParamDef {
  index: number;
  name: string;
  min: number;
  max: number;
  defaultValue: number;
}

function useParamDefs(mfxType: number): ParamDef[] {
  return useMemo(() => {
    const count = mfx_type_param_count(mfxType);
    const defs: ParamDef[] = [];
    for (let i = 0; i < count; i++) {
      const d = mfx_param_def(mfxType, i);
      if (d) {
        defs.push({ index: d.index, name: d.name, min: d.min, max: d.max, defaultValue: d.defaultValue });
        d.free();
      }
    }
    return defs;
  }, [mfxType]);
}

// ---------------------------------------------------------------------------
// Custom panel sub-props
// ---------------------------------------------------------------------------

interface CustomPanelProps {
  params: number[];
  paramDefs: ParamDef[];
  onParam: (index: number, value: number) => void;
}

// ---------------------------------------------------------------------------
// Type 1: Equalizer
// ---------------------------------------------------------------------------
// Params: 0=LowFreq(0-1) 1=LowGain(-15..15) 2=Mid1Freq(200-8000)
//         3=Mid1Gain(-15..15) 4=Mid1Q(0-4) 5=Mid2Freq(200-8000)
//         6=Mid2Gain(-15..15) 7=Mid2Q(0-4) 8=HighFreq(0-2)
//         9=HighGain(-15..15) 10=Level(0-127)

const Q_LABELS = ["0.5", "1.0", "2.0", "4.0", "8.0"];
const LOW_FREQ_OPTIONS = [{ value: 0, label: "200Hz" }, { value: 1, label: "400Hz" }];
const HIGH_FREQ_OPTIONS = [
  { value: 0, label: "2kHz" },
  { value: 1, label: "4kHz" },
  { value: 2, label: "8kHz" },
];

function signedGain(v: number): string {
  return v > 0 ? `+${v}` : String(v);
}

function EqualizerPanel({ params, paramDefs, onParam }: CustomPanelProps) {
  const p = (i: number) => params[i] ?? paramDefs[i]?.defaultValue ?? 0;
  const fmtHiFreq = (v: number) => HIGH_FREQ_OPTIONS[v]?.label ?? String(v);
  const fmtLoFreq = (v: number) => LOW_FREQ_OPTIONS[v]?.label ?? String(v);
  return (
    <div className={css.eqBands}>
      {/* HI band */}
      <div className={css.eqBandRow}>
        <span className={css.eqBandTag} style={{ color: "#c66" }}>HI</span>
        <div className={css.eqKnobs}>
          <SynthKnob label="Gain" value={p(9)} min={-15} max={15} defaultValue={0}
            onChange={(v) => onParam(9, v)} formatValue={signedGain} color="#c66" />
          <SynthKnob label="Freq" value={p(8)} min={0} max={2} defaultValue={1}
            onChange={(v) => onParam(8, v)} formatValue={fmtHiFreq} color="#c66" title="High Frequency" />
        </div>
      </div>
      {/* MID2 band */}
      <div className={css.eqBandRow}>
        <span className={css.eqBandTag} style={{ color: "#cc6" }}>MID2</span>
        <div className={css.eqKnobs}>
          <SynthKnob label="Gain" value={p(6)} min={-15} max={15} defaultValue={0}
            onChange={(v) => onParam(6, v)} formatValue={signedGain} color="#cc6" />
          <SynthKnob label="Freq" value={p(5)} min={200} max={8000} defaultValue={2000}
            onChange={(v) => onParam(5, v)} formatValue={(v) => `${v}`} color="#cc6" title="Mid2 Frequency" />
          <SynthKnob label="Q" value={p(7)} min={0} max={4} defaultValue={2}
            onChange={(v) => onParam(7, v)} formatValue={(v) => Q_LABELS[v] ?? String(v)} color="#cc6" />
        </div>
      </div>
      {/* MID1 band */}
      <div className={css.eqBandRow}>
        <span className={css.eqBandTag} style={{ color: "#6c6" }}>MID1</span>
        <div className={css.eqKnobs}>
          <SynthKnob label="Gain" value={p(3)} min={-15} max={15} defaultValue={0}
            onChange={(v) => onParam(3, v)} formatValue={signedGain} color="#6c6" />
          <SynthKnob label="Freq" value={p(2)} min={200} max={8000} defaultValue={1000}
            onChange={(v) => onParam(2, v)} formatValue={(v) => `${v}`} color="#6c6" title="Mid1 Frequency" />
          <SynthKnob label="Q" value={p(4)} min={0} max={4} defaultValue={2}
            onChange={(v) => onParam(4, v)} formatValue={(v) => Q_LABELS[v] ?? String(v)} color="#6c6" />
        </div>
      </div>
      {/* LO band */}
      <div className={css.eqBandRow}>
        <span className={css.eqBandTag} style={{ color: "#66c" }}>LO</span>
        <div className={css.eqKnobs}>
          <SynthKnob label="Gain" value={p(1)} min={-15} max={15} defaultValue={0}
            onChange={(v) => onParam(1, v)} formatValue={signedGain} color="#66c" />
          <SynthKnob label="Freq" value={p(0)} min={0} max={1} defaultValue={0}
            onChange={(v) => onParam(0, v)} formatValue={fmtLoFreq} color="#66c" title="Low Frequency" />
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Type 31: Compressor
// ---------------------------------------------------------------------------
// Params: 0=Attack(0-127) 1=Threshold(0-127) 2=PostGain(0-18)
//         3=LowGain(-15..15) 4=HighGain(-15..15) 5=Level(0-127)

function CompressorPanel({ params, paramDefs, onParam }: CustomPanelProps) {
  const p = (i: number) => params[i] ?? paramDefs[i]?.defaultValue ?? 0;
  return (
    <div className={css.customLayout}>
      <div className={css.customRow}>
        <SynthKnob label="Lo Gain" value={p(3)} min={-15} max={15} defaultValue={0}
          onChange={(v) => onParam(3, v)} formatValue={signedGain} color="#6ae"
          title="Low Gain" />
        <SynthKnob label="Hi Gain" value={p(4)} min={-15} max={15} defaultValue={0}
          onChange={(v) => onParam(4, v)} formatValue={signedGain} color="#6ae"
          title="High Gain" />
      </div>
      <div className={css.customRow}>
        <SynthFader label="Atk" value={p(0)} min={0} max={127} defaultValue={64}
          onChange={(v) => onParam(0, v)} formatValue={(v) => String(v)} compact
          title="Attack" />
        <SynthFader label="Thr" value={p(1)} min={0} max={127} defaultValue={64}
          onChange={(v) => onParam(1, v)} formatValue={(v) => String(v)} compact
          title="Threshold" />
        <SynthFader label="Gain" value={p(2)} min={0} max={18} defaultValue={0}
          onChange={(v) => onParam(2, v)} formatValue={(v) => `+${v}dB`} compact
          title="Post Gain (Make-up)" />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Type 22: Chorus
// ---------------------------------------------------------------------------
// Params: 0=FilterType(0-2) 1=CutoffFreq(200-8000) 2=PreDelay(0-1000)
//         3=Rate(0-127) 4=Depth(0-127) 5=Phase(0-180)
//         6=LowGain(-15..15) 7=HighGain(-15..15) 8=Balance(0-100) 9=Level(0-127)

const CHORUS_FILTER_OPTIONS = [
  { value: 0, label: "OFF" },
  { value: 1, label: "LPF" },
  { value: 2, label: "HPF" },
];

function ChorusPanel({ params, paramDefs, onParam }: CustomPanelProps) {
  const p = (i: number) => params[i] ?? paramDefs[i]?.defaultValue ?? 0;
  return (
    <div className={css.customLayout}>
      <div className={css.customRow}>
        <SynthSwitch label="Filter" value={p(0)} options={CHORUS_FILTER_OPTIONS}
          onChange={(v) => onParam(0, v)} vertical={true} />
        <SynthKnob label="Cutoff" value={p(1)} min={200} max={8000} defaultValue={800}
          onChange={(v) => onParam(1, v)} formatValue={(v) => `${v}`} color="#ea6" />
      </div>
      <div className={css.customRow}>
        <SynthKnob label="Rate" value={p(3)} min={0} max={127} defaultValue={64}
          onChange={(v) => onParam(3, v)} formatValue={(v) => String(v)} color="#6ae" />
        <SynthKnob label="Depth" value={p(4)} min={0} max={127} defaultValue={64}
          onChange={(v) => onParam(4, v)} formatValue={(v) => String(v)} color="#6ae" />
      </div>
      <div className={css.customRow}>
        <SynthKnob label="Pre Dly" value={p(2)} min={0} max={1000} defaultValue={0}
          onChange={(v) => onParam(2, v)} formatValue={(v) => `${(v / 10).toFixed(1)}`} color="#a8e"
          title="Pre Delay" />
        <SynthKnob label="Phase" value={p(5)} min={0} max={180} defaultValue={0}
          onChange={(v) => onParam(5, v)} formatValue={(v) => `${v}\u00B0`} color="#ea6" />
      </div>
      <div className={css.customRow}>
        <SynthKnob label="Balance" value={p(8)} min={0} max={100} defaultValue={50}
          onChange={(v) => onParam(8, v)} formatValue={(v) => `D${100 - v}:E${v}`} color="#c8a"
          title="Dry/Effect Balance" />
      </div>
      <div className={css.customRow}>
        <SynthKnob label="Lo Gain" value={p(6)} min={-15} max={15} defaultValue={0}
          onChange={(v) => onParam(6, v)} formatValue={signedGain} color="#6ae"
          title="Low Gain" />
        <SynthKnob label="Hi Gain" value={p(7)} min={-15} max={15} defaultValue={0}
          onChange={(v) => onParam(7, v)} formatValue={signedGain} color="#6ae"
          title="High Gain" />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Type 28/29: Overdrive / Distortion
// ---------------------------------------------------------------------------
// Params: 0=Drive(0-127) 1=Tone(0-127) 2=AmpSw(0-1) 3=AmpType(0-3)
//         4=LowGain(-15..15) 5=HighGain(-15..15) 6=Pan(-64..63) 7=Level(0-127)

const AMP_TYPE_OPTIONS = [
  { value: 0, label: "Small" },
  { value: 1, label: "Built-In" },
  { value: 2, label: "2-Stack" },
  { value: 3, label: "3-Stack" },
];

function OverdrivePanel({ params, paramDefs, onParam }: CustomPanelProps) {
  const p = (i: number) => params[i] ?? paramDefs[i]?.defaultValue ?? 0;
  return (
    <div className={css.customLayout}>
      <div className={css.customRow}>
        <SynthKnob label="Drive" value={p(0)} min={0} max={127} defaultValue={64}
          onChange={(v) => onParam(0, v)} formatValue={(v) => String(v)} color="#f64" />
        <SynthKnob label="Tone" value={p(1)} min={0} max={127} defaultValue={64}
          onChange={(v) => onParam(1, v)} formatValue={(v) => String(v)} color="#ea6" />
      </div>
      <div className={css.customRow}>
        <SynthSwitch label="Amp" value={p(2)}
          options={[{ value: 0, label: "OFF" }, { value: 1, label: "ON" }]}
          onChange={(v) => onParam(2, v)} />
        <SynthSwitch label="Type" value={p(3)} options={AMP_TYPE_OPTIONS}
          onChange={(v) => onParam(3, v)} vertical={true} title="Amp Type" />
      </div>
      <div className={css.customRow}>
        <SynthKnob label="Pan" value={p(6)} min={-64} max={63} defaultValue={0}
          onChange={(v) => onParam(6, v)} formatValue={(v) => v === 0 ? "C" : v < 0 ? `L${-v}` : `R${v}`} color="#c8a" />
      </div>
      <div className={css.customRow}>
        <SynthKnob label="Lo Gain" value={p(4)} min={-15} max={15} defaultValue={0}
          onChange={(v) => onParam(4, v)} formatValue={signedGain} color="#6ae" title="Low Gain" />
        <SynthKnob label="Hi Gain" value={p(5)} min={-15} max={15} defaultValue={0}
          onChange={(v) => onParam(5, v)} formatValue={signedGain} color="#6ae" title="High Gain" />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Type 34: Delay
// ---------------------------------------------------------------------------
// Params: 0=DelayLeft(0-1300) 1=DelayRight(0-1300) 2=PhaseL(0-1)
//         3=PhaseR(0-1) 4=FeedbackMode(0-1) 5=Feedback(-98..98)
//         6=HFDamp(200-8000) 7=LowGain(-15..15) 8=HighGain(-15..15)
//         9=Balance(0-100) 10=Level(0-127)

const FB_MODE_OPTIONS = [{ value: 0, label: "Normal" }, { value: 1, label: "Cross" }];

function DelayPanel({ params, paramDefs, onParam }: CustomPanelProps) {
  const p = (i: number) => params[i] ?? paramDefs[i]?.defaultValue ?? 0;
  return (
    <div className={css.customLayout}>
      <div className={css.customRow}>
        <SynthSwitch label="Ph L" value={p(2)}
          options={[{ value: 0, label: "NOR" }, { value: 1, label: "INV" }]}
          onChange={(v) => onParam(2, v)} title="Phase Left" />
        <SynthSwitch label="Ph R" value={p(3)}
          options={[{ value: 0, label: "NOR" }, { value: 1, label: "INV" }]}
          onChange={(v) => onParam(3, v)} title="Phase Right" />
        <SynthSwitch label="FB" value={p(4)} options={FB_MODE_OPTIONS}
          onChange={(v) => onParam(4, v)} title="Feedback Mode" />
      </div>
      <div className={css.customRow}>
        <SynthKnob label="Fdbk" value={p(5)} min={-98} max={98} defaultValue={0}
          onChange={(v) => onParam(5, v)} formatValue={(v) => `${v}%`} color="#e86"
          title="Feedback" />
        <SynthKnob label="HF Damp" value={p(6)} min={200} max={8000} defaultValue={8000}
          onChange={(v) => onParam(6, v)} formatValue={(v) => `${v}`} color="#ea6" />
        <SynthKnob label="Balance" value={p(9)} min={0} max={100} defaultValue={50}
          onChange={(v) => onParam(9, v)} formatValue={(v) => `D${100 - v}:E${v}`} color="#c8a"
          title="Dry/Effect Balance" />
      </div>
      <div className={css.customRow}>
        <SynthKnob label="Lo Gain" value={p(7)} min={-15} max={15} defaultValue={0}
          onChange={(v) => onParam(7, v)} formatValue={signedGain} color="#6ae" title="Low Gain" />
        <SynthKnob label="Hi Gain" value={p(8)} min={-15} max={15} defaultValue={0}
          onChange={(v) => onParam(8, v)} formatValue={signedGain} color="#6ae" title="High Gain" />
      </div>
      <div className={css.customRow}>
        <SynthFader label="Time L" value={p(0)} min={0} max={1300} defaultValue={200}
          onChange={(v) => onParam(0, v)} formatValue={(v) => `${v}ms`} compact
          title="Delay Time Left" />
        <SynthFader label="Time R" value={p(1)} min={0} max={1300} defaultValue={200}
          onChange={(v) => onParam(1, v)} formatValue={(v) => `${v}ms`} compact
          title="Delay Time Right" />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Generic fallback (grid of knobs/switches)
// ---------------------------------------------------------------------------

/** Names that are always grouped into the "tone EQ" row. */
const TONE_EQ_NAMES = new Set(["Low Gain", "High Gain"]);
/** Names handled by the shared output section. */
const OUTPUT_NAMES = new Set(["Level"]);
/** Names rendered near the output (balance/mix). */
const BALANCE_NAMES = new Set(["Balance", "Mix"]);

function GenericPanel({ params, paramDefs, onParam }: CustomPanelProps) {
  // Partition params into: main, switches, toneEq, balance
  const main: { def: ParamDef; i: number }[] = [];
  const switches: { def: ParamDef; i: number }[] = [];
  const toneEq: { def: ParamDef; i: number }[] = [];
  const balance: { def: ParamDef; i: number }[] = [];

  for (let i = 0; i < paramDefs.length; i++) {
    const def = paramDefs[i]!;
    if (OUTPUT_NAMES.has(def.name)) continue;
    if (TONE_EQ_NAMES.has(def.name)) { toneEq.push({ def, i }); continue; }
    if (BALANCE_NAMES.has(def.name)) { balance.push({ def, i }); continue; }
    const range = def.max - def.min;
    if (range <= 1) { switches.push({ def, i }); continue; }
    main.push({ def, i });
  }

  // Render main knobs in rows of 2
  const mainRows: { def: ParamDef; i: number }[][] = [];
  for (let j = 0; j < main.length; j += 2) {
    mainRows.push(main.slice(j, j + 2));
  }

  const renderKnob = (item: { def: ParamDef; i: number }, color: string) => {
    const val = params[item.i] ?? item.def.defaultValue;
    return (
      <SynthKnob
        key={item.def.index}
        label={item.def.name}
        value={val}
        min={item.def.min}
        max={item.def.max}
        defaultValue={item.def.defaultValue}
        onChange={(v) => onParam(item.i, v)}
        formatValue={(v) => String(v)}
        color={color}
      />
    );
  };

  return (
    <div className={css.customLayout}>
      {/* Switches row */}
      {switches.length > 0 && (
        <div className={css.customRow}>
          {switches.map((item) => {
            const val = params[item.i] ?? item.def.defaultValue;
            return (
              <SynthSwitch
                key={item.def.index}
                label={item.def.name}
                value={val}
                options={[
                  { value: item.def.min, label: "OFF" },
                  { value: item.def.max, label: "ON" },
                ]}
                onChange={(v) => onParam(item.i, v)}
              />
            );
          })}
        </div>
      )}
      {/* Main knobs in rows of 2 */}
      {mainRows.map((row, ri) => (
        <div key={ri} className={css.customRow}>
          {row.map((item) => renderKnob(item, "#c8a"))}
        </div>
      ))}
      {/* Balance/Mix */}
      {balance.length > 0 && (
        <div className={css.customRow}>
          {balance.map((item) => renderKnob(item, "#c8a"))}
        </div>
      )}
      {/* Tone EQ row (Lo Gain / Hi Gain) */}
      {toneEq.length > 0 && (
        <div className={css.customRow}>
          {toneEq.map((item) => renderKnob(item, "#6ae"))}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Custom panel dispatcher
// ---------------------------------------------------------------------------

function TypePanel({ mfxType, params, paramDefs, onParam }: CustomPanelProps & { mfxType: number }) {
  switch (mfxType) {
    case 1:
      return <EqualizerPanel params={params} paramDefs={paramDefs} onParam={onParam} />;
    case 22:
      return <ChorusPanel params={params} paramDefs={paramDefs} onParam={onParam} />;
    case 28:
    case 29:
      return <OverdrivePanel params={params} paramDefs={paramDefs} onParam={onParam} />;
    case 31:
      return <CompressorPanel params={params} paramDefs={paramDefs} onParam={onParam} />;
    case 34:
      return <DelayPanel params={params} paramDefs={paramDefs} onParam={onParam} />;
    default:
      return paramDefs.length > 0
        ? <GenericPanel params={params} paramDefs={paramDefs} onParam={onParam} />
        : null;
  }
}

// ---------------------------------------------------------------------------
// Main MfxPanel component
// ---------------------------------------------------------------------------

export function MfxPanel({
  mfx,
  onTypeChange,
  onHeaderParam,
  onNibParam,
}: Props) {
  const paramDefs = useParamDefs(mfx.mfxType);
  const levelParamIndex = paramDefs.findIndex((d) => d.name === "Level");

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

        {/* Type-specific or generic parameter panel */}
        <TypePanel
          mfxType={mfx.mfxType}
          params={mfx.params}
          paramDefs={paramDefs}
          onParam={onNibParam}
        />

        {/* Output section — sends + level, like a mixer channel strip */}
        <div className={css.mfxOutput}>
          <div className={css.mfxSends}>
            <SynthKnob label="FX1" value={mfx.chorusSend} min={0} max={127} defaultValue={0}
              onChange={(v) => onHeaderParam(0x02, v)} formatValue={(v) => String(v)} color="#668"
              title="Chorus Send Level" />
            <SynthKnob label="FX2" value={mfx.reverbSend} min={0} max={127} defaultValue={0}
              onChange={(v) => onHeaderParam(0x03, v)} formatValue={(v) => String(v)} color="#686"
              title="Reverb Send Level" />
          </div>
          {levelParamIndex >= 0 && (
            <div className={css.mfxLevelFader}>
              <SynthFader label="Level" value={mfx.params[levelParamIndex] ?? 127}
                min={0} max={127} defaultValue={127}
                onChange={(v) => onNibParam(levelParamIndex, v)}
                formatValue={(v) => String(v)} trackHeight={250} />
            </div>
          )}
        </div>

        {/* Macros — CC-to-parameter mappings */}
        <div className={css.macroSection}>
          <div className={css.macroHeader}>
            <span className={css.macroTitle}>MACROS</span>
            <span className={css.macroColLabel}>SRC / DST</span>
            <span className={css.macroColLabel}>DEPTH</span>
          </div>
          {[0, 1, 2, 3].map((slot) => {
            const ctrl = mfx.controls[slot] ?? { source: 0, sens: 64, assign: 0 };
            return (
              <div key={slot} className={css.macroSlot}>
                <span className={css.macroNum}>{slot + 1}</span>
                <div className={css.macroSelectors}>
                  <select
                    className={css.macroSelect}
                    value={ctrl.source}
                    onChange={(e) => onHeaderParam(0x05 + slot * 2, Number(e.target.value))}
                    title="Source"
                  >
                    {CTRL_SOURCE_NAMES.map((name, i) => (
                      <option key={i} value={i}>{name}</option>
                    ))}
                  </select>
                  <select
                    className={css.macroSelect}
                    value={ctrl.assign}
                    onChange={(e) => onHeaderParam(0x0D + slot, Number(e.target.value))}
                    title="Destination"
                  >
                    <option value={0}>OFF</option>
                    {paramDefs.map((def) => (
                      <option key={def.index} value={def.index}>{def.name}</option>
                    ))}
                  </select>
                </div>
                <SynthKnob
                  label=""
                  value={ctrl.sens}
                  min={1}
                  max={127}
                  defaultValue={64}
                  onChange={(v) => onHeaderParam(0x06 + slot * 2, v)}
                  formatValue={(v) => String(v - 64)}
                  color="#8ac"
                  title={`Macro ${slot + 1} Depth`}
                />
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
