import { afterEach, describe, expect, it, vi } from "vitest";
import { WavePicker, type ExpansionMarking } from "./WavePicker";
import { waveSourceOf, type WaveSource } from "./waveCatalog";
import { PartialWave } from "./pcm-synth/PartialRow";
import { WmtWave } from "./pcm-drum/NoteControls";
import { PcmdWmtField } from "../pkg/integral_wasm.js";
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
    /^\S+ \d{4} /.test(b.textContent ?? ""),
  )[index];
  if (!found) throw new Error(`no name button ${index}`);
  return found;
}

function searchBox(): HTMLInputElement {
  const input = document.body.querySelector("input");
  if (!input) throw new Error("no search box");
  return input;
}

describe("WavePicker", () => {
  it("shows group, padded Wave Number and name on each row", () => {
    const { container } = picker({ l: 1, r: 2 });
    expect(nameButton(container, 0).textContent).toBe("INT 0001 StGrand pA L");
    expect(nameButton(container, 1).textContent).toBe("INT 0002 StGrand pA R");
  });

  it("steps through the group's Waves down to OFF", () => {
    const { container, onChange } = picker({ l: 1 });
    const [prevL, prevR] = [...container.querySelectorAll('[aria-label="Previous Wave"]')];
    const [nextL] = [...container.querySelectorAll('[aria-label="Next Wave"]')];
    click(nextL!);
    click(prevL!);
    expect(onChange.mock.calls).toEqual([
      ["l", 2],
      ["l", 0],
    ]);
    // R is OFF: nothing below it.
    expect((prevR as HTMLButtonElement).disabled).toBe(true);
  });

  it("can't step past the group's last Wave", () => {
    const { container } = picker({ l: INT.waves.length });
    const nextL = container.querySelector('[aria-label="Next Wave"]');
    expect((nextL as HTMLButtonElement).disabled).toBe(true);
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
    const pair = button(container, "pair");
    expect(pair.disabled).toBe(false);
    click(pair);
    expect(onChange).toHaveBeenCalledWith("r", 2);
  });

  it("disables pair when L has no Stereo partner", () => {
    const { container } = picker({ l: 2 });
    expect(button(container, "pair").disabled).toBe(true);
  });

  it("clears R to OFF", () => {
    const { container, onChange } = picker({ l: 1, r: 2 });
    click(button(container, "×"));
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
    const setP = vi.fn();
    const setNibP = vi.fn();
    const { container } = render(
      <PartialWave partial={partial} setP={setP} setNibP={setNibP} />,
    );
    return { container, setP, setNibP };
  }

  it("writes L at 0x2C and R at 0x30 via the nibble path", () => {
    const { container, setP, setNibP } = partialWave({
      waveGroupType: 0,
      waveGroupId: 0,
      waveNumberL: 1,
      waveNumberR: 0,
    });
    click(container.querySelector('[aria-label="Next Wave"]')!);
    click(button(container, "pair"));
    expect(setNibP.mock.calls).toEqual([
      [0x2c, 2],
      [0x30, 2],
    ]);
    expect(setP).not.toHaveBeenCalled();
  });

  it("writes Group Type at 0x27 and ID at 0x28 together", () => {
    const { container, setP, setNibP } = partialWave({
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
    expect(setP.mock.calls).toEqual([[0x27, 1]]);
    expect(setNibP.mock.calls).toEqual([[0x28, 7]]);
  });

  it("labels a reserved group by its raw Type and ID", () => {
    const { container } = partialWave({
      waveGroupType: 2,
      waveGroupId: 0,
      waveNumberL: 5,
      waveNumberR: 0,
    });
    expect(container.textContent).toContain("Type 2 ID 0 0005 ?");
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
    expect(nameButton(container, 0).textContent).toBe("INT 0001 StGrand pA L");

    click(container.querySelector('[aria-label="Next Wave"]')!);
    click(button(container, "×"));
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
    expect(nameButton(container, 0).textContent).toBe(`SRX-07 0003 ${SRX07.name(3)}`);
    expect(nameButton(container, 1).textContent).toBe("SRX-07 0000 OFF");
  });
});
