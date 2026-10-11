import { PACK_EXTENSION, toneTypeOf } from "./librarian";
import { PackRow, ProgressBar } from "./PackPane";
import type { UseLibraryExportResult, ExportRun } from "./useLibraryExport";
import type { UseMixerResult } from "./useMixer";
import css from "./PackPane.module.css";

interface Props {
  libraryExport: UseLibraryExportResult;
  mixer: UseMixerResult;
  /** An import is running: the device is busy. */
  busy: boolean;
}

/** The Pack pane while building an export: its items, file name and Export. */
export function PackExportPane({ libraryExport, mixer, busy }: Props) {
  const { items, run, add, remove } = libraryExport;
  const { state } = mixer;
  const part = state.selectedPart;
  const partState = state.parts[part];
  const partType = partState ? toneTypeOf(partState.toneBankMsb) : undefined;
  const running = run?.phase === "reading";

  return (
    <>
      <header className={css.head}>
        <span className={css.title}>Pack · {items.length}</span>
        <button
          className={css.button}
          disabled={!partType || running}
          onClick={() =>
            partType &&
            add([
              {
                source: { source: "partTone", kind: partType.kind, part },
                name: partState?.toneName ?? "",
              },
            ])
          }
          title="The selected Part's tone, unsaved edits included"
        >
          + Part {part + 1} tone
        </button>
        <button
          className={css.button}
          disabled={running}
          onClick={() =>
            add([{ source: { source: "studioSet" }, name: state.studioSetName }])
          }
          title="The current Studio Set, unsaved edits included"
        >
          + Current Studio Set
        </button>
      </header>
      <div className={css.body}>
        {items.length === 0 ? (
          <div className={css.empty}>
            Select User slots on the left and add them, or add the current Part
            tone or Studio Set.
          </div>
        ) : (
          items.map((item) => (
            <PackRow
              key={item.key}
              type={item.type}
              label={item.label}
              name={item.name}
              className={item.auto ? css.auto : undefined}
              lead={
                item.auto && (
                  <span title="Used by a Studio Set of the Pack: added with it">+</span>
                )
              }
            >
              {item.status && (
                <span className={`${css.status} ${css[item.status] ?? ""}`}>
                  {item.status}
                </span>
              )}
              {!item.auto && !running && (
                <button
                  className={css.remove}
                  onClick={() => remove(item.key)}
                  title="Remove from the Pack"
                >
                  ✕
                </button>
              )}
            </PackRow>
          ))
        )}
      </div>
      <footer className={css.foot}>
        <span className={css.hint}>
          The User tones a Studio Set uses are added, marked +, once the set is
          read. Export only reads from the device.
        </span>
        {run && <ExportStatus run={run} />}
        <div className={css.actions}>
          <label className={css.fileName}>
            File
            <input
              className={css.fileInput}
              value={libraryExport.fileBase}
              disabled={running}
              onChange={(e) => libraryExport.setFileBase(e.target.value)}
            />
            {PACK_EXTENSION}
          </label>
          {running ? (
            <button className={css.button} onClick={libraryExport.cancel}>
              Cancel
            </button>
          ) : (
            <button
              className={css.primary}
              disabled={items.length === 0 || libraryExport.fileBase === "" || busy}
              onClick={libraryExport.start}
            >
              Export
            </button>
          )}
        </div>
      </footer>
    </>
  );
}

function ExportStatus({ run }: { run: ExportRun }) {
  if (run.phase === "failed") {
    return <span className={css.error}>Export failed: {run.error}</span>;
  }
  return (
    <ProgressBar done={run.read} total={run.total}>
      {run.phase === "reading"
        ? `Reading ${run.read}/${run.total} from the INTEGRA-7…`
        : `${run.fileName} downloaded (${run.total} entries). Nothing on the device changed.`}
    </ProgressBar>
  );
}
