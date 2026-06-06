/**
 * Node.js entry point for wasm-spatial-core.
 *
 * Uses the wasm-pack `nodejs` target for server-side batch processing without
 * a browser or COOP/COEP headers. The WASM module runs at near-native speed
 * while remaining portable across Linux, macOS, and Windows.
 *
 * @example
 * ```ts
 * import { loadSpatialCoreNode, batchPointCloudToTileset } from "wasm-spatial-core/node";
 * import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
 * import { join } from "node:path";
 *
 * const core = await loadSpatialCoreNode();
 * const las = readFileSync("scan.laz");
 * const result = await batchPointCloudToTileset(core, las);
 *
 * mkdirSync("output/tiles", { recursive: true });
 * writeFileSync("output/tileset.json", result.tilesetJson);
 * result.tiles.forEach((data, i) => {
 *   writeFileSync(join("output/tiles", result.tileUris[i]), data);
 * });
 * ```
 *
 * @packageDocumentation
 */

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export {
  batchPointCloudToTileset,
  batchGeotiffToTerrain,
  type PointCloudBatchOptions,
  type PointCloudBatchResult,
  type GeotiffBatchOptions,
  type GeotiffBatchResult,
} from "./batch.js";

// Re-export commonly used APIs from the Node.js WASM bindings.
export {
  version,
  parsePointCloudAuto,
  buildOctree,
  generateTileset,
  generateTilesetWithSpacing,
  estimatePointSpacing,
  parseGeotiff,
  encodeTerrainTileset,
  encodeQuantizedMesh,
  parseGeoJsonCoords,
  batchWgs84ToGcj02,
  batchWgs84ToMercator,
  TilesetResult,
  LasPointCloud,
  GeotiffInfo,
} from "./pkg-node/wasm_spatial_core.js";

const __dirname = dirname(fileURLToPath(import.meta.url));

let corePromise: Promise<Record<string, unknown>> | null = null;

/**
 * Initialise and return the wasm-spatial-core API for Node.js.
 *
 * Safe to call multiple times — subsequent calls return the same instance.
 */
export async function loadSpatialCoreNode(): Promise<Record<string, unknown>> {
  if (!corePromise) {
    corePromise = (async () => {
      const mod = await import("./pkg-node/wasm_spatial_core.js");
      if (typeof mod.default === "function") {
        const wasmPath = join(__dirname, "pkg-node/wasm_spatial_core_bg.wasm");
        await mod.default(readFileSync(wasmPath));
      }
      const { default: _init, ...api } = mod;
      return api;
    })();
  }
  return corePromise;
}
