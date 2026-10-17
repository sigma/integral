import css from "./MixerLoadWarning.module.css";

interface Props {
  /** Labels of the mixer blocks whose read gave up. */
  failed: string[];
  /** Read the whole mixer state again. */
  onRetry: () => void;
}

/** Non-blocking warning for mixer blocks the device never answered. */
export function MixerLoadWarning({ failed, onRetry }: Props) {
  return (
    <div className={css.warning} role="status" title={failed.join(", ")}>
      {failed.length} {failed.length === 1 ? "block" : "blocks"} failed to
      load{" "}
      <button className={css.retry} onClick={onRetry}>
        Retry
      </button>
    </div>
  );
}
