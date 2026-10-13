import { afterEach, describe, expect, it, vi } from "vitest";
import { IntegraService } from "./integra";
import { studioSetType } from "./librarian";
import type { MidiPortPair } from "./midi";

/** Catalog replies come back at the catalog query address. */
const CATALOG_REPLY = [0x0f, 0x00, 0x03, 0x02];
/** SN Acoustic preset bank, a tone catalog bank. */
const SNA_MSB = 0x59;
const SNA_LSB = 0x40;

/** A catalog reply DT1: `[MSB, LSB, PC, 00, 00, name(16)]`. */
function catalogEntry(
  msb: number,
  lsb: number,
  pc: number,
  name: string,
): Uint8Array {
  const body = [
    ...CATALOG_REPLY,
    msb,
    lsb,
    pc,
    0,
    0,
    ...Array.from(name.padEnd(16).slice(0, 16), (c) => c.charCodeAt(0)),
  ];
  const sum = body.reduce((s, b) => s + b, 0);
  return new Uint8Array([
    0xf0,
    0x41,
    0x10,
    0x00,
    0x00,
    0x64,
    0x12,
    ...body,
    (128 - (sum % 128)) % 128,
    0xf7,
  ]);
}

function fakePort(): MidiPortPair {
  const input = Object.assign(new EventTarget(), { open: async () => input });
  return {
    id: "test",
    name: "test",
    input,
    output: { send: vi.fn() },
  } as unknown as MidiPortPair;
}

function receive(port: MidiPortPair, bytes: Uint8Array): void {
  port.input.dispatchEvent(
    Object.assign(new Event("midimessage"), { data: bytes }),
  );
}

describe("catalog reads", () => {
  let service: IntegraService | undefined;
  afterEach(() => service?.destroy());

  it("a Studio Set read takes only Studio Set entries", async () => {
    const port = fakePort();
    service = new IntegraService(port, 0x10);
    const read = service.requestStudioSetNames();
    const msb = studioSetType().bankMsb;
    for (let pc = 0; pc < 64; pc++)
      receive(port, catalogEntry(SNA_MSB, SNA_LSB, pc, `Tone ${pc}`));
    for (let pc = 0; pc < 64; pc++)
      receive(port, catalogEntry(msb, 0, pc, `Set ${pc}`));
    const names = await read;
    expect(names.size).toBe(64);
    expect(names.get(0)).toBe("Set 0");
    expect(names.get(63)).toBe("Set 63");
  });

  it("a tone catalog page takes only its own bank's entries", async () => {
    const port = fakePort();
    service = new IntegraService(port, 0x10);
    const page = service.requestToneCatalogPage(SNA_MSB, SNA_LSB, 0, 2);
    receive(port, catalogEntry(studioSetType().bankMsb, 0, 0, "Set 0"));
    receive(port, catalogEntry(SNA_MSB, SNA_LSB + 1, 0, "Other LSB"));
    receive(port, catalogEntry(SNA_MSB, SNA_LSB, 0, "Tone 0"));
    receive(port, catalogEntry(SNA_MSB, SNA_LSB, 1, "Tone 1"));
    expect((await page).map((e) => e.name)).toEqual(["Tone 0", "Tone 1"]);
  });
});
