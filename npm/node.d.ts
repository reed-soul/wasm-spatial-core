/**
 * Node.js type declarations for wasm-spatial-core.
 */

export {
  batchPointCloudToTileset,
  batchGeotiffToTerrain,
  type PointCloudBatchOptions,
  type PointCloudBatchResult,
  type GeotiffBatchOptions,
  type GeotiffBatchResult,
} from "./batch";

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
} from "./pkg-node/wasm_spatial_core";

export declare function loadSpatialCoreNode(): Promise<Record<string, unknown>>;
