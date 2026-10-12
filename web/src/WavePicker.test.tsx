import { afterEach, describe, expect, it, vi } from "vitest";
import { WavePicker, type ExpansionMarking } from "./WavePicker";
import { waveSourceOf, type WaveSource } from "./waveCatalog";
import { PartialWave } from "./pcm-synth/PartialRow";
import { WmtWave } from "./pcm-drum/NoteControls";
import { OscPcmWave } from "./sn-synth/PartialRow";
import { PcmdWmtField, PcmsPartialField, SnsPartialField } from "../pkg/integral_wasm.js";
import type { PartialSetter } from "./pcm-synth/types";
import { button, cleanup, click, options, render, typeInto } from "./test/dom";

const INT = waveSourceOf(0, 0)!;
const SRX07 = waveSourceOf(1, 7)!;

afterEach(cleanup);

function picker(props: {
  source?: WaveSource;
  l?: number;
  r?: number;
  marking?: ExpansionMarking;
}) {
  const onChange = vi.fn();
  const onGroupChange = vi.fn();
  const { container } = render(
    <WavePicker
      source={props.source ?? INT}
      stereo
      l={props.l ?? 1}
      r={props.r ?? 0}
      onChange={onChange}
      onGroupChange={onGroupChange}
      marking={props.marking}
    />,
  );
  return { container, onChange, onGroupChange };
}

/** The collapsed `group number name` button of row `index` (0 = L). */
function nameButton(container: HTMLElement, index: number): HTMLButtonElement {
  const found = [...container.querySelectorAll("button")].filter((b) =>
    /^\d{4} /.test(b.textContent ?? ""),
  )[index];
  if (!found) throw new Error(`no name button ${index}`);
  return found;
}

/** Pick Wave `number` from row `index`'s popover. */
function pickWave(container: HTMLElement, index: number, number: number): void {
  click(nameButton(container, index));
  const option = [...document.body.querySelectorAll('[role="option"]')].find((o) =>
    o.textContent?.startsWith(String(number).padStart(4, "0")),
  );
  click(option!);
}

function searchBox(): HTMLInputElement {
  const input = document.body.querySelector("input");
  if (!input) throw new Error("no search box");
  return input;
}

describe("WavePicker", () => {
  it("shows the padded Wave Number and name on each row, the group once", () => {
    const { container } = picker({ l: 1, r: 2 });
    expect(nameButton(container, 0).textContent).toBe("0001 StGrand pA L");
    expect(nameButton(container, 1).textContent).toBe("0002 StGrand pA R");
    expect(button(container, "INT ▾")).toBeTruthy();
  });

  it("searches names by substring, case-insensitively", () => {
    const { container, onChange } = picker({});
    click(nameButton(container, 0));
    const all = options(document.body).length;
    typeInto(searchBox(), "saw");
    const [off, ...found] = options(document.body);
    expect(off).toContain("OFF");
    expect(found.length).toBeGreaterThan(0);
    expect(found.length + 1).toBeLessThan(all);
    for (const name of found) expect(name.toLowerCase()).toContain("saw");
    click(document.body.querySelectorAll('[role="option"]')[1]!);
    expect(onChange).toHaveBeenCalledWith("l", expect.any(Number));
    expect(document.body.querySelector("input")).toBeNull();
  });

  it("filters by family chip and picks OFF", () => {
    const { container, onChange } = picker({});
    click(nameButton(container, 0));
    const chip = [...document.body.querySelectorAll("[aria-pressed]")].find(
      (c) => c.textContent?.startsWith("StGrand "),
    );
    click(chip!);
    const [, ...found] = options(document.body);
    const expected = INT.waves.filter((w) => w.family === "StGrand").length;
    expect(found).toHaveLength(expected);
    for (const name of found) expect(name).toContain("StGrand");
    click(button(document.body, "0000OFF"));
    expect(onChange).toHaveBeenCalledWith("l", 0);
  });

  it("pairs R with L's Stereo partner", () => {
    const { container, onChange } = picker({ l: 1 });
    const pair = button(container, "Pair");
    expect(pair.disabled).toBe(false);
    click(pair);
    expect(onChange).toHaveBeenCalledWith("r", 2);
  });

  it("disables Pair when L has no Stereo partner", () => {
    const { container } = picker({ l: 2 });
    expect(button(container, "Pair").disabled).toBe(true);
  });

  it("clears R to OFF", () => {
    const { container, onChange } = picker({ l: 1, r: 2 });
    click(button(container, "Clear R"));
    expect(onChange).toHaveBeenCalledWith("r", 0);
  });

  it("warns when L is OFF but R isn't", () => {
    expect(picker({ l: 0, r: 5 }).container.querySelector('[role="alert"]')).not.toBeNull();
    cleanup();
    expect(picker({ l: 1, r: 5 }).container.querySelector('[role="alert"]')).toBeNull();
    cleanup();
    expect(picker({ l: 0, r: 0 }).container.querySelector('[role="alert"]')).toBeNull();
  });

  it("marks a group whose Expansion isn't loaded, with a Load action", () => {
    const onLoad = vi.fn();
    const marking: ExpansionMarking = {
      isUnloaded: (e) => e?.name === "SRX-07",
      onLoad,
    };
    const { container, onGroupChange } = picker({ source: SRX07, marking });
    expect(container.textContent).toContain("SRX-07 not loaded");
    click(button(container, "Load"));
    expect(onLoad).toHaveBeenCalledWith(SRX07.expansion);

    // The dropdown marks it too, and still lets it be picked.
    click(button(container, "SRX-07 ▾"));
    const rows = [...document.body.querySelectorAll('[role="listbox"] > div')];
    const marked = rows.filter((r) => r.textContent?.includes("not loaded"));
    expect(marked.map((r) => r.querySelector('[role="option"]')?.textContent)).toEqual([
      `SRX-07 ${SRX07.waves.length}`,
    ]);
    click(marked[0]!.querySelector('[role="option"]')!);
    expect(onGroupChange).toHaveBeenCalledWith(SRX07);
  });

  it("shows no marker while loaded state is unknown or loaded", () => {
    const { container } = picker({
      source: SRX07,
      marking: { isUnloaded: () => false, onLoad: () => {} },
    });
    expect(container.textContent).not.toContain("not loaded");
    cleanup();
    expect(picker({ source: SRX07 }).container.textContent).not.toContain("not loaded");
  });
});

