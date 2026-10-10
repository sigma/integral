import { useState, useEffect, useCallback } from "react";
import css from "./SaveDialog.module.css";

/** A user slot the dialog can save into. */
export interface SaveSlot {
  /** Stable React key. */
  key: string;
  /** Slot number as shown on the device (e.g. "0129", "64"). */
  number: string;
  /** Name currently stored in the slot. */
  name: string;
}

interface Props<T extends SaveSlot> {
  /** Dialog title, e.g. "Save Tone — Part 1". */
  title: string;
  /** Name of what is being saved (shown in the confirmation). */
  sourceName: string;
  slots: T[];
  loading: boolean;
  /** Perform the save; resolves when the device reports it finished. */
  onSave: (slot: T) => Promise<void>;
  onClose: () => void;
}

type Phase =
  | { kind: "pick" }
  | { kind: "confirm" }
  | { kind: "saving" }
  | { kind: "error"; message: string };

/** Modal that picks a user slot, confirms the overwrite, and runs the save. */
export function SaveDialog<T extends SaveSlot>({
  title,
  sourceName,
  slots,
  loading,
  onSave,
  onClose,
}: Props<T>) {
  const [selected, setSelected] = useState<T | null>(null);
  const [phase, setPhase] = useState<Phase>({ kind: "pick" });
  const saving = phase.kind === "saving";

  // Escape backs out of the confirmation, else closes (never mid-save).
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || phase.kind === "saving") return;
      if (phase.kind === "confirm") setPhase({ kind: "pick" });
      else onClose();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [onClose, phase.kind]);

  const handleSave = useCallback(async () => {
    if (!selected) return;
    setPhase({ kind: "saving" });
    try {
      await onSave(selected);
      onClose();
    } catch (e) {
      setPhase({
        kind: "error",
        message: e instanceof Error ? e.message : String(e),
      });
    }
  }, [selected, onSave, onClose]);

  return (
    <div className={css.overlay} onClick={saving ? undefined : onClose}>
      <div className={css.modal} onClick={(e) => e.stopPropagation()}>
        <div className={css.header}>
          <span className={css.title}>{title}</span>
          <button
            className={css.closeButton}
            onClick={onClose}
            disabled={saving}
          >
            ✕
          </button>
        </div>
        <div className={css.slotList}>
          {slots.length === 0 ? (
            <div className={css.loading}>
              {loading ? "Loading slots..." : "No reply from the INTEGRA-7"}
            </div>
          ) : (
            slots.map((slot) => {
              const isActive = slot.key === selected?.key;
              return (
                <button
                  key={slot.key}
                  className={`${css.slot} ${isActive ? css.slotActive : ""}`}
                  onClick={() => {
                    setSelected(slot);
                    setPhase({ kind: "pick" });
                  }}
                  disabled={saving}
                >
                  <span className={css.slotNumber}>{slot.number}</span>
                  <span className={css.slotName}>{slot.name}</span>
                </button>
              );
            })
          )}
        </div>
        <div className={css.footer}>
          <span className={css.status}>
            {phase.kind === "pick" &&
              (selected
                ? `Save "${sourceName}" to ${selected.number}`
                : "Choose a user slot")}
            {phase.kind === "confirm" &&
              selected &&
              `Overwrite ${selected.number} "${selected.name}" with "${sourceName}"?`}
            {phase.kind === "saving" && "Saving..."}
            {phase.kind === "error" && (
              <span className={css.error}>{phase.message}</span>
            )}
          </span>
          {phase.kind === "confirm" ? (
            <>
              <button
                className={css.footerButton}
                onClick={() => setPhase({ kind: "pick" })}
              >
                Cancel
              </button>
              <button className={css.footerButton} onClick={handleSave}>
                Overwrite
              </button>
            </>
          ) : (
            <button
              className={css.footerButton}
              onClick={() => setPhase({ kind: "confirm" })}
              disabled={!selected || saving}
            >
              Save
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
