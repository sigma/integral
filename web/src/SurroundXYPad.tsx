import { useCallback, useRef } from "react";
import type { SurroundPartState } from "./types";
import css from "./SurroundXYPad.module.css";

interface PartDot {
  index: number;
  label: string;
  state: SurroundPartState;
}

interface Props {
  parts: PartDot[];
  selectedPart: number | null;
  onSelect: (index: number) => void;
  onMove: (index: number, lr: number, fb: number) => void;
}

/** Map MIDI value (0–127) to 0–1 range. */
function midiToNorm(v: number): number {
  return v / 127;
}

/** Map 0–1 range to MIDI value (0–127). */
function normToMidi(v: number): number {
  return Math.round(Math.max(0, Math.min(127, v * 127)));
}

/** Format LR/FB for display: -64 to +63. */
function formatPos(v: number): string {
  const display = v - 64;
  if (display === 0) return "C";
  return display > 0 ? `+${display}` : String(display);
}

export function SurroundXYPad({ parts, selectedPart, onSelect, onMove }: Props) {
  const padRef = useRef<HTMLDivElement>(null);
  const dragging = useRef<number | null>(null);

  const getXY = useCallback((e: React.PointerEvent | PointerEvent) => {
    const rect = padRef.current?.getBoundingClientRect();
    if (!rect) return null;
    const x = (e.clientX - rect.left) / rect.width;
    const y = (e.clientY - rect.top) / rect.height;
    return { x: Math.max(0, Math.min(1, x)), y: Math.max(0, Math.min(1, y)) };
  }, []);

  const handlePointerDown = useCallback(
    (e: React.PointerEvent, index: number) => {
      e.stopPropagation();
      dragging.current = index;
      onSelect(index);
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
    },
    [onSelect],
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (dragging.current === null) return;
      const xy = getXY(e);
      if (!xy) return;
      onMove(dragging.current, normToMidi(xy.x), normToMidi(1 - xy.y));
    },
    [getXY, onMove],
  );

  const handlePointerUp = useCallback(() => {
    dragging.current = null;
  }, []);

  const handlePadClick = useCallback(
    (e: React.PointerEvent) => {
      if (selectedPart === null) return;
      const xy = getXY(e);
      if (!xy) return;
      onMove(selectedPart, normToMidi(xy.x), normToMidi(1 - xy.y));
    },
    [selectedPart, getXY, onMove],
  );

  return (
    <div className={css.container}>
      <div className={css.labels}>
        <span className={css.labelTop}>Front</span>
        <span className={css.labelBottom}>Back</span>
        <span className={css.labelLeft}>L</span>
        <span className={css.labelRight}>R</span>
      </div>
      <div
        ref={padRef}
        className={css.pad}
        onPointerDown={handlePadClick}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
      >
        {/* Grid lines */}
        <div className={css.gridH} />
        <div className={css.gridV} />

        {/* Part dots */}
        {parts.map((p) => {
          const x = midiToNorm(p.state.lr) * 100;
          const y = (1 - midiToNorm(p.state.fb)) * 100;
          const isSelected = p.index === selectedPart;
          return (
            <div
              key={p.index}
              className={`${css.dot} ${isSelected ? css.dotSelected : ""}`}
              style={{ left: `${x}%`, top: `${y}%` }}
              onPointerDown={(e) => handlePointerDown(e, p.index)}
              title={`${p.label}: LR=${formatPos(p.state.lr)} FB=${formatPos(p.state.fb)}`}
            >
              <span className={css.dotLabel}>{p.label}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
