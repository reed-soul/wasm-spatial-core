/**
 * wasm-spatial-core — TypeScript convenience wrapper
 *
 * Re-exports the auto-generated wasm-bindgen bindings with a
 * higher-level initialisation helper and typed interfaces.
 *
 * @packageDocumentation
 * @author  Qingxi
 * @license MIT
 * @copyright 2026 智启未来 (Zhiqi Weilai)
 */

// Re-export everything from the auto-generated wasm-bindgen module
export {
  default as initWasm,
  version,
  // ── Copy-based API ──────────────────────────────────────────
  batchWgs84ToGcj02,
  batchGcj02ToWgs84,
  batchWgs84ToBd09,
  batchBd09ToWgs84,
  batchGcj02ToBd09,
  batchBd09ToGcj02,
  batchWgs84ToMercator,
  batchMercatorToWgs84,
  batchWgs84ToCgcs2000,
  // ── Pipeline Transforms (combined conversions) ────────────
  batchWgs84ToGcj02Mercator,
  batchWgs84ToBd09Mercator,
  // ── UTM Projection ───────────────────────────────────────
  wgs84ToUtm,
  utmToWgs84,
  batchWgs84ToUtm,
  batchUtmToWgs84,
  batchWgs84ToUtmInPlace,
  batchUtmToWgs84InPlace,
  // ── Zero-copy in-place API ─────────────────────────────────
  batchWgs84ToGcj02InPlace,
  batchGcj02ToWgs84InPlace,
  batchWgs84ToBd09InPlace,
  batchBd09ToWgs84InPlace,
  batchGcj02ToBd09InPlace,
  batchBd09ToGcj02InPlace,
  batchWgs84ToMercatorInPlace,
  batchMercatorToWgs84InPlace,
  batchWgs84ToCgcs2000InPlace,
  batchWgs84ToGcj02MercatorInPlace,
  batchWgs84ToBd09MercatorInPlace,
  // ── Utilities ──────────────────────────────────────────────
  cgcs2000IsWgs84Compatible,
  // ── Geohash ───────────────────────────────────────────────
  geohashEncode,
  geohashDecode,
  geohashNeighbors,
  // ── Coordinate Normalization ─────────────────────────────
  normalizeCoords,
  denormalizeCoords,
  // ── GeoJSON ────────────────────────────────────────────────
  parseGeoJsonCoords,
  countGeoJsonFeatures,
  parseGeoJsonProperties,
  parseGeoJsonFeatures,
  GeoJsonFeaturesResult,
  geoJsonFromCoords,
  geoJsonFeatureCollection,
  filterGeoJsonByProperty,
  filterGeoJsonByBBox,
  countGeoJsonByProperty,
  // ── GeoJSON Editing ──────────────────────────────────────
  addProperty,
  renameProperty,
  removeProperty,
  // ── GeoJSON Streaming ──────────────────────────────────────
  parseGeoJsonStream,
  parseGeoJsonPerFeature,
  // ── Lazy GeoJSON (O(single feature) memory) ──────────────
  parseGeoJsonLazy,
  LazyGeoJsonIter,
  // ── Spatial Indexing ───────────────────────────────────────
  SpatialIndex,
  SpatialEdgeIndex,
  computeBounds,
  computeBoundsMulti,
  // ── Vector Tile Slicing ────────────────────────────────────
  VectorTileEngine,
  VectorTileOptions,
  // ── MVT Decoding ──────────────────────────────────────────
  decodeMvt,
  decodeMvtToGeoJson,
  MvtLayer,
  MvtFeature,
  // ── Cesium Native Adapter ──────────────────────────────────
  batchWgs84ToCartesian3,
  CesiumMeshGeometry,
  generateCesiumGeometry,
  Cesium3DTile,
  generate3DTile,
  // ── Point Cloud (requires `point-cloud` feature) ──────────
  parseLasHeader,
  LasHeader,
  parseLasHeaderOnly,
  LasHeaderInfo,
  parseLasPoints,
  LasPointCloud,
  parseLasPointsWithProgress,
  parseLasPointAt,
  PointData,
  parsePcdAscii,
  PcdPointCloud,
  parsePcdBinary,
  decimateVoxelGrid,
  decimateVoxelGridWithProgress,
  decimateRandom,
  generateInterleavedVertexBuffer,
  generateIndexedGeometry,
  colorizeByHeight,
  colorizeByIntensity,
  applyColorRamp,
  // ── Point Cloud Streaming ──────────────────────────────────
  PointCloudStreamer,
  computeRegionByteRange,
  supportsLaz,
  lazStatus,
  parsePointCloudAuto,
  parseLazPoints,
  parseLazPointsStream,
  parseCopcHeader,
  readCopcChunk,
  readCopcRegion,
  // ── Point Cloud Normals ──────────────────────────────────
  estimateNormals,
  flipNormals,
  // ── Octree ────────────────────────────────────────────────
  buildOctree,
  Octree,
  octreeMemoryUsage,
  // ── 3D Tiles Point Cloud (pnts) ──────────────────────────
  encodePntsTile,
  generateTileset,
  TilesetResult,
  // ── LOD ───────────────────────────────────────────────────
  computeScreenSpaceError,
  getVisibleTiles,
  // ── IFC/BIM (Experimental) ────────────────────────────────
  parseIfcGeometry,
  IfcGeometryResult,
  IfcMesh,
  // ── glTF / GLB Writer ──────────────────────────────────────
  GltfBuilder,
  pointCloudToGlb,
  terrainToGlb,
  meshToGlb,
  // ── GeoTIFF / Terrain ──────────────────────────────────────
  parseGeotiff,
  encodeQuantizedMesh,
  encodeTerrainTileset,
  ColorRamp,
  hillshade,
  contourLines,
  // ── Web Worker ──────────────────────────────────────────────
  WorkerHandle,
  WorkerOptions,
  supportsWorker,
  processChunked,
  // ── TIN & Interpolation ───────────────────────────────────
  buildTin,
  tinInterpolate,
  // ── Spatial Analysis ───────────────────────────────────────
  haversineDistance,
  vincentyDistance,
  rhumbDistance,
  rhumbBearing,
  bearing,
  destination,
  midpoint,
  bufferPoint,
  bufferLineString,
  boundingBox,
  centroid,
  convexHull,
  concaveHull,
  // ── Spatial Clustering ────────────────────────────────────
  clusterByDensity,
  clusterByGrid,
  // ── CRS Utilities ────────────────────────────────────────
  crsInfo,
  getSupportedCrs,
  bestCrsForRegion,
  isInChina,
  // ── Topology ──────────────────────────────────────────────
  polygonArea,
  areaWithHoles,
  polylineLength,
  simplifyDouglasPeucker,
  isPointInRing,
  polygonIntersection,
  polygonUnion,
  // ── Spatial Predicates ──────────────────────────────────────
  contains,
  touches,
  disjoint,
  polygonIntersects,
  // ── Coordinate Quality ────────────────────────────────────
  validateCoords,
  ValidationResult,
  cleanCoords,
  deduplicateCoords,
  // ── Coordinate Sorting & Gridding ─────────────────────────
  sortCoordsByLng,
  sortCoordsByLat,
  gridIndex,
  // ── WKB / WKT ────────────────────────────────────────────
  parseWkb,
  parseWkt,
  toWkb,
  toWkt,
  // ── Memory Management ──────────────────────────────────────
  memoryInfo,
  MemoryInfo,
  setInputSizeLimit,
  getInputSizeLimit,
  getAllocatedBytes,
} from "./wasm_spatial_core.js";

