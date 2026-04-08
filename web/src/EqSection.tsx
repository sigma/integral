import { useRef, useLayoutEffect, useState, useCallback } from "react";
import { EqKnob } from "./EqKnob";
import type { EqState } from "./types";
import css from "./EqSection.module.css";

const LOW_FREQ_VALUES = ["200", "400"];
const MID_FREQ_VALUES = [
  "200", "250", "315", "400", "500", "630", "800",
  "1.0k", "1.2k", "1.6k", "2.0k", "2.5k", "3.1k",
  "4.0k", "5.0k", "6.3k", "8.0k",
];
const HIGH_FREQ_VALUES = ["2.0k", "4.0k", "8.0k"];
const Q_VALUES = ["0.5", "1.0", "2.0", "4.0", "8.0"];

function formatGain(v: number): string {
  const db = v - 15;
  return `${db >= 0 ? "+" : ""}${db}dB`;
}

function formatFreq(v: number, table: string[]): string {
  return table[v] ?? `${v}`;
}

function formatQ(v: number): string {
  return Q_VALUES[v] ?? `${v}`;
}

interface Props {
  eq: EqState;
  onToggleSwitch: () => void;
  onParam: (paramOffset: number, value: number) => void;
  showSwitch?: boolean;
  /** Offset added to all param indices. Part EQ has switch at 0 so params start at 1.
   *  Master EQ has no switch so params start at 0. Default: 1 (Part EQ). */
  paramBase?: number;
}

interface DividerLines {
  w: number;
  h: number;
  topPath: string;
  botPath: string;
  hiLabel: { x: number; y: number };
  midLabel: { x: number; y: number };
  loLabel: { x: number; y: number };
}

