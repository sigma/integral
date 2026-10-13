import { useEffect, useRef, useState } from "react";
import type { IntegraService } from "./integra";
import { isFree, packFileBase, PACK_EXTENSION, slotLabel, type LibraryType } from "./librarian";
import { ProgressBar } from "./PackPane";
import type { ToneCatalog } from "./toneCatalog";
import { downloadUndo, refreshNames } from "./useLibraryImport";
import type { UseMixerResult } from "./useMixer";
import type { UserSlot } from "./userSlots";
import css from "./ReinitDialog.module.css";

interface Props {
  service: IntegraService;
  mixer: UseMixerResult;
  catalog: ToneCatalog;
  type: LibraryType;
  /** The User slots to clear. */
  slots: UserSlot[];
  onClose: () => void;
}

type Phase =
  | { kind: "confirm" }
  | { kind: "undo" }
  | { kind: "storing"; done: number }
  | { kind: "done"; stored: number; error?: string };

/**
 * Modal that confirms clearing User slots back to INIT, then has the device
 * store INIT into them, showing its progress.
 */
export function ReinitDialog({ service, mixer, catalog, type, slots, onClose }: Props) {
  const [phase, setPhase] = useState<Phase>({ kind: "confirm" });
  const [undo, setUndo] = useState(true);
  const abortRef = useRef<AbortController | null>(null);
  const running = phase.kind === "undo" || phase.kind === "storing";
  /** The slots holding something: the ones the clear erases. */
  const used = slots.filter((s) => !isFree(type, s.name));
  const plural = slots.length === 1 ? "" : "s";
  const undoFile = `${packFileBase(`UNDO${type.short}`)}${PACK_EXTENSION}`;

  // Escape closes, never mid-run.
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !running) onClose();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [onClose, running]);

  const run = async (): Promise<void> => {
    const controller = new AbortController();
    abortRef.current = controller;
    const { signal } = controller;
    const { loadStudioSetNames } = mixer;
    try {
      if (undo && used.length > 0) {
        setPhase({ kind: "undo" });
        const sources = used.map((s) => ({ source: "user" as const, kind: type.kind, slot: s.slot }));
        await downloadUndo(service, undoFile, sources, signal);
      }
      setPhase({ kind: "storing", done: 0 });
      const result = await service.reinitSlots(
        type.kind,
        slots.map((s) => s.slot),
        mixer.state.selectedPart,
        signal,
        (statuses) =>
          setPhase({
            kind: "storing",
            done: statuses.filter((s) => s === "stored" || s === "failed").length,
          }),
      );
      const stored = result.statuses.filter((s) => s === "stored").length;
      setPhase({ kind: "done", stored, error: result.error });
      mixer.reload();
      if (stored > 0) refreshNames([type.kind], catalog, loadStudioSetNames);
    } catch (e) {
      setPhase({ kind: "done", stored: 0, error: e instanceof Error ? e.message : String(e) });
    }
  };

  return (
    <div className={css.overlay} onClick={running ? undefined : onClose}>
      <div className={css.modal} onClick={(e) => e.stopPropagation()}>
        <div className={css.header}>
          <span className={css.title}>
            Re-init {slots.length} {type.label} slot{plural}
          </span>
          <button className={css.closeButton} onClick={onClose} disabled={running}>
            ✕
          </button>
        </div>
        <div className={css.body}>
          {phase.kind === "confirm" &&
            (used.length > 0 ? (
              <>
                <b>
                  {used.length} slot{used.length === 1 ? "" : "s"} will be erased:
                </b>
                <ul className={css.list}>
                  {used.map((s) => (
                    <li key={s.key}>
                      {slotLabel(type, s.slot)} {s.name}
                    </li>
                  ))}
                </ul>
                <label className={css.check}>
                  <input
                    type="checkbox"
                    checked={undo}
                    onChange={(e) => setUndo(e.target.checked)}
                  />
                  Export them first ({undoFile})
                </label>
              </>
            ) : (
              <span>The selected slots are already free.</span>
            ))}
          {phase.kind === "undo" && (
            <ProgressBar done={0} total={0}>
              Exporting the slots to erase into {undoFile}…
            </ProgressBar>
          )}
          {phase.kind === "storing" && (
            <ProgressBar done={phase.done} total={slots.length}>
              Re-initialized {phase.done}/{slots.length}. Cancel stops after the current slot.
            </ProgressBar>
          )}
          {phase.kind === "done" && (
            <span>
              Re-initialized {phase.stored}/{slots.length} slot{plural}.
              {phase.error && <span className={css.error}> {phase.error}</span>}
            </span>
          )}
        </div>
        <div className={css.footer}>
          <span className={css.status}>
            {phase.kind === "confirm" && "About 2–3 s per slot. Your current sound is restored afterwards."}
          </span>
          {phase.kind === "confirm" && (
            <>
              <button className={css.footerButton} onClick={onClose}>
                Cancel
              </button>
              <button className={css.footerButton} onClick={() => void run()}>
                Re-init
              </button>
            </>
          )}
          {running && (
            <button className={css.footerButton} onClick={() => abortRef.current?.abort()}>
              Cancel
            </button>
          )}
          {phase.kind === "done" && (
            <button className={css.footerButton} onClick={onClose}>
              Close
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
