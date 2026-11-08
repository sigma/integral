import { act } from "react";
import { afterEach, describe, expect, it } from "vitest";
import {
  ParameterFamily,
  PluginKind,
  ToneType,
  WasmDeviceState,
  mfx_block_size,
} from "../pkg/integral_wasm.js";
import { PcmDrumEditor } from "./PcmDrumEditor";
import { GESTURE_POLL_MS, HostGestures, useHostGestures } from "./hostGestures";
import type { PageMessage, PluginBinding } from "./plugin";
import { SynthKnob } from "./synth-ui/SynthKnob";
import { cleanup, dragUp, render } from "./test/dom";
import { fakeBridge } from "./test/pluginBridge";
import { fakeToneService } from "./test/toneService";

afterEach(cleanup);

const begin = (id: string): PageMessage => ({ type: "beginEdit", id });
const perform = (id: string, value: number): PageMessage => ({ type: "performEdit", id, value });
const end = (id: string): PageMessage => ({ type: "endEdit", id });

describe("HostGestures", () => {
  function gestures() {
    const posted: PageMessage[] = [];
    return { gestures: new HostGestures((m) => posted.push(m)), posted };
  }

  it("begins on the first write of a drag, performs each, and ends on change", () => {
    const { gestures: g, posted } = gestures();
    g.input([{ id: "a", value: 1 }], true);
    g.input([{ id: "a", value: 2 }], true);
    g.input([{ id: "a", value: 3 }], true);
    expect(posted).toEqual([begin("a"), perform("a", 1), perform("a", 2), perform("a", 3)]);
    g.change();
    expect(posted.slice(4)).toEqual([end("a")]);
  });

  it("brackets a write with no pointer held on its own", () => {
    const { gestures: g, posted } = gestures();
    g.input([{ id: "a", value: 7 }], false);
    expect(posted).toEqual([begin("a"), perform("a", 7), end("a")]);
    g.input([{ id: "a", value: 8 }], false);
    expect(posted.slice(3)).toEqual([begin("a"), perform("a", 8), end("a")]);
  });

  it("keeps a gesture per parameter and ends them in the order they began", () => {
    const { gestures: g, posted } = gestures();
    g.input([{ id: "a", value: 1 }, { id: "b", value: 2 }], true);
    g.input([{ id: "a", value: 3 }], true);
    g.change();
    expect(posted).toEqual([
      begin("a"),
      perform("a", 1),
      begin("b"),
      perform("b", 2),
      perform("a", 3),
      end("a"),
      end("b"),
    ]);
  });

  it("does nothing on change when no gesture is open", () => {
    const { gestures: g, posted } = gestures();
    g.change();
    g.input([], true);
    g.change();
    expect(posted).toEqual([]);
  });

  it("begins again after a change", () => {
    const { gestures: g, posted } = gestures();
    g.input([{ id: "a", value: 1 }], true);
    g.change();
    g.input([{ id: "a", value: 2 }], true);
    g.change();
    expect(posted).toEqual([
      begin("a"),
      perform("a", 1),
      end("a"),
      begin("a"),
      perform("a", 2),
      end("a"),
    ]);
  });
});

