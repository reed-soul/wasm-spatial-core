#!/usr/bin/env node
/**
 * LAS → 3D Tiles via wasm-spatial-core (nodejs target).
 *
 * Usage:
 *   WASM_PKG_DIR=pkg-node node run-wasm.mjs <input.las>  > results-wasm.jsonl
 */
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { performance } from "node:perf_hooks";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "../..");

const lasPath = resolve(process.argv[2] || "bench/head-to-head/fixtures/canonical_500k.las");

function rssMb() {
  return process.memoryUsage().rss / (1024 * 1024);
}

function record({ ok, version, wallMs, rss, outputBytes, tiles, error, notes }) {
  const row = {
    engine: "wasm-spatial-core",
    version: version ?? null,
    ok,
    wall_ms: wallMs ?? null,
    rss_mb: rss ?? null,
    output_bytes: outputBytes ?? 0,
    tiles: tiles ?? 0,
    error: error ?? null,
    notes: notes ?? "parsePointCloudAuto → generateTileset (octree + pnts encode in one call)",
  };
  process.stdout.write(JSON.stringify(row) + "\n");
}

async function loadCore() {
  const pkgDir = resolve(
    process.env.WASM_PKG_DIR || join(repoRoot, "pkg-node"),
  );
  const jsPath = join(pkgDir, "wasm_spatial_core.js");
  const wasmPath = join(pkgDir, "wasm_spatial_core_bg.wasm");
  if (!existsSync(jsPath)) {
    throw new Error(
      `WASM glue not found at ${jsPath}. Run: wasm-pack build --target nodejs --release --out-dir pkg-node -- --features point-cloud`,
    );
  }
  const mod = await import(pathToFileURL(jsPath).href);
  if (typeof mod.default === "function" && existsSync(wasmPath)) {
    await mod.default(readFileSync(wasmPath));
  } else if (typeof mod.default === "function") {
    await mod.default();
  }
  return mod;
}

function runOnce(core, bytes) {
  const cloud = core.parsePointCloudAuto(bytes);
  const positions = cloud.positions;
  const colors = cloud.colors;
  const tileset = core.generateTileset(
    positions,
    50_000,
    21,
    colors ?? undefined,
  );
  const json = tileset.tilesetJson();
  let outputBytes = Buffer.byteLength(json, "utf8");
  const tileCount = Number(tileset.tileCount);
  for (let i = 0; i < tileCount; i++) {
    const tile = tileset.tile(i);
    outputBytes += tile.byteLength;
  }
  return { tileCount, outputBytes, pointCount: Number(cloud.pointCount) };
}

async function main() {
  if (!existsSync(lasPath)) {
    record({
      ok: false,
      error: `LAS fixture not found: ${lasPath}. Generate with: cargo run --example gen_test_las --features point-cloud -- 500000 ${lasPath}`,
    });
    process.exit(1);
  }

  const core = await loadCore();
  const version = typeof core.version === "function" ? core.version() : null;
  const bytes = readFileSync(lasPath);

  // Warmup (not timed)
  runOnce(core, bytes);

  const times = [];
  let last = null;
  const peakRss = rssMb();
  for (let i = 0; i < 5; i++) {
    const t0 = performance.now();
    last = runOnce(core, bytes);
    times.push(performance.now() - t0);
  }
  times.sort((a, b) => a - b);
  const trimmed = times.slice(1, 4);
  const wallMs = trimmed.reduce((a, b) => a + b, 0) / trimmed.length;

  record({
    ok: true,
    version,
    wallMs,
    rss: Math.max(peakRss, rssMb()),
    outputBytes: last.outputBytes,
    tiles: last.tileCount,
  });
}

main().catch((err) => {
  record({ ok: false, error: String(err && err.message ? err.message : err) });
  process.exit(1);
});
