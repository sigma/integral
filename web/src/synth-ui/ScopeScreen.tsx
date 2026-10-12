import type { ReactNode } from "react";
import css from "./Scope.module.css";

/**
 * Oscilloscope screen filling its grid cell without adding to its height.
 * Children are SVG traces in a 0..1 box, y = 0 at the top.
 */
export function ScopeScreen({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div className={`${css.slot} ${className ?? ""}`}>
      <svg className={css.fill} viewBox="-0.05 -0.15 1.1 1.3" preserveAspectRatio="none">
        {children}
      </svg>
    </div>
  );
}

/** A polyline path through `n + 1` samples of `y(x)` for x in 0..1. */
export function sampled(n: number, y: (x: number) => number): string {
  return Array.from({ length: n + 1 }, (_, i) => `${i ? "L" : "M"} ${i / n} ${y(i / n)}`).join(" ");
}
