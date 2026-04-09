import { useState, useCallback } from "react";
import { EqKnob } from "./EqKnob";
import { SurroundXYPad } from "./SurroundXYPad";
import type { UseMixerResult } from "./useMixer";
import css from "./SurroundPage.module.css";

const ROOM_TYPES = ["Room 1", "Room 2", "Hall 1", "Hall 2"];
const ROOM_SIZES = ["Small", "Medium", "Large"];

interface Props {
  mixer: UseMixerResult;
}

export function SurroundPage({ mixer }: Props) {
  const { surround } = mixer.state;
  const [selectedDot, setSelectedDot] = useState<number>(0);

  const parts = surround.parts.map((s, i) => ({
    index: i,
    label: String(i + 1),
    state: s,
  }));
  // Add ext part as index 16
  parts.push({ index: 16, label: "EX", state: surround.ext });

  const handleMove = useCallback(
    (index: number, lr: number, fb: number) => {
      if (index < 16) {
        mixer.setPartSurroundLr(index, lr);
        mixer.setPartSurroundFb(index, fb);
      } else {
        // Ext part uses surround common params
        mixer.setSurroundParam(0x07, lr);
        mixer.setSurroundParam(0x08, fb);
      }
    },
    [mixer],
  );

  const sel = selectedDot < 16 ? surround.parts[selectedDot] : surround.ext;
  const selLabel = selectedDot < 16 ? `Part ${selectedDot + 1}` : "Ext Part";

  return (
    <div className={css.page}>
      <div className={css.header}>
        <button
          className={`${css.switchBtn} ${surround.enabled ? css.on : ""}`}
          onClick={() => mixer.setSurroundParam(0x00, surround.enabled ? 0 : 1)}
        >
          Surround {surround.enabled ? "ON" : "OFF"}
        </button>
      </div>
      <div className={css.body}>
        <div className={css.padArea}>
          <SurroundXYPad
            parts={parts}
            selectedPart={selectedDot}
            onSelect={setSelectedDot}
            onMove={handleMove}
          />
        </div>
        <div className={css.controls}>
          {/* Room controls */}
          <div className={css.section}>
            <h3 className={css.sectionTitle}>Room</h3>
            <div className={css.row}>
              <label className={css.selectLabel}>
                Type
                <select
                  className={css.select}
                  value={surround.roomType}
                  onChange={(e) => mixer.setSurroundParam(0x01, Number(e.target.value))}
                >
                  {ROOM_TYPES.map((name, i) => (
                    <option key={i} value={i}>{name}</option>
                  ))}
                </select>
              </label>
              <label className={css.selectLabel}>
                Size
                <select
                  className={css.select}
                  value={surround.roomSize}
                  onChange={(e) => mixer.setSurroundParam(0x03, Number(e.target.value))}
                >
                  {ROOM_SIZES.map((name, i) => (
                    <option key={i} value={i}>{name}</option>
                  ))}
                </select>
              </label>
            </div>
            <div className={css.knobRow}>
              <EqKnob label="Depth" value={surround.depth} min={0} max={100} defaultValue={50}
                onChange={(v) => mixer.setSurroundParam(0x0C, v)}
                formatValue={(v) => String(v)} color="#4a6cf7" />
              <EqKnob label="Amb Lvl" value={surround.ambienceLevel} min={0} max={127} defaultValue={64}
                onChange={(v) => mixer.setSurroundParam(0x02, v)}
                formatValue={(v) => String(v)} color="#4a6cf7" />
              <EqKnob label="Amb Time" value={surround.ambienceTime} min={0} max={100} defaultValue={50}
                onChange={(v) => mixer.setSurroundParam(0x04, v)}
                formatValue={(v) => String(v)} color="#4a6cf7" />
              <EqKnob label="Density" value={surround.ambienceDensity} min={0} max={100} defaultValue={50}
                onChange={(v) => mixer.setSurroundParam(0x05, v)}
                formatValue={(v) => String(v)} color="#4a6cf7" />
              <EqKnob label="HF Damp" value={surround.ambienceHfDamp} min={0} max={100} defaultValue={50}
                onChange={(v) => mixer.setSurroundParam(0x06, v)}
                formatValue={(v) => String(v)} color="#4a6cf7" />
            </div>
          </div>

          {/* Selected part controls */}
          {sel && (
            <div className={css.section}>
              <h3 className={css.sectionTitle}>{selLabel}</h3>
              <div className={css.knobRow}>
                <EqKnob label="L-R" value={sel.lr} min={0} max={127} defaultValue={64}
                  onChange={(v) => handleMove(selectedDot, v, sel.fb)}
                  formatValue={(v) => { const d = v - 64; return d === 0 ? "C" : d > 0 ? `R${d}` : `L${-d}`; }}
                  color="#ffaa44" />
                <EqKnob label="F-B" value={sel.fb} min={0} max={127} defaultValue={64}
                  onChange={(v) => handleMove(selectedDot, sel.lr, v)}
                  formatValue={(v) => { const d = v - 64; return d === 0 ? "C" : d > 0 ? `F${d}` : `B${-d}`; }}
                  color="#ffaa44" />
                <EqKnob label="Width" value={sel.width} min={0} max={32} defaultValue={16}
                  onChange={(v) => {
                    if (selectedDot < 16) mixer.setPartSurroundWidth(selectedDot, v);
                    else mixer.setSurroundParam(0x09, v);
                  }}
                  formatValue={(v) => String(v)} color="#ffaa44" />
                <EqKnob label="AMB" value={sel.ambienceSend} min={0} max={127} defaultValue={0}
                  onChange={(v) => {
                    if (selectedDot < 16) mixer.setPartSurroundAmbienceSend(selectedDot, v);
                    else mixer.setSurroundParam(0x0A, v);
                  }}
                  formatValue={(v) => String(v)} color="#ffaa44" />
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
