// PROTOTYPE ONLY (throwaway): fetch Roland's Wave name lists from the public
// mirror and evaluate them in a vm sandbox (no text parsing) into JSON.
import vm from "node:vm";
import { writeFile } from "node:fs/promises";

const BASE =
  "https://raw.githubusercontent.com/SambaGodschynski/integra7-editor-web/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/preset/";

const SOURCES = [
  { id: 0, name: "INT", file: "wave_list_xv5080.js", key: "xv5080" },
  { id: 5, name: "SRX-05", file: "wave_list_srx05.js", key: "srx05" },
  { id: 7, name: "SRX-07", file: "wave_list_srx07.js", key: "srx07" },
];

const groups = [];
for (const s of SOURCES) {
  const res = await fetch(BASE + s.file);
  if (!res.ok) throw new Error(`${s.file}: HTTP ${res.status}`);
  const src = await res.text();
  const globals = { parameter: { wave: {} } };
  vm.runInNewContext(src, { globals });
  const arr = globals.parameter.wave[s.key];
  if (!Array.isArray(arr)) throw new Error(`${s.file}: no array at wave.${s.key}`);
  // Index 0 is OFF; drop it so Wave Number = array index + 1.
  const waves = arr.slice(1).map((n) => String(n).trimEnd());
  console.log(`${s.name}: ${waves.length} waves (first: ${waves[0]})`);
  groups.push({ id: s.id, name: s.name, waves });
}

await writeFile(
  new URL("./waves.prototype.json", import.meta.url),
  JSON.stringify({ groups }, null, 1) + "\n",
);
