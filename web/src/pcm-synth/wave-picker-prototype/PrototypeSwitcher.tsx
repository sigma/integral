// PROTOTYPE ONLY (throwaway): floating ← / → variant switcher (dev builds only).
import { useEffect } from "react";
import css from "./prototype.module.css";
import { VARIANTS, setVariant, useVariant } from "./variantStore.prototype";

function isEditable(el: Element | null): boolean {
  if (!(el instanceof HTMLElement)) return false;
  return el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT" || el.isContentEditable;
}

export function PrototypeSwitcher() {
  const current = useVariant();
  const idx = VARIANTS.findIndex((v) => v.id === current);
  const step = (d: number) => {
    const next = VARIANTS[(idx + d + VARIANTS.length) % VARIANTS.length];
    if (next) setVariant(next.id);
  };

  useEffect(() => {
    if (!import.meta.env.DEV) return;
    const onKey = (e: KeyboardEvent) => {
      if (isEditable(document.activeElement)) return;
      if (e.key === "ArrowLeft") step(-1);
      else if (e.key === "ArrowRight") step(1);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  if (!import.meta.env.DEV) return null;
  return (
    <div className={css.switcher}>
      <button className={css.button} onClick={() => step(-1)}>←</button>
      <span className={css.switcherLabel}>{VARIANTS[idx]?.label}</span>
      <button className={css.button} onClick={() => step(1)}>→</button>
    </div>
  );
}
