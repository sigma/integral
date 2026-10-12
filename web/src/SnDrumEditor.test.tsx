import { act } from "react";
import { afterEach, describe, expect, it } from "vitest";
import {
  WasmDeviceState,
  snd_common_address,
  snd_note_address,
} from "../pkg/integral_wasm.js";
import { SnDrumEditor } from "./SnDrumEditor";
import { cleanup, render } from "./test/dom";
import { arrowUp, fakeToneService, slider } from "./test/toneService";

afterEach(cleanup);

const PART = 1;

describe("SnDrumEditor", () => {
  it("sends a Common and a Note edit to their fields", async () => {
    const { service, sentDt1s } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<SnDrumEditor partIndex={PART} service={service} />);
    await act(async () => {});
    // The editor opens on key 36 (C2, kick drum): the Note edit below goes there.
    const key = 36;

    arrowUp(slider(container, "Amb Lv"));
    arrowUp(slider(container, "Attack"));

    // Ambience Level is Common `00 11`; Attack is Note `00 0C`.
    expect(sentDt1s()).toEqual([
      { address: [...snd_common_address(PART).slice(0, 3), 17], data: [1] },
      { address: [...snd_note_address(PART, key).slice(0, 3), 12], data: [1] },
    ]);
  });
});
