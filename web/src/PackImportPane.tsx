import { Fragment, useState } from "react";
import { isFree, libraryTypes, slotLabel, type LibraryType } from "./librarian";
import { PackRow, ProgressBar } from "./PackPane";
import {
  placedSlot,
  type ImportEntry,
  type ImportRun,
  type UseLibraryImportResult,
} from "./useLibraryImport";
import type { UserSlot } from "./userSlots";
import css from "./PackPane.module.css";

interface Props {
  libraryImport: UseLibraryImportResult;
  /** The User slots of the type shown on the left, for the destination list. */
  slots: UserSlot[];
  slotsType: LibraryType;
  /** The selected Part, which a lone tone loads into. */
  part: number;
  /** Focus an entry and show its type's slots on the left. */
  onFocus: (entry: ImportEntry) => void;
  /** An export is running: the device is busy. */
  busy: boolean;
  /** Motional Surround is ON: it mutes the Studio Set Reverb. */
  surroundOn: boolean;
}

/** Where an entry in Temporary goes. */
function temporaryLabel(type: LibraryType, part: number): string {
  return type.studioSet ? "Temporary Studio Set" : `Part ${part + 1}`;
}

/** The Pack pane once a file is opened: entries, destinations, import. */
export function PackImportPane({
  libraryImport: imp,
  slots,
  slotsType,
  part,
  onFocus,
  busy,
  surroundOn,
}: Props) {
  const [confirming, setConfirming] = useState(false);
  const [undo, setUndo] = useState(true);
  const [applyEffects, setApplyEffects] = useState(false);
  /** The last entry ticked or unticked: ⇧-click spans from it. */
  const [anchor, setAnchor] = useState<number | null>(null);
  const { file, entries, run, locked } = imp;
  const types = libraryTypes();
  /** Entries in the order shown: grouped by type. */
  const ordered = types.flatMap((type) => entries.filter((e) => e.type.kind === type.kind));
  const allIncluded = entries.every((e) => e.placement !== "excluded");

  /** Include or leave out `span`, touching only entries that change. */
  const setIncluded = (span: ImportEntry[], included: boolean): void =>
    imp.setIncluded(
      span
        .filter((e) => e.placement !== "temporary" && (e.placement !== "excluded") !== included)
        .map((e) => e.index),
      included,
    );

  /** Tick or untick `entry`, or with ⇧ everything from the anchor to it. */
  const toggle = (entry: ImportEntry, included: boolean, range: boolean): void => {
    const from = range && anchor !== null ? ordered.findIndex((e) => e.index === anchor) : -1;
    const to = ordered.indexOf(entry);
    setIncluded(
      from < 0 ? [entry] : ordered.slice(Math.min(from, to), Math.max(from, to) + 1),
      included,
    );
    setAnchor(entry.index);
  };
  const included = entries.filter((e) => e.placement !== "excluded");
  const overwriting = new Set(imp.overwrites.map((o) => o.entry));
  const runPart = run?.part ?? part;

  const startImport = (): void => {
    setConfirming(false);
    imp.start(undo, applyEffects);
  };

  return (
    <>
      <header className={css.head}>
        <span className={css.title}>
          {file ? `${file.name} · ${file.label}` : "Open a Pack"}
        </span>
        <label className={css.button}>
          Open…
          <input
            type="file"
            accept=".svd,.syx,.mid,.midi"
            hidden
            disabled={imp.busy}
            onChange={(e) => {
              const picked = e.target.files?.[0];
              e.target.value = "";
              if (picked) {
                setConfirming(false);
                setApplyEffects(false);
                imp.open(picked);
              }
            }}
          />
        </label>
        {file && (
          <button
            className={css.button}
            disabled={locked || imp.temporary}
            onClick={() => setIncluded(ordered, !allIncluded)}
          >
            {allIncluded ? "Untick all" : "Tick all"}
          </button>
        )}
        {file && (
          <button className={css.button} disabled={imp.busy} onClick={imp.close}>
            Close
          </button>
        )}
      </header>
      {(imp.opening || imp.openError) && (
        <div className={css.notice}>
          {imp.opening ? (
            <span className={css.hint}>Reading the file and the device's User slots...</span>
          ) : (
            <span className={css.error}>{imp.openError}</span>
          )}
        </div>
      )}
      {(imp.warnings.length > 0 || (imp.unplaced > 0 && !locked)) && (
        <ul className={css.warnings}>
          {imp.unplaced > 0 && !locked && (
            <li className={css.blocking}>
              {imp.unplaced} {imp.unplaced === 1 ? "entry has" : "entries have"} no
              destination: click a slot on the left, pick one in its row, or untick it.
            </li>
          )}
          {imp.warnings.map((w, i) => (
            <li key={i}>{w}</li>
          ))}
        </ul>
      )}
      <div className={css.body}>
        {!file ? (
          <div className={css.empty}>
            Open a .SVD file (a sound export or a backup) or a .syx or .mid SysEx
            dump, or drop it here.
          </div>
        ) : (
          types.map((type) => {
            const group = entries.filter((e) => e.type.kind === type.kind);
            if (group.length === 0) return null;
            return (
              <div key={type.kind}>
                <div className={css.groupHead}>
                  {type.label} · {group.length}
                </div>
                {group.map((entry) => (
                  <Fragment key={entry.index}>
                    <PackRow
                      type={entry.type}
                      label={entry.origin}
                      name={entry.info.name}
                      className={`${css.selectable} ${imp.focused === entry.index ? css.focused : ""} ${entry.placement === "excluded" ? css.excluded : ""}`}
                      onClick={() => onFocus(entry)}
                      lead={
                        <input
                          type="checkbox"
                          checked={entry.placement !== "excluded"}
                          disabled={locked || entry.placement === "temporary"}
                          readOnly
                          onClick={(e) => {
                            e.stopPropagation();
                            toggle(entry, e.currentTarget.checked, e.shiftKey);
                          }}
                          title="Import this entry (⇧-click: a range)"
                        />
                      }
                    >
                      {entry.source && <span className={css.source}>{entry.source}</span>}
                      {entry.report.length > 0 && (
                        <button
                          className={entry.lossy > 0 ? css.lossy : css.notes}
                          aria-expanded={imp.reportShown.has(entry.index)}
                          title={
                            entry.lossy > 0
                              ? "What converting this Foreign patch dropped or approximated"
                              : "Settings this Foreign patch leaves to the Studio Set"
                          }
                          onClick={(e) => {
                            e.stopPropagation();
                            imp.toggleReport(entry.index);
                          }}
                        >
                          {entry.lossy > 0 ? `lossy · ${entry.lossy}` : `notes · ${entry.report.length}`}
                        </button>
                      )}
                      {run && <EntryStatus status={run.statuses[entry.index]} />}
                      <span className={css.destination}>
                        {entry.placement === "temporary" ? (
                          <>
                            {temporaryLabel(entry.type, runPart)}
                            {entry.effects && (
                              <label className={css.check}>
                                <input
                                  type="checkbox"
                                  checked={applyEffects}
                                  disabled={locked}
                                  onChange={(e) => setApplyEffects(e.target.checked)}
                                />
                                Apply its Chorus/Reverb to the Temporary Studio Set
                              </label>
                            )}
                            {entry.effects && applyEffects && surroundOn && (
                              <span className={css.error}>
                                Motional Surround is ON, which mutes the applied Reverb; switch it
                                off to hear it
                              </span>
                            )}
                          </>
                        ) : imp.focused === entry.index &&
                          !locked &&
                          entry.placement !== "excluded" &&
                          slotsType.kind === entry.type.kind ? (
                          <DestinationSelect
                            entry={entry}
                            entries={entries}
                            slots={slots}
                            onAssign={(slot) => imp.assign(entry.index, slot)}
                          />
                        ) : (
                          <DestinationLabel entry={entry} overwrite={overwriting.has(entry.index)} />
                        )}
                      </span>
                    </PackRow>
                    {imp.reportShown.has(entry.index) && (
                      <ul className={css.conversion}>
                        {entry.report.map((item, i) => (
                          <li key={i}>{item}</li>
                        ))}
                      </ul>
                    )}
                  </Fragment>
                ))}
              </div>
            );
          })
        )}
      </div>
      {file && (
        <footer className={css.foot}>
          {run ? (
            <ImportProgress run={run} entries={entries} onCancel={imp.cancel} onClose={imp.close} />
          ) : confirming ? (
            <>
              <div className={css.confirm}>
                <b>
                  {imp.overwrites.length} User slot{imp.overwrites.length === 1 ? "" : "s"} will
                  be overwritten:
                </b>
                <ul className={css.overwrites}>
                  {imp.overwrites.map((o) => (
                    <li key={o.entry}>
                      {o.type.short} {slotLabel(o.type, o.slot)} <s>{o.current}</s> → {o.incoming}
                    </li>
                  ))}
                </ul>
                <label className={css.check}>
                  <input type="checkbox" checked={undo} onChange={(e) => setUndo(e.target.checked)} />
                  Export the overwritten slots first (undo Pack)
                </label>
              </div>
              <div className={css.actions}>
                <button className={css.button} onClick={() => setConfirming(false)}>
                  Back
                </button>
                <button className={css.danger} onClick={startImport}>
                  Overwrite and import
                </button>
              </div>
            </>
          ) : (
            <div className={css.actions}>
              <span className={css.hint}>
                {imp.temporary
                  ? "A single entry loads into Temporary for audition; nothing is stored."
                  : "About 2–3 s per entry. Your current sound is restored afterwards."}
              </span>
              <button
                className={css.primary}
                disabled={busy || imp.unplaced > 0 || included.length === 0}
                onClick={() => (imp.overwrites.length > 0 ? setConfirming(true) : startImport())}
              >
                {imp.temporary
                  ? `Load into ${temporaryLabel(included[0]?.type ?? slotsType, part)}`
                  : `Import ${included.length}…`}
              </button>
            </div>
          )}
        </footer>
      )}
    </>
  );
}

