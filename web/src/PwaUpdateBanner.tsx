import { useEffect, useRef, useState } from "react";
import { registerSW } from "virtual:pwa-register";
import css from "./PwaUpdateBanner.module.css";

/**
 * Mounted once, at the app root: registers the service worker and, when a new
 * deploy is waiting (prompt mode), offers to reload. Reload applies the update;
 * Dismiss keeps the session — the update applies on the next natural reload.
 */
export function PwaUpdateBanner() {
  const [ready, setReady] = useState(false);
  const [dismissed, setDismissed] = useState(false);
  // registerSW's returned updater: skips waiting, then reloads the page.
  const update = useRef<() => Promise<void>>(() => Promise.resolve());

  useEffect(() => {
    update.current = registerSW({
      onNeedRefresh: () => {
        setReady(true);
        // A second notification is a newer update, worth showing even if an
        // earlier one was dismissed.
        setDismissed(false);
      },
    });
  }, []);

  if (!ready || dismissed) return null;
  return (
    <div className={css.banner} role="status">
      New version available
      <button
        className={css.reload}
        onClick={() => void update.current()}
        aria-label="Reload to update"
      >
        Reload
      </button>
      <button className={css.dismiss} onClick={() => setDismissed(true)}>
        Dismiss
      </button>
    </div>
  );
}
