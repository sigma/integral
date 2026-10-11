// Checks a `vite build` output for the PWA invariants of issue #202:
// the manifest's start_url and scope equal the Vite base, and the service
// worker precaches the WASM binary and every hashed JS and CSS asset.
//
// Usage: node scripts/check-pwa-build.mjs <outDir> <base>
//
// The precache list is read by running sw.js against a stub Workbox, not by
// pattern-matching its minified source.
import { readFileSync, readdirSync } from "node:fs";
import { extname, join } from "node:path";
import { runInNewContext } from "node:vm";

const [outDir, base] = process.argv.slice(2);
if (!outDir || !base) {
  console.error("usage: check-pwa-build.mjs <outDir> <base>");
  process.exit(2);
}

const failures = [];

const manifest = JSON.parse(
  readFileSync(join(outDir, "manifest.webmanifest"), "utf8"),
);
for (const key of ["start_url", "scope"]) {
  if (manifest[key] !== base) {
    failures.push(`manifest ${key} is ${manifest[key]}, want ${base}`);
  }
}

// A Workbox stand-in: every member is a no-op function (or class), except
// precacheAndRoute, which records its entries.
const precached = new Set();
const noop = new Proxy(function () {}, {
  get: (_target, prop) =>
    prop === "precacheAndRoute"
      ? (entries) => entries.forEach((e) => precached.add(e.url ?? e))
      : noop,
  apply: () => noop,
  construct: () => noop,
});
const define = (_deps, factory) => factory(noop);
const self = { define, addEventListener: noop, skipWaiting: noop };
runInNewContext(readFileSync(join(outDir, "sw.js"), "utf8"), { self, define });

const assets = readdirSync(join(outDir, "assets"));
const required = assets.filter((f) =>
  [".js", ".css", ".wasm"].includes(extname(f)),
);
if (!required.some((f) => extname(f) === ".wasm")) {
  failures.push("no .wasm file in assets/");
}
for (const file of required) {
  if (!precached.has(`assets/${file}`)) {
    failures.push(`assets/${file} is not precached`);
  }
}

if (failures.length > 0) {
  console.error(`PWA build check failed for ${outDir} (base ${base}):`);
  for (const f of failures) console.error(`  - ${f}`);
  process.exit(1);
}
console.log(
  `PWA build check passed for ${outDir}: base ${base}, ${precached.size} precached`,
);
