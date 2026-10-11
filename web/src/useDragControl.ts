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
   * Pixels of vertical drag per 1-unit change at normal sensitivity, capped so
   * a full sweep never exceeds `MAX_SWEEP_PX`. Lower = more sensitive. Default: 1.
   */
  pxPerStep?: number;
}

/** Longest drag, in pixels, that sweeps the whole range at normal sensitivity. */
const MAX_SWEEP_PX = 300;
/** Minimum pixels per 1-unit change with Shift held, so wide ranges can be set to single steps. */
const FINE_PX_PER_STEP = 5;

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
      accumulator.current += dy;

      const normal = Math.min(pxPerStep, MAX_SWEEP_PX / Math.max(1, max - min));
      const px = e.shiftKey ? Math.max(normal, FINE_PX_PER_STEP) : normal;
      const steps = Math.trunc(accumulator.current / px);
      if (steps !== 0) {
        accumulator.current -= steps * px;
        onChange(clamp(value + steps));
      }
    },
    [value, onChange, min, max, pxPerStep],
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
