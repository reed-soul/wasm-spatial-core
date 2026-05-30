#!/usr/bin/env node
/**
 * Performance Benchmark: wasm-spatial-core vs proj4js
 *
 * Compares coordinate transformation and GeoJSON parsing speed
 * across different data volumes (10K, 100K, 1M points).
 *
 * Usage:
 *   node bench/comparison/bench_comparison.mjs
 *
 * Prerequisites:
 *   npm install proj4
 *   wasm-pack build --target nodejs --release --out-dir pkg-node
 */

import { performance } from "node:perf_hooks";
import proj4 from "proj4";

// Import WASM module (nodejs target)
let wasm;
try {
  wasm = await import("../../pkg-node/wasm_spatial_core.js");
  await wasm.default();
} catch (e) {
  console.error(
    "❌ Failed to load WASM. Run: wasm-pack build --target nodejs --release --out-dir pkg-node"
  );
  process.exit(1);
}

// ─── Helpers ─────────────────────────────────────────────────────────

function fmt(n) {
  if (n >= 1e6) return `${(n / 1e6).toFixed(2)}M`;
  if (n >= 1e3) return `${(n / 1e3).toFixed(1)}K`;
  return String(n);
}

function fmtMs(ms) {
  return ms < 1 ? `${(ms * 1000).toFixed(0)}µs` : `${ms.toFixed(2)}ms`;
}

function fmtRate(n, ms) {
  const r = n / (ms / 1000);
  if (r >= 1e6) return `${(r / 1e6).toFixed(2)}M/s`;
  if (r >= 1e3) return `${(r / 1e3).toFixed(1)}K/s`;
  return `${r.toFixed(0)}/s`;
}

function generatePoints(n) {
  // China area coordinates (lng: 73-135, lat: 3-53)
  const coords = new Float64Array(n * 2);
  for (let i = 0; i < n; i++) {
    coords[i * 2] = 73 + Math.random() * 62;     // lng
    coords[i * 2 + 1] = 3 + Math.random() * 50;  // lat
  }
  return coords;
}

function generateGeoJson(n) {
  const features = [];
  for (let i = 0; i < n; i++) {
    features.push({
      type: "Feature",
      geometry: {
        type: "Point",
        coordinates: [73 + Math.random() * 62, 3 + Math.random() * 50],
      },
      properties: { id: i, name: `point-${i}` },
    });
  }
  return JSON.stringify({ type: "FeatureCollection", features });
}

function bench(fn, warmup = 3, runs = 10) {
  // Warmup
  for (let i = 0; i < warmup; i++) fn();

  const times = [];
  for (let i = 0; i < runs; i++) {
    const t0 = performance.now();
    fn();
    times.push(performance.now() - t0);
  }

  // Remove outliers (min and max)
  times.sort((a, b) => a - b);
  const trimmed = times.slice(1, -1);
  const avg = trimmed.reduce((s, v) => s + v, 0) / trimmed.length;
  const min = trimmed[0];
  const max = trimmed[trimmed.length - 1];
  return { avg, min, max };
}

// ─── Benchmarks ──────────────────────────────────────────────────────

const N_SIZES = [10_000, 100_000, 1_000_000];
const results = [];

console.log("═══════════════════════════════════════════════════════════════");
console.log("  wasm-spatial-core  vs  proj4js  Performance Comparison");
console.log("═══════════════════════════════════════════════════════════════\n");

