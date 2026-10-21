// Checks a `vite build --mode plugin` output for what the plugin embeds
// (#365, ADR 0004): no service worker, manifest or registration script, a
// WASM binary, and an index.html that loads its assets from base `/`.
//
// Usage: node scripts/check-plugin-build.mjs <outDir>
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { extname, join } from "node:path";

const [outDir] = process.argv.slice(2);
if (!outDir) {
  console.error("usage: check-plugin-build.mjs <outDir>");
  process.exit(2);
}

const failures = [];

for (const file of ["sw.js", "registerSW.js", "manifest.webmanifest"]) {
  if (existsSync(join(outDir, file))) failures.push(`${file} is emitted`);
}

const assets = readdirSync(join(outDir, "assets"));
if (!assets.some((f) => extname(f) === ".wasm")) {
  failures.push("no .wasm file in assets/");
}

const html = readFileSync(join(outDir, "index.html"), "utf8");
if (html.includes("manifest.webmanifest")) {
  failures.push("index.html links the web manifest");
}
const entry = assets.find((f) => extname(f) === ".js" && f.startsWith("index-"));
if (!entry || !html.includes(`src="/assets/${entry}"`)) {
  failures.push(`index.html does not load /assets/${entry ?? "index-*.js"}`);
}

if (failures.length > 0) {
  console.error(`Plugin build check failed for ${outDir}:`);
  for (const f of failures) console.error(`  - ${f}`);
  process.exit(1);
}
console.log(`Plugin build OK: ${outDir}`);
