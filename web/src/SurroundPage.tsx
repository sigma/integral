import { useState, useCallback } from "react";
import { EqKnob } from "./EqKnob";
import { SurroundXYPad } from "./SurroundXYPad";
import type { UseMixerResult } from "./useMixer";
import { deviceSpec } from "./deviceSpec";
import css from "./SurroundPage.module.css";

interface Props {
  mixer: UseMixerResult;
}

export function SurroundPage({ mixer }: Props) {
  const { surround } = mixer.state;
  const spec = deviceSpec();
  const [selectedPart, setSelectedPart] = useState<number>(0);

  const parts = surround.parts.map((s, i) => ({
    index: i,
    label: String(i + 1),
    state: s,
  }));
  parts.push({ index: 16, label: "EX", state: surround.ext });

  const handleMove = useCallback(
    (index: number, lr: number, fb: number) => {
      if (index < 16) {
        mixer.setPartSurroundLr(index, lr);
        mixer.setPartSurroundFb(index, fb);
      } else {
        mixer.setSurroundParam(0x07, lr);
        mixer.setSurroundParam(0x08, fb);
      }
    },
    [mixer],
  );

  return (
    <div className={css.page}>
      {/* Header: surround switch + room controls */}
      <div className={css.header}>
        <button
          className={`${css.switchBtn} ${surround.enabled ? css.on : ""}`}
          onClick={() => mixer.setSurroundParam(0x00, surround.enabled ? 0 : 1)}
        >
          {surround.enabled ? "ON" : "OFF"}
        </button>
        <label className={css.selectLabel}>
          Room
          <select className={css.select} value={surround.roomType}
            onChange={(e) => mixer.setSurroundParam(0x01, Number(e.target.value))}>
            {spec.surround_room_types.map((n, i) => <option key={i} value={i}>{n}</option>)}
          </select>
        </label>
        <label className={css.selectLabel}>
          Size
          <select className={css.select} value={surround.roomSize}
            onChange={(e) => mixer.setSurroundParam(0x03, Number(e.target.value))}>
            {spec.surround_room_sizes.map((n, i) => <option key={i} value={i}>{n}</option>)}
          </select>
        </label>
        <EqKnob label="Depth" value={surround.depth} min={0} max={100} defaultValue={50}
          onChange={(v) => mixer.setSurroundParam(0x0C, v)} formatValue={(v) => String(v)} color="#4a6cf7" />
        <EqKnob label="Amb Lvl" value={surround.ambienceLevel} min={0} max={127} defaultValue={64}
          onChange={(v) => mixer.setSurroundParam(0x02, v)} formatValue={(v) => String(v)} color="#4a6cf7" />
        <EqKnob label="Amb Time" value={surround.ambienceTime} min={0} max={100} defaultValue={50}
          onChange={(v) => mixer.setSurroundParam(0x04, v)} formatValue={(v) => String(v)} color="#4a6cf7" />
        <EqKnob label="Density" value={surround.ambienceDensity} min={0} max={100} defaultValue={50}
          onChange={(v) => mixer.setSurroundParam(0x05, v)} formatValue={(v) => String(v)} color="#4a6cf7" />
        <EqKnob label="HF Damp" value={surround.ambienceHfDamp} min={0} max={100} defaultValue={50}
          onChange={(v) => mixer.setSurroundParam(0x06, v)} formatValue={(v) => String(v)} color="#4a6cf7" />
      </div>

      {/* Part boxes: 16 parts + EX, each with Width + AMB knobs */}
      <div className={css.partStrip}>
        {parts.map((p) => {
          const isSelected = p.index === selectedPart;
          return (
            <div
              key={p.index}
              className={`${css.partBox} ${isSelected ? css.partBoxSelected : ""}`}
              onClick={() => setSelectedPart(p.index)}
              title={p.index < 16 ? mixer.state.parts[p.index]?.toneName || "" : "External Input"}
            >
              <span className={css.partLabel}>{p.label}</span>
              <span className={css.partTone}>
                {p.index < 16 ? (mixer.state.parts[p.index]?.toneName || "—") : "Ext"}
              </span>
              <div className={css.partKnobs}>
                <EqKnob label="W" value={p.state.width} min={0} max={32} defaultValue={16}
                  onChange={(v) => {
                    if (p.index < 16) mixer.setPartSurroundWidth(p.index, v);
                    else mixer.setSurroundParam(0x09, v);
                  }}
                  formatValue={(v) => String(v)} color="#ffaa44" />
                <EqKnob label="AMB" value={p.state.ambienceSend} min={0} max={127} defaultValue={0}
                  onChange={(v) => {
                    if (p.index < 16) mixer.setPartSurroundAmbienceSend(p.index, v);
                    else mixer.setSurroundParam(0x0A, v);
                  }}
                  formatValue={(v) => String(v)} color="#a6f" />
              </div>
            </div>
          );
        })}
      </div>

      {/* XY Pad: fills remaining space */}
      <div className={css.padArea}>
        <SurroundXYPad
          parts={parts}
          selectedPart={selectedPart}
          onSelect={setSelectedPart}
          onMove={handleMove}
        />
      </div>
    </div>
  );
}
