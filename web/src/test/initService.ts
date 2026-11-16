import { act } from "react";
import { vi } from "vitest";
import type { IntegraService } from "../integra";

/** A service whose Init answers when the test calls `finish`, its re-reads recorded. */
export function fakeInitService() {
  let finish: (error?: string) => void = () => {};
  const initialize = vi.fn(
    () =>
      new Promise<string | undefined>((resolve) => {
        finish = resolve;
      }),
  );
  const reReadPart = vi.fn();
  const reReadPartBlock = vi.fn();
  const service = { initialize, device: { reReadPart, reReadPartBlock } } as unknown as IntegraService;
  return {
    service,
    initialize,
    reReadPart,
    reReadPartBlock,
    finish: async (error?: string) => {
      await act(async () => finish(error));
    },
  };
}
