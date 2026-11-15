/** System Common fixtures for component tests. */

import { type SystemField, WasmDeviceState, system_common_size } from "../../pkg/integral_wasm.js";
import { dt1Drainer, sevenBit } from "./toneService";

/** Bytes in a System Common dump. */
const BLOCK_LEN = sevenBit(system_common_size());

/** A System Common dump holding `values`, encoded by the core's own setter. */
export function systemCommonDump(values: ReadonlyArray<[SystemField, number]>): Uint8Array {
  const block = new Uint8Array(BLOCK_LEN);
  const device = new WasmDeviceState(0x10);
  for (const [field, value] of values) device.setSystemField(field, value);
  for (const { address, data } of dt1Drainer(device)()) block.set(data, address[3]!);
  device.free();
  return block;
}
