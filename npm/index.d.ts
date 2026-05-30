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
export { default as initWasm, version, batchWgs84ToGcj02, batchGcj02ToWgs84, batchWgs84ToBd09, batchBd09ToWgs84, batchGcj02ToBd09, batchBd09ToGcj02, batchWgs84ToMercator, batchMercatorToWgs84, batchWgs84ToCgcs2000, batchWgs84ToGcj02Mercator, batchWgs84ToBd09Mercator, batchWgs84ToGcj02InPlace, batchGcj02ToWgs84InPlace, batchWgs84ToBd09InPlace, batchBd09ToWgs84InPlace, batchGcj02ToBd09InPlace, batchBd09ToGcj02InPlace, batchWgs84ToMercatorInPlace, batchMercatorToWgs84InPlace, batchWgs84ToCgcs2000InPlace, batchWgs84ToGcj02MercatorInPlace, batchWgs84ToBd09MercatorInPlace, cgcs2000IsWgs84Compatible, geohashEncode, geohashDecode, geohashNeighbors, normalizeCoords, denormalizeCoords, parseGeoJsonCoords, countGeoJsonFeatures, parseGeoJsonProperties, parseGeoJsonFeatures, GeoJsonFeaturesResult, geoJsonFromCoords, geoJsonFeatureCollection, filterGeoJsonByProperty, filterGeoJsonByBBox, countGeoJsonByProperty, parseGeoJsonStream, parseGeoJsonPerFeature, parseGeoJsonLazy, LazyGeoJsonIter, SpatialIndex, SpatialEdgeIndex, computeBounds, computeBoundsMulti, VectorTileEngine, VectorTileOptions, decodeMvt, decodeMvtToGeoJson, MvtLayer, MvtFeature, batchWgs84ToCartesian3, CesiumMeshGeometry, generateCesiumGeometry, Cesium3DTile, generate3DTile, parseLasHeader, parseLasPoints, parseLasPointsWithProgress, parseLasHeaderOnly, computeLasPointOffset, parseLasPointAt, decimateVoxelGrid, decimateVoxelGridWithProgress, decimateRandom, parsePcdAscii, parsePcdBinary, generateInterleavedVertexBuffer, generateIndexedGeometry, colorizeByHeight, colorizeByIntensity, applyColorRamp, parseIfcGeometry, IfcGeometryResult, IfcMesh, GltfBuilder, buildTin, tinInterpolate, haversineDistance, bearing, destination, midpoint, bufferPoint, bufferLineString, boundingBox, centroid, polygonArea, areaWithHoles, polylineLength, simplifyDouglasPeucker, isPointInRing, polygonIntersection, polygonUnion, contains, touches, disjoint, polygonIntersects, validateCoords, ValidationResult, cleanCoords, deduplicateCoords, sortCoordsByLng, sortCoordsByLat, gridIndex, memoryInfo, MemoryInfo, setInputSizeLimit, getInputSizeLimit, getAllocatedBytes, } from "./wasm_spatial_core.js";
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
export declare function loadSpatialCore(): Promise<{
    applyColorRamp(positions: Float32Array, colors: Float32Array): Float32Array;
    areaWithHoles(rings: Float64Array, ring_sizes: Uint32Array): number;
    batchBd09ToGcj02(coords: Float64Array): Float64Array;
    batchBd09ToGcj02InPlace(coords: Float64Array): void;
    batchBd09ToWgs84(coords: Float64Array): Float64Array;
    batchBd09ToWgs84InPlace(coords: Float64Array): void;
    batchGcj02ToBd09(coords: Float64Array): Float64Array;
    batchGcj02ToBd09InPlace(coords: Float64Array): void;
    batchGcj02ToWgs84(coords: Float64Array): Float64Array;
    batchGcj02ToWgs84InPlace(coords: Float64Array): void;
    batchMercatorToWgs84(coords: Float64Array): Float64Array;
    batchMercatorToWgs84InPlace(coords: Float64Array): void;
    batchWgs84ToBd09(coords: Float64Array): Float64Array;
    batchWgs84ToBd09InPlace(coords: Float64Array): void;
    batchWgs84ToBd09Mercator(coords: Float64Array): Float64Array;
    batchWgs84ToBd09MercatorInPlace(coords: Float64Array): void;
    batchWgs84ToCartesian3(coords: Float64Array): Float64Array;
    batchWgs84ToCgcs2000(coords: Float64Array): Float64Array;
    batchWgs84ToCgcs2000InPlace(_coords: Float64Array): void;
    batchWgs84ToGcj02(coords: Float64Array): Float64Array;
    batchWgs84ToGcj02InPlace(coords: Float64Array): void;
    batchWgs84ToGcj02Mercator(coords: Float64Array): Float64Array;
    batchWgs84ToGcj02MercatorInPlace(coords: Float64Array): void;
    batchWgs84ToMercator(coords: Float64Array): Float64Array;
    batchWgs84ToMercatorInPlace(coords: Float64Array): void;
    bearing(lng1: number, lat1: number, lng2: number, lat2: number): number;
    boundingBox(coords: Float64Array): Float64Array;
    bufferLineString(coords: Float64Array, radius_meters: number, segments?: number | null): Float64Array;
    bufferPoint(lng: number, lat: number, radius_meters: number, segments?: number | null): Float64Array;
    buildTin(points: Float64Array): import("./wasm_spatial_core.js").TinResult;
    centroid(coords: Float64Array): Float64Array;
    cgcs2000IsWgs84Compatible(): boolean;
    cleanCoords(coords: Float64Array, strategy: string): Float64Array;
    colorizeByHeight(positions: Float32Array, min_z: number, max_z: number, low_color: Float32Array, high_color: Float32Array): Float32Array;
    colorizeByIntensity(positions: Float32Array, intensities: Float32Array): Float32Array;
    computeBounds(coords: Float64Array): Float64Array;
    computeBoundsMulti(buffers: Array<any>): Float64Array;
    computeLasPointOffset(header_info: import("./wasm_spatial_core.js").LasHeaderInfo, point_index: number, _point_format: number): number;
    contains(outer_ring: Float64Array, point_x: number, point_y: number): boolean;
    countGeoJsonByProperty(input: string, key: string): string;
    countGeoJsonFeatures(input: string): number;
    decimateRandom(positions: Float32Array, colors: Uint8Array, target_count: number): object;
    decimateVoxelGrid(positions: Float32Array, colors: Uint8Array, cell_size: number): object;
    decimateVoxelGridWithProgress(positions: Float32Array, colors: Uint8Array, cell_size: number, on_progress: Function): object;
    decodeMvt(bytes: Uint8Array): import("./wasm_spatial_core.js").MvtLayer;
    decodeMvtToGeoJson(bytes: Uint8Array): string;
    deduplicateCoords(coords: Float64Array, tolerance: number): Float64Array;
    denormalizeCoords(normals: Float64Array, source_bounds: Float64Array): Float64Array;
    destination(lng: number, lat: number, bearing_deg: number, distance_m: number): Float64Array;
    disjoint(ring1: Float64Array, ring2: Float64Array): boolean;
    filterGeoJsonByBBox(input: string, min_lng: number, min_lat: number, max_lng: number, max_lat: number): string;
    filterGeoJsonByProperty(input: string, key: string, value: string): string;
    generate3DTile(geojson_str: string, height_property?: string | null): import("./wasm_spatial_core.js").Cesium3DTile;
    generateCesiumGeometry(geojson_str: string, height_property?: string | null): import("./wasm_spatial_core.js").CesiumMeshGeometry;
    generateIndexedGeometry(positions: Float32Array): object;
    generateInterleavedVertexBuffer(positions: Float32Array, colors: Uint8Array, normals: Float32Array): Float32Array;
    geoJsonFeatureCollection(coords: Float64Array, types: string, properties_json: string): string;
    geoJsonFromCoords(coords: Float64Array, geometry_type: string): string;
    geohashDecode(hash: string): Float64Array;
    geohashEncode(lng: number, lat: number, precision: number): string;
    geohashNeighbors(hash: string): Array<any>;
    getAllocatedBytes(): number;
    getInputSizeLimit(): number;
    gridIndex(coords: Float64Array, cell_size_deg: number): Float64Array;
    haversineDistance(lng1: number, lat1: number, lng2: number, lat2: number): number;
    init(): void;
    isPointInRing(point_x: number, point_y: number, ring_coords: Float64Array): boolean;
    memoryInfo(): import("./wasm_spatial_core.js").MemoryInfo;
    midpoint(lng1: number, lat1: number, lng2: number, lat2: number): Float64Array;
    normalizeCoords(coords: Float64Array, target_bounds: Float64Array): Float64Array;
    parseGeoJsonCoords(input: string): Float64Array;
    parseGeoJsonFeatures(input: string): import("./wasm_spatial_core.js").GeoJsonFeaturesResult;
    parseGeoJsonLazy(input: string): import("./wasm_spatial_core.js").LazyGeoJsonIter;
    parseGeoJsonPerFeature(input: string): Array<any>;
    parseGeoJsonProperties(input: string): string;
    parseGeoJsonStream(input: string, chunk_size: number, on_chunk: Function): number;
    parseIfcGeometry(text: string): import("./wasm_spatial_core.js").IfcGeometryResult;
    parseLasHeader(bytes: Uint8Array): import("./wasm_spatial_core.js").LasHeader;
    parseLasHeaderOnly(bytes: Uint8Array): import("./wasm_spatial_core.js").LasHeaderInfo;
    parseLasPointAt(bytes: Uint8Array, offset: number, point_format: number): import("./wasm_spatial_core.js").PointData;
    parseLasPoints(bytes: Uint8Array): import("./wasm_spatial_core.js").LasPointCloud;
    parseLasPointsWithProgress(bytes: Uint8Array, on_progress: Function): import("./wasm_spatial_core.js").LasPointCloud;
    parsePcdAscii(text: string): import("./wasm_spatial_core.js").PcdPointCloud;
    parsePcdBinary(bytes: Uint8Array): import("./wasm_spatial_core.js").PcdPointCloud;
    polygonArea(coords: Float64Array): number;
    polygonIntersection(ring1: Float64Array, ring2: Float64Array): Float64Array;
    polygonIntersects(ring1: Float64Array, ring2: Float64Array): boolean;
    polygonUnion(ring1: Float64Array, ring2: Float64Array): Float64Array;
    polylineLength(coords: Float64Array): number;
    setInputSizeLimit(bytes: number): void;
    simplifyDouglasPeucker(coords: Float64Array, tolerance: number): Float64Array;
    sortCoordsByLat(coords: Float64Array): Float64Array;
    sortCoordsByLng(coords: Float64Array): Float64Array;
    tinInterpolate(tin: import("./wasm_spatial_core.js").TinResult, x: number, y: number): number;
    touches(ring1: Float64Array, ring2: Float64Array): boolean;
    validateCoords(coords: Float64Array, crs: string): import("./wasm_spatial_core.js").ValidationResult;
    version(): string;
    initSync(module: {
        module: import("./wasm_spatial_core.js").SyncInitInput;
    } | import("./wasm_spatial_core.js").SyncInitInput): import("./wasm_spatial_core.js").InitOutput;
    Cesium3DTile: typeof import("./wasm_spatial_core.js").Cesium3DTile;
    CesiumMeshGeometry: typeof import("./wasm_spatial_core.js").CesiumMeshGeometry;
    GeoJsonFeaturesResult: typeof import("./wasm_spatial_core.js").GeoJsonFeaturesResult;
    GltfBuilder: typeof import("./wasm_spatial_core.js").GltfBuilder;
    IfcGeometryResult: typeof import("./wasm_spatial_core.js").IfcGeometryResult;
    IfcMesh: typeof import("./wasm_spatial_core.js").IfcMesh;
    LasHeader: typeof import("./wasm_spatial_core.js").LasHeader;
    LasHeaderInfo: typeof import("./wasm_spatial_core.js").LasHeaderInfo;
    LasPointCloud: typeof import("./wasm_spatial_core.js").LasPointCloud;
    LazyGeoJsonIter: typeof import("./wasm_spatial_core.js").LazyGeoJsonIter;
    MemoryInfo: typeof import("./wasm_spatial_core.js").MemoryInfo;
    MvtFeature: typeof import("./wasm_spatial_core.js").MvtFeature;
    MvtLayer: typeof import("./wasm_spatial_core.js").MvtLayer;
    PcdPointCloud: typeof import("./wasm_spatial_core.js").PcdPointCloud;
    PointData: typeof import("./wasm_spatial_core.js").PointData;
    SpatialEdgeIndex: typeof import("./wasm_spatial_core.js").SpatialEdgeIndex;
    SpatialIndex: typeof import("./wasm_spatial_core.js").SpatialIndex;
    TinResult: typeof import("./wasm_spatial_core.js").TinResult;
    ValidationResult: typeof import("./wasm_spatial_core.js").ValidationResult;
    VectorTileEngine: typeof import("./wasm_spatial_core.js").VectorTileEngine;
    VectorTileOptions: typeof import("./wasm_spatial_core.js").VectorTileOptions;
}>;
