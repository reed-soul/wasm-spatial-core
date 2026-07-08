#!/usr/bin/env node
// wasm-spatial-core runner: LAS → octree → 3D Tiles (pnts) end-to-end.
//
// Reports wall time, peak RSS, output bytes, tile count, and point count.
// Emits one JSON object per input to stdout (JSONL).
//
// Usage:
//   wasm-pack build --target nodejs --release --out-dir pkg-node
//   node bench/head-to-head/run-wasm.mjs [input.las ...]
//
// Default inputs (if none given): test-data/large/synthetic_500k.las.

import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { writeFileSync, existsSync } from "node:fs";
import {
  bench,
  peakRssMb,
  readInput,
  fileDigest,
  makeResult,
  resolvePkgDir,
  HARDWARE,
} from "./shared.mjs";

const ENGINES_VERSION_WASM = "see wasm.version()";

async function loadCore() {
  const dir = resolvePkgDir();
  const wasmJs = pathToFileURL(resolve(`${dir}/wasm_spatial_core.js`)).href;
  const mod = await import(wasmJs);
  if (mod.default && typeof mod.default === "function") {
    const { readFileSync } = await import("node:fs");
    const { resolve } = await import("node:path");
    await mod.default(readFileSync(resolve(`${dir}/wasm_spatial_core_bg.wasm`)));
  }
  return mod;
}

function runPipelineOnce(core, bytes) {
  // ── Stage 1: parse LAS → positions/colors ──
  const t0 = performance.now();
  const cloud = core.parsePointCloudAuto(bytes);
  const positions = cloud.positions;
  const colors = cloud.colors;
  const pointCount = cloud.pointCount;
  const tParse = performance.now() - t0;

  // ── Stage 2: octree + tileset (generateTileset does both) ──
  const t1 = performance.now();
  const tileset = core.generateTileset(positions, 50_000, 21, colors ?? undefined);
  const tilesetJson = tileset.tilesetJson();
  const tileCount = tileset.tileCount;
  // sum output bytes across all tiles
  let outputBytes = 0;
  for (let i = 0; i < tileCount; i++) {
    const tile = tileset.tile(i);
    if (tile) outputBytes += tile.byteLength;
  }
  // include tileset.json bytes
  outputBytes += Buffer.byteLength(tilesetJson, "utf8");
  const tTiles = performance.now() - t1;

  return { pointCount, tileCount, outputBytes, tParse, tTiles };
}

async function main() {
  const inputs = process.argv.slice(2);
  const defaultInput = "test-data/large/synthetic_500k.las";
  const targets = inputs.length ? inputs : [defaultInput];

  const core = await loadCore();
  const wasmVersion = core.version();

  for (const input of targets) {
    const bytes = readInput(input);
    const digest = fileDigest(input);
    const sizeMb = (bytes.length / 1e6).toFixed(2);

    // Verify parse once first (catches bad input before timing)
    const verify = runPipelineOnce(core, bytes);
    if (verify.pointCount === 0) {
      console.error(`  ⚠️  ${input}: parsed 0 points, skipping`);
      continue;
    }

    const rssBefore = peakRssMb();
    const stageSamples = { total: [], parse: [] };
    const timed = bench(() => {
      const r = runPipelineOnce(core, bytes);
      stageSamples.total.push(r.tParse + r.tTiles);
      stageSamples.parse.push(r.tParse);
      return r;
    }, { warmup: 1, runs: 5 });
    const rssAfter = peakRssMb();

    const avgParse =
      stageSamples.parse.reduce((s, v) => s + v, 0) / stageSamples.parse.length;

    const result = makeResult({
      engine: "wasm-spatial-core",
      engine_version: wasmVersion,
      input,
      input_digest_sha256_12: digest,
      input_size_mb: Number(sizeMb),
      point_count: verify.pointCount,
      output_bytes: verify.outputBytes,
      tile_count: verify.tileCount,
      wall_ms: {
        total: Number(timed.avg.toFixed(1)),
        parse_ms: Number(avgParse.toFixed(1)),
        min: Number(timed.min.toFixed(1)),
        max: Number(timed.max.toFixed(1)),
        runs: timed.runs,
      },
      peak_rss_mb: Math.max(rssBefore, rssAfter),
      notes: "parsePointCloudAuto → generateTileset (octree + pnts encode in one call)",
    });

    process.stdout.write(JSON.stringify(result) + "\n");
    console.error(
      `  ✅ ${input}: ${verify.pointCount} pts → ${verify.tileCount} tiles, ` +
        `${(verify.outputBytes / 1e6).toFixed(2)}MB out, ${timed.avg.toFixed(0)}ms`,
    );
  }
}

main().catch((e) => {
  console.error("💥 run-wasm failed:", e);
  process.exit(1);
});
