import { useState } from "react";

interface Props {
  value: number;
  min: number;
  max: number;
  onCommit: (value: number) => void;
  className?: string;
}

/**
 * Integer field that commits on Enter or blur, clamped to `[min, max]`.
 * Holds the typed text locally so "1" on the way to "120" isn't clamped mid-edit.
 */
export function NumberInput({ value, min, max, onCommit, className }: Props) {
  const [draft, setDraft] = useState<string | null>(null);

  const commit = () => {
    if (draft === null) return;
    const parsed = Number.parseInt(draft, 10);
    if (Number.isFinite(parsed)) onCommit(Math.min(max, Math.max(min, parsed)));
    setDraft(null);
  };

  return (
    <input
      className={className}
      type="number"
      min={min}
      max={max}
      value={draft ?? value}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => e.key === "Enter" && commit()}
    />
  );
}
