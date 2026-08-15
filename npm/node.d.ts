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
export { batchPointCloudToTileset, batchGeotiffToTerrain, type PointCloudBatchOptions, type PointCloudBatchResult, type GeotiffBatchOptions, type GeotiffBatchResult, } from "./batch.js";
export { version, parsePointCloudAuto, buildOctree, generateTileset, generateTilesetWithSpacing, estimatePointSpacing, parseGeotiff, encodeTerrainTileset, encodeQuantizedMesh, parseGeoJsonCoords, batchWgs84ToGcj02, batchWgs84ToMercator, TilesetResult, LasPointCloud, GeotiffInfo, } from "./pkg-node/wasm_spatial_core.js";
type PkgNodeModule = typeof import("./pkg-node/wasm_spatial_core.js");
/** The full wasm-bindgen Node.js API (everything except the init entry). */
export type SpatialCoreNodeApi = Omit<PkgNodeModule, "default">;
/**
 * Initialise and return the wasm-spatial-core API for Node.js.
 *
 * Safe to call multiple times — subsequent calls return the same instance.
 */
export declare function loadSpatialCoreNode(): Promise<SpatialCoreNodeApi>;
