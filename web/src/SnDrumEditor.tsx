import { useEffect, useState, useCallback, useRef, useMemo } from "react";
import {
  snd_common_address,
  snd_common_size,
  snd_mfx_address,
  snd_note_address,
  snd_note_size,
  mfx_block_size,
  ToneType,
} from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import { deviceSpec } from "./deviceSpec";
import {
  SynthKnob,
  SynthFader,
  SynthSelect,
  SectionPanel,
  FaderGroup,
  FaderGroupSep,
  OutputStrip,
} from "./synth-ui";
import { MfxPanel } from "./MfxPanel";
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
  // Common param setter
  // ---------------------------------------------------------------------------
  const setCommonParam = useCallback(
    (offset: number, value: number) => {
      service.device.setSndCommonParam(partIndex, offset, value);
      setCommon((prev) => {
        if (!prev) return prev;
        const next = { ...prev };
        switch (offset) {
          case 0x10: next.kitLevel = value; break;
          case 0x11: next.ambienceLevel = value; break;
          case 0x12: next.phraseNumber = value; break;
          case 0x13: next.tfxSwitch = value; break;
        }
        return next;
      });
    },
    [partIndex, service],
  );

  // ---------------------------------------------------------------------------
  // Note param setter
  // ---------------------------------------------------------------------------
  const setNoteParam = useCallback(
    (offset: number, value: number) => {
      service.device.setSndNoteParam(partIndex, selectedKey, offset, value);
      setNoteData((prev) => {
        if (!prev) return prev;
        const next = { ...prev };
        switch (offset) {
          case 0x04: next.level = value; break;
          case 0x05: next.pan = value; break;
          case 0x06: next.chorusSend = value; break;
          case 0x07: next.reverbSend = value; break;
          case 0x0C: next.attack = value; break;
          case 0x0D: next.decay = value; break;
          case 0x0E: next.brilliance = value; break;
          case 0x0F: next.variation = value; break;
          case 0x10: next.dynamicRange = value; break;
          case 0x11: next.stereoWidth = value; break;
          case 0x12: next.outputAssign = value; break;
        }
        // Update cache
        noteCache.current.set(selectedKey, next);
        return next;
      });
    },
    [partIndex, selectedKey, service],
  );

  // ---------------------------------------------------------------------------
  // Nibblized note param setter (inst number, tune)
  // ---------------------------------------------------------------------------
  const setNoteNibParam = useCallback(
    (offset: number, value: number, field: keyof SnDrumNote) => {
      service.device.setSndNoteNibParam(partIndex, selectedKey, offset, value);
      setNoteData((prev) => {
        if (!prev) return prev;
        const next = { ...prev, [field]: value };
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

      {/* Three areas side by side: Common | Key Editor | MFX */}
      <div className={css.mainArea}>
        {common && <CommonPanel common={common} onChange={setCommonParam} />}

        <div className={css.keyEditorArea}>
          <SectionPanel label="DRUM INST" accentColor="#f93">
            <DrumKeyGrid
              firstKey={keys.first}
              lastKey={keys.last}
              selectedKey={selectedKey}
              onSelect={handleKeySelect}
              keyOutputAssigns={keyOutputAssigns}
            />
            {noteLoading ? (
              <div className={css.noteLoadingPlaceholder}>Loading note data...</div>
            ) : noteData ? (
              <NoteControls
                keyNumber={selectedKey}
                note={noteData}
                onChange={setNoteParam}
                onNibChange={setNoteNibParam}
              />
            ) : (
              <div className={css.noteLoadingPlaceholder}>Select a key to edit</div>
            )}
          </SectionPanel>
        </div>

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
// Common Controls Panel
// ---------------------------------------------------------------------------

function CommonPanel({
  common,
  onChange,
}: {
  common: SnDrumCommon;
  onChange: (offset: number, value: number) => void;
}) {
  return (
    <SectionPanel label="COMMON" accentColor="#8cf">
      <FaderGroup>
        <SynthFader label="Kit Lv" value={common.kitLevel} min={0} max={127} defaultValue={127}
          onChange={(v) => onChange(0x10, v)} formatValue={(v) => String(v)} compact
          title="Kit Level" />
        <FaderGroupSep />
        <SynthFader label="Amb Lv" value={common.ambienceLevel} min={0} max={127} defaultValue={64}
          onChange={(v) => onChange(0x11, v)} formatValue={(v) => String(v)} compact
          title="Ambience Level" />
      </FaderGroup>
    </SectionPanel>
  );
}

// ---------------------------------------------------------------------------
// Per-key note controls
// ---------------------------------------------------------------------------

function NoteControls({
  keyNumber,
  note,
  onChange,
  onNibChange,
}: {
  keyNumber: number;
  note: SnDrumNote;
  onChange: (offset: number, value: number) => void;
  onNibChange: (offset: number, value: number, field: keyof SnDrumNote) => void;
}) {
  const [instDraft, setInstDraft] = useState(String(note.instNumber));

  // Sync draft when note changes (key selection)
  useEffect(() => {
    setInstDraft(String(note.instNumber));
  }, [note.instNumber]);

  const commitInstNumber = () => {
    const n = Number(instDraft);
    if (Number.isInteger(n) && n >= 0 && n <= deviceSpec().sn_drum_inst_number_max) {
      onNibChange(0x00, n, "instNumber");
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
          onChange={(v) => onChange(0x12, v)} />
      </div>
      <div className={css.noteControls}>
        {/* Strip: Pan + Sends + Level — reuses the shared OutputStrip */}
        <SectionPanel label="LEVEL / PAN" accentColor="#8cf">
          <OutputStrip
            pan={note.pan} onPanChange={(v) => onChange(0x05, v)} panFormat={panFmt}
            fx1={note.chorusSend} fx2={note.reverbSend}
            level={note.level}
            onFx1Change={(v) => onChange(0x06, v)}
            onFx2Change={(v) => onChange(0x07, v)}
            onLevelChange={(v) => onChange(0x04, v)}
            trackHeight={250}
          />
        </SectionPanel>

        {/* Tone shaping */}
        <SectionPanel label="TONE" accentColor="#fc8">
          <div className={css.knobRow}>
            <SynthKnob label="Attack" value={note.attack} min={0} max={100} defaultValue={100}
              onChange={(v) => onChange(0x0C, v)} formatValue={(v) => `${v}%`} color="#fc8" />
            <SynthKnob label="Decay" value={note.decay} min={1} max={64} defaultValue={64}
              onChange={(v) => onChange(0x0D, v)} formatValue={decayFmt} color="#fc8" />
            <SynthKnob label="Brill" value={note.brilliance} min={49} max={76} defaultValue={64} center={64}
              onChange={(v) => onChange(0x0E, v)} formatValue={brillianceFmt} color="#fc8"
              title="Brilliance" />
          </div>
          <div className={css.knobRow}>
            <SynthKnob label="DynRng" value={note.dynamicRange} min={0} max={63} defaultValue={63}
              onChange={(v) => onChange(0x10, v)} formatValue={(v) => String(v)} color="#a6f"
              title="Dynamic Range" />
            <SynthKnob label="Width" value={note.stereoWidth} min={0} max={127} defaultValue={64}
              onChange={(v) => onChange(0x11, v)} formatValue={(v) => String(v)} color="#a6f"
              title="Stereo Width" />
          </div>
          <div className={css.noteSelects}>
            <SynthSelect label="Variation" value={note.variation} options={VARIATION_OPTIONS}
              onChange={(v) => onChange(0x0F, v)} />
            <SynthSelect label="Output" value={note.outputAssign} options={OUTPUT_ASSIGN_OPTIONS}
              onChange={(v) => onChange(0x12, v)} />
          </div>
        </SectionPanel>
      </div>
    </>
  );
}