describe("a Control drag in a plugin editor", () => {
  /** The Part (0-based) whose level the knob sets. */
  const PART = 2;

  function Harness({
    device,
    bridge,
    kind,
    binding,
    write = (d, v) => d.setPartLevel(PART, v),
    relay = true,
  }: {
    device: WasmDeviceState;
    bridge: ReturnType<typeof fakeBridge>["bridge"];
    kind: PluginKind;
    binding: PluginBinding | null;
    /** What the knob writes. */
    write?: (device: WasmDeviceState, value: number) => void;
    /** Whether the instance can tell the host of edits. */
    relay?: boolean;
  }) {
    const { service } = fakeToneService(device);
    useHostGestures(service, bridge, kind, binding, relay);
    return (
      <SynthKnob
        label="Level"
        value={64}
        min={0}
        max={127}
        defaultValue={100}
        family={ParameterFamily.Filter}
        onChange={(v) => write(device, v)}
      />
    );
  }

  function mount(kind = PluginKind.StudioSet, binding: PluginBinding | null = {}) {
    const device = new WasmDeviceState(0x10);
    const fake = fakeBridge();
    const { container } = render(<Harness device={device} bridge={fake.bridge} kind={kind} binding={binding} />);
    const knob = container.querySelector('[role="slider"]')!;
    return { knob, posted: fake.posted };
  }

  /** Hold the pointer down on `knob` and move it, as the first part of a drag. */
  function startDrag(knob: Element) {
    (knob as Element & { setPointerCapture: () => void }).setPointerCapture = () => {};
    const event = (type: string, y: number) => {
      const e = new MouseEvent(type, { bubbles: true, cancelable: true, clientX: 0, clientY: y });
      Object.defineProperty(e, "pointerId", { value: 1 });
      act(() => {
        knob.dispatchEvent(e);
      });
    };
    event("pointerdown", 100);
    event("pointermove", 90);
  }

  /** Let the poll collect what a held drag wrote. */
  const afterPoll = () => new Promise<void>((resolve) => setTimeout(resolve, GESTURE_POLL_MS * 4));

  const gestureOf = (posted: PageMessage[]) =>
    posted.map((m) => ("id" in m ? `${m.type} ${m.id}` : m.type));

  it("begins on its first write, performs each write's value, and ends on release", () => {
    const { knob, posted } = mount();
    dragUp(knob, 10);
    const types = posted.map((m) => m.type);
    expect(types[0]).toBe("beginEdit");
    expect(types[types.length - 1]).toBe("endEdit");
    expect(types.filter((t) => t === "beginEdit")).toHaveLength(1);
    expect(types.filter((t) => t === "endEdit")).toHaveLength(1);
    expect(types.slice(1, -1).every((t) => t === "performEdit")).toBe(true);
    expect(types.length).toBeGreaterThan(2);

    // All of one parameter: the Studio Set's level of that Part.
    const ids = new Set(posted.map((m) => ("id" in m ? m.id : "")));
    expect(ids.size).toBe(1);
    const [id] = [...ids];
    expect(id).toContain("/");

    // Each perform carries the value the knob wrote: the drag's last is the highest.
    const values = posted.flatMap((m) => (m.type === "performEdit" ? [m.value] : []));
    expect(values).toEqual([...values].sort((a, b) => a - b));
    expect(values[values.length - 1]).toBeGreaterThan(64);
  });

  it("names the parameter by the address written, which holds the Part", () => {
    const device = new WasmDeviceState(0x10);
    device.recordWrites(true);
    device.setPartLevel(PART, 50);
    device.setPartLevel(PART + 1, 60);
    const writes = device.takeHostWrites(PluginKind.StudioSet);
    expect(writes.map((w) => w.value)).toEqual([50, 60]);
    expect(new Set(writes.map((w) => w.id)).size).toBe(2);
  });

  it("brackets a click of its own, with no pointer held for the write", () => {
    const device = new WasmDeviceState(0x10);
    const fake = fakeBridge();
    const { container } = render(
      <Harness device={device} bridge={fake.bridge} kind={PluginKind.StudioSet} binding={{}} />,
    );
    const knob = container.querySelector('[role="slider"]')!;
    // A key press steps the knob: one write, no pointer.
    act(() => {
      knob.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }));
    });
    return new Promise<void>((resolve, reject) => {
      setTimeout(() => {
        try {
          expect(gestureOf(fake.posted).map((t) => t.split(" ")[0])).toEqual([
            "beginEdit",
            "performEdit",
            "endEdit",
          ]);
          resolve();
        } catch (e) {
          reject(e);
        }
      }, GESTURE_POLL_MS * 4);
    });
  });

  it("sends no gesture when the instance cannot tell the host of edits", () => {
    const device = new WasmDeviceState(0x10);
    const fake = fakeBridge();
    const { container } = render(
      <Harness device={device} bridge={fake.bridge} kind={PluginKind.StudioSet} binding={{}} relay={false} />,
    );
    dragUp(container.querySelector('[role="slider"]')!, 10);
    expect(fake.posted).toEqual([]);
  });

  it("sends nothing for an unbound instance", () => {
    const { knob, posted } = mount(PluginKind.PcmSynth, null);
    dragUp(knob, 10);
    expect(posted).toEqual([]);
  });

  it("ends the gesture of a drag when the Binding is lost", async () => {
    const device = new WasmDeviceState(0x10);
    const fake = fakeBridge();
    const element = (binding: PluginBinding | null) => (
      <Harness device={device} bridge={fake.bridge} kind={PluginKind.StudioSet} binding={binding} />
    );
    const { container, rerender } = render(element({}));
    startDrag(container.querySelector('[role="slider"]')!);
    await afterPoll();
    expect(fake.posted.map((m) => m.type)).toEqual(["beginEdit", "performEdit"]);
    rerender(element(null));
    expect(fake.posted.map((m) => m.type)).toEqual(["beginEdit", "performEdit", "endEdit"]);
  });

  it("sends nothing for a write the plugin's Binding does not own", () => {
    // A Tone plugin bound to Part 6 owns Part 6's tone, not Part 3's mixer level.
    const { knob, posted } = mount(PluginKind.PcmSynth, { part: 5 });
    dragUp(knob, 10);
    expect(posted).toEqual([]);
  });

  it("ends the gesture it left open when the editor goes away", () => {
    const { knob, posted } = mount();
    (knob as Element & { setPointerCapture: () => void }).setPointerCapture = () => {};
    const event = (type: string, y: number) => {
      const e = new MouseEvent(type, { bubbles: true, cancelable: true, clientX: 0, clientY: y });
      Object.defineProperty(e, "pointerId", { value: 1 });
      act(() => {
        knob.dispatchEvent(e);
      });
    };
    event("pointerdown", 100);
    event("pointermove", 90);
    // The pointer is still down: the poll collects the write.
    return new Promise<void>((resolve, reject) => {
      setTimeout(() => {
        try {
          expect(posted.map((m) => m.type)).toEqual(["beginEdit", "performEdit"]);
          cleanup();
          expect(posted[posted.length - 1]?.type).toBe("endEdit");
          resolve();
        } catch (e) {
          reject(e);
        }
      }, GESTURE_POLL_MS * 4);
    });
  });

  it.each([
    ["the window loses focus", () => window.dispatchEvent(new Event("blur"))],
    ["the pointer is cancelled", () => window.dispatchEvent(new Event("pointercancel"))],
    ["the pointer capture is lost", () => window.dispatchEvent(new Event("lostpointercapture"))],
  ])("ends a drag's gesture when %s", async (_what, release) => {
    const { knob, posted } = mount();
    startDrag(knob);
    await afterPoll();
    expect(posted.map((m) => m.type)).toEqual(["beginEdit", "performEdit"]);
    act(release);
    expect(posted.map((m) => m.type)).toEqual(["beginEdit", "performEdit", "endEdit"]);
  });

  it("sends no gesture while an editor opens and loads", async () => {
    const device = new WasmDeviceState(0x10);
    const fake = fakeBridge();
    function Editor() {
      const { service } = fakeToneService(device);
      useHostGestures(service, fake.bridge, PluginKind.PcmDrum, { part: 1 });
      return <PcmDrumEditor partIndex={1} service={service} scope="kit" />;
    }
    render(<Editor />);
    await act(async () => {});
    await afterPoll();
    expect(fake.posted).toEqual([]);
  });

  describe("on an MFX type's parameter", () => {
    const PART_BOUND = { part: 2 };
    const mfxKnob = (device: WasmDeviceState, value: number) =>
      device.setMfxNibParam(2, ToneType.SnSynth, 0, value);

    function mountMfx(prepare: (device: WasmDeviceState) => void) {
      const device = new WasmDeviceState(0x10);
      prepare(device);
      const fake = fakeBridge();
      const { container } = render(
        <Harness device={device} bridge={fake.bridge} kind={PluginKind.SnSynth} binding={PART_BOUND} write={mfxKnob} />,
      );
      return { knob: container.querySelector('[role="slider"]')!, posted: fake.posted };
    }

    it("sends a gesture once the page set the MFX type", () => {
      const { knob, posted } = mountMfx((d) => d.setMfxType(2, ToneType.SnSynth, 34));
      dragUp(knob, 10);
      const ids = new Set(posted.flatMap((m) => ("id" in m ? [m.id] : [])));
      expect([...ids]).toHaveLength(1);
      expect([...ids][0]).toMatch(/^mfx\/34\//);
      expect(posted[0]!.type).toBe("beginEdit");
      expect(posted[posted.length - 1]!.type).toBe("endEdit");
    });

    it("sends a gesture once the page read the MFX block", () => {
      const block = new Uint8Array(mfx_block_size().reduce((n, b) => (n << 7) | b, 0));
      block[0] = 34;
      const { knob, posted } = mountMfx((d) => d.applyMfxBlock(2, block));
      dragUp(knob, 10);
      expect(posted.some((m) => "id" in m && m.id.startsWith("mfx/34/"))).toBe(true);
    });

    it("sends nothing while the MFX type is unknown", () => {
      const { knob, posted } = mountMfx(() => {});
      dragUp(knob, 10);
      expect(posted).toEqual([]);
    });
  });
});
