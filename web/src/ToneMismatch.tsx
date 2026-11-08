import css from "./ToneMismatch.module.css";

interface Props {
  /** What the plugin says about the mismatch. */
  text: string;
  /** The name of the tone the plugin loads onto the Part. */
  defaultTone: string | null;
  /** A request to load it is pending: the button waits for the Part to hold it. */
  pending: boolean;
  /** Load the plugin's default tone onto the Part. */
  onLoad: () => void;
}

/**
 * The tone page of a plugin whose Part holds another tone type, in place of
 * the tone editor: the plugin is inert, and the one thing to do is to load
 * its default tone (named on the button).
 */
export function ToneMismatch({ text, defaultTone, pending, onLoad }: Props) {
  return (
    <div className={css.panel} data-mismatch>
      <p className={css.text}>{text}</p>
      {defaultTone !== null && (
        <button
          type="button"
          className={css.action}
          onClick={onLoad}
          disabled={pending}
          title={`Select ${defaultTone}, the first Preset tone of this plugin's type, on the Part`}
        >
          Load {defaultTone}
        </button>
      )}
    </div>
  );
}
