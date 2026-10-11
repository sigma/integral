import { useState, useEffect, useCallback } from "react";
import { SlotRow, type SaveSlot } from "./SlotRow";
import css from "./SaveDialog.module.css";

interface Props<T extends SaveSlot> {
  /** Dialog title, e.g. "Save Tone — Part 1". */
  title: string;
  /** Current name of what is being saved; editable before saving. */
  sourceName: string;
  /** Maximum name length the device stores. */
  maxNameLength: number;
  slots: T[];
  loading: boolean;
  /**
   * Perform the save under `name`; resolves when the device reports it
   * finished.
   */
  onSave: (slot: T, name: string) => Promise<void>;
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
  maxNameLength,
  slots,
  loading,
  onSave,
  onClose,
}: Props<T>) {
  const [selected, setSelected] = useState<T | null>(null);
  const [phase, setPhase] = useState<Phase>({ kind: "pick" });
  const [name, setName] = useState(sourceName.slice(0, maxNameLength));
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
      await onSave(selected, name);
      onClose();
    } catch (e) {
      setPhase({
        kind: "error",
        message: e instanceof Error ? e.message : String(e),
      });
    }
  }, [selected, name, onSave, onClose]);

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
        <label className={css.nameRow}>
          <span className={css.nameLabel}>Name</span>
          <input
            className={css.nameInput}
            value={name}
            maxLength={maxNameLength}
            disabled={saving}
            onChange={(e) =>
              // The device stores printable ASCII only.
              setName(e.target.value.replace(/[^\x20-\x7E]/g, ""))
            }
          />
        </label>
        <div className={css.slotList}>
          {slots.length === 0 ? (
            <div className={css.loading}>
              {loading ? "Loading slots..." : "No reply from the INTEGRA-7"}
            </div>
          ) : (
            slots.map((slot) => (
              <SlotRow
                key={slot.key}
                slot={slot}
                active={slot.key === selected?.key}
                disabled={saving}
                onClick={() => {
                  setSelected(slot);
                  setPhase({ kind: "pick" });
                }}
              />
            ))
          )}
        </div>
        <div className={css.footer}>
          <span className={css.status}>
            {phase.kind === "pick" &&
              (selected
                ? `Save "${name}" to ${selected.number}`
                : "Choose a user slot")}
            {phase.kind === "confirm" &&
              selected &&
              `Overwrite ${selected.number} "${selected.name}" with "${name}"?`}
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
              disabled={!selected || saving || name.trim() === ""}
            >
              Save
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