function EntryStatus({ status }: { status: string | undefined }) {
  if (!status || status === "excluded") return null;
  return <span className={`${css.status} ${css[status] ?? ""}`}>{status}</span>;
}

function DestinationLabel({ entry, overwrite }: { entry: ImportEntry; overwrite: boolean }) {
  if (entry.placement === "excluded") return <span className={css.hint}>—</span>;
  const slot = placedSlot(entry.placement);
  if (slot === undefined) return <span className={css.unplaced}>no slot</span>;
  return (
    <span className={overwrite ? css.overwrite : undefined}>
      → {slotLabel(entry.type, slot)}
      {overwrite && " (overwrite)"}
    </span>
  );
}

/** Pick the User slot an entry goes to. */
function DestinationSelect({
  entry,
  entries,
  slots,
  onAssign,
}: {
  entry: ImportEntry;
  entries: ImportEntry[];
  slots: UserSlot[];
  onAssign: (slot: number) => void;
}) {
  const takenBy = new Map<number, string>();
  for (const other of entries) {
    const slot = placedSlot(other.placement);
    if (other.type.kind === entry.type.kind && other.index !== entry.index && slot !== undefined) {
      takenBy.set(slot, other.info.name);
    }
  }
  return (
    <select
      className={css.select}
      value={placedSlot(entry.placement) ?? ""}
      onClick={(e) => e.stopPropagation()}
      onChange={(e) => onAssign(Number(e.target.value))}
    >
      <option value="" disabled>
        no slot
      </option>
      {slots.map((s) => {
        const other = takenBy.get(s.slot);
        return (
          <option key={s.key} value={s.slot}>
            {s.number} {s.name}
            {isFree(entry.type, s.name) ? "" : " (overwrite)"}
            {other ? ` ← ${other}` : ""}
          </option>
        );
      })}
    </select>
  );
}

