import { afterEach, describe, expect, it, vi } from "vitest";
import {
  holdUntil,
  workerTimers,
  type TimerWorker,
  type Timers,
} from "./timedSend";

/** A worker that keeps what it is asked and fires a timer when the test says. */
function fakeWorker() {
  const posted: [number, number][] = [];
  const worker: TimerWorker = {
    postMessage: (message) => posted.push(message),
    onmessage: null,
  };
  const fire = (id: number) =>
    worker.onmessage?.(new MessageEvent("message", { data: id }));
  return { worker, posted, fire };
}

afterEach(() => {
  vi.useRealTimers();
});

describe("holdUntil", () => {
  const timers = () => {
    const held: number[] = [];
    const t: Timers = { set: (_, ms) => held.push(ms), clear: () => {} };
    return { t, held };
  };

  it("delivers at once with no timestamp or one that has come", () => {
    const { t, held } = timers();
    const deliver = vi.fn();
    holdUntil(undefined, deliver, t, () => 100);
    holdUntil(100, deliver, t, () => 100);
    holdUntil(50, deliver, t, () => 100);
    expect(deliver).toHaveBeenCalledTimes(3);
    expect(held).toEqual([]);
  });

  it("holds a future send for the time left", () => {
    const { t, held } = timers();
    const deliver = vi.fn();
    holdUntil(130, deliver, t, () => 100);
    expect(deliver).not.toHaveBeenCalled();
    expect(held).toEqual([30]);
  });
});

describe("workerTimers", () => {
  it("runs each timer in the worker and calls it back when the worker says", () => {
    const fake = fakeWorker();
    const make = vi.fn(() => fake.worker);
    const timers = workerTimers(make);
    expect(make).not.toHaveBeenCalled();

    const first = vi.fn();
    const second = vi.fn();
    const a = timers.set(first, 40);
    const b = timers.set(second, 10);
    expect(make).toHaveBeenCalledTimes(1);
    expect(fake.posted).toEqual([
      [a, 40],
      [b, 10],
    ]);

    fake.fire(b as number);
    expect(second).toHaveBeenCalledTimes(1);
    expect(first).not.toHaveBeenCalled();
    fake.fire(b as number);
    expect(second).toHaveBeenCalledTimes(1);
  });

  it("stops a cleared timer in the worker and never calls it", () => {
    const fake = fakeWorker();
    const timers = workerTimers(() => fake.worker);
    const callback = vi.fn();
    const id = timers.set(callback, 40);
    timers.clear(id);
    expect(fake.posted[fake.posted.length - 1]).toEqual([id, -1]);
    fake.fire(id as number);
    expect(callback).not.toHaveBeenCalled();
  });

  it("falls back to the page's timers where there is no worker", () => {
    vi.useFakeTimers();
    for (const make of [
      () => null,
      () => {
        throw new Error("refused");
      },
    ]) {
      const timers = workerTimers(make);
      const callback = vi.fn();
      timers.set(callback, 20);
      const cleared = vi.fn();
      timers.clear(timers.set(cleared, 20));
      vi.advanceTimersByTime(20);
      expect(callback).toHaveBeenCalledTimes(1);
      expect(cleared).not.toHaveBeenCalled();
    }
  });

  it("works with no Worker at all, as in jsdom", () => {
    vi.useFakeTimers();
    const timers = workerTimers();
    const callback = vi.fn();
    timers.set(callback, 5);
    vi.advanceTimersByTime(5);
    expect(callback).toHaveBeenCalledTimes(1);
  });
});
