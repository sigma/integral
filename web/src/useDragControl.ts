/**
 * Reusable hook for pointer-drag value control (knobs and faders).
 *
 * Encapsulates the common pattern: pointerDown → track delta → apply to value.
 * Supports shift-key fine control and double-click-to-reset.
 */

import { useCallback, useRef } from "react";

interface UseDragControlOptions {
  /** Current value. */
  value: number;
  /** Minimum value. */
  min: number;
  /** Maximum value. */
  max: number;
  /** Value to restore on double-click. */
  defaultValue: number;
  /** Called with the new clamped integer value. */
  onChange: (value: number) => void;
  /**
   * Pixels of vertical drag per 1-unit change at normal sensitivity.
   * Lower = more sensitive. Default: 1.
   */
  pxPerStep?: number;
  /** Shift-key sensitivity multiplier (0–1). Default: 0.2. */
  shiftScale?: number;
}

interface UseDragControlResult {
  onPointerDown: (e: React.PointerEvent) => void;
  onPointerMove: (e: React.PointerEvent) => void;
  onPointerUp: () => void;
  onDoubleClick: () => void;
  onKeyDown: (e: React.KeyboardEvent) => void;
}

/**
 * Hook that provides pointer-drag and keyboard handlers for a slider-style
 * control. Accumulates sub-pixel drag deltas so fine-grained ranges work
 * smoothly.
 */
export function useDragControl({
  value,
  min,
  max,
  defaultValue,
  onChange,
  pxPerStep = 1,
  shiftScale = 0.2,
}: UseDragControlOptions): UseDragControlResult {
  const dragging = useRef(false);
  const lastY = useRef(0);
  const accumulator = useRef(0);

  const clamp = (v: number) => Math.round(Math.max(min, Math.min(max, v)));

  const onPointerDown = useCallback((e: React.PointerEvent) => {
    dragging.current = true;
    lastY.current = e.clientY;
    accumulator.current = 0;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }, []);

  const onPointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!dragging.current) return;
      const dy = lastY.current - e.clientY;
      lastY.current = e.clientY;
      const sensitivity = e.shiftKey ? shiftScale : 1;
      accumulator.current += dy * sensitivity;

      const steps = Math.trunc(accumulator.current / pxPerStep);
      if (steps !== 0) {
        accumulator.current -= steps * pxPerStep;
        onChange(clamp(value + steps));
      }
    },
    [value, onChange, min, max, pxPerStep, shiftScale],
  );

  const onPointerUp = useCallback(() => {
    dragging.current = false;
  }, []);

  const onDoubleClick = useCallback(() => {
    onChange(defaultValue);
  }, [onChange, defaultValue]);

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      const step = e.shiftKey ? 10 : 1;
      if (e.key === "ArrowUp" || e.key === "ArrowRight") {
        e.preventDefault();
        onChange(clamp(value + step));
      } else if (e.key === "ArrowDown" || e.key === "ArrowLeft") {
        e.preventDefault();
        onChange(clamp(value - step));
      } else if (e.key === "Home") {
        e.preventDefault();
        onChange(max);
      } else if (e.key === "End") {
        e.preventDefault();
        onChange(min);
      }
    },
    [value, onChange, min, max],
  );

  return { onPointerDown, onPointerMove, onPointerUp, onDoubleClick, onKeyDown };
}
