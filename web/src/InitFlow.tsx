/**
 * The flow every "Init…" item shares (#288): confirm, run the device's Init,
 * keep the area busy and inert until "finished", then read it back through
 * the paths that already exist.
 */

import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { deviceSpec } from "./deviceSpec";
import { covers, describeInit, type InitScope } from "./init";
import { LINK_LOST_ERROR, type IntegraService } from "./integra";
import type { UseMixerResult } from "./useMixer";
import css from "./InitFlow.module.css";

/** What an area's "Init…" item and its views use. */
export interface InitFlow {
  /** Ask to reset `scope`: opens its confirmation. Ignored while another Init is asked or runs. */
  request: (scope: InitScope) => void;
  /** The Init running, until the device answers "finished" or it fails. */
  running: InitScope | null;
  /** How many Inits have reset Part `part`'s tone: a tone editor remounts on a change. */
  rereads: (part: number) => number;
}

const NO_FLOW: InitFlow = {
  request: () => {
    throw new Error("No InitProvider above this Init item");
  },
  running: null,
  rereads: () => 0,
};

const InitContext = createContext<InitFlow>(NO_FLOW);

/** The shared Init flow. Outside an `InitProvider`, nothing runs and `request` throws. */
export function useInit(): InitFlow {
  return useContext(InitContext);
}

type Phase =
  | { kind: "idle" }
  | { kind: "confirm"; scope: InitScope }
  | { kind: "running"; scope: InitScope }
  /** `outcome` says what the failure left: nothing reset, or an area read again or unknown. */
  | { kind: "failed"; scope: InitScope; error: string; outcome: string };

interface ProviderProps {
  /** None while no INTEGRA-7 is connected: a failure that lost the link still shows. */
  service: IntegraService | null;
  mixer: Pick<UseMixerResult, "reload">;
  children: ReactNode;
}

/**
 * Runs one Init at a time for the views below, and shows its confirmation
 * and failure. Keep it mounted across a lost link, so the failure shows.
 */
export function InitProvider({ service, mixer, children }: ProviderProps) {
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });
  const [rereadCounts, setRereadCounts] = useState<ReadonlyMap<number, number>>(new Map());
  const { reload } = mixer;

  const request = useCallback((scope: InitScope) => {
    setPhase((current) => (current.kind === "idle" ? { kind: "confirm", scope } : current));
  }, []);
  const close = useCallback(() => setPhase({ kind: "idle" }), []);

  /** Read back what `scope`'s Init reset. */
  const reread = useCallback(
    (service: IntegraService, scope: InitScope) => {
      const allParts = Array.from({ length: deviceSpec().part_count }, (_, part) => part);
      // The Parts whose tone the Init reset or reloaded.
      let tones: number[] = [];
      switch (scope.kind) {
        case "studio-set":
          reload();
          tones = allParts;
          break;
        case "sound-ctrl":
          for (const part of allParts) service.device.reReadPart(part);
          break;
        case "part":
        case "tone":
          // The Part's dump, and its tone's name with it.
          service.device.reReadPart(scope.part);
          tones = [scope.part];
          break;
        case "partial":
          tones = [scope.part];
          break;
      }
      if (tones.length === 0) return;
      setRereadCounts((prev) => {
        const next = new Map(prev);
        for (const part of tones) next.set(part, (next.get(part) ?? 0) + 1);
        return next;
      });
    },
    [reload],
  );

  const run = useCallback(
    async (scope: InitScope) => {
      if (!service) {
        setPhase({ kind: "failed", scope, error: "no INTEGRA-7 is connected", outcome: "Nothing was reset." });
        return;
      }
      setPhase({ kind: "running", scope });
      let error: string | undefined;
      try {
        error = await service.initialize(scope);
      } catch (e) {
        // Refused before sending: a save, an import or a load is running.
        const reason = e instanceof Error ? e.message : String(e);
        setPhase({ kind: "failed", scope, error: reason, outcome: "Nothing was reset." });
        return;
      }
      if (error === LINK_LOST_ERROR) {
        setPhase({ kind: "failed", scope, error, outcome: "What it resets is unknown until it is read again." });
        return;
      }
      // A timeout may have reset the area all the same: read it back too.
      reread(service, scope);
      setPhase(
        error === undefined
          ? { kind: "idle" }
          : { kind: "failed", scope, error, outcome: "It is read again from the device." },
      );
    },
    [service, reread],
  );

  const flow = useMemo<InitFlow>(
    () => ({
      request,
      running: phase.kind === "running" ? phase.scope : null,
      rereads: (part) => rereadCounts.get(part) ?? 0,
    }),
    [request, phase, rereadCounts],
  );

  return (
    <InitContext.Provider value={flow}>
      {children}
      {phase.kind === "confirm" && (
        <InitDialog scope={phase.scope} onClose={close} onConfirm={() => void run(phase.scope)} />
      )}
      {phase.kind === "failed" && (
        <InitDialog scope={phase.scope} error={`${phase.error}. ${phase.outcome}`} onClose={close} />
      )}
    </InitContext.Provider>
  );
}

interface DialogProps {
  scope: InitScope;
  /** Why the Init failed and what that left; absent while it is being confirmed. */
  error?: string;
  /** Absent once the Init failed. */
  onConfirm?: () => void;
  onClose: () => void;
}

/** The Init's confirmation, or its failure. */
function InitDialog({ scope, error, onConfirm, onClose }: DialogProps) {
  const { title, resets } = describeInit(scope);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [onClose]);

  return (
    <div className={css.overlay} onClick={onClose}>
      <div
        className={css.modal}
        role={error === undefined ? "dialog" : "alertdialog"}
        aria-modal="true"
        aria-label={title}
        onClick={(e) => e.stopPropagation()}
      >
        <div className={css.header}>
          <span className={css.title}>{title}</span>
          <button className={css.closeButton} onClick={onClose}>
            ✕
          </button>
        </div>
        <div className={css.body}>
          {error === undefined ? (
            <>
              <span>This resets {resets}</span>
              <span>Only the Temporary memory changes: stored User slots aren't touched.</span>
            </>
          ) : (
            <span className={css.error} role="alert">
              {title} failed: {error}
            </span>
          )}
        </div>
        <div className={css.footer}>
          <span className={css.status} />
          {onConfirm ? (
            <>
              <button className={css.footerButton} onClick={onClose}>
                Cancel
              </button>
              <button className={css.footerButton} onClick={onConfirm}>
                Init
              </button>
            </>
          ) : (
            <button className={css.footerButton} onClick={onClose}>
              Close
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

/**
 * Wraps an area's controls, laid out as if unwrapped: dimmed, inert and
 * `aria-busy` while an Init that resets all of `area` runs.
 */
export function InitGuard({ area, children }: { area: InitScope; children: ReactNode }) {
  const { running } = useInit();
  const busy = running !== null && covers(running, area);
  return (
    <div className={`${css.guard} ${busy ? css.busy : ""}`} inert={busy} aria-busy={busy}>
      {children}
    </div>
  );
}

/**
 * An area's "Init…" item: asks for `scope`'s Init, which confirms first.
 * Named by what it resets; disabled while an Init runs. `className` styles it
 * as its place's buttons (a page header's); a small item by default.
 */
export function InitItem({ scope, className = css.item }: { scope: InitScope; className?: string }) {
  const { request, running } = useInit();
  const name = `${describeInit(scope).title}…`;
  return (
    <button
      type="button"
      className={className}
      aria-label={name}
      title={name}
      disabled={running !== null}
      onClick={() => request(scope)}
    >
      Init…
    </button>
  );
}