describe("PCM Synth partial wiring", () => {
  function partialWave(partial: {
    waveGroupType: number;
    waveGroupId: number;
    waveNumberL: number;
    waveNumberR: number;
  }) {
    const onParam = vi.fn();
    const set: PartialSetter = (field, key, index = 0) => (v) =>
      onParam(field, key, index, v);
    const { container } = render(<PartialWave partial={partial} set={set} />);
    return { container, onParam };
  }

  it("writes L and R through the core Wave Number fields", () => {
    const { container, onParam } = partialWave({
      waveGroupType: 0,
      waveGroupId: 0,
      waveNumberL: 1,
      waveNumberR: 0,
    });
    pickWave(container, 0, 2);
    click(button(container, "Pair"));
    expect(onParam.mock.calls).toEqual([
      [PcmsPartialField.WaveNumberL, "waveNumberL", 0, 2],
      [PcmsPartialField.WaveNumberR, "waveNumberR", 0, 2],
    ]);
  });

  it("writes Group Type and ID together", () => {
    const { container, onParam } = partialWave({
      waveGroupType: 0,
      waveGroupId: 0,
      waveNumberL: 1,
      waveNumberR: 0,
    });
    click(button(container, "INT ▾"));
    const srx07 = [...document.body.querySelectorAll('[role="option"]')].find((o) =>
      o.textContent?.startsWith("SRX-07 "),
    );
    click(srx07!);
    expect(onParam.mock.calls).toEqual([
      [PcmsPartialField.WaveGroupType, "waveGroupType", 0, 1],
      [PcmsPartialField.WaveGroupId, "waveGroupId", 0, 7],
    ]);
  });

  it("labels a reserved group by its raw Type and ID", () => {
    const { container } = partialWave({
      waveGroupType: 2,
      waveGroupId: 0,
      waveNumberL: 5,
      waveNumberR: 0,
    });
    expect(button(container, "Type 2 ID 0 ▾")).toBeTruthy();
    expect(container.textContent).toContain("0005 ?");
  });
});

describe("PCM Drum WMT wiring", () => {
  it("writes through the core WMT fields, and follows the selected key", () => {
    const onWmtParam = vi.fn();
    const kick = { waveGroupType: 0, waveGroupId: 0, waveNumberL: 1, waveNumberR: 2 };
    const snare = { waveGroupType: 1, waveGroupId: 7, waveNumberL: 3, waveNumberR: 0 };
    const { container, rerender } = render(
      <WmtWave wmt={kick} index={2} onWmtParam={onWmtParam} />,
    );
    expect(nameButton(container, 0).textContent).toBe("0001 StGrand pA L");

    pickWave(container, 0, 2);
    click(button(container, "Clear R"));
    click(button(container, "INT ▾"));
    const srx07 = [...document.body.querySelectorAll('[role="option"]')].find((o) =>
      o.textContent?.startsWith("SRX-07 "),
    );
    click(srx07!);
    expect(onWmtParam.mock.calls).toEqual([
      [2, PcmdWmtField.WaveNumberL, "waveNumberL", 2],
      [2, PcmdWmtField.WaveNumberR, "waveNumberR", 0],
      [2, PcmdWmtField.WaveGroupType, "waveGroupType", 1],
      [2, PcmdWmtField.WaveGroupId, "waveGroupId", 7],
    ]);

    // Another key: its own group and Waves.
    rerender(<WmtWave wmt={snare} index={2} onWmtParam={onWmtParam} />);
    expect(nameButton(container, 0).textContent).toBe(`0003 ${SRX07.name(3)}`);
    expect(nameButton(container, 1).textContent).toBe("0000 OFF");
  });
});

describe("SN Synth OSC PCM Wave wiring", () => {
  it("shows raw value 1 as JP-8 Saw and picks through the core Wave Number field", () => {
    const onChange = vi.fn();
    const { container } = render(<OscPcmWave waveNumber={1} onChange={onChange} />);
    const name = container.querySelector("button")!;
    expect(name.textContent).toBe("0001 JP-8 Saw");
    // Just the name: no group label, steppers, dropdown, R, pair or marker.
    expect(container.querySelector('[aria-label="Next Wave"]')).toBeNull();
    expect(container.querySelector('[aria-haspopup="listbox"]')).toBeNull();
    expect(container.querySelector('[aria-label="Pair"]')).toBeNull();

    click(name);
    const second = [...document.body.querySelectorAll('[role="option"]')].find((o) =>
      o.textContent?.startsWith("0002"),
    );
    click(second!);
    expect(onChange.mock.calls).toEqual([[SnsPartialField.WaveNumber, 2]]);
  });
});