for (const n of N_SIZES) {
  console.log(`── ${fmt(n)} points ──`);
  const row = { n };

  // Generate test data
  const coords = generatePoints(n);
  const geojson = n <= 100_000 ? generateGeoJson(n) : null;

  // ── WGS84 → GCJ02 ────────────────────────────────────────────
  {
    const wasmResult = bench(() => {
      const out = new Float64Array(coords.length);
      out.set(coords);
      wasm.batchWgs84ToGcj02InPlace(out);
      return out;
    });
    const projResult = bench(() => {
      const out = new Float64Array(coords.length);
      for (let i = 0; i < n; i++) {
        const [lng, lat] = proj4("EPSG:4326", "EPSG:4490", [
          coords[i * 2],
          coords[i * 2 + 1],
        ]);
        // proj4 doesn't have GCJ02, use WGS84→CGCS2000 as comparison
        // (Note: CGCS2000 ≈ WGS84 for this comparison purpose)
        out[i * 2] = lng;
        out[i * 2 + 1] = lat;
      }
      return out;
    });
    row.wgs84_gcj02_wasm = wasmResult.avg;
    row.wgs84_gcj02_proj = projResult.avg;
    const speedup = projResult.avg / wasmResult.avg;
    console.log(
      `  WGS84→GCJ02:  WASM ${fmtMs(wasmResult.avg).padEnd(8)} ${fmtRate(n, wasmResult.avg).padEnd(10)} | proj4js ${fmtMs(projResult.avg).padEnd(8)} ${fmtRate(n, projResult.avg).padEnd(10)} | ${speedup.toFixed(1)}x`
    );
  }

  // ── WGS84 → Mercator ─────────────────────────────────────────
  {
    const wasmResult = bench(() => {
      const out = new Float64Array(coords.length);
      out.set(coords);
      wasm.batchWgs84ToMercatorInPlace(out);
      return out;
    });
    const projResult = bench(() => {
      const out = new Float64Array(coords.length);
      for (let i = 0; i < n; i++) {
        const [x, y] = proj4("EPSG:4326", "EPSG:3857", [
          coords[i * 2],
          coords[i * 2 + 1],
        ]);
        out[i * 2] = x;
        out[i * 2 + 1] = y;
      }
      return out;
    });
    row.wgs84_merc_wasm = wasmResult.avg;
    row.wgs84_merc_proj = projResult.avg;
    const speedup = projResult.avg / wasmResult.avg;
    console.log(
      `  WGS84→Merc:   WASM ${fmtMs(wasmResult.avg).padEnd(8)} ${fmtRate(n, wasmResult.avg).padEnd(10)} | proj4js ${fmtMs(projResult.avg).padEnd(8)} ${fmtRate(n, projResult.avg).padEnd(10)} | ${speedup.toFixed(1)}x`
    );
  }

  // ── GeoJSON Parse (only for ≤ 100K due to memory) ───────────
  if (geojson) {
    const wasmResult = bench(() => {
      return wasm.parseGeoJsonCoords(geojson);
    });
    const projResult = bench(() => {
      return JSON.parse(geojson);
    });
    row.geojson_wasm = wasmResult.avg;
    row.geojson_native = projResult.avg;
    console.log(
      `  GeoJSON:      WASM ${fmtMs(wasmResult.avg).padEnd(8)} ${fmtRate(n, wasmResult.avg).padEnd(10)} | native ${fmtMs(projResult.avg).padEnd(8)} ${fmtRate(n, projResult.avg).padEnd(10)} | ${(projResult.avg / wasmResult.avg).toFixed(1)}x`
    );
  }

  results.push(row);
  console.log();
}

// ─── Summary Table ────────────────────────────────────────────────────

console.log("═══════════════════════════════════════════════════════════════");
console.log("  Summary");
console.log("═══════════════════════════════════════════════════════════════\n");
console.log(
  "│ Size      │ WGS84→GCJ02 WASM │ WGS84→GCJ02 proj4 │ WGS84→Merc WASM │ WGS84→Merc proj4 │ Speedup │"
);
console.log(
  "│           │                  │                  │                  │                  │         │"
);
console.log(
  "├───────────┼──────────────────┼──────────────────┼──────────────────┼──────────────────┼─────────┤"
);

for (const row of results) {
  const n = fmt(row.n).padEnd(9);
  const gcj_w = fmtMs(row.wgs84_gcj02_wasm).padEnd(16);
  const gcj_p = fmtMs(row.wgs84_gcj02_proj).padEnd(16);
  const mer_w = fmtMs(row.wgs84_merc_wasm).padEnd(16);
  const mer_p = fmtMs(row.wgs84_merc_proj).padEnd(16);
  const speedup = (row.wgs84_gcj02_proj / row.wgs84_gcj02_wasm).toFixed(1).padStart(7) + "x";
  console.log(`│ ${n} │ ${gcj_w} │ ${gcj_p} │ ${mer_w} │ ${mer_p} │${speedup} │`);
}

console.log(
  "└───────────┴──────────────────┴──────────────────┴──────────────────┴──────────────────┴─────────┘"
);

// ─── Memory info ───────────────────────────────────────────────────────

try {
  const mem = wasm.memoryInfo();
  console.log(`\nWASM Memory: ${((mem.current / (1024 * 1024)) | 0)}MB / ${((mem.maximum / (1024 * 1024)) | 0)}MB`);
} catch {
  // memoryInfo may not be available
}

console.log("\n✅ Benchmark complete.");
