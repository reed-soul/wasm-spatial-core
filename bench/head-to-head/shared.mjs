// Shared helpers for the wasm-spatial-core head-to-head benchmarks.
//
// Fairness controls live here so every runner (wasm / py3dtiles / loaders.gl)
// reports results in the SAME schema and uses the SAME input data.
//
// Result schema (one object per (engine, input)):
//   {
//     engine: "wasm-spatial-core" | "py3dtiles" | "loaders.gl",
//     engine_version: string,
//     input: string,            // input identifier, e.g. "synthetic_500k.las" or "synth-1m"
//     point_count: number,      // points actually consumed (for verification)
//     output_bytes: number,     // total bytes of generated tiles
//     tile_count: number,       // number of tiles/files generated
//     wall_ms: { avg, min, max, runs },
//     peak_rss_mb: number,
//     notes: string,            // caveats, errors, "could not generate pnts", etc.
//     hardware: { node, platform, arch, cpus },
//     timestamp: ISO string
//   }

import { readFileSync, existsSync, statSync } from "node:fs";
import { createHash } from "node:crypto";
import os from "node:os";

export const HARDWARE = {
  node: process.version,
  platform: process.platform,
  arch: process.arch,
  cpus: os.cpus().length,
  cpu_model: os.cpus()[0]?.model || "unknown",
  total_mem_mb: Math.round(os.totalmem() / 1024 / 1024),
};

/** Trimmed-mean timer. Returns { avg, min, max, runs } in ms. */
export function bench(fn, { warmup = 2, runs = 5 } = {}) {
  for (let i = 0; i < warmup; i++) fn();
  const times = [];
  for (let i = 0; i < runs; i++) {
    const t0 = performance.now();
    fn();
    times.push(performance.now() - t0);
  }
  times.sort((a, b) => a - b);
  // drop min and max (trimmed mean) unless very few samples
  const trimmed = times.length > 3 ? times.slice(1, -1) : times;
  const avg = trimmed.reduce((s, v) => s + v, 0) / trimmed.length;
  return { avg, min: trimmed[0], max: trimmed[trimmed.length - 1], runs };
}

/** Peak RSS in MB via resource usage (Node). */
export function peakRssMb() {
  return Math.round(process.memoryUsage().rss / 1024 / 1024);
}

/** Read an input file; throw clearly if missing. */
export function readInput(path) {
  if (!existsSync(path)) {
    throw new Error(`Input not found: ${path}. Generate it first (see README).`);
  }
  return readFileSync(path);
}

/** SHA-256 of a file, first 12 hex chars — for reproducibility provenance. */
export function fileDigest(path) {
  const buf = readFileSync(path);
  return createHash("sha256").update(buf).digest("hex").slice(0, 12);
}

/** Build a result object in the canonical schema. */
export function makeResult(fields) {
  return {
    timestamp: new Date().toISOString(),
    hardware: HARDWARE,
    ...fields,
  };
}

/**
 * Locate the WASM pkg. Order: WASM_PKG_DIR env, npm/pkg-node, ./pkg-node, ../pkg-node.
 * Mirrors the pattern in tests/node_batch_test.mjs.
 */
export function resolvePkgDir() {
  const candidates = [
    process.env.WASM_PKG_DIR,
    "npm/pkg-node",
    "./pkg-node",
    "../pkg-node",
  ].filter(Boolean);
  for (const c of candidates) {
    if (existsSync(`${c}/wasm_spatial_core.js`)) return c;
  }
  throw new Error(
    "WASM pkg-node not found. Run: wasm-pack build --target nodejs --release --out-dir pkg-node (or set WASM_PKG_DIR).",
  );
}

export function fmtMs(ms) {
  return ms < 1 ? `${(ms * 1000).toFixed(0)}µs` : `${ms.toFixed(1)}ms`;
}

export function fmtBytes(b) {
  if (b >= 1e6) return `${(b / 1e6).toFixed(2)}MB`;
  if (b >= 1e3) return `${(b / 1e3).toFixed(1)}KB`;
  return `${b}B`;
}
