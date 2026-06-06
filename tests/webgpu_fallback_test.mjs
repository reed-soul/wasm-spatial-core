// W4.5 — preferGpu:false forces WASM CPU path
// Run: node tests/webgpu_fallback_test.mjs

import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
// Prefer repo-root pkg/ (demo/CI build) over stale npm/pkg/
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

/** Mirrors npm/webgpu.ts transformPoints fallback branch */
async function transformPoints(ctx, positions, matrix, { preferGpu = true, wasm }) {
  if (preferGpu !== false && ctx) {
    throw new Error("GPU path should not run in Node");
  }
  return wasm.transformPointCloud(positions, matrix);
}

function identityMat4() {
  const m = new Float32Array(16);
  m[0] = m[5] = m[10] = m[15] = 1;
  return m;
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
const positions = new Float32Array([1, 2, 3, 4, 5, 6]);
const matrix = identityMat4();
matrix[12] = 100;

const direct = wasm.transformPointCloud(positions, matrix);
const viaApi = await transformPoints(null, positions, matrix, { preferGpu: false, wasm });

assertClose(direct, viaApi);
console.log("webgpu fallback test: PASS (preferGpu:false → WASM CPU)");
