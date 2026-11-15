import { act } from "react";
import { afterEach, describe, expect, it } from "vitest";
import {
  SystemField,
  WasmDeviceState,
  system_common_address,
  system_field_spec,
  system_group_label,
  system_group_note,
  system_groups,
  system_volatile_notice,
} from "../pkg/integral_wasm.js";
import { SystemPage } from "./SystemPage";
import { button, cleanup, click, option, render } from "./test/dom";
import { systemCommonDump } from "./test/system";
import {
  arrowUp,
  dt1Drainer,
  fakeToneService,
  slider,
  type SentDt1,
} from "./test/toneService";

afterEach(cleanup);

/** The DT1s the core sends for `edits`, in order. */
function expectedDt1s(edits: ReadonlyArray<[SystemField, number]>): SentDt1[] {
  const device = new WasmDeviceState(0x10);
  for (const [field, value] of edits) device.setSystemField(field, value);
  const sent = dt1Drainer(device)();
  device.free();
  return sent;
}

/** A page over a fake device whose System Common reads `dumps` in turn (the last one again after). */
async function page(...dumps: Uint8Array[]) {
  const { service, sentDt1s } = fakeToneService(new WasmDeviceState(0x10));
  const reads: number[][] = [];
  service.requestData = async (address: number[]) => {
    reads.push(address);
    return dumps[Math.min(reads.length, dumps.length) - 1]!;
  };
  const { container } = render(<SystemPage service={service} />);
  await act(async () => {});
  return { container, reads, sentDt1s };
}

const spec = (field: SystemField) => system_field_spec(field);
const control = (scope: ParentNode, field: SystemField) => {
  const found = scope.querySelector(`[data-field="${spec(field).name}"]`);
  if (!found) throw new Error(`no ${spec(field).name} control`);
  return found;
};
const valueOf = (scope: ParentNode, field: SystemField) =>
  Number(slider(scope, spec(field).label).getAttribute("aria-valuenow"));

describe("SystemPage", () => {
  it("reads System Common when it opens and shows each group with what it read", async () => {
    const { container, reads } = await page(
      systemCommonDump([
        [SystemField.SystemTempo, 121],
        [SystemField.MasterKeyShift, 66],
      ]),
    );

    expect(reads).toEqual([Array.from(system_common_address())]);
    const text = container.textContent ?? "";
    for (const group of system_groups())
      expect(text).toContain(system_group_label(group));
    expect(valueOf(container, SystemField.SystemTempo)).toBe(121);
    expect(valueOf(container, SystemField.MasterKeyShift)).toBe(66);
  });

  it("always says the settings last until power-off, and where Ext Part Source Select is set", async () => {
    const { container } = await page(systemCommonDump([]));
    expect(container.querySelector('[role="note"]')?.textContent).toBe(
      system_volatile_notice(),
    );
    const notes = system_groups().flatMap((g) => system_group_note(g) ?? []);
    expect(notes).not.toEqual([]);
    for (const note of notes) expect(container.textContent).toContain(note);
  });

  it("re-reads on Refresh", async () => {
    const { container, reads } = await page(
      systemCommonDump([[SystemField.SystemTempo, 121]]),
      systemCommonDump([[SystemField.SystemTempo, 90]]),
    );
    await act(async () => click(button(container, "Refresh")));

    expect(reads).toHaveLength(2);
    expect(valueOf(container, SystemField.SystemTempo)).toBe(90);
  });

  it("keeps showing what it read while Refresh reads again", async () => {
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    let answer: (data: Uint8Array) => void = () => {};
    let first = true;
    service.requestData = () => {
      if (first) {
        first = false;
        return Promise.resolve(systemCommonDump([[SystemField.SystemTempo, 121]]));
      }
      return new Promise((resolve) => (answer = resolve));
    };
    const { container } = render(<SystemPage service={service} />);
    await act(async () => {});
    await act(async () => click(button(container, "Refresh")));

    expect(button(container, "Refresh").disabled).toBe(true);
    expect(valueOf(container, SystemField.SystemTempo)).toBe(121);
    await act(async () => answer(systemCommonDump([[SystemField.SystemTempo, 90]])));
    expect(valueOf(container, SystemField.SystemTempo)).toBe(90);
  });

  it("sends one edit per group and shows it", async () => {
    const { container, sentDt1s } = await page(
      systemCommonDump([
        [SystemField.MasterKeyShift, 64],
        [SystemField.SystemTempo, 120],
      ]),
    );

    // Tuning, Sync/Tempo: knobs.
    arrowUp(slider(container, spec(SystemField.MasterKeyShift).label));
    arrowUp(slider(container, spec(SystemField.SystemTempo).label));
    // Control: a picker.
    click(
      control(container, SystemField.StudioSetControlChannel).querySelector(
        "button",
      )!,
    );
    click(
      option(
        document,
        spec(SystemField.StudioSetControlChannel).formatValue(1),
      ),
    );
    // MIDI Rx, Output, USB: switches.
    for (const field of [
      SystemField.ReceiveProgramChange,
      SystemField.TwoChOutputMode,
      SystemField.UsbAudioSamplingRate,
    ]) {
      click(control(container, field).querySelector("button")!);
    }

    expect(sentDt1s()).toEqual(
      expectedDt1s([
        [SystemField.MasterKeyShift, 65],
        [SystemField.SystemTempo, 121],
        [SystemField.StudioSetControlChannel, 1],
        [SystemField.ReceiveProgramChange, 1],
        [SystemField.TwoChOutputMode, 1],
        [SystemField.UsbAudioSamplingRate, 1],
      ]),
    );
    expect(valueOf(container, SystemField.SystemTempo)).toBe(121);
    expect(
      control(container, SystemField.StudioSetControlChannel).textContent,
    ).toContain(spec(SystemField.StudioSetControlChannel).formatValue(1));
  });

  it("names a System Control Source from the Control Source list", async () => {
    const { container } = await page(
      systemCommonDump([[SystemField.SystemControl1Source, 1]]),
    );
    expect(
      control(container, SystemField.SystemControl1Source).textContent,
    ).toContain(spec(SystemField.SystemControl1Source).formatValue(1));
  });

  it("shows a value it doesn't write as read", async () => {
    // The USB rate's byte, as a panel edit leaves it: 96 kHz, which no edit here sends.
    const block = systemCommonDump([]);
    const [usb] = expectedDt1s([[SystemField.UsbAudioSamplingRate, 0]]);
    block[usb!.address[3]!] = 2;
    const { container } = await page(block);

    expect(
      control(container, SystemField.UsbAudioSamplingRate).textContent,
    ).toContain(
      `Reads ${spec(SystemField.UsbAudioSamplingRate).formatValue(2)}`,
    );
  });

  it("says when the read fails, and Refresh reads again", async () => {
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    let fail = true;
    service.requestData = async () => {
      if (fail) throw new Error("RQ1 timeout for 02 00 00 00");
      return systemCommonDump([[SystemField.SystemTempo, 121]]);
    };
    const { container } = render(<SystemPage service={service} />);
    await act(async () => {});
    expect(container.querySelector('[role="alert"]')?.textContent).toContain(
      "RQ1 timeout",
    );

    fail = false;
    await act(async () => click(button(container, "Refresh")));
    expect(container.querySelector('[role="alert"]')).toBeNull();
    expect(valueOf(container, SystemField.SystemTempo)).toBe(121);
  });
});
