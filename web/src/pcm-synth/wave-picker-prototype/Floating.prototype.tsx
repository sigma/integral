// PROTOTYPE ONLY (throwaway): a portal anchored under an element, so it is not
// clipped by the SectionPanel's overflow. Closes on outside click / Escape.
import { useEffect, useRef, type ReactNode, type RefObject } from "react";
import { createPortal } from "react-dom";
import css from "./prototype.module.css";

export function Floating({
  anchor,
  onClose,
  width,
  children,
}: {
  anchor: RefObject<HTMLElement | null>;
  onClose: () => void;
  width: number;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const onDown = (e: PointerEvent) => {
      const t = e.target as Node;
      if (ref.current?.contains(t) || anchor.current?.contains(t)) return;
      onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("pointerdown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("pointerdown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [anchor, onClose]);

  const rect = anchor.current?.getBoundingClientRect();
  const top = (rect?.bottom ?? 0) + 2;
  const left = Math.min(rect?.left ?? 0, window.innerWidth - width - 8);
  return createPortal(
    <div ref={ref} className={css.floating} style={{ top, left, width, maxHeight: window.innerHeight - top - 60 }}>
      {children}
    </div>,
    document.body,
  );
}
