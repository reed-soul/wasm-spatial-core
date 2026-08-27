/**
 * Node.js entry point for wasm-spatial-core.
 *
 * Uses the wasm-pack `nodejs` target for server-side batch processing without
 * a browser or COOP/COEP headers. The WASM module runs at near-native speed
 * while remaining portable across Linux, macOS, and Windows.
 *
 * The nodejs-target glue is CommonJS with top-level side effects, which
 * Node's ESM named-export detection (cjs-module-lexer) cannot analyse —
 * so this entry loads it via `createRequire` instead of ESM re-exports.
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
import { createRequire } from "node:module";
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

const __dirname = dirname(fileURLToPath(import.meta.url));
const nodeRequire = createRequire(import.meta.url);

// Loaded through createRequire: ESM `export { x } from <cjs>` relies on
// cjs-module-lexer, which fails on the nodejs-target glue.
type PkgNodeModule = typeof import("./pkg-node/wasm_spatial_core.js");

const bindings = nodeRequire("./pkg-node/wasm_spatial_core.js") as PkgNodeModule;

/** The full wasm-bindgen Node.js API (everything except the init entry). */
export type SpatialCoreNodeApi = Omit<PkgNodeModule, "default">;

// Re-export commonly used APIs from the Node.js WASM bindings as real ESM
// bindings (values captured from the CJS module object).
export const version = bindings.version;
export const parsePointCloudAuto = bindings.parsePointCloudAuto;
export const buildOctree = bindings.buildOctree;
export const generateTileset = bindings.generateTileset;
export const generateTilesetWithSpacing = bindings.generateTilesetWithSpacing;
export const estimatePointSpacing = bindings.estimatePointSpacing;
export const parseGeotiff = bindings.parseGeotiff;
export const encodeTerrainTileset = bindings.encodeTerrainTileset;
export const encodeQuantizedMesh = bindings.encodeQuantizedMesh;
export const parseGeoJsonCoords = bindings.parseGeoJsonCoords;
export const batchWgs84ToGcj02 = bindings.batchWgs84ToGcj02;
export const batchWgs84ToMercator = bindings.batchWgs84ToMercator;
export const TilesetResult = bindings.TilesetResult;
export const LasPointCloud = bindings.LasPointCloud;
export const GeotiffInfo = bindings.GeotiffInfo;

let corePromise: Promise<SpatialCoreNodeApi> | null = null;

/**
 * Initialise and return the wasm-spatial-core API for Node.js.
 *
 * Safe to call multiple times — subsequent calls return the same instance.
 */
export async function loadSpatialCoreNode(): Promise<SpatialCoreNodeApi> {
  if (!corePromise) {
    corePromise = Promise.resolve(bindings as SpatialCoreNodeApi);
  }
  return corePromise;
}
