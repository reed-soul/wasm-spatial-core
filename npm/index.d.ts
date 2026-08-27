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
export { default as initWasm, version, batchWgs84ToGcj02, batchGcj02ToWgs84, batchWgs84ToBd09, batchBd09ToWgs84, batchGcj02ToBd09, batchBd09ToGcj02, batchWgs84ToMercator, batchMercatorToWgs84, batchWgs84ToCgcs2000, batchWgs84ToGcj02Mercator, batchWgs84ToBd09Mercator, wgs84ToUtm, utmToWgs84, batchWgs84ToUtm, batchUtmToWgs84, batchWgs84ToUtmInPlace, batchUtmToWgs84InPlace, batchWgs84ToGcj02InPlace, batchGcj02ToWgs84InPlace, batchWgs84ToBd09InPlace, batchBd09ToWgs84InPlace, batchGcj02ToBd09InPlace, batchBd09ToGcj02InPlace, batchWgs84ToMercatorInPlace, batchMercatorToWgs84InPlace, batchWgs84ToCgcs2000InPlace, batchWgs84ToGcj02MercatorInPlace, batchWgs84ToBd09MercatorInPlace, cgcs2000IsWgs84Compatible, geohashEncode, geohashDecode, geohashNeighbors, normalizeCoords, denormalizeCoords, parseGeoJsonCoords, countGeoJsonFeatures, parseGeoJsonProperties, parseGeoJsonFeatures, GeoJsonFeaturesResult, geoJsonFromCoords, geoJsonFeatureCollection, filterGeoJsonByProperty, filterGeoJsonByBBox, countGeoJsonByProperty, addProperty, renameProperty, removeProperty, parseGeoJsonStream, parseGeoJsonPerFeature, parseGeoJsonLazy, LazyGeoJsonIter, SpatialIndex, SpatialEdgeIndex, computeBounds, computeBoundsMulti, VectorTileEngine, VectorTileOptions, decodeMvt, decodeMvtToGeoJson, MvtLayer, MvtFeature, batchWgs84ToCartesian3, CesiumMeshGeometry, generateCesiumGeometry, Cesium3DTile, generate3DTile, parseLasHeader, LasHeader, parseLasHeaderOnly, LasHeaderInfo, parseLasPoints, LasPointCloud, parseLasPointsWithProgress, parseLasPointAt, PointData, parsePcdAscii, PcdPointCloud, parsePcdBinary, decimateVoxelGrid, decimateVoxelGridWithProgress, decimateRandom, generateInterleavedVertexBuffer, generateIndexedGeometry, colorizeByHeight, colorizeByIntensity, applyColorRamp, PointCloudStreamer, computeRegionByteRange, supportsLaz, lazStatus, parsePointCloudAuto, parseLazPoints, parseLazPointsStream, parseCopcHeader, readCopcChunk, readCopcChunkStandalone, readCopcRegion, copcQueryRanges, copcEstimateDownloadSize, estimateNormals, flipNormals, buildOctree, Octree, OctreeChunkBuilder, octreeMemoryUsage, encodePntsTile, generateTileset, generateTilesetWithSpacing, estimatePointSpacing, generateTilesetIncremental, TilesetResult, pointCloudChunkFromBuffers, PointCloudChunk, exportPointCloudToPnts, parseGlb, createEnuFrame, computeSvdAlignment, computeScreenSpaceError, getVisibleTiles, parseIfcGeometry, IfcGeometryResult, IfcMesh, GltfBuilder, pointCloudToGlb, terrainToGlb, meshToGlb, parseGeotiff, encodeQuantizedMesh, encodeTerrainTileset, ColorRamp, hillshade, contourLines, WorkerHandle, WorkerOptions, supportsWorker, processChunked, buildTin, tinInterpolate, haversineDistance, vincentyDistance, rhumbDistance, rhumbBearing, bearing, destination, midpoint, bufferPoint, bufferLineString, boundingBox, centroid, convexHull, concaveHull, clusterByDensity, clusterByGrid, crsInfo, getSupportedCrs, bestCrsForRegion, suggestCrsHeuristic, isInChina, getInputLimits, polygonArea, areaWithHoles, polylineLength, simplifyDouglasPeucker, isPointInRing, polygonIntersection, polygonUnion, contains, touches, disjoint, polygonIntersects, validateCoords, ValidationResult, cleanCoords, deduplicateCoords, sortCoordsByLng, sortCoordsByLat, gridIndex, parseWkb, parseWkt, toWkb, toWkt, memoryInfo, MemoryInfo, setInputSizeLimit, getInputSizeLimit, getAllocatedBytes, } from "./pkg/wasm_spatial_core.js";
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
 * Callback for the chunked GeoJSON parser (`parseGeoJsonStream`).
 * Parses the full input JSON first, then delivers coordinate batches.
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
    addProperty(input: string, key: string, value: string): string;
    applyColorRamp(positions: Float32Array, colors: Float32Array): Float32Array;
    applyTerrainColorRamp(heights: Float32Array, min_z: number, max_z: number, ramp: number): Uint8Array;
    applyTilesetPatch(base: import("./index.js").TilesetResult, patch: import("./pkg/wasm_spatial_core.js").TilesetPatch): import("./index.js").TilesetResult;
    areaWithHoles(rings: Float64Array, ring_sizes: Uint32Array): number;
    autoDecimate(positions: Float32Array, target_count: number, method: number): Float32Array;
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
    batchUtmToWgs84(utm_coords: Float64Array): Float64Array;
    batchUtmToWgs84InPlace(coords: Float64Array): void;
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
    batchWgs84ToUtm(coords: Float64Array): Float64Array;
    batchWgs84ToUtmInPlace(coords: Float64Array): void;
    bearing(lng1: number, lat1: number, lng2: number, lat2: number): number;
    bestCrsForRegion(min_lng: number, min_lat: number, max_lng: number, max_lat: number): string;
    boundingBox(coords: Float64Array): Float64Array;
    bufferLineString(coords: Float64Array, radius_meters: number, segments?: number | null): Float64Array;
    bufferPoint(lng: number, lat: number, radius_meters: number, segments?: number | null): Float64Array;
    buildColorRamp(colors: Uint8Array, num_steps: number): Uint8Array;
    buildOctree(positions: Float32Array, max_points_per_node?: number | null, max_depth?: number | null): import("./index.js").Octree;
    buildOctreeParallel(positions: Float32Array, max_points_per_node?: number | null, max_depth?: number | null): import("./index.js").Octree;
    buildOctreeParallelWithAbort(positions: Float32Array, max_points_per_node: number | null | undefined, max_depth: number | null | undefined, should_abort: Function): import("./index.js").Octree;
    buildOctreeWithAbort(positions: Float32Array, max_points_per_node: number | null | undefined, max_depth: number | null | undefined, should_abort: Function): import("./index.js").Octree;
    buildTin(points: Float64Array): import("./pkg/wasm_spatial_core.js").TinResult;
    centroid(coords: Float64Array): Float64Array;
    cgcs2000IsWgs84Compatible(): boolean;
    checkMemoryAvailable(estimated_bytes: number): boolean;
    cleanCoords(coords: Float64Array, strategy: string): Float64Array;
    clusterByDensity(coords: Float64Array, epsilon: number, min_points: number): Float64Array;
    clusterByGrid(coords: Float64Array, cell_size: number, min_points: number): Float64Array;
    colorizeByClassification(classes: Uint8Array): Uint8Array;
    colorizeByHeatmap(values: Float32Array, min: number, max: number): Uint8Array;
    colorizeByHeight(positions: Float32Array, min_z: number, max_z: number, low_color: Float32Array, high_color: Float32Array): Float32Array;
    colorizeByIntensity(positions: Float32Array, intensities: Float32Array): Float32Array;
    computeBounds(coords: Float64Array): Float64Array;
    computeBoundsMulti(buffers: Array<any>): Float64Array;
    computeLasPointOffset(header_info: import("./index.js").LasHeaderInfo, point_index: number, _point_format: number): number;
    computeRegionByteRange(point_offset: number, point_record_length: number, start_index: number, count: number): object;
    computeScreenSpaceError(geometric_error: number, distance: number, fov: number, screen_height: number): number;
    computeSvdAlignment(source: Float64Array, target: Float64Array, allow_scale: boolean): import("./pkg/wasm_spatial_core.js").WasmAlignmentResult;
    computeSvdAlignmentRansac(source: Float64Array, target: Float64Array, allow_scale: boolean, inlier_threshold: number, max_iterations: number, seed: bigint): import("./pkg/wasm_spatial_core.js").WasmAlignmentResult;
    computeSvdAlignmentRansacWeighted(source: Float64Array, target: Float64Array, allow_scale: boolean, inlier_threshold: number, max_iterations: number, seed: bigint, weights: Float64Array): import("./pkg/wasm_spatial_core.js").WasmAlignmentResult;
    computeSvdAlignmentWeighted(source: Float64Array, target: Float64Array, allow_scale: boolean, weights: Float64Array): import("./pkg/wasm_spatial_core.js").WasmAlignmentResult;
    concaveHull(coords: Float64Array, alpha: number): Float64Array;
    contains(outer_ring: Float64Array, point_x: number, point_y: number): boolean;
    contourLines(heights: Float32Array, width: number, height: number, interval: number): Array<any>;
    convexHull(coords: Float64Array): Float64Array;
    copcEstimateDownloadSize(copc_info_json: string): number;
    copcQueryRanges(copc_info_json: string, min_x: number, min_y: number, min_z: number, max_x: number, max_y: number, max_z: number): string;
    countGeoJsonByProperty(input: string, key: string): string;
    countGeoJsonFeatures(input: string): number;
    createEnuFrame(anchor: Float64Array): import("./pkg/wasm_spatial_core.js").WasmEnuFrame;
    crsInfo(code: string): string;
    decimateRandom(positions: Float32Array, colors: Uint8Array, target_count: number): object;
    decimateVoxelGrid(positions: Float32Array, colors: Uint8Array, cell_size: number): object;
    decimateVoxelGridWithProgress(positions: Float32Array, colors: Uint8Array, cell_size: number, on_progress: Function): object;
    decodeMvt(bytes: Uint8Array): import("./index.js").MvtLayer;
    decodeMvtToGeoJson(bytes: Uint8Array): string;
    decodeOct16Normal(encoded: number): Float32Array;
    deduplicateCoords(coords: Float64Array, tolerance: number): Float64Array;
    denormalizeCoords(normals: Float64Array, source_bounds: Float64Array): Float64Array;
    dequantizePositions(quantized: Uint16Array, bounds: import("./pkg/wasm_spatial_core.js").QuantBounds, bits?: number | null): Float32Array;
    destination(lng: number, lat: number, bearing_deg: number, distance_m: number): Float64Array;
    disjoint(ring1: Float64Array, ring2: Float64Array): boolean;
    dracoStatus(): string;
    e57Status(): string;
    encodeB3dmTile(glb_bytes: Uint8Array, batch_length: number, batch_table_json?: string | null): Uint8Array;
    encodeI3dmTile(glb_bytes: Uint8Array, positions: Float32Array, orientations?: Float32Array | null, scales?: Float32Array | null): Uint8Array;
    encodeOct16Normal(nx: number, ny: number, nz: number): number;
    encodePntsTile(positions: Float32Array, center_x: number, center_y: number, center_z: number, colors?: Uint8Array | null): Uint8Array;
    encodePntsTileWithNormals(positions: Float32Array, normals: Float32Array, center_x: number, center_y: number, center_z: number, colors?: Uint8Array | null): Uint8Array;
    encodeQuantizedMesh(heights: Float32Array, width: number, height: number, bounds: Float64Array, center: Float64Array): import("./pkg/wasm_spatial_core.js").QuantizedMeshResult;
    encodeTerrainTileset(heights: Float32Array, width: number, height: number, bounds: Float64Array, center: Float64Array, max_zoom: number): import("./pkg/wasm_spatial_core.js").TerrainTilesetResult;
    encodeTerrainTmsPyramid(heights: Float32Array, width: number, height: number, bounds: Float64Array, center: Float64Array, max_zoom: number): import("./pkg/wasm_spatial_core.js").WasmTmsPyramid;
    estimateJobBytes(op: string, point_count: number, leaf_count: number, has_color: boolean, raster_width: number, raster_height: number): number;
    estimateMemoryForPoints(num_points: number, has_color: boolean, has_normals: boolean): number;
    estimateNormals(positions: Float32Array, k: number): Float32Array;
    estimateOctreeMemory(num_points: number): number;
    estimatePointSpacing(positions: Float32Array, sample_size?: number | null): number;
    exportPointCloudToPnts(positions: Float32Array, colors?: Uint8Array | null, tile_uri?: string | null): import("./pkg/wasm_spatial_core.js").WasmPointCloudTileExport;
    filterByBounds(positions: Float32Array, colors: any, min_x: number, min_y: number, min_z: number, max_x: number, max_y: number, max_z: number): import("./pkg/wasm_spatial_core.js").FilteredResult;
    filterByClassification(positions: Float32Array, colors: any, classifications: Uint8Array, class_ids: Uint8Array): import("./pkg/wasm_spatial_core.js").FilteredResult;
    filterGeoJsonByBBox(input: string, min_lng: number, min_lat: number, max_lng: number, max_lat: number): string;
    filterGeoJsonByProperty(input: string, key: string, value: string): string;
    flipNormals(normals: Float32Array, positions: Float32Array): Float32Array;
    generate3DTile(geojson_str: string, height_property?: string | null): import("./index.js").Cesium3DTile;
    generateCesiumGeometry(geojson_str: string, height_property?: string | null): import("./index.js").CesiumMeshGeometry;
    generateIndexedGeometry(positions: Float32Array): object;
    generateInterleavedVertexBuffer(positions: Float32Array, colors: Uint8Array, normals: Float32Array): Float32Array;
    generateTileset(positions: Float32Array, max_points_per_node?: number | null, max_depth?: number | null, colors?: Uint8Array | null): import("./index.js").TilesetResult;
    generateTilesetIncremental(octree: import("./index.js").Octree, positions: Float32Array, colors: Uint8Array | null | undefined, on_tile: Function, should_abort?: Function | null): string;
    generateTilesetWithAbort(positions: Float32Array, max_points_per_node: number | null | undefined, max_depth: number | null | undefined, colors: Uint8Array | null | undefined, should_abort: Function): import("./index.js").TilesetResult;
    generateTilesetWithSpacing(positions: Float32Array, max_points_per_node?: number | null, max_depth?: number | null, colors?: Uint8Array | null, avg_spacing?: number | null, spacing_factor?: number | null): import("./index.js").TilesetResult;
    geoJsonFeatureCollection(coords: Float64Array, types: string, properties_json: string): string;
    geoJsonFromCoords(coords: Float64Array, geometry_type: string): string;
    geohashDecode(hash: string): Float64Array;
    geohashEncode(lng: number, lat: number, precision: number): string;
    geohashNeighbors(hash: string): Array<any>;
    geotiffStatus(): string;
    getAllocatedBytes(): number;
    getInputLimits(): string;
    getInputSizeLimit(): number;
    getMaxWasmMemory(): number;
    getSupportedCrs(): string;
    getVisibleTiles(positions: Float32Array, camera_x: number, camera_y: number, camera_z: number, camera_fov: number, screen_width: number, screen_height: number, max_points_per_node?: number | null, max_depth?: number | null, sse_threshold?: number | null): Uint32Array;
    gpxTrackStats(input: string): string;
    gridIndex(coords: Float64Array, cell_size_deg: number): Float64Array;
    haversineDistance(lng1: number, lat1: number, lng2: number, lat2: number): number;
    hillshade(heights: Float32Array, width: number, height: number, azimuth_deg: number, altitude_deg: number): Uint8Array;
    init(): void;
    isInChina(lng: number, lat: number): boolean;
    isPointInRing(point_x: number, point_y: number, ring_coords: Float64Array): boolean;
    lazStatus(): string;
    memoryInfo(): import("./index.js").MemoryInfo;
    mergePointClouds(positions_a: Float32Array, colors_a: any, positions_b: Float32Array, colors_b: any): import("./pkg/wasm_spatial_core.js").FilteredResult;
    meshToGlb(vertices: Float32Array, indices: Uint32Array, normals?: Float32Array | null): Uint8Array;
    midpoint(lng1: number, lat1: number, lng2: number, lat2: number): Float64Array;
    mvtLayerInfo(bytes: Uint8Array): string;
    mvtToGeoJson(bytes: Uint8Array, extent: number, x: number, y: number, z: number): string;
    normalizeCoords(coords: Float64Array, target_bounds: Float64Array): Float64Array;
    octreeMemoryUsage(node_count: number, internal_count: number, point_count: number): number;
    parseCopcHeader(bytes: Uint8Array): object;
    parseGeoJsonCoords(input: string): Float64Array;
    parseGeoJsonFeatures(input: string): import("./index.js").GeoJsonFeaturesResult;
    parseGeoJsonLazy(input: string): import("./index.js").LazyGeoJsonIter;
    parseGeoJsonPerFeature(input: string): Array<any>;
    parseGeoJsonProperties(input: string): string;
    parseGeoJsonStream(input: string, chunk_size: number, on_chunk: Function): number;
    parseGeotiff(bytes: Uint8Array): import("./pkg/wasm_spatial_core.js").GeotiffInfo;
    parseGeotiffTile(bytes: Uint8Array, tile_index: number): Float32Array;
    parseGlb(bytes: Uint8Array): import("./pkg/wasm_spatial_core.js").WasmMeshChunk;
    parseGpx(input: string): Float64Array;
    parseGpxWithElevation(input: string): Float64Array;
    parseIfcGeometry(text: string): import("./index.js").IfcGeometryResult;
    parseLasHeader(bytes: Uint8Array): import("./index.js").LasHeader;
    parseLasHeaderOnly(bytes: Uint8Array): import("./index.js").LasHeaderInfo;
    parseLasPointAt(bytes: Uint8Array, offset: number, point_format: number): import("./index.js").PointData;
    parseLasPoints(bytes: Uint8Array): import("./index.js").LasPointCloud;
    parseLasPointsWithProgress(bytes: Uint8Array, on_progress: Function): import("./index.js").LasPointCloud;
    parseLasPointsWithProgressAndAbort(bytes: Uint8Array, on_progress: Function, should_abort: Function): import("./index.js").LasPointCloud;
    parseLazPoints(bytes: Uint8Array): import("./index.js").LasPointCloud;
    parseLazPointsStream(bytes: Uint8Array, on_progress: Function): import("./index.js").LasPointCloud;
    parseLazPointsStreamWithAbort(bytes: Uint8Array, on_progress: Function, should_abort: Function): import("./index.js").LasPointCloud;
    parseObjVertices(text: string): Float32Array;
    parseObjWithNormals(text: string): object;
    parsePcdAscii(text: string): import("./index.js").PcdPointCloud;
    parsePcdBinary(bytes: Uint8Array): import("./index.js").PcdPointCloud;
    parsePly(bytes: Uint8Array): import("./pkg/wasm_spatial_core.js").PlyResult;
    parsePointCloudAuto(bytes: Uint8Array): import("./index.js").LasPointCloud;
    parseTopojson(input: string): Float64Array;
    parseWkb(bytes: Uint8Array): Float64Array;
    parseWkt(input: string): Float64Array;
    pointCloudAnalysis(positions: Float32Array, colors: any): import("./pkg/wasm_spatial_core.js").PointCloudStats;
    pointCloudBounds(positions: Float32Array): Float64Array;
    pointCloudCentroid(positions: Float32Array): Float64Array;
    pointCloudChunkFromBuffers(positions: Float32Array, colors?: Uint8Array | null, source_format?: string | null): import("./index.js").PointCloudChunk;
    pointCloudStats(positions: Float32Array): string;
    pointCloudToGlb(positions: Float32Array, colors?: Uint8Array | null, normals?: Float32Array | null): Uint8Array;
    polygonArea(coords: Float64Array): number;
    polygonIntersection(ring1: Float64Array, ring2: Float64Array): Float64Array;
    polygonIntersects(ring1: Float64Array, ring2: Float64Array): boolean;
    polygonUnion(ring1: Float64Array, ring2: Float64Array): Float64Array;
    polylineLength(coords: Float64Array): number;
    processChunked(positions: Float32Array, colors: Uint8Array | null | undefined, max_points_per_node: number | null | undefined, max_depth: number | null | undefined, on_chunk: Function): Promise<any>;
    processChunkedWithAbort(positions: Float32Array, colors: Uint8Array | null | undefined, max_points_per_node: number | null | undefined, max_depth: number | null | undefined, on_chunk: Function, should_abort: Function): Promise<any>;
    quantizePositions(positions: Float32Array, bits?: number | null): import("./pkg/wasm_spatial_core.js").QuantizeResult;
    readCopcChunk(bytes: Uint8Array, chunk_offset: number, chunk_size: number, expected_points: number, header_bytes: Uint8Array): import("./index.js").LasPointCloud;
    readCopcChunkStandalone(chunk_bytes: Uint8Array, expected_points: number, header_bytes: Uint8Array): import("./index.js").LasPointCloud;
    readCopcRegion(bytes: Uint8Array, min_x: number, min_y: number, min_z: number, max_x: number, max_y: number, max_z: number): import("./index.js").LasPointCloud;
    removeProperty(input: string, key: string): string;
    renameProperty(input: string, old_key: string, new_key: string): string;
    rhumbBearing(lng1: number, lat1: number, lng2: number, lat2: number): number;
    rhumbDistance(lng1: number, lat1: number, lng2: number, lat2: number): number;
    rotatePointCloud(positions: Float32Array, axis: Float32Array, angle: number): Float32Array;
    scalePointCloud(positions: Float32Array, sx: number, sy: number, sz: number): Float32Array;
    setInputSizeLimit(bytes: number): void;
    setMaxWasmMemory(bytes: number): void;
    simplifyDouglasPeucker(coords: Float64Array, tolerance: number): Float64Array;
    sortCoordsByLat(coords: Float64Array): Float64Array;
    sortCoordsByLng(coords: Float64Array): Float64Array;
    suggestCrsHeuristic(min_lng: number, min_lat: number, max_lng: number, max_lat: number): string;
    supportsDraco(): boolean;
    supportsE57(): boolean;
    supportsGeotiff(): boolean;
    supportsLaz(): boolean;
    supportsMeshIngest(): boolean;
    supportsMultiThread(): boolean;
    supportsWorker(): boolean;
    terrainToGlb(heights: Float32Array, width: number, height: number, bounds: Float64Array): Uint8Array;
    threadCount(): number;
    tinInterpolate(tin: import("./pkg/wasm_spatial_core.js").TinResult, x: number, y: number): number;
    toWkb(coords: Float64Array, geometry_type: string): Uint8Array;
    toWkt(coords: Float64Array, geometry_type: string): string;
    topojsonToGeojson(input: string): string;
    touches(ring1: Float64Array, ring2: Float64Array): boolean;
    transformPointCloud(positions: Float32Array, matrix: Float32Array): Float32Array;
    translatePointCloud(positions: Float32Array, dx: number, dy: number, dz: number): Float32Array;
    utmToWgs84(zone: number, easting: number, northing: number, is_north: boolean): Float64Array;
    validateCoords(coords: Float64Array, crs: string): import("./index.js").ValidationResult;
    version(): string;
    vincentyDistance(lng1: number, lat1: number, lng2: number, lat2: number): number;
    wgs84ToUtm(lng: number, lat: number): Float64Array;
    initSync(module: {
        module: import("./pkg/wasm_spatial_core.js").SyncInitInput;
    } | import("./pkg/wasm_spatial_core.js").SyncInitInput): import("./pkg/wasm_spatial_core.js").InitOutput;
    Cesium3DTile: typeof import("./index.js").Cesium3DTile;
    CesiumMeshGeometry: typeof import("./index.js").CesiumMeshGeometry;
    ColorRamp: typeof import("./index.js").ColorRamp;
    FilteredResult: typeof import("./pkg/wasm_spatial_core.js").FilteredResult;
    GeoJsonFeaturesResult: typeof import("./index.js").GeoJsonFeaturesResult;
    GeotiffInfo: typeof import("./pkg/wasm_spatial_core.js").GeotiffInfo;
    GltfBuilder: typeof import("./index.js").GltfBuilder;
    IfcGeometryResult: typeof import("./index.js").IfcGeometryResult;
    IfcMesh: typeof import("./index.js").IfcMesh;
    LasHeader: typeof import("./index.js").LasHeader;
    LasHeaderInfo: typeof import("./index.js").LasHeaderInfo;
    LasPointCloud: typeof import("./index.js").LasPointCloud;
    LazyGeoJsonIter: typeof import("./index.js").LazyGeoJsonIter;
    MemoryInfo: typeof import("./index.js").MemoryInfo;
    MvtFeature: typeof import("./index.js").MvtFeature;
    MvtLayer: typeof import("./index.js").MvtLayer;
    Octree: typeof import("./index.js").Octree;
    OctreeChunkBuilder: typeof import("./index.js").OctreeChunkBuilder;
    PcdPointCloud: typeof import("./index.js").PcdPointCloud;
    PlyResult: typeof import("./pkg/wasm_spatial_core.js").PlyResult;
    PointCloudChunk: typeof import("./index.js").PointCloudChunk;
    PointCloudStats: typeof import("./pkg/wasm_spatial_core.js").PointCloudStats;
    PointCloudStreamer: typeof import("./index.js").PointCloudStreamer;
    PointData: typeof import("./index.js").PointData;
    ProcessingContext: typeof import("./pkg/wasm_spatial_core.js").ProcessingContext;
    QuantBounds: typeof import("./pkg/wasm_spatial_core.js").QuantBounds;
    QuantizeResult: typeof import("./pkg/wasm_spatial_core.js").QuantizeResult;
    QuantizedMeshResult: typeof import("./pkg/wasm_spatial_core.js").QuantizedMeshResult;
    SpatialEdgeIndex: typeof import("./index.js").SpatialEdgeIndex;
    SpatialIndex: typeof import("./index.js").SpatialIndex;
    TerrainTilesetResult: typeof import("./pkg/wasm_spatial_core.js").TerrainTilesetResult;
    TilesetPatch: typeof import("./pkg/wasm_spatial_core.js").TilesetPatch;
    TilesetResult: typeof import("./index.js").TilesetResult;
    TinResult: typeof import("./pkg/wasm_spatial_core.js").TinResult;
    ValidationResult: typeof import("./index.js").ValidationResult;
    VectorTileEngine: typeof import("./index.js").VectorTileEngine;
    VectorTileOptions: typeof import("./index.js").VectorTileOptions;
    WasmAlignmentResult: typeof import("./pkg/wasm_spatial_core.js").WasmAlignmentResult;
    WasmEnuFrame: typeof import("./pkg/wasm_spatial_core.js").WasmEnuFrame;
    WasmMeshChunk: typeof import("./pkg/wasm_spatial_core.js").WasmMeshChunk;
    WasmPointCloudTileExport: typeof import("./pkg/wasm_spatial_core.js").WasmPointCloudTileExport;
    WasmTmsPyramid: typeof import("./pkg/wasm_spatial_core.js").WasmTmsPyramid;
    WorkerHandle: typeof import("./index.js").WorkerHandle;
    WorkerOptions: typeof import("./index.js").WorkerOptions;
}>;
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
