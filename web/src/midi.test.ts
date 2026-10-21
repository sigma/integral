import { describe, expect, it } from "vitest";
import { LAST_PORT_KEY, findPortByName, portMemory, type MidiPortPair } from "./midi";
import { memoryStorage } from "./test/storage";

const pair = (name: string) => ({ id: `id-${name}`, name }) as MidiPortPair;

describe("portMemory", () => {
  it("recalls nothing before a port is remembered", () => {
    expect(portMemory(memoryStorage()).recall()).toBeNull();
  });

  it("recalls the port it remembered, across instances on one storage", () => {
    const storage = memoryStorage();
    portMemory(storage).remember("INTEGRA-7");
    expect(storage.getItem(LAST_PORT_KEY)).toBe("INTEGRA-7");
    expect(portMemory(storage).recall()).toBe("INTEGRA-7");
  });

  it("is a no-op without storage", () => {
    const memory = portMemory(null);
    memory.remember("INTEGRA-7");
    expect(memory.recall()).toBeNull();
  });

  it("survives a storage that throws", () => {
    const throwing = {
      getItem: () => {
        throw new Error("SecurityError");
      },
      setItem: () => {
        throw new Error("QuotaExceededError");
      },
    } as unknown as Storage;
    const memory = portMemory(throwing);
    expect(() => memory.remember("INTEGRA-7")).not.toThrow();
    expect(memory.recall()).toBeNull();
  });
});

describe("findPortByName", () => {
  it("finds the pair with that exact name", () => {
    const pairs = [pair("IAC Bus"), pair("INTEGRA-7")];
    expect(findPortByName(pairs, "INTEGRA-7")?.id).toBe("id-INTEGRA-7");
  });

  it("finds nothing for a missing or absent name", () => {
    const pairs = [pair("IAC Bus")];
    expect(findPortByName(pairs, "INTEGRA-7")).toBeUndefined();
    expect(findPortByName(pairs, null)).toBeUndefined();
  });
});
