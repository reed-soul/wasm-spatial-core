// Node.js batch API smoke test
// Run: node tests/node_batch_test.mjs

import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const pkgNode = existsSync(join(__dirname, "..", "npm", "pkg-node"))
  ? resolve(join(__dirname, "..", "npm", "pkg-node"))
  : resolve(join(__dirname, "..", "pkg-node"));

async function loadCore() {
  const wasmJs = pathToFileURL(join(pkgNode, "wasm_spatial_core.js")).href;
  const mod = await import(wasmJs);
  if (mod.default && typeof mod.default === "function") {
    await mod.default(readFileSync(join(pkgNode, "wasm_spatial_core_bg.wasm")));
  }
  return mod;
}

async function batchPointCloudToTileset(core, bytes, options = {}) {
  const cloud = core.parsePointCloudAuto(bytes);
  const positions = cloud.positions;
  const colors = cloud.colors;
  const maxPointsPerNode = options.maxPointsPerNode ?? 50_000;
  const maxDepth = options.maxDepth ?? 21;
  const estimatedSpacing = core.estimatePointSpacing(positions, 1000);
  const tileset = core.generateTileset(
    positions,
    maxPointsPerNode,
    maxDepth,
    colors ?? undefined,
  );
  const tiles = [];
  const tileUris = [];
  for (let i = 0; i < tileset.tileCount; i++) {
    tiles.push(tileset.tile(i));
    tileUris.push(tileset.tileUri(i));
  }
  return {
    pointCount: cloud.pointCount,
    tilesetJson: tileset.tilesetJson(),
    tileCount: tileset.tileCount,
    estimatedSpacing,
    tiles,
    tileUris,
  };
}

async function main() {
  const core = await loadCore();
  console.log(`  ℹ️  Node core version: ${core.version()}`);

  // Minimal LAS 1.2 file with 25 points (5×5 grid)
  function buildMinimalLas(points) {
    const headerSize = 230;
    const recordLen = 20;
    const buf = new Uint8Array(headerSize + points.length * recordLen);
    const view = new DataView(buf.buffer);
    buf.set([0x4c, 0x41, 0x53, 0x46], 0); // LASF
    view.setUint32(96, headerSize, true);
    // ASPRS LAS spec: number of VLRs at offset 100 (u32, here 0);
    // point count lives at offset 107 (u32).
    view.setUint32(100, 0, true); // number of VLRs
    buf[104] = 0; // format 0
    view.setUint16(105, recordLen, true);
    view.setUint32(107, points.length, true); // num points (ASPRS offset 107)
    view.setFloat64(131, 1.0, true);
    view.setFloat64(139, 1.0, true);
    view.setFloat64(147, 1.0, true);
    let minX = Infinity, minY = Infinity, minZ = Infinity;
    let maxX = -Infinity, maxY = -Infinity, maxZ = -Infinity;
    for (const [x, y, z] of points) {
      minX = Math.min(minX, x); minY = Math.min(minY, y); minZ = Math.min(minZ, z);
      maxX = Math.max(maxX, x); maxY = Math.max(maxY, y); maxZ = Math.max(maxZ, z);
    }
    view.setFloat64(179, maxX, true);
    view.setFloat64(187, maxY, true);
    view.setFloat64(195, maxZ, true);
    view.setFloat64(203, minX, true);
    view.setFloat64(211, minY, true);
    view.setFloat64(219, minZ, true);
    points.forEach(([x, y, z], i) => {
      const base = headerSize + i * recordLen;
      view.setInt32(base, Math.round(x), true);
      view.setInt32(base + 4, Math.round(y), true);
      view.setInt32(base + 8, Math.round(z), true);
    });
    return buf;
  }

  const points = Array.from({ length: 25 }, (_, i) => [
    (i % 5) * 2,
    Math.floor(i / 5) * 2,
    0,
  ]);
  const las = buildMinimalLas(points);

  const result = await batchPointCloudToTileset(core, las, {
    maxPointsPerNode: 10,
    maxDepth: 4,
  });

  if (result.pointCount !== 25) {
    throw new Error(`Expected 25 points, got ${result.pointCount}`);
  }
  if (!result.tilesetJson.includes("geometricError")) {
    throw new Error("tilesetJson missing geometricError");
  }
  if (result.tileCount < 1) {
    throw new Error("Expected at least one tile");
  }
  if (!(result.estimatedSpacing > 0)) {
    throw new Error(`Expected positive estimated spacing, got ${result.estimatedSpacing}`);
  }

  console.log(`  ✅ batchPointCloudToTileset: ${result.pointCount} pts → ${result.tileCount} tiles`);
  console.log(`  ✅ estimated spacing: ${result.estimatedSpacing.toFixed(3)}`);
}

main().catch((err) => {
  console.error("💥 Node batch test failed:", err);
  process.exit(1);
});
