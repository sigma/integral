import { useEffect, useState, useCallback, useRef, useMemo } from "react";
import {
  snd_common_address,
  snd_common_size,
  snd_mfx_address,
  snd_note_address,
  snd_note_size,
  mfx_block_size,
  SndCommonField,
  SndNoteField,
  ToneType,
} from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import { deviceSpec } from "./deviceSpec";
import {
  SynthKnob,
  SynthFader,
  SynthSelect,
  SectionPanel,
  OutputStrip,
} from "./synth-ui";
import { MfxPanel } from "./MfxPanel";
import { SignalArrows } from "./synth-ui/SignalArrows";
import { useMfxHandlers } from "./useMfxHandlers";
import type { MfxState } from "./MfxPanel";
import { DrumKeyGrid } from "./DrumKeyGrid";
import { useKeyPrefetch } from "./useKeyPrefetch";
import { noteName } from "./pcm-drum/types";
import css from "./SnDrumEditor.module.css";

// ---------------------------------------------------------------------------
// Types mirroring the Rust serde output
// ---------------------------------------------------------------------------

interface SnDrumCommon {
  kitName: string;
  kitLevel: number;
  ambienceLevel: number;
  phraseNumber: number;
  tfxSwitch: number;
}

interface SnDrumNote {
  instNumber: number;
  level: number;
  pan: number;
  chorusSend: number;
  reverbSend: number;
  tune: number;
  attack: number;
  decay: number;
  brilliance: number;
  variation: number;
  dynamicRange: number;
  stereoWidth: number;
  outputAssign: number;
}

/** The keys of `T` holding a number. */
type NumberKey<T> = { [K in keyof T]: T[K] extends number ? K : never }[keyof T];

/** Sets a Common field: `field` addresses it on the device, `key` names it
 *  in {@link SnDrumCommon}. */
type CommonParamHandler = (
  field: SndCommonField,
  key: NumberKey<SnDrumCommon>,
  value: number,
) => void;

/** Sets a field of the selected key's Note: `field` addresses it on the
 *  device, `key` names it in {@link SnDrumNote}. */
type NoteParamHandler = (field: SndNoteField, key: keyof SnDrumNote, value: number) => void;

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

const VARIATION_OPTIONS = [
  { value: 0, label: "OFF" },
  { value: 1, label: "FLAM1" },
  { value: 2, label: "FLAM2" },
  { value: 3, label: "FLAM3" },
  { value: 4, label: "BUZZ1" },
  { value: 5, label: "BUZZ2" },
  { value: 6, label: "BUZZ3" },
  { value: 7, label: "ROLL" },
];

const OUTPUT_ASSIGN_OPTIONS = [
  { value: 0, label: "PART" },
  { value: 1, label: "C+EQ1" },
  { value: 2, label: "C+EQ2" },
  { value: 3, label: "C+EQ3" },
  { value: 4, label: "C+EQ4" },
  { value: 5, label: "C+EQ5" },
  { value: 6, label: "C+EQ6" },
];

function panFmt(raw: number): string {
  if (raw === 64) return "C";
  if (raw < 64) return `L${64 - raw}`;
  return `${raw - 64}R`;
}

function decayFmt(raw: number): string {
  // raw 1–64, display -63 to 0
  return String(raw - 64);
}

