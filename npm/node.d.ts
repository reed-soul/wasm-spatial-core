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
export { batchPointCloudToTileset, batchGeotiffToTerrain, type PointCloudBatchOptions, type PointCloudBatchResult, type GeotiffBatchOptions, type GeotiffBatchResult, } from "./batch.js";
type PkgNodeModule = typeof import("./pkg-node/wasm_spatial_core.js");
/** The full wasm-bindgen Node.js API (everything except the init entry). */
export type SpatialCoreNodeApi = Omit<PkgNodeModule, "default">;
export declare const version: typeof import("./pkg-node/wasm_spatial_core.js").version;
export declare const parsePointCloudAuto: typeof import("./pkg-node/wasm_spatial_core.js").parsePointCloudAuto;
export declare const buildOctree: typeof import("./pkg-node/wasm_spatial_core.js").buildOctree;
export declare const generateTileset: typeof import("./pkg-node/wasm_spatial_core.js").generateTileset;
export declare const generateTilesetWithSpacing: typeof import("./pkg-node/wasm_spatial_core.js").generateTilesetWithSpacing;
export declare const estimatePointSpacing: typeof import("./pkg-node/wasm_spatial_core.js").estimatePointSpacing;
export declare const parseGeotiff: typeof import("./pkg-node/wasm_spatial_core.js").parseGeotiff;
export declare const encodeTerrainTileset: typeof import("./pkg-node/wasm_spatial_core.js").encodeTerrainTileset;
export declare const encodeQuantizedMesh: typeof import("./pkg-node/wasm_spatial_core.js").encodeQuantizedMesh;
export declare const parseGeoJsonCoords: typeof import("./pkg-node/wasm_spatial_core.js").parseGeoJsonCoords;
export declare const batchWgs84ToGcj02: typeof import("./pkg-node/wasm_spatial_core.js").batchWgs84ToGcj02;
export declare const batchWgs84ToMercator: typeof import("./pkg-node/wasm_spatial_core.js").batchWgs84ToMercator;
export declare const TilesetResult: typeof import("./pkg-node/wasm_spatial_core.js").TilesetResult;
export declare const LasPointCloud: typeof import("./pkg-node/wasm_spatial_core.js").LasPointCloud;
export declare const GeotiffInfo: typeof import("./pkg-node/wasm_spatial_core.js").GeotiffInfo;
/**
 * Initialise and return the wasm-spatial-core API for Node.js.
 *
 * Safe to call multiple times — subsequent calls return the same instance.
 */
export declare function loadSpatialCoreNode(): Promise<SpatialCoreNodeApi>;
