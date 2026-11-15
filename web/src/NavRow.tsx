/**
 * The browser's navigation row
 * ([#584](https://github.com/sigma/integral/issues/584)): one row nested left to right like a
 * breadcrumb, Device, then Studio Set, Part and Tone. Each scope's group holds
 * its selector, the controls that act on it, then the pages that show it.
 *
 * To extend it:
 * - a page goes in its scope's list in {@link SCOPE_PAGES} (and `PageTab`);
 * - a control goes in its scope's `controls` in {@link NavRow} (the
 *   selectors and SAVEs live in `ScopeControls.tsx`).
 *
 * The plugin keeps its own tab row and `PluginBar` (web first).
 */

import type { ReactNode } from "react";
import { Lockable } from "./Lockable";
import { InitGuard } from "./InitFlow";
import type { InitScope } from "./init";
import { PartControls, StudioSetControls, ToneControls, type PartSettingsToggle } from "./ScopeControls";
import { PAGE_NAMES, type PageTab } from "./pluginPages";
import type { IntegraService } from "./integra";
import type { ToneCatalog } from "./toneCatalog";
import type { UseMixerResult } from "./useMixer";
import type { UseExpansionsResult } from "./useExpansions";
import type { Expansion } from "./expansionCatalog";
import css from "./NavRow.module.css";

/**
 * The Temporary Studio Set, as an Init's area. Its Init resets every Part and
 * its tone, so the Studio Set, Part and Tone controls all edit it.
 */
const STUDIO_SET: InitScope = { kind: "studio-set" };

/** The scopes of the row, left to right. */
export type Scope = "device" | "studioSet" | "part" | "tone";

/** Each scope's name, as its group is labelled. */
export const SCOPE_NAMES: Record<Scope, string> = {
  device: "Device",
  studioSet: "Studio Set",
  part: "Part",
  tone: "Tone",
};

/** The pages of each scope, in the row's order: a page sits in the scope that decides what it shows. */
export const SCOPE_PAGES: Record<Scope, readonly PageTab[]> = {
  device: ["system", "expansions", "library"],
  studioSet: ["mixer", "surround", "routing", "sequencer"],
  part: [],
  tone: ["toneEdit"],
};

/** The pages of the app in a browser, in the row's order; the Sequencer once it is up. */
export function browserPages(sequencer: boolean): readonly PageTab[] {
  return Object.values(SCOPE_PAGES)
    .flat()
    .filter((page) => sequencer || page !== "sequencer");
}

interface NavScopeProps {
  scope: Scope;
  /** The scope's selector and the controls acting on it; inert while `locked`. */
  controls?: ReactNode;
  /** The area `controls` edit: they are busy and inert while an Init that resets it runs. */
  initArea?: InitScope;
  /** Whether `controls` are locked (a Pack import); the pages and `after` stay live. */
  locked?: boolean;
  /** The pages the app shows, of which the scope's are its group's. */
  pages: readonly PageTab[];
  /** The page shown, none while the app shows no page. */
  tab: PageTab | undefined;
  onTab: (page: PageTab) => void;
  /** What follows the scope's pages (the Studio Set's transport). */
  after?: ReactNode;
}

/** One scope's group of the row: its name, its controls, then its pages. */
export function NavScope({
  scope,
  controls,
  initArea,
  locked = false,
  pages,
  tab,
  onTab,
  after,
}: NavScopeProps) {
  const name = SCOPE_NAMES[scope];
  const own = SCOPE_PAGES[scope].filter((page) => pages.includes(page));
  return (
    <div className={css.scope} role="group" aria-label={name}>
      <span className={css.name} aria-hidden>
        {name.toUpperCase()}
      </span>
      {controls && (
        <Lockable locked={locked}>
          {initArea ? <InitGuard area={initArea}>{controls}</InitGuard> : controls}
        </Lockable>
      )}
      {controls && own.length > 0 && <span className={css.divider} aria-hidden />}
      {own.map((page) => (
        <button
          key={page}
          data-page={page}
          className={`${css.page} ${tab === page ? css.pageActive : ""}`}
          aria-current={tab === page ? "page" : undefined}
          onClick={() => onTab(page)}
        >
          {pageLabel(PAGE_NAMES[page], name)}
        </button>
      ))}
      {after}
    </div>
  );
}

/** `full`, its leading `scope` name visible to a screen reader only. */
// In its scope's group, a page name that starts with the scope's shows only
// the rest ("Edit" under TONE); the plugin's tabs, which have no groups, show
// it whole.
function pageLabel(full: string, scope: string): ReactNode {
  const prefix = `${scope} `;
  if (!full.startsWith(prefix)) return full;
  return (
    <>
      <span className={css.visuallyHidden} data-scope-name>
        {prefix}
      </span>
      {full.slice(prefix.length)}
    </>
  );
}

interface NavRowProps {
  mixer: UseMixerResult;
  service: IntegraService;
  /** The tone catalog, shared with the other pages. */
  catalog: ToneCatalog;
  expansions: UseExpansionsResult;
  /** Open the Expansions page with `expansion` staged. */
  onLoadExpansion: (expansion: Expansion) => void;
  /** The pages the app shows ({@link browserPages}). */
  pages: readonly PageTab[];
  /** The page shown, none while the app shows no page. */
  tab: PageTab | undefined;
  onTab: (page: PageTab) => void;
  /** The Sequencer's transport, after the Studio Set's pages; none without a Sequencer. */
  transport?: ReactNode;
  /** A Pack import runs: the Studio Set, Part and Tone controls are inert. */
  locked: boolean;
  /** The Part group's ⚙: the Part settings drawer's toggle; none, no ⚙. */
  partSettings?: PartSettingsToggle;
}

/** The browser's navigation row, by scope. */
export function NavRow({
  mixer,
  service,
  catalog,
  expansions,
  onLoadExpansion,
  pages,
  tab,
  onTab,
  transport,
  locked,
  partSettings,
}: NavRowProps) {
  const nav = { pages, tab, onTab, locked };
  return (
    <nav className={css.row}>
      <NavScope scope="device" {...nav} />
      <NavScope
        scope="studioSet"
        {...nav}
        initArea={STUDIO_SET}
        controls={<StudioSetControls mixer={mixer} service={service} />}
        after={transport}
      />
      <NavScope
        scope="part"
        {...nav}
        initArea={STUDIO_SET}
        controls={<PartControls mixer={mixer} settings={partSettings} />}
      />
      <NavScope
        scope="tone"
        {...nav}
        initArea={STUDIO_SET}
        controls={
          <ToneControls
            mixer={mixer}
            service={service}
            catalog={catalog}
            expansions={expansions}
            onLoadExpansion={onLoadExpansion}
          />
        }
      />
    </nav>
  );
}