/** Progress of a running import, then its report. */
function ImportProgress({
  run,
  entries,
  onCancel,
  onClose,
}: {
  run: ImportRun;
  entries: ImportEntry[];
  onCancel: () => void;
  onClose: () => void;
}) {
  const included = entries.filter((e) => run.statuses[e.index] !== "excluded");
  const finished = included.filter((e) => {
    const s = run.statuses[e.index];
    return s === "stored" || s === "loaded" || s === "failed";
  });
  const stored = included.filter((e) => run.statuses[e.index] === "stored");
  const notStored = included.filter((e) => run.statuses[e.index] !== "stored");
  const loaded = included.find((e) => run.statuses[e.index] === "loaded");

  if (run.phase !== "done") {
    return (
      <div className={css.actions}>
        <div className={css.grow}>
          <ProgressBar done={finished.length} total={run.phase === "undo" ? 0 : included.length}>
            {run.phase === "undo"
              ? `Exporting the slots to overwrite into ${run.undoFile ?? ""}…`
              : run.temporary
                ? "Loading into Temporary…"
                : `Stored ${finished.length}/${included.length}. Cancel stops after the current slot.`}
          </ProgressBar>
        </div>
        {!run.temporary && (
          <button className={css.button} onClick={onCancel}>
            Cancel
          </button>
        )}
      </div>
    );
  }

  return (
    <div className={css.actions}>
      <div className={css.report}>
        {run.temporary ? (
          loaded ? (
            <span>
              Loaded “{loaded.info.name}” into {temporaryLabel(loaded.type, run.part)}. Use SAVE in
              the top bar to keep it.
            </span>
          ) : (
            <span className={css.error}>Not loaded: {run.error}</span>
          )
        ) : (
          <>
            <span>
              Stored {stored.length} of {included.length}.
              {run.undoFile && ` Undo Pack: ${run.undoFile}.`}
            </span>
            {notStored.length > 0 && (
              <span className={css.error}>
                Not stored:{" "}
                {notStored
                  .map((e) => `${e.info.name} (${run.statuses[e.index] === "failed" ? "failed" : "not stored"})`)
                  .join(", ")}
              </span>
            )}
            {run.error && <span className={css.error}>Stopped: {run.error}</span>}
            <span className={css.hint}>
              The Temporary areas borrowed for storing were restored.
            </span>
          </>
        )}
      </div>
      <button className={css.primary} onClick={onClose}>
        Close
      </button>
    </div>
  );
}