export function EqSection({ eq, onToggleSwitch, onParam, showSwitch = true, paramBase = 1 }: Props) {
  const knobAreaRef = useRef<HTMLDivElement>(null);
  const knobRefs = useRef<Record<string, HTMLDivElement | null>>({});
  const [lines, setLines] = useState<DividerLines | null>(null);

  const setKnobRef = useCallback((key: string) => (el: HTMLDivElement | null) => {
    knobRefs.current[key] = el;
  }, []);

  const measure = useCallback(() => {
    const area = knobAreaRef.current;
    if (!area) return;
    const refs = knobRefs.current;

    const aRect = area.getBoundingClientRect();
    const cy = (key: string): number => {
      const el = refs[key];
      if (!el) return 0;
      const r = el.getBoundingClientRect();
      return r.top + r.height / 2 - aRect.top;
    };
    const cx = (key: string): number => {
      const el = refs[key];
      if (!el) return 0;
      const r = el.getBoundingClientRect();
      return r.left + r.width / 2 - aRect.left;
    };

    const w = area.offsetWidth;
    const h = area.offsetHeight;
    if (w === 0 || h === 0) return;

    // Top divider: horizontal from left edge → midpoint(HI Gain, MID Gain),
    //              diagonal to midpoint(HI Freq, MID Freq),
    //              horizontal to right edge
    const tLx = cx("hiGain");
    const tLy = (cy("hiGain") + cy("midGain")) / 2;
    const tRx = cx("hiFreq");
    const tRy = (cy("hiFreq") + cy("midFreq")) / 2;
    const topPath = `M 0 ${tLy} L ${tLx} ${tLy} L ${tRx} ${tRy} L ${w} ${tRy}`;

    // Bottom divider: same pattern
    const bLx = cx("midQ");
    const bLy = (cy("midQ") + cy("loGain")) / 2;
    const bRx = cx("loFreq");
    const bRy = (cy("midFreq") + cy("loFreq")) / 2;
    const botPath = `M 0 ${bLy} L ${bLx} ${bLy} L ${bRx} ${bRy} L ${w} ${bRy}`;

    // Labels: HI right of HI Gain, MID left of MID Freq, LO right of LO Gain
    const rightColX = cx("hiFreq");
    const leftColX = cx("hiGain");

    setLines({
      w, h,
      topPath,
      botPath,
      hiLabel: { x: rightColX, y: cy("hiGain") },
      midLabel: { x: leftColX, y: cy("midFreq") },
      loLabel: { x: rightColX, y: cy("loGain") },
    });
  }, []);

  useLayoutEffect(() => {
    // Measure once after first render
    requestAnimationFrame(measure);

    // Re-measure on resize
    const area = knobAreaRef.current;
    if (!area) return;
    const observer = new ResizeObserver(measure);
    observer.observe(area);
    return () => observer.disconnect();
  }, [measure]);

  return (
    <div className={css.section}>
      {showSwitch && (
        <button
          className={`${css.switchButton} ${eq.enabled ? css.switchOn : css.switchOff}`}
          onClick={onToggleSwitch}
        >
          {eq.enabled ? "EQ ON" : "EQ OFF"}
        </button>
      )}

      <div className={css.knobArea} ref={knobAreaRef}>
        {lines && (
          <svg className={css.dividers} viewBox={`0 0 ${lines.w} ${lines.h}`}>
            <path d={lines.topPath} fill="none" stroke="#3a3a5a" strokeWidth="2" />
            <path d={lines.botPath} fill="none" stroke="#3a3a5a" strokeWidth="2" />
            <text x={lines.hiLabel.x} y={lines.hiLabel.y + 6} textAnchor="middle" fill="#555" fontSize="18" fontWeight="900" fontFamily="sans-serif">HI</text>
            <text x={lines.midLabel.x} y={lines.midLabel.y + 6} textAnchor="middle" fill="#555" fontSize="18" fontWeight="900" fontFamily="sans-serif">MID</text>
            <text x={lines.loLabel.x} y={lines.loLabel.y + 6} textAnchor="middle" fill="#555" fontSize="18" fontWeight="900" fontFamily="sans-serif">LO</text>
          </svg>
        )}

        <div className={css.knobs}>
          <div className={css.r1} ref={setKnobRef("hiGain")}>
            <EqKnob
              label="Gain" value={eq.highGain} min={0} max={30} defaultValue={15}
              onChange={(v) => onParam(paramBase + 6, v)} formatValue={formatGain} color="#c66"
            />
          </div>
          <div className={css.r2} ref={setKnobRef("hiFreq")}>
            <EqKnob
              label="Freq" value={eq.highFreq} min={0} max={2} defaultValue={1}
              onChange={(v) => onParam(paramBase + 5, v)}
              formatValue={(v) => formatFreq(v, HIGH_FREQ_VALUES)} color="#c66"
            />
          </div>
          <div className={css.r3} ref={setKnobRef("midGain")}>
            <EqKnob
              label="Gain" value={eq.midGain} min={0} max={30} defaultValue={15}
              onChange={(v) => onParam(paramBase + 3, v)} formatValue={formatGain} color="#6c6"
            />
          </div>
          <div className={css.r4} ref={setKnobRef("midFreq")}>
            <EqKnob
              label="Freq" value={eq.midFreq} min={0} max={16} defaultValue={7}
              onChange={(v) => onParam(paramBase + 2, v)}
              formatValue={(v) => formatFreq(v, MID_FREQ_VALUES)} color="#6c6"
            />
          </div>
          <div className={css.r5} ref={setKnobRef("midQ")}>
            <EqKnob
              label="Q" value={eq.midQ} min={0} max={4} defaultValue={1}
              onChange={(v) => onParam(paramBase + 4, v)} formatValue={formatQ} color="#6c6"
            />
          </div>
          <div className={css.r6} ref={setKnobRef("loFreq")}>
            <EqKnob
              label="Freq" value={eq.lowFreq} min={0} max={1} defaultValue={1}
              onChange={(v) => onParam(paramBase + 0, v)}
              formatValue={(v) => formatFreq(v, LOW_FREQ_VALUES)} color="#66c"
            />
          </div>
          <div className={css.r7} ref={setKnobRef("loGain")}>
            <EqKnob
              label="Gain" value={eq.lowGain} min={0} max={30} defaultValue={15}
              onChange={(v) => onParam(paramBase + 1, v)} formatValue={formatGain} color="#66c"
            />
          </div>
        </div>
      </div>
    </div>
  );
}