function brillianceFmt(raw: number): string {
  // raw 49–76, display -15 to +12
  const v = raw - 64;
  return v > 0 ? `+${v}` : String(v);
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface Props {
  partIndex: number;
  service: IntegraService;
}

export function SnDrumEditor({ partIndex, service }: Props) {
  const keys = deviceSpec().sn_drum_keys;
  const [common, setCommon] = useState<SnDrumCommon | null>(null);
  const [mfx, setMfx] = useState<MfxState | null>(null);
  const [loading, setLoading] = useState(true);
  const [selectedKey, setSelectedKey] = useState(36); // C2 — kick drum
  const [noteData, setNoteData] = useState<SnDrumNote | null>(null);
  const [noteLoading, setNoteLoading] = useState(false);
  const noteCache = useRef<Map<number, SnDrumNote>>(new Map());

  // Load common + MFX on mount / part change
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    noteCache.current.clear();

    async function load() {
      try {
        // Common
        const commonData = await service.requestData(
          Array.from(snd_common_address(partIndex)),
          Array.from(snd_common_size()),
        );
        if (cancelled) return;
        const c = service.device.applySndCommon(commonData) as SnDrumCommon | null;
        if (c) setCommon(c);

        // MFX
        const mfxData = await service.requestData(
          Array.from(snd_mfx_address(partIndex)),
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

  // Prefetch every key so the grid shows routing up front
  const fetchNote = useCallback(
    async (k: number) =>
      service.device.applySndNote(
        await service.requestData(
          Array.from(snd_note_address(partIndex, k)),
          Array.from(snd_note_size()),
        ),
      ) as SnDrumNote | null,
    [partIndex, service],
  );
  const cacheVersion = useKeyPrefetch(!loading, keys.first, keys.last, noteCache, fetchNote);

  // Load note data on key selection change
  useEffect(() => {
    const { first, last } = deviceSpec().sn_drum_keys;
    if (selectedKey < first || selectedKey > last) return;

    const cached = noteCache.current.get(selectedKey);
    if (cached) {
      setNoteData(cached);
      return;
    }

    let cancelled = false;
    setNoteLoading(true);

    async function loadNote() {
      try {
        const data = await service.requestData(
          Array.from(snd_note_address(partIndex, selectedKey)),
          Array.from(snd_note_size()),
        );
        if (cancelled) return;
        const n = service.device.applySndNote(data) as SnDrumNote | null;
        if (n) {
          noteCache.current.set(selectedKey, n);
          setNoteData(n);
        }
      } catch {
        // Timeout
      }
      if (!cancelled) setNoteLoading(false);
    }

    loadNote();
    return () => { cancelled = true; };
  }, [selectedKey, partIndex, service]);

  // ---------------------------------------------------------------------------
  // Key selection with note preview
  // ---------------------------------------------------------------------------
  const handleKeySelect = useCallback(
    (key: number) => {
      setSelectedKey(key);
      // Preview the note on the part's receive channel
      const rs = service.device.readState() as { parts?: { receiveChannel?: number }[] };
      const ch = rs?.parts?.[partIndex]?.receiveChannel ?? partIndex;
      service.sendNoteOn(ch, key, 100);
      setTimeout(() => {
        service.sendNoteOff(ch, key);
      }, 300);
    },
    [partIndex, service],
  );

  // ---------------------------------------------------------------------------
  // Param setters: send to the device, then patch the local copy
  // ---------------------------------------------------------------------------
  const setCommonParam = useCallback<CommonParamHandler>(
    (field, key, value) => {
      service.device.setSndCommonParam(partIndex, field, value);
      setCommon((prev) => (prev ? { ...prev, [key]: value } : prev));
    },
    [partIndex, service],
  );

  const setNoteParam = useCallback<NoteParamHandler>(
    (field, key, value) => {
      service.device.setSndNoteParam(partIndex, selectedKey, field, value);
      setNoteData((prev) => {
        if (!prev) return prev;
        const next = { ...prev, [key]: value };
        noteCache.current.set(selectedKey, next);
        return next;
      });
    },
    [partIndex, selectedKey, service],
  );

  const mfxHandlers = useMfxHandlers(service, partIndex, ToneType.SnDrum, setMfx);

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  // eslint-disable-next-line react-hooks/exhaustive-deps
  const keyOutputAssigns = useMemo(() => {
    const m = new Map<number, number>();
    for (const [k, n] of noteCache.current) {
      m.set(k, n.outputAssign);
    }
    return m;
  }, [noteData, selectedKey, cacheVersion]);

  if (loading) {
    return <div className={css.loading}>Loading SN-D drum kit data...</div>;
  }

  return (
    <div className={css.editor}>
      {/* Kit name */}
      {common && (
        <div className={css.kitName}>{common.kitName || "(unnamed kit)"}</div>
      )}

      {/* The key editor (kit levels beside the key grid), then the MFX it feeds */}
      <div className={css.mainArea}>
        <div className={css.keyEditorArea} data-node="drum-key">
          <SectionPanel label="DRUM INST" accentColor="#f93">
            <div className={css.keyGridRow}>
              {common ? <KitLevels common={common} onChange={setCommonParam} /> : <div />}
              <DrumKeyGrid
                firstKey={keys.first}
                lastKey={keys.last}
                selectedKey={selectedKey}
                onSelect={handleKeySelect}
                keyOutputAssigns={keyOutputAssigns}
              />
            </div>
            {noteLoading ? (
              <div className={css.noteLoadingPlaceholder}>Loading note data...</div>
            ) : noteData ? (
              <NoteControls
                keyNumber={selectedKey}
                note={noteData}
                onChange={setNoteParam}
              />
            ) : (
              <div className={css.noteLoadingPlaceholder}>Select a key to edit</div>
            )}
          </SectionPanel>
        </div>

        {mfx && (
          <div className={css.mfxSidebar} data-node="mfx">
            <MfxPanel
              mfx={mfx}
              {...mfxHandlers}
            />
          </div>
        )}
        {/* The kit's keys all feed the MFX. */}
        <SignalArrows edges={[{ from: "drum-key", to: "mfx" }]} />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Kit levels: bare faders left of the key grid, as tall as a key's Level
// ---------------------------------------------------------------------------

/** Track height of a key's Level fader, which the kit's level faders match. */
const LEVEL_TRACK_HEIGHT = 250;

function KitLevels({
  common,
  onChange,
}: {
  common: SnDrumCommon;
  onChange: CommonParamHandler;
}) {
  const set =
    (field: SndCommonField, key: Parameters<CommonParamHandler>[1]) =>
    (v: number) =>
      onChange(field, key, v);
  return (
    <div className={css.kitLevels}>
      <SynthFader label="Kit Lv" value={common.kitLevel} min={0} max={127} defaultValue={127}
        onChange={set(SndCommonField.KitLevel, "kitLevel")} formatValue={(v) => String(v)}
        trackHeight={LEVEL_TRACK_HEIGHT} title="Kit Level" />
      <SynthFader label="Amb Lv" value={common.ambienceLevel} min={0} max={127} defaultValue={64}
        onChange={set(SndCommonField.AmbienceLevel, "ambienceLevel")} formatValue={(v) => String(v)}
        trackHeight={LEVEL_TRACK_HEIGHT} title="Ambience Level" />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Per-key note controls
// ---------------------------------------------------------------------------

function NoteControls({
  keyNumber,
  note,
  onChange,
}: {
  keyNumber: number;
  note: SnDrumNote;
  onChange: NoteParamHandler;
}) {
  const set =
    (field: SndNoteField, key: Parameters<NoteParamHandler>[1]) =>
    (v: number) =>
      onChange(field, key, v);
  const [instDraft, setInstDraft] = useState(String(note.instNumber));

  // Sync draft when note changes (key selection)
  useEffect(() => {
    setInstDraft(String(note.instNumber));
  }, [note.instNumber]);

  const commitInstNumber = () => {
    const n = Number(instDraft);
    if (Number.isInteger(n) && n >= 0 && n <= deviceSpec().sn_drum_inst_number_max) {
      onChange(SndNoteField.InstNumber, "instNumber", n);
      setInstDraft(String(n));
    } else {
      // Revert to current value
      setInstDraft(String(note.instNumber));
    }
  };

  return (
    <>
      <div className={css.instNumberRow}>
        <span className={css.instNumberLabel}>
          Key {keyNumber} ({noteName(keyNumber)})
        </span>
        <label className={css.instInputLabel}>
          Inst #
          <input
            className={css.instInput}
            type="number"
            min={0}
            max={deviceSpec().sn_drum_inst_number_max}
            value={instDraft}
            onChange={(e) => setInstDraft(e.target.value)}
            onBlur={commitInstNumber}
            onKeyDown={(e) => { if (e.key === "Enter") commitInstNumber(); }}
          />
        </label>
        <SynthSelect label="Out" value={note.outputAssign}
          options={OUTPUT_ASSIGN_OPTIONS}
          onChange={set(SndNoteField.OutputAssign, "outputAssign")} />
      </div>
      <div className={css.noteControls}>
        {/* Strip: Pan + Sends + Level — reuses the shared OutputStrip */}
        <SectionPanel label="LEVEL / PAN" accentColor="#8cf">
          <OutputStrip
            pan={note.pan} onPanChange={set(SndNoteField.Pan, "pan")} panFormat={panFmt}
            fx1={note.chorusSend} fx2={note.reverbSend}
            level={note.level}
            onFx1Change={set(SndNoteField.ChorusSend, "chorusSend")}
            onFx2Change={set(SndNoteField.ReverbSend, "reverbSend")}
            onLevelChange={set(SndNoteField.Level, "level")}
            trackHeight={LEVEL_TRACK_HEIGHT}
          />
        </SectionPanel>

        {/* Tone shaping */}
        <SectionPanel label="TONE" accentColor="#fc8">
          <div className={css.knobRow}>
            <SynthKnob label="Attack" value={note.attack} min={0} max={100} defaultValue={100}
              onChange={set(SndNoteField.Attack, "attack")} formatValue={(v) => `${v}%`} color="#fc8" />
            <SynthKnob label="Decay" value={note.decay} min={1} max={64} defaultValue={64}
              onChange={set(SndNoteField.Decay, "decay")} formatValue={decayFmt} color="#fc8" />
            <SynthKnob label="Brill" value={note.brilliance} min={49} max={76} defaultValue={64} center={64}
              onChange={set(SndNoteField.Brilliance, "brilliance")} formatValue={brillianceFmt} color="#fc8"
              title="Brilliance" />
          </div>
          <div className={css.knobRow}>
            <SynthKnob label="DynRng" value={note.dynamicRange} min={0} max={63} defaultValue={63}
              onChange={set(SndNoteField.DynamicRange, "dynamicRange")} formatValue={(v) => String(v)} color="#a6f"
              title="Dynamic Range" />
            <SynthKnob label="Width" value={note.stereoWidth} min={0} max={127} defaultValue={64}
              onChange={set(SndNoteField.StereoWidth, "stereoWidth")} formatValue={(v) => String(v)} color="#a6f"
              title="Stereo Width" />
          </div>
          <div className={css.noteSelects}>
            <SynthSelect label="Variation" value={note.variation} options={VARIATION_OPTIONS}
              onChange={set(SndNoteField.Variation, "variation")} />
            <SynthSelect label="Output" value={note.outputAssign} options={OUTPUT_ASSIGN_OPTIONS}
              onChange={set(SndNoteField.OutputAssign, "outputAssign")} />
          </div>
        </SectionPanel>
      </div>
    </>
  );
}