// ---------------------------------------------------------------------------
// Convenience types
// ---------------------------------------------------------------------------

/** Supported coordinate reference systems. */
export type CRS =
  | "WGS84"       // EPSG:4326
  | "GCJ02"       // China encrypted (Gaode / Amap)
  | "BD09"        // Baidu
  | "CGCS2000"    // China Geodetic Coordinate System 2000
  | "EPSG:3857";  // Web Mercator

/** Options for batch coordinate conversion. */
export interface ConvertOptions {
  /** Source CRS — defaults to `"WGS84"`. */
  from?: CRS;
  /** Target CRS — defaults to `"GCJ02"`. */
  to?: CRS;
  /**
   * If `true`, use the zero-copy in-place API.
   * The input buffer will be mutated directly.
   * @default false
   */
  inPlace?: boolean;
}

/**
 * Callback for the streaming GeoJSON parser.
 *
 * @param coords   — Flat `Float64Array` with coordinate pairs for this chunk.
 * @param processed — Number of features processed so far.
 * @param total     — Total number of features.
 *
 * ```ts
 * const onChunk: StreamChunkCallback = (coords, processed, total) => {
 *   progressBar.value = processed / total;
 *   gl.bufferSubData(gl.ARRAY_BUFFER, offset, coords);
 * };
 * ```
 */
export type StreamChunkCallback = (
  coords: Float64Array,
  processed: number,
  total: number,
) => void;

/**
 * High-level helper: initialise the WASM module and return the public API.
 *
 * ```ts
 * import { loadSpatialCore } from "wasm-spatial-core";
 *
 * const core = await loadSpatialCore();
 * console.log(core.version());
 * ```
 */
export async function loadSpatialCore() {
  const { default: init, ...api } = await import("./wasm_spatial_core.js");
  await init();
  return api;
}
