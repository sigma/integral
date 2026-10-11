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
