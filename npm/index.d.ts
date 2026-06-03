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
export { default as initWasm, version, batchWgs84ToGcj02, batchGcj02ToWgs84, batchWgs84ToBd09, batchBd09ToWgs84, batchGcj02ToBd09, batchBd09ToGcj02, batchWgs84ToMercator, batchMercatorToWgs84, batchWgs84ToCgcs2000, batchWgs84ToGcj02Mercator, batchWgs84ToBd09Mercator, wgs84ToUtm, utmToWgs84, batchWgs84ToUtm, batchUtmToWgs84, batchWgs84ToUtmInPlace, batchUtmToWgs84InPlace, batchWgs84ToGcj02InPlace, batchGcj02ToWgs84InPlace, batchWgs84ToBd09InPlace, batchBd09ToWgs84InPlace, batchGcj02ToBd09InPlace, batchBd09ToGcj02InPlace, batchWgs84ToMercatorInPlace, batchMercatorToWgs84InPlace, batchWgs84ToCgcs2000InPlace, batchWgs84ToGcj02MercatorInPlace, batchWgs84ToBd09MercatorInPlace, cgcs2000IsWgs84Compatible, geohashEncode, geohashDecode, geohashNeighbors, normalizeCoords, denormalizeCoords, parseGeoJsonCoords, countGeoJsonFeatures, parseGeoJsonProperties, parseGeoJsonFeatures, GeoJsonFeaturesResult, geoJsonFromCoords, geoJsonFeatureCollection, filterGeoJsonByProperty, filterGeoJsonByBBox, countGeoJsonByProperty, addProperty, renameProperty, removeProperty, parseGeoJsonStream, parseGeoJsonPerFeature, parseGeoJsonLazy, LazyGeoJsonIter, SpatialIndex, SpatialEdgeIndex, computeBounds, computeBoundsMulti, VectorTileEngine, VectorTileOptions, decodeMvt, decodeMvtToGeoJson, MvtLayer, MvtFeature, batchWgs84ToCartesian3, CesiumMeshGeometry, generateCesiumGeometry, Cesium3DTile, generate3DTile, parseLasHeader, LasHeader, parseLasHeaderOnly, LasHeaderInfo, parseLasPoints, LasPointCloud, parseLasPointsWithProgress, parseLasPointAt, PointData, parsePcdAscii, PcdPointCloud, parsePcdBinary, decimateVoxelGrid, decimateVoxelGridWithProgress, decimateRandom, generateInterleavedVertexBuffer, generateIndexedGeometry, colorizeByHeight, colorizeByIntensity, applyColorRamp, PointCloudStreamer, computeRegionByteRange, supportsLaz, lazStatus, parsePointCloudAuto, parseLazPoints, parseLazPointsStream, parseCopcHeader, readCopcChunk, readCopcRegion, estimateNormals, flipNormals, buildOctree, Octree, octreeMemoryUsage, encodePntsTile, generateTileset, TilesetResult, computeScreenSpaceError, getVisibleTiles, parseIfcGeometry, IfcGeometryResult, IfcMesh, GltfBuilder, pointCloudToGlb, terrainToGlb, meshToGlb, parseGeotiff, encodeQuantizedMesh, encodeTerrainTileset, ColorRamp, hillshade, contourLines, WorkerHandle, WorkerOptions, supportsWorker, processChunked, buildTin, tinInterpolate, haversineDistance, vincentyDistance, rhumbDistance, rhumbBearing, bearing, destination, midpoint, bufferPoint, bufferLineString, boundingBox, centroid, convexHull, concaveHull, clusterByDensity, clusterByGrid, crsInfo, getSupportedCrs, bestCrsForRegion, isInChina, polygonArea, areaWithHoles, polylineLength, simplifyDouglasPeucker, isPointInRing, polygonIntersection, polygonUnion, contains, touches, disjoint, polygonIntersects, validateCoords, ValidationResult, cleanCoords, deduplicateCoords, sortCoordsByLng, sortCoordsByLat, gridIndex, parseWkb, parseWkt, toWkb, toWkt, memoryInfo, MemoryInfo, setInputSizeLimit, getInputSizeLimit, getAllocatedBytes, } from "./wasm_spatial_core.js";
/** Supported coordinate reference systems. */
export type CRS = "WGS84" | "GCJ02" | "BD09" | "CGCS2000" | "EPSG:3857";
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
export type StreamChunkCallback = (coords: Float64Array, processed: number, total: number) => void;
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
export declare function loadSpatialCore(): Promise<any>;
/**
 * Draco compression utilities.
 *
 * @example
 * ```ts
 * import { loadSpatialCore, compressTilesetWithDraco } from "wasm-spatial-core";
 * import { createEncoderModule } from "draco3d";
 *
 * const wasm = await loadSpatialCore();
 * const encoderModule = await createEncoderModule({});
 * const tileset = wasm.generateTileset(positions, 50000, 21);
 * const results = compressTilesetWithDraco(tileset, encoderModule);
 * ```
 *
 * @module draco
 */
export { compressPntsTileWithDraco, compressTilesetWithDraco, buildDracoTileset, type DracoCompressOptions, type DracoTileResult, type TilesetLike, } from "./draco.js";
