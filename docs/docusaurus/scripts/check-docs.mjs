#!/usr/bin/env node
/**
 * Documentation consistency check.
 *
 * The docs render their pin and configuration tables from data files
 * (src/data/interface.json and src/data/config.json). This script verifies
 * those data files still agree with the source of truth in the SRAM22 Rust
 * crate, for these structural attributes (not prose, timing, constraints, or behavior):
 *
 *   • interface.json  ⟷  the LEF pin emitter in src/abs.rs
 *   • config.json     ⟷  the SramConfig struct in src/blocks/sram/mod.rs
 *
 * Exits non-zero (and prints a diff) on any mismatch.
 */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const siteRoot = resolve(here, "..");
const repoRoot = resolve(siteRoot, "..", "..");

const read = (p) => readFileSync(p, "utf8");
const json = (p) => JSON.parse(read(p));

let failures = 0;
const fail = (msg) => {
  failures++;
  console.error(`  ✗ ${msg}`);
};
const ok = (msg) => console.log(`  ✓ ${msg}`);

// --------------------------------------------------------------------------
// 1. Pins:  src/data/interface.json  vs  src/abs.rs
// --------------------------------------------------------------------------
console.log("Checking pin interface (interface.json ⟷ src/abs.rs)…");

const iface = json(resolve(siteRoot, "src/data/interface.json"));
const absRs = read(resolve(repoRoot, "src/abs.rs"));

const LAYER = { m1: "met1", m2: "met2" };
const WIDTH = {
  "params.data_width()": "data_width",
  "params.wmask_width()": "wmask_width",
  "params.addr_width()": "addr_width",
  "1": "1",
};
const DIR = { Output: "output", Input: "input", Inout: "inout" };

// Match tuples like:  ("dout", m1, params.data_width(), LefPinDirection::Output ...)
const tupleRe =
  /\(\s*"(\w+)"\s*,\s*(m1|m2)\s*,\s*([^,]+?)\s*,\s*LefPinDirection::(\w+)/g;
const rustPins = new Map();
for (const m of absRs.matchAll(tupleRe)) {
  const [, name, layer, widthRaw, dir] = m;
  const width = WIDTH[widthRaw.trim()];
  if (width === undefined) {
    fail(`abs.rs pin "${name}" has unrecognized width expression \`${widthRaw.trim()}\``);
    continue;
  }
  rustPins.set(name, { layer: LAYER[layer], width, direction: DIR[dir] });
}

if (rustPins.size === 0) {
  fail("could not parse any pins from src/abs.rs (did the format change?)");
} else {
  ok(`parsed ${rustPins.size} pins from src/abs.rs`);
}

const docPins = new Map(
  iface.pins.map((p) => [
    p.name,
    { layer: p.layer, width: p.width, direction: p.direction },
  ]),
);

// same set of names
const rustNames = [...rustPins.keys()].sort();
const docNames = [...docPins.keys()].sort();
const onlyRust = rustNames.filter((n) => !docPins.has(n));
const onlyDoc = docNames.filter((n) => !rustPins.has(n));
if (onlyRust.length) fail(`pins in abs.rs but missing from docs: ${onlyRust.join(", ")}`);
if (onlyDoc.length) fail(`pins in docs but not in abs.rs: ${onlyDoc.join(", ")}`);

// per-pin attributes
const failsBeforeAttrs = failures;
for (const [name, r] of rustPins) {
  const d = docPins.get(name);
  if (!d) continue;
  for (const key of ["direction", "width", "layer"]) {
    if (r[key] !== d[key]) {
      fail(`pin "${name}" ${key}: abs.rs=\`${r[key]}\` but docs=\`${d[key]}\``);
    }
  }
}
if (failures === failsBeforeAttrs && !onlyRust.length && !onlyDoc.length)
  ok("pin names, directions, widths, and layers match");

// --------------------------------------------------------------------------
// 2. Config options:  src/data/config.json  vs  SramConfig in mod.rs
// --------------------------------------------------------------------------
console.log("\nChecking config options (config.json ⟷ SramConfig)…");

const cfg = json(resolve(siteRoot, "src/data/config.json"));
const modRs = read(resolve(repoRoot, "src/blocks/sram/mod.rs"));

const structM = modRs.match(/pub struct SramConfig\s*\{([\s\S]*?)\}/);
if (!structM) {
  fail("could not find `pub struct SramConfig` in src/blocks/sram/mod.rs");
} else {
  const fieldRe = /^\s*pub\s+(\w+)\s*:/gm;
  const rustFields = [...structM[1].matchAll(fieldRe)].map((m) => m[1]).sort();
  const docOptions = cfg.options.map((o) => o.name).sort();

  const onlyStruct = rustFields.filter((f) => !docOptions.includes(f));
  const onlyCfg = docOptions.filter((f) => !rustFields.includes(f));
  if (onlyStruct.length)
    fail(`SramConfig fields not documented in config.json: ${onlyStruct.join(", ")}`);
  if (onlyCfg.length)
    fail(`config.json options not in SramConfig: ${onlyCfg.join(", ")}`);
  if (!onlyStruct.length && !onlyCfg.length)
    ok(`config options match SramConfig fields (${rustFields.join(", ")})`);
}

// --------------------------------------------------------------------------
console.log("");
if (failures) {
  console.error(`✗ docs consistency check FAILED with ${failures} mismatch(es).`);
  console.error(
    "  Update the data files in docs/docusaurus/src/data/ (or the Rust source) so they agree.",
  );
  process.exit(1);
}
console.log("✓ pin attributes and configuration field names match the SRAM22 source.");
