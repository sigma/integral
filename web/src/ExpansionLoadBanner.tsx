import { useEffect, useState } from "react";
import type { RunningExpansionLoad } from "./useExpansions";
import css from "./ExpansionLoadBanner.module.css";

/** Elapsed-time refresh interval (ms). */
const TICK_MS = 250;

/** App-wide banner while an Expansion load runs: what loads, and for how long. */
export function ExpansionLoadBanner({ load }: { load: RunningExpansionLoad }) {
  const [now, setNow] = useState(() => performance.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(performance.now()), TICK_MS);
    return () => clearInterval(timer);
  }, []);
  const seconds = Math.max(0, Math.floor((now - load.startedAt) / 1000));
  const what = load.names.length > 0 ? load.names.join(", ") : "Expansion slots";
  return (
    <div className={css.banner} role="status">
      Loading {what}… {seconds} s
    </div>
  );
}
