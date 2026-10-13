import { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ExportSource } from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import { libraryType } from "./librarian";
import { ReinitDialog } from "./ReinitDialog";
import { button, cleanup, click, render } from "./test/dom";
import type { ToneCatalog } from "./toneCatalog";
import type { UseMixerResult } from "./useMixer";
import type { UserSlot } from "./userSlots";

afterEach(cleanup);

function slot(index: number, name: string): UserSlot {
  return { key: String(index), number: String(index + 1), name, slot: index, msb: 89, lsb: 0, pc: index };
}

/** Let the dialog's async run settle. */
async function settle(): Promise<void> {
  for (let i = 0; i < 5; i++) await act(async () => {});
}

describe("ReinitDialog", () => {
  it("exports the used slots, then stores INIT into every selected slot", async () => {
    const exported: ExportSource[][] = [];
    let reinit: unknown[] = [];
    const service = {
      exportPack: async (sources: ExportSource[]) => {
        exported.push(sources);
        return { toSvd: () => new Uint8Array(), free: () => {} };
      },
      reinitSlots: async (kind: string, slots: number[], part: number) => {
        reinit = [kind, slots, part];
        return { statuses: ["stored", "stored"], error: undefined };
      },
    } as unknown as IntegraService;
    const refresh = vi.fn();
    const catalog = { load: async () => [], refresh } as unknown as ToneCatalog;
    const mixer = {
      state: { selectedPart: 2 },
      reload: () => {},
      loadStudioSetNames: async () => new Map(),
    } as unknown as UseMixerResult;
    // jsdom has no object URLs: the undo download needs them.
    URL.createObjectURL = () => "blob:";
    URL.revokeObjectURL = () => {};

    const { container } = render(
      <ReinitDialog
        service={service}
        mixer={mixer}
        catalog={catalog}
        type={libraryType("sn-synth")}
        slots={[slot(3, "Lead"), slot(7, "INIT TONE")]}
        onClose={() => {}}
      />,
    );
    expect(container.textContent).toContain("1 slot will be erased");
    click(button(container, "Re-init"));
    await settle();

    expect(exported).toEqual([[{ source: "user", kind: "sn-synth", slot: 3 }]]);
    expect(reinit).toEqual(["sn-synth", [3, 7], 2]);
    expect(refresh).toHaveBeenCalled();
    expect(container.textContent).toContain("Re-initialized 2/2 slots.");
  });
});
