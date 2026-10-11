import type { MouseEvent, ReactNode } from "react";
import type { Expansion } from "./expansionCatalog";
import css from "./NotLoadedMarker.module.css";

interface Props {
  /** Name of the Expansion that is not loaded. */
  expansion: string;
  /** Name the Expansion in the marker, where nothing nearby does. */
  named?: boolean;
}

/** Marks a tone or bank whose Expansion is not loaded: it plays silence. */
export function NotLoadedMarker({ expansion, named = false }: Props) {
  return (
    <span
      className={css.marker}
      title={`${expansion} is not loaded: its tones play silence`}
    >
      {named ? `${expansion} not loaded` : "not loaded"}
    </span>
  );
}

/** Marks a loaded Expansion; `children` replaces the "loaded" text. */
export function LoadedMarker({
  children = "loaded",
}: {
  children?: ReactNode;
}) {
  return <span className={css.loaded}>{children}</span>;
}

/** The Load action of an unloaded Expansion: stages it on the Expansions
 *  page (nothing loads yet). */
export function LoadExpansionButton({
  expansion,
  onLoad,
}: {
  expansion: Expansion;
  onLoad: (expansion: Expansion, event: MouseEvent<HTMLButtonElement>) => void;
}) {
  return (
    <button
      type="button"
      className={css.loadButton}
      onClick={(e) => onLoad(expansion, e)}
      title={`Stage ${expansion.name} on the Expansions page`}
    >
      Load
    </button>
  );
}
