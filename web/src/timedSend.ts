/**
 * Timed sends for a port pair that has no scheduler underneath (the Device
 * service's, in the browser and in the plugin): a send with a future
 * `timestamp` (the Sequencer's) is held in the page until then, as Web MIDI
 * would hold it in the browser.
 *
 * Browsers throttle main-thread timers in a background tab (to about once a
 * second, then less), which would play a hidden tab's Sequencer late and
 * bunched. A dedicated Worker's timers are not throttled, which the
 * Sequencer's own tick already relies on, so {@link workerTimers} holds sends
 * there and hands them back to the page when due.
 */

/** Timers a pair schedules with, so tests can run them by hand. */
export interface Timers {
  set(callback: () => void, ms: number): unknown;
  clear(handle: unknown): void;
}

/** The page's own `setTimeout`, which a background tab throttles. */
export const mainThreadTimers: Timers = {
  set: (callback, ms) => setTimeout(callback, ms),
  clear: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>),
};

/** The part of a `Worker` {@link workerTimers} uses. */
export interface TimerWorker {
  postMessage(message: [number, number]): void;
  onmessage: ((event: MessageEvent<number>) => void) | null;
}

/**
 * The worker's script: `[id, ms]` starts (or restarts) timer `id`, a
 * negative `ms` stops it, and a timer that fires posts its `id` back.
 */
const TIMER_WORKER = `const held = new Map();
onmessage = ({ data: [id, ms] }) => {
  clearTimeout(held.get(id));
  held.delete(id);
  if (ms >= 0) held.set(id, setTimeout(() => { held.delete(id); postMessage(id); }, ms));
};`;

function blobWorker(): TimerWorker | null {
  if (typeof Worker === "undefined") return null;
  const url = URL.createObjectURL(new Blob([TIMER_WORKER], { type: "text/javascript" }));
  return new Worker(url) as TimerWorker;
}

/**
 * Timers that run in a dedicated Worker, so a background tab does not
 * throttle them. The worker starts at the first `set`; where there is none
 * (no `Worker`, or it cannot be made), they are {@link mainThreadTimers}.
 *
 * Not for the plugin's webview: its Content Security Policy refuses a
 * `blob:` worker, and nothing there sends timed messages.
 */
export function workerTimers(makeWorker: () => TimerWorker | null = blobWorker): Timers {
  // `undefined` until the first `set` has tried to start one.
  let worker: TimerWorker | null | undefined;
  const due = new Map<number, () => void>();
  let next = 0;

  const start = (): TimerWorker | null => {
    if (worker !== undefined) return worker;
    try {
      worker = makeWorker();
    } catch {
      worker = null;
    }
    if (worker)
      worker.onmessage = ({ data: id }) => {
        const callback = due.get(id);
        due.delete(id);
        callback?.();
      };
    return worker;
  };

  return {
    set(callback, ms) {
      const w = start();
      if (!w) return mainThreadTimers.set(callback, ms);
      const id = ++next;
      due.set(id, callback);
      w.postMessage([id, ms]);
      return id;
    },
    clear(handle) {
      if (!worker) {
        mainThreadTimers.clear(handle);
        return;
      }
      const id = handle as number;
      if (due.delete(id)) worker.postMessage([id, -1]);
    },
  };
}

/**
 * Call `deliver` at `timestamp` on the `now` clock (`performance.now()`'s),
 * on `timers`; at once if it has no `timestamp` or the time has come.
 */
export function holdUntil(
  timestamp: number | undefined,
  deliver: () => void,
  timers: Timers,
  now: () => number,
): void {
  const delay = timestamp === undefined ? 0 : timestamp - now();
  if (delay > 0) timers.set(deliver, delay);
  else deliver();
}
