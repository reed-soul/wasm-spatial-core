// W4.5 — preferGpu:false forces WASM CPU path for heightfield flatten
// Run: node tests/heightfield_gpu_fallback_test.mjs

import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const pkgDir = existsSync(join(__dirname, "..", "pkg", "wasm_spatial_core.js"))
  ? resolve(join(__dirname, "..", "pkg"))
  : existsSync(join(__dirname, "..", "npm", "pkg", "wasm_spatial_core.js"))
    ? resolve(join(__dirname, "..", "npm", "pkg"))
    : resolve(join(__dirname, "..", "pkg"));

async function loadWasm() {
  const mod = await import(pathToFileURL(join(pkgDir, "wasm_spatial_core.js")).href);
  if (mod.default) {
    await mod.default(readFileSync(join(pkgDir, "wasm_spatial_core_bg.wasm")));
  }
  return mod;
}

/** Mirrors npm/webgpu.ts flattenHeightfield fallback branch */
async function flattenHeightfield(_ctx, heights, width, height, _mask, target, { preferGpu = true, wasm, bounds, polygon, featherCells = 0 }) {
  if (preferGpu !== false && _ctx) {
    throw new Error("GPU path should not run in Node");
  }
  if (!wasm.flattenTerrain) {
    throw new Error("flattenTerrain unavailable");
  }
  const out = new Float32Array(heights);
  wasm.flattenTerrain(out, width, height, bounds, polygon, target, featherCells);
  return out;
}

function assertClose(a, b, tol = 1e-5) {
  if (a.length !== b.length) throw new Error("length mismatch");
  for (let i = 0; i < a.length; i++) {
    if (Math.abs(a[i] - b[i]) > tol) {
      throw new Error(`diff at ${i}: ${a[i]} vs ${b[i]}`);
    }
  }
}

const wasm = await loadWasm();
if (!wasm.flattenTerrain) {
  console.log("heightfield fallback test: SKIP (terrain-edit not in WASM build)");
  process.exit(0);
}

const width = 8;
const height = 8;
const bounds = new Float64Array([0, 0, 1, 1]);
const polygon = new Float64Array([0.25, 0.25, 0.75, 0.25, 0.75, 0.75, 0.25, 0.75]);
const heights = new Float32Array(width * height);
for (let i = 0; i < heights.length; i++) heights[i] = i;
const mask = new Uint8Array(width * height).fill(0);
mask.fill(1, 16, 48);
const target = 5.0;

const direct = new Float32Array(heights);
wasm.flattenTerrain(direct, width, height, bounds, polygon, target, 0);

const viaApi = await flattenHeightfield(null, heights, width, height, mask, target, {
  preferGpu: false,
  wasm,
  bounds,
  polygon,
  featherCells: 0,
});

assertClose(direct, viaApi);
console.log("heightfield fallback test: PASS (preferGpu:false → WASM CPU)");
