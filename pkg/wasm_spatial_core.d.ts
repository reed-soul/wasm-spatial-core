/* tslint:disable */
/* eslint-disable */

/**
 * A Cesium 3D Tiles b3dm tile containing a triangulated batched mesh.
 */
export class Cesium3DTile {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Serialize this tile to a complete b3dm binary blob.
     *
     * b3dm layout:
     * ```text
     * [Header 28 bytes] [BatchTable JSON] [FeatureTable JSON + BIN] [Body]
     * ```
     *
     * Header (28 bytes, little-endian):
     * - magic: "b3dm" (4 bytes)
     * - version: 1 (u32)
     * - byteLength (u32) — total tile size
     * - featureTableJSONByteLength (u32)
     * - featureTableBinaryByteLength (u32)
     * - batchTableJSONByteLength (u32)
     * - batchTableBinaryByteLength (u32)
     */
    toBytes(): Uint8Array;
    readonly batchTableJson: string;
    readonly featureBatchIds: Uint32Array;
}

/**
 * Contains triangulated mesh data ready for Cesium.Geometry
 */
export class CesiumMeshGeometry {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    readonly indices: Uint32Array;
    readonly positions: Float64Array;
}

/**
 * Result of structured GeoJSON feature parsing.
 *
 * Contains per-feature coordinate buffers, offsets, counts, and geometry types.
 */
export class GeoJsonFeaturesResult {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * All coordinates as a flat `Float64Array`.
     */
    readonly coordinates: Float64Array;
    /**
     * Per-feature coordinate pair count.
     */
    readonly counts: Uint32Array;
    /**
     * Per-feature starting offset into the coordinate buffer.
     */
    readonly offsets: Uint32Array;
    /**
     * Comma-separated geometry type for each feature.
     */
    readonly types: string;
}

/**
 * glTF 2.0 builder — collect meshes and materials, then export as GLB or JSON.
 */
export class GltfBuilder {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Add a material with base color (RGBA 0–1 range).
     */
    addMaterial(r: number, g: number, b: number, a: number): number;
    /**
     * Add a mesh with positions, indices, and optional normals.
     *
     * - `positions`: Flat `Float32Array` `[x0, y0, z0, x1, y1, z1, ...]`
     * - `indices`: Flat `Uint32Array` `[i0, i1, i2, ...]`
     * - `normals`: Optional flat `Float32Array` `[nx0, ny0, nz0, ...]` (may be `null`)
     * - `material_index`: Material index (0-based), or `-1` for no material.
     */
    addMesh(positions: Float32Array, indices: Uint32Array, normals: Float32Array, material_index: number): void;
    /**
     * Create a new empty glTF builder.
     */
    constructor();
    /**
     * Export as binary GLB (`Uint8Array`).
     */
    toGlb(): Uint8Array;
    /**
     * Export as glTF JSON string (no binary — positions/indices as base64).
     */
    toGltfJson(): string;
}

/**
 * Result of parsing IFC geometry.
 */
export class IfcGeometryResult {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Total number of meshes extracted (JS getter delegates to impl method).
     */
    readonly meshCount: number;
    /**
     * Array of extracted meshes.
     */
    readonly meshes: Array<any>;
}

/**
 * A single mesh extracted from an IFC entity.
 */
export class IfcMesh {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Triangle indices as `Uint32Array` `[i0, i1, i2, ...]`.
     */
    readonly indices: Uint32Array;
    /**
     * Vertex positions as `Float64Array` `[x0, y0, z0, x1, y1, z1, ...]`.
     */
    readonly positions: Float64Array;
    /**
     * Number of triangles.
     */
    readonly triangleCount: number;
    /**
     * Number of vertices.
     */
    readonly vertexCount: number;
}

/**
 * A lazy GeoJSON FeatureCollection iterator.
 *
 * Parses features one at a time using a manual JSON state machine,
 * without building the full DOM. Memory peak is O(single feature)
 * instead of O(all features).
 *
 * ## Usage (JS)
 *
 * ```js
 * const iter = parseGeoJsonLazy(hugeGeoJsonStr);
 * let feature;
 * while ((feature = iter.nextFeature()) !== null) {
 *   // feature is a Float64Array of [lng0, lat0, lng1, lat1, ...]
 *   gl.bufferSubData(gl.ARRAY_BUFFER, offset, feature);
 *   offset += feature.byteLength;
 * }
 * console.log(`Processed ${iter.remaining()} features`);
 * iter.free();
 * ```
 */
export class LazyGeoJsonIter {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Advance to the next feature and return its coordinates as a `Float64Array`.
     *
     * Returns `null` (JS undefined) when all features have been consumed.
     */
    nextFeature(): Float64Array | undefined;
    /**
     * Get the remaining unconsumed feature count.
     */
    remaining(): number;
    /**
     * Get the total feature count.
     */
    readonly total: number;
}

/**
 * WASM linear memory usage info.
 */
export class MemoryInfo {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Remaining free memory (in bytes).
     */
    readonly remaining: number;
    /**
     * Total WASM linear memory allocated (in bytes).
     */
    readonly total: number;
    /**
     * Approximate used memory (in bytes).
     */
    readonly used: number;
}

/**
 * A decoded MVT feature with geometry, type, and tags.
 */
export class MvtFeature {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Tag count.
     */
    tagCount(): number;
    /**
     * Get a tag key by index.
     */
    tagKey(index: number): string;
    /**
     * Get a tag value by index.
     */
    tagValue(index: number): string;
    /**
     * Flat tile-space coordinates as `Float64Array`.
     */
    readonly geometry: Float64Array;
    /**
     * Geometry type: 0=Unknown, 1=Point, 2=LineString, 3=Polygon.
     */
    readonly geometry_type: number;
    /**
     * Feature ID (0 if not set).
     */
    readonly id: number;
}

/**
 * A decoded MVT layer with structured feature data.
 */
export class MvtLayer {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Get feature by index.
     */
    featureAt(index: number): MvtFeature;
    /**
     * Number of features in this layer.
     */
    featureCount(): number;
    /**
     * Layer extent (typically 4096).
     */
    readonly extent: number;
    /**
     * Layer name.
     */
    readonly name: string;
}

/**
 * WASM-accessible octree handle.
 */
export class Octree {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Bounding box of node at `index` as a `Float64Array` of 6 values.
     */
    nodeBounds(index: number): Float64Array;
    /**
     * Children indices of node at `index`, or `null` if leaf.
     */
    nodeChildren(index: number): Array<any> | undefined;
    /**
     * Total number of nodes (internal + leaf).
     */
    nodeCount(): number;
    /**
     * Depth level of node at `index`.
     */
    nodeLevel(index: number): number;
    /**
     * Point count of node at `index`.
     */
    nodePointCount(index: number): number;
    /**
     * Root bounding box as a `Float64Array` of 6 values:
     * `[min_x, min_y, min_z, max_x, max_y, max_z]`.
     */
    rootBounds(): Float64Array;
    /**
     * Maximum tree depth.
     */
    readonly depth: number;
    /**
     * Total number of indexed points.
     */
    readonly totalPoints: number;
}

/**
 * A spatial index for 2D line segments using an R-Tree.
 *
 * Indexes individual edges (line segments) from LineString geometries.
 * Supports bounding box queries to find all edges that intersect with
 * a given rectangular area. Useful for viewport-based progressive loading
 * of road networks, pipelines, and other linear features.
 */
export class SpatialEdgeIndex {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Find the nearest edge to a given query coordinate.
     * Returns the ID of the nearest edge, or `null` if the index is empty.
     *
     * Distance is measured as the minimum Euclidean distance from the
     * query point to any point on the edge.
     */
    nearestNeighbor(query_x: number, query_y: number): number | undefined;
    /**
     * Build a spatial edge index from line segments.
     *
     * Input format: a flat `Float64Array` of line segment endpoints
     * `[x0, y0, x1, y1, x2, y2, x3, y3, ...]` where each consecutive
     * pair of 2D points forms an edge (line segment).
     *
     * Each edge is assigned an ID equal to its sequential index
     * (0 for the first edge, 1 for the second, etc.).
     */
    constructor(segments: Float64Array);
    /**
     * Search for all edges within a given bounding box.
     * Returns a `Uint32Array` containing the IDs of matching edges.
     *
     * An edge matches if its bounding box intersects the query envelope.
     */
    searchBBox(min_x: number, min_y: number, max_x: number, max_y: number): Uint32Array;
    /**
     * Get the total number of edges in the index.
     */
    size(): number;
}

/**
 * A high-performance spatial index using an R-Tree.
 */
export class SpatialIndex {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Find the K nearest neighbors to a given query coordinate.
     * Returns a `Uint32Array` containing the IDs, ordered by distance (nearest first).
     * If `k` is greater than the number of points, returns all points.
     */
    kNearestNeighbors(query_x: number, query_y: number, k: number): Uint32Array;
    /**
     * Find the nearest point to a given query coordinate.
     * Returns the ID of the nearest point, or `null` if the index is empty.
     */
    nearestNeighbor(query_x: number, query_y: number): number | undefined;
    /**
     * Build a spatial index from a flat Float64Array of coordinates `[lng0, lat0, lng1, lat1, ...]`.
     * Each coordinate pair is assigned an ID equal to its index (i.e. `0` for the first pair, `1` for the second).
     */
    constructor(coords: Float64Array);
    /**
     * Search for all points within a given bounding box.
     * Returns a `Uint32Array` containing the IDs of the points.
     */
    searchBBox(min_x: number, min_y: number, max_x: number, max_y: number): Uint32Array;
    /**
     * Get the total number of points in the index.
     */
    size(): number;
}

/**
 * WASM-accessible tileset result handle.
 */
export class TilesetResult {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Get tile binary data as `Uint8Array`.
     */
    tile(index: number): Uint8Array;
    /**
     * Tile bounding box as `Float64Array`.
     */
    tileBounds(index: number): Float64Array;
    /**
     * Get tile URI string.
     */
    tileUri(index: number): string | undefined;
    /**
     * The tileset.json content.
     */
    tilesetJson(): string;
    /**
     * Number of tiles.
     */
    readonly tileCount: number;
    /**
     * Total bytes across all tiles.
     */
    readonly totalBytes: number;
}

/**
 * Result of building a TIN from scattered 3D points.
 */
export class TinResult {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Triangle indices `[i0,i1,i2, i3,i4,i5, ...]`.
     */
    readonly indices: Uint32Array;
    /**
     * Flat vertex positions `[x0,y0,z0, x1,y1,z1, ...]`.
     */
    readonly positions: Float64Array;
    /**
     * Number of triangles.
     */
    readonly triangleCount: number;
    /**
     * Number of vertices.
     */
    readonly vertexCount: number;
}

/**
 * Result of coordinate validation.
 */
export class ValidationResult {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    readonly invalid_count: number;
    readonly invalid_indices: Uint32Array;
    readonly valid_count: number;
}

/**
 * A high-performance vector tile engine.
 *
 * Creates a pre-indexed GeoJSONVT structure from a GeoJSON string,
 * then can efficiently slice tiles by `(z, x, y)`. Feature properties
 * from the original GeoJSON are preserved as MVT tags.
 *
 * Supports optional LRU caching via `getTileCached` / `clearTileCache`.
 */
export class VectorTileEngine {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Get the number of cached tiles.
     */
    cacheSize(): number;
    /**
     * Clear the tile LRU cache.
     */
    clearTileCache(): void;
    /**
     * Request a tile by `z, x, y` coordinates.
     * Returns a raw `Uint8Array` representing the MVT (PBF) protobuf.
     * If the tile is empty or out of bounds, returns an empty array.
     *
     * Feature properties (`name`, `id`, `class`, and any other fields)
     * from the original GeoJSON are automatically encoded as MVT tags.
     */
    getTile(z: number, x: number, y: number): Uint8Array;
    /**
     * Request a tile with LRU caching (max 64 tiles).
     *
     * If the tile was previously requested, returns the cached result
     * without re-computing. Otherwise, generates the tile, caches it,
     * and returns it.
     *
     * Use `clearTileCache()` to evict all cached tiles.
     */
    getTileCached(z: number, x: number, y: number): Uint8Array;
    /**
     * Create a new VectorTileEngine from a GeoJSON string.
     *
     * The `layer_name` parameter controls the layer name embedded in the
     * MVT protobuf output. Defaults to `"default"`.
     */
    constructor(geojson_str: string, options: VectorTileOptions, layer_name?: string | null);
    /**
     * Get the layer name used by this engine.
     */
    layerName: string;
}

/**
 * Options for vector tile generation.
 */
export class VectorTileOptions {
    free(): void;
    [Symbol.dispose](): void;
    constructor();
    buffer: number;
    extent: number;
    generate_id: boolean;
    index_max_points: number;
    index_max_zoom: number;
    line_metrics: boolean;
    max_zoom: number;
    tolerance: number;
}

/**
 * Add a property to all features in a GeoJSON FeatureCollection.
 *
 * Operates at the `serde_json::Value` level — no full GeoJSON DOM
 * construction, just lightweight JSON manipulation.
 *
 * # Arguments
 *
 * * `input` — GeoJSON string (FeatureCollection only).
 * * `key` — Property key to add.
 * * `value` — Property value (parsed as JSON: strings, numbers, booleans).
 *
 * # Returns
 *
 * Modified GeoJSON string with the property added to every feature.
 */
export function addProperty(input: string, key: string, value: string): string;

/**
 * Calculate the area of a polygon with holes in square meters.
 * Exterior ring area minus the sum of all hole areas.
 *
 * # Arguments
 * - `rings`: Flat `Float64Array` containing all rings concatenated.
 * - `ringSizes`: `Uint32Array` where each element is the number of *coordinates*
 *   (not points) in each ring. First ring = exterior, rest = holes.
 *
 * Each ring must be closed (first point = last point).
 */
export function areaWithHoles(rings: Float64Array, ring_sizes: Uint32Array): number;

/**
 * Batch BD-09 → GCJ-02. Returns a **new** `Float64Array`.
 */
export function batchBd09ToGcj02(coords: Float64Array): Float64Array;

/**
 * **[Zero-Copy]** In-place BD-09 → GCJ-02.
 */
export function batchBd09ToGcj02InPlace(coords: Float64Array): void;

/**
 * Batch BD-09 → WGS-84. Returns a **new** `Float64Array`.
 */
export function batchBd09ToWgs84(coords: Float64Array): Float64Array;

/**
 * **[Zero-Copy]** In-place BD-09 → WGS-84.
 */
export function batchBd09ToWgs84InPlace(coords: Float64Array): void;

/**
 * Batch GCJ-02 → BD-09. Returns a **new** `Float64Array`.
 */
export function batchGcj02ToBd09(coords: Float64Array): Float64Array;

/**
 * **[Zero-Copy]** In-place GCJ-02 → BD-09.
 */
export function batchGcj02ToBd09InPlace(coords: Float64Array): void;

/**
 * Batch GCJ-02 → WGS-84. Returns a **new** `Float64Array`.
 */
export function batchGcj02ToWgs84(coords: Float64Array): Float64Array;

/**
 * **[Zero-Copy]** In-place GCJ-02 → WGS-84.
 */
export function batchGcj02ToWgs84InPlace(coords: Float64Array): void;

/**
 * Batch Web Mercator (EPSG:3857) → WGS-84. Returns a **new** `Float64Array`.
 */
export function batchMercatorToWgs84(coords: Float64Array): Float64Array;

/**
 * **[Zero-Copy]** In-place Web Mercator (EPSG:3857) → WGS-84.
 */
export function batchMercatorToWgs84InPlace(coords: Float64Array): void;

/**
 * Convert batch UTM coordinates to WGS84.
 *
 * Input: flat `[zone, easting, northing, zone, easting, northing, ...]`.
 * Output: flat `[lng, lat, lng, lat, ...]`.
 */
export function batchUtmToWgs84(utm_coords: Float64Array): Float64Array;

/**
 * Convert batch UTM to WGS84 in-place.
 *
 * Input layout: `[zone, easting, northing, ...]`.
 * Output layout: `[lng, lat, 0, ...]` (third component zeroed).
 */
export function batchUtmToWgs84InPlace(coords: Float64Array): void;

/**
 * Batch WGS-84 → BD-09. Returns a **new** `Float64Array`.
 */
export function batchWgs84ToBd09(coords: Float64Array): Float64Array;

/**
 * **[Zero-Copy]** In-place WGS-84 → BD-09.
 */
export function batchWgs84ToBd09InPlace(coords: Float64Array): void;

/**
 * Batch WGS-84 → BD-09 → Web Mercator. Returns a **new** `Float64Array`.
 */
export function batchWgs84ToBd09Mercator(coords: Float64Array): Float64Array;

/**
 * **[Zero-Copy]** In-place WGS-84 → BD-09 → Web Mercator.
 */
export function batchWgs84ToBd09MercatorInPlace(coords: Float64Array): void;

/**
 * Batch convert a flat array of `[lng, lat, ...]` into `[x, y, z, ...]`.
 */
export function batchWgs84ToCartesian3(coords: Float64Array): Float64Array;

/**
 * Batch "WGS-84 → CGCS2000" — identity transform. Returns a copy.
 *
 * See [`cgcs2000_is_wgs84_compatible`] for precision details.
 */
export function batchWgs84ToCgcs2000(coords: Float64Array): Float64Array;

/**
 * **[Zero-Copy]** In-place "WGS-84 → CGCS2000" — identity transform.
 *
 * Provided for API completeness. Since CGCS2000 ≈ WGS-84 (< 1 cm difference),
 * this is a no-op. The buffer is returned unchanged.
 *
 * If your pipeline requires an explicit CGCS2000 step, call this to make the
 * intent clear in code without incurring any runtime cost.
 */
export function batchWgs84ToCgcs2000InPlace(_coords: Float64Array): void;

/**
 * Batch WGS-84 → GCJ-02. Returns a **new** `Float64Array`.
 *
 * For large datasets, prefer the `InPlace` variant to avoid copies.
 */
export function batchWgs84ToGcj02(coords: Float64Array): Float64Array;

/**
 * **[Zero-Copy]** In-place WGS-84 → GCJ-02.
 *
 * Mutates the input `[lng, lat, …]` buffer directly in WASM linear memory.
 * ```js
 * const buf = new Float64Array(wasmMemory.buffer, ptr, len);
 * wasm.batchWgs84ToGcj02InPlace(buf);
 * // buf is now in GCJ-02 — no copy occurred
 * ```
 */
export function batchWgs84ToGcj02InPlace(coords: Float64Array): void;

/**
 * Batch WGS-84 → GCJ-02 → Web Mercator. Returns a **new** `Float64Array`.
 */
export function batchWgs84ToGcj02Mercator(coords: Float64Array): Float64Array;

/**
 * **[Zero-Copy]** In-place WGS-84 → GCJ-02 → Web Mercator.
 *
 * Most common pipeline for Chinese web map applications.
 */
export function batchWgs84ToGcj02MercatorInPlace(coords: Float64Array): void;

/**
 * Batch WGS-84 → Web Mercator (EPSG:3857). Returns a **new** `Float64Array`.
 */
export function batchWgs84ToMercator(coords: Float64Array): Float64Array;

/**
 * **[Zero-Copy]** In-place WGS-84 → Web Mercator (EPSG:3857).
 */
export function batchWgs84ToMercatorInPlace(coords: Float64Array): void;

/**
 * Convert batch WGS84 coordinates to UTM.
 *
 * Input: flat `[lng0, lat0, lng1, lat1, ...]`.
 * Output: flat `[zone, easting, northing, zone, easting, northing, ...]`.
 */
export function batchWgs84ToUtm(coords: Float64Array): Float64Array;

/**
 * Convert batch WGS84 to UTM in-place.
 *
 * The input buffer must be pre-allocated with 3 values per point (same as output).
 * Input layout: `[lng, lat, 0, lng, lat, 0, ...]`.
 * Output layout: `[zone, easting, northing, zone, easting, northing, ...]`.
 */
export function batchWgs84ToUtmInPlace(coords: Float64Array): void;

/**
 * Calculate the initial bearing (forward azimuth) from point 1 to point 2.
 *
 * Returns the bearing in degrees [0, 360), where 0 = North, 90 = East,
 * 180 = South, 270 = West.
 *
 * # Arguments
 * - `lng1`: Longitude of origin in degrees.
 * - `lat1`: Latitude of origin in degrees.
 * - `lng2`: Longitude of destination in degrees.
 * - `lat2`: Latitude of destination in degrees.
 */
export function bearing(lng1: number, lat1: number, lng2: number, lat2: number): number;

/**
 * Recommend the best CRS for a geographic region.
 *
 * # Arguments
 * - `min_lng`, `min_lat`, `max_lng`, `max_lat`: Bounding box in degrees.
 *
 * # Returns
 * JSON string with `crs` (recommended CRS code) and `reason`.
 */
export function bestCrsForRegion(min_lng: number, min_lat: number, max_lng: number, max_lat: number): string;

/**
 * Compute the axis-aligned bounding box of a set of coordinates.
 *
 * Returns `[minLng, minLat, maxLng, maxLat]`.
 */
export function boundingBox(coords: Float64Array): Float64Array;

/**
 * Generate a buffer polygon around a line string (union of point buffers).
 *
 * Returns a flat `Float64Array` of polygon vertices `[lng0, lat0, ...]`.
 * Note: this is a simplified implementation that produces a convex hull of
 * all circle vertices around each line point. For production use with
 * concave results, consider `geo`'s `BooleanOps` union.
 */
export function bufferLineString(coords: Float64Array, radius_meters: number, segments?: number | null): Float64Array;

/**
 * Generate a buffer polygon around a point.
 *
 * Returns a flat `Float64Array` of polygon vertices `[lng0, lat0, lng1, lat1, ...]`
 * forming a circle approximation around the given point.
 */
export function bufferPoint(lng: number, lat: number, radius_meters: number, segments?: number | null): Float64Array;

/**
 * Build an octree from a flat `[x, y, z, ...]` position buffer.
 *
 * The input buffer is **not** modified (a copy is made internally).
 *
 * # Arguments
 * * `positions` — `Float32Array` of `[x, y, z, ...]` triples.
 * * `max_points_per_node` — Max points per leaf (default: 50 000).
 * * `max_depth` — Max tree depth (default: 21).
 */
export function buildOctree(positions: Float32Array, max_points_per_node?: number | null, max_depth?: number | null): Octree;

/**
 * Build a TIN from scattered 3D points using the Bowyer-Watson algorithm.
 *
 * # Arguments
 * - `points`: Flat `Float64Array` `[x0,y0,z0, x1,y1,z1, ...]`
 *
 * # Returns
 * `TinResult` with deduplicated positions and triangle indices.
 */
export function buildTin(points: Float64Array): TinResult;

/**
 * Compute the centroid (mean center) of a set of coordinates.
 *
 * Returns `[lng, lat]`.
 */
export function centroid(coords: Float64Array): Float64Array;

/**
 * Check if CGCS2000 and WGS-84 are equivalent for the caller's precision.
 *
 * CGCS2000 and WGS-84 share virtually identical ellipsoid parameters.
 * The difference is sub-centimetre level (< 0.11 mm at epoch 2000.0).
 *
 * For engineering-grade accuracy (> 1 cm), they are interchangeable.
 * This function returns `true`, indicating the identity transform is valid.
 *
 * For geodetic-survey-grade work (mm-level), users should apply an
 * epoch-dependent tectonic plate motion model — this is outside the
 * scope of a browser-based library.
 */
export function cgcs2000IsWgs84Compatible(): boolean;

/**
 * Clean coordinate data by removing, clamping, or snapping invalid values.
 *
 * # Arguments
 *
 * * `coords` — Flat `Float64Array` `[lng0, lat0, lng1, lat1, …]`
 * * `strategy` — One of: `"remove"`, `"clamp"`, `"snap"`
 */
export function cleanCoords(coords: Float64Array, strategy: string): Float64Array;

/**
 * Density-based spatial clustering (simplified DBSCAN).
 *
 * # Arguments
 * - `coords`: Flat `Float64Array` `[lng0, lat0, lng1, lat1, ...]`.
 * - `epsilon`: Neighborhood radius in meters.
 * - `min_points`: Minimum points in a neighborhood to form a cluster.
 *
 * # Returns
 * Flat `Float64Array` of cluster IDs (one per point). -1 = noise.
 */
export function clusterByDensity(coords: Float64Array, epsilon: number, min_points: number): Float64Array;

/**
 * Grid-based spatial clustering.
 *
 * Divides space into `cell_size`-sized grid cells. Cells with fewer than
 * `min_points` are discarded. Returns cluster centers as flat `Float64Array`.
 *
 * # Arguments
 * - `coords`: Flat `Float64Array` `[lng0, lat0, lng1, lat1, ...]`.
 * - `cell_size`: Grid cell size in meters.
 * - `min_points`: Minimum points per cell to form a valid cluster.
 *
 * # Returns
 * Flat `Float64Array` of cluster centers `[lng, lat, lng, lat, ...]`.
 */
export function clusterByGrid(coords: Float64Array, cell_size: number, min_points: number): Float64Array;

/**
 * Compute the axis-aligned bounding box of a set of 2D coordinates.
 *
 * Input: flat `Float64Array` of `[lng0, lat0, lng1, lat1, ...]`.
 * Output: `Float64Array` of `[minLng, minLat, maxLng, maxLat]`.
 *
 * Uses a manual 4-wide f64 comparison pattern for efficient vectorization
 * hints to the LLVM backend (effectively SIMD-style without explicit SIMD intrinsics).
 */
export function computeBounds(coords: Float64Array): Float64Array;

/**
 * Compute the merged bounding box of multiple coordinate buffers.
 *
 * Input: a JS `Array` of `Float64Array` coordinate buffers.
 * Output: `Float64Array` of `[minLng, minLat, maxLng, maxLat]`.
 *
 * Equivalent to calling `computeBounds` on each buffer individually
 * and then merging the results, but processes all buffers in a single pass
 * for better cache locality.
 */
export function computeBoundsMulti(buffers: Array<any>): Float64Array;

/**
 * Compute an approximate concave hull using alpha shape (simplified).
 *
 * # Arguments
 * - `coords`: Flat `Float64Array` `[lng0, lat0, lng1, lat1, ...]`.
 * - `alpha`: Controls concavity. Larger values → more convex (α → ∞ gives convex hull).
 *   Smaller values → more concave. Typical range: 0.1–10.0.
 *
 * # Returns
 * Flat `Float64Array` of concave hull vertices (closed: first == last).
 */
export function concaveHull(coords: Float64Array, alpha: number): Float64Array;

/**
 * Check if a point is inside a polygon using the `geo` crate's algorithm.
 *
 * Alias for `isPointInRing` using the robust `geo::Contains` trait.
 *
 * # Arguments
 *
 * * `outer_ring` — Flat `Float64Array` `[lng0,lat0, ...]`
 * * `point_x` — Point longitude
 * * `point_y` — Point latitude
 */
export function contains(outer_ring: Float64Array, point_x: number, point_y: number): boolean;

/**
 * Compute the convex hull of a set of 2D points using Andrew's monotone chain algorithm.
 *
 * # Arguments
 * - `coords`: Flat `Float64Array` `[lng0, lat0, lng1, lat1, ...]`.
 *
 * # Returns
 * Flat `Float64Array` of convex hull vertices (closed: first == last).
 */
export function convexHull(coords: Float64Array): Float64Array;

/**
 * Count GeoJSON features by property value (COUNT ... GROUP BY).
 *
 * Returns a JSON object mapping property values to their counts.
 *
 * # Arguments
 *
 * * `input` — GeoJSON string
 * * `key` — Property name to count by
 *
 * # Returns
 *
 * JSON string like `{"value1": 5, "value2": 3}`.
 */
export function countGeoJsonByProperty(input: string, key: string): string;

/**
 * Return the total number of features in a GeoJSON string.
 *
 * Useful for progress reporting before parsing a very large file.
 */
export function countGeoJsonFeatures(input: string): number;

/**
 * Return JSON info for a specific CRS code.
 *
 * # Arguments
 * - `code`: CRS code string, e.g. `"EPSG:4326"`, `"GCJ-02"`, `"BD-09"`.
 *
 * # Returns
 * JSON object with `name`, `description`, `bounds`, `unit`.
 */
export function crsInfo(code: string): string;

/**
 * Decode MVT (Mapbox Vector Tile) protobuf bytes into structured layer data.
 *
 * ## Parameters
 *
 * - `bytes` — Raw MVT protobuf bytes (typically from a `.pbf` tile file).
 *
 * ## Returns
 *
 * A `MvtLayer` (the first layer in the tile).
 *
 * ## Usage (JS)
 *
 * ```js
 * const response = await fetch('/tiles/10/868/387.pbf');
 * const buffer = await response.arrayBuffer();
 * const layer = decodeMvt(new Uint8Array(buffer));
 * console.log(layer.name(), layer.extent(), layer.featureCount());
 * const feat = layer.featureAt(0);
 * console.log(feat.geometryType(), feat.geometry());
 * ```
 */
export function decodeMvt(bytes: Uint8Array): MvtLayer;

/**
 * Decode an MVT tile and convert all features to a GeoJSON FeatureCollection string.
 *
 * ## Parameters
 *
 * - `bytes` — Raw MVT protobuf bytes.
 *
 * ## Returns
 *
 * A GeoJSON FeatureCollection string with all features from the first layer.
 * Coordinates are in tile space (0..extent).
 *
 * ## Usage (JS)
 *
 * ```js
 * const response = await fetch('/tiles/10/868/387.pbf');
 * const geojson = decodeMvtToGeoJson(new Uint8Array(await response.arrayBuffer()));
 * // geojson = '{"type":"FeatureCollection","features":[...]}'
 * ```
 */
export function decodeMvtToGeoJson(bytes: Uint8Array): string;

/**
 * Deduplicate coordinates within a tolerance.
 *
 * Keeps the first occurrence of each coordinate pair within `tolerance` distance.
 *
 * # Arguments
 *
 * * `coords` — Flat `Float64Array` `[lng0, lat0, lng1, lat1, …]`
 * * `tolerance` — Maximum distance (in coordinate units) for two points to be considered duplicates
 */
export function deduplicateCoords(coords: Float64Array, tolerance: number): Float64Array;

/**
 * Denormalize coordinates from [0,1] back to geographic coordinates.
 *
 * # Arguments
 *
 * * `normals` — Flat `Float64Array` of normalized coordinates in [0,1].
 * * `source_bounds` — `Float64Array` `[minLng, minLat, maxLng, maxLat]`.
 *
 * # Returns
 *
 * New `Float64Array` with denormalized geographic coordinates.
 */
export function denormalizeCoords(normals: Float64Array, source_bounds: Float64Array): Float64Array;

/**
 * Calculate the destination point given a start point, bearing, and distance.
 *
 * Uses the direct geodesic problem solution.
 *
 * # Arguments
 * - `lng`: Origin longitude in degrees.
 * - `lat`: Origin latitude in degrees.
 * - `bearing_deg`: Bearing in degrees (0 = North, 90 = East).
 * - `distance_m`: Distance in meters.
 *
 * Returns `Float64Array` `[lng, lat]` of the destination point.
 */
export function destination(lng: number, lat: number, bearing_deg: number, distance_m: number): Float64Array;

/**
 * Check if two polygons are disjoint (share no points at all).
 *
 * # Arguments
 *
 * * `ring1` — First polygon as flat closed ring
 * * `ring2` — Second polygon as flat closed ring
 */
export function disjoint(ring1: Float64Array, ring2: Float64Array): boolean;

/**
 * Encode a point cloud tile into 3D Tiles Point Cloud (pnts) binary format.
 *
 * # Arguments
 * * `positions` — `Float32Array` of `[x, y, z, ...]`.
 * * `center_x`, `center_y`, `center_z` — Tile center coordinates.
 * * `colors` — Optional `Uint8Array` of `[r, g, b, ...]`.
 *
 * Returns a `Uint8Array` containing the complete `.pnts` binary.
 */
export function encodePntsTile(positions: Float32Array, center_x: number, center_y: number, center_z: number, colors?: Uint8Array | null): Uint8Array;

/**
 * Filter GeoJSON features by bounding box.
 *
 * Keeps features that have at least one vertex inside the specified bbox.
 *
 * # Arguments
 *
 * * `input` — GeoJSON string
 * * `min_lng` — Minimum longitude
 * * `min_lat` — Minimum latitude
 * * `max_lng` — Maximum longitude
 * * `max_lat` — Maximum latitude
 *
 * # Returns
 *
 * Filtered GeoJSON FeatureCollection string.
 */
export function filterGeoJsonByBBox(input: string, min_lng: number, min_lat: number, max_lng: number, max_lat: number): string;

/**
 * Filter GeoJSON features by property value.
 *
 * # Arguments
 *
 * * `input` — GeoJSON string (Feature or FeatureCollection)
 * * `key` — Property name to filter on
 * * `value` — Property value to match (string representation)
 *
 * # Returns
 *
 * Filtered GeoJSON FeatureCollection string.
 */
export function filterGeoJsonByProperty(input: string, key: string, value: string): string;

/**
 * Generate a complete b3dm 3D Tile from GeoJSON polygons/multipolygons.
 *
 * Reuses `generate_cesium_geometry` internally for triangulation, then
 * wraps the result in the b3dm binary envelope suitable for Cesium's
 * `Cesium3DTileset`.
 */
export function generate3DTile(geojson_str: string, height_property?: string | null): Cesium3DTile;

/**
 * Generate triangulated mesh from GeoJSON Polygons/MultiPolygons.
 */
export function generateCesiumGeometry(geojson_str: string, height_property?: string | null): CesiumMeshGeometry;

/**
 * WASM export: generate a tileset from octree and point data.
 */
export function generateTileset(positions: Float32Array, max_points_per_node?: number | null, max_depth?: number | null, colors?: Uint8Array | null): TilesetResult;

/**
 * Generate a GeoJSON FeatureCollection string from multiple features.
 *
 * # Arguments
 *
 * * `coords` — Flat `Float64Array` with all feature coordinates concatenated
 * * `types` — Comma-separated geometry types (one per feature)
 * * `properties_json` — Properties for each feature, separated by `\x01` (unit separator).
 *   Each segment should be a valid JSON object string. Use `"{}"` for empty properties.
 *
 * # Returns
 *
 * A JSON string representing a GeoJSON FeatureCollection.
 *
 * # Example (JS)
 * ```js
 * const coords = new Float64Array([116.4, 39.9, 121.5, 31.2]);
 * const json = core.geoJsonFeatureCollection(coords, "Point,Point", '{"name":"BJ"}\x01{"name":"SH"}');
 * ```
 */
export function geoJsonFeatureCollection(coords: Float64Array, types: string, properties_json: string): string;

/**
 * Generate a standard GeoJSON Feature string from coordinate buffer and geometry type.
 *
 * # Arguments
 *
 * * `coords` — Flat `Float64Array` `[lng0, lat0, lng1, lat1, …]`
 * * `geometry_type` — One of: `"Point"`, `"LineString"`, `"Polygon"`, `"MultiPoint"`
 *
 * # Returns
 *
 * A JSON string representing a GeoJSON Feature.
 *
 * # Example (JS)
 * ```js
 * const coords = new Float64Array([116.404, 39.915]);
 * const json = core.geoJsonFromCoords(coords, "Point");
 * // json = '{"type":"Feature","geometry":{"type":"Point","coordinates":[116.404,39.915]},"properties":{}}'
 * ```
 */
export function geoJsonFromCoords(coords: Float64Array, geometry_type: string): string;

/**
 * Decode a Geohash string into `[longitude, latitude, width, height]`.
 *
 * Returns a `Float64Array` with:
 * - `[0]` center longitude
 * - `[1]` center latitude
 * - `[2]` bounding box width in degrees
 * - `[3]` bounding box height in degrees
 */
export function geohashDecode(hash: string): Float64Array;

/**
 * Encode (longitude, latitude) to a Geohash string with given precision (1-12).
 */
export function geohashEncode(lng: number, lat: number, precision: number): string;

/**
 * Get the 8 neighboring Geohash cells (N, NE, E, SE, S, SW, W, NW).
 *
 * Returns a `JsValue` (Array) of 8 Geohash strings.
 */
export function geohashNeighbors(hash: string): Array<any>;

/**
 * Get the approximate number of allocated bytes in WASM linear memory.
 *
 * This reads the current `memory.buffer.byteLength`. Note that WASM memory
 * only grows (never shrinks), so this value is the peak allocation size.
 *
 * Returns 0 on non-WASM targets.
 */
export function getAllocatedBytes(): number;

/**
 * Get the current input size limit in bytes.
 *
 * Returns 100 MB (104,857,600) if not changed.
 */
export function getInputSizeLimit(): number;

/**
 * Return a JSON array of supported coordinate reference systems.
 *
 * Each entry contains `code`, `name`, `description`.
 */
export function getSupportedCrs(): string;

/**
 * Assign each point to a spatial grid cell.
 *
 * # Arguments
 *
 * * `coords` — Flat `Float64Array` `[lng0, lat0, lng1, lat1, …]`
 * * `cell_size_deg` — Grid cell size in degrees
 *
 * # Returns
 *
 * `Float64Array` with one grid ID per point. Points in the same cell get the same ID.
 */
export function gridIndex(coords: Float64Array, cell_size_deg: number): Float64Array;

/**
 * Calculate the Haversine distance between two WGS-84 points in meters.
 *
 * # Arguments
 * - `lng1`: Longitude of point 1 in degrees.
 * - `lat1`: Latitude of point 1 in degrees.
 * - `lng2`: Longitude of point 2 in degrees.
 * - `lat2`: Latitude of point 2 in degrees.
 *
 * Returns the great-circle distance in meters.
 */
export function haversineDistance(lng1: number, lat1: number, lng2: number, lat2: number): number;

/**
 * Initialize the WASM module. Call this once before any other function.
 *
 * Sets up the panic hook for better error messages in the browser console.
 */
export function init(): void;

/**
 * Check whether a coordinate falls within China's approximate bounding box.
 *
 * Uses the same bounds as the GCJ-02 offset check: lng ∈ [73.66, 135.05],
 * lat ∈ [3.86, 53.55].
 *
 * # Arguments
 * - `lng`: Longitude in degrees.
 * - `lat`: Latitude in degrees.
 *
 * # Returns
 * `true` if the coordinate is within China's approximate territory.
 */
export function isInChina(lng: number, lat: number): boolean;

/**
 * Test if a point is inside a polygon ring using the ray-casting algorithm.
 *
 * # Arguments
 * - `point_x`: Longitude of the test point.
 * - `point_y`: Latitude of the test point.
 * - `ring_coords`: Flat `Float64Array` `[lng0,lat0, lng1,lat1, ...]` defining the ring.
 *   The ring does **not** need to be explicitly closed.
 *
 * Returns `true` if the point is inside the ring.
 */
export function isPointInRing(point_x: number, point_y: number, ring_coords: Float64Array): boolean;

/**
 *
 * Provides insight into WASM linear memory allocation, useful for monitoring
 * large spatial data processing workloads.
 *
 * **Note:** Only available in WASM runtime. On native, returns zeros.
 */
export function memoryInfo(): MemoryInfo;

/**
 * Calculate the midpoint between two WGS-84 points on the great circle.
 *
 * # Arguments
 * - `lng1`: Longitude of point 1 in degrees.
 * - `lat1`: Latitude of point 1 in degrees.
 * - `lng2`: Longitude of point 2 in degrees.
 * - `lat2`: Latitude of point 2 in degrees.
 *
 * Returns `Float64Array` `[lng, lat]` of the midpoint.
 */
export function midpoint(lng1: number, lat1: number, lng2: number, lat2: number): Float64Array;

/**
 * Normalize coordinates to [0,1] range.
 *
 * # Arguments
 *
 * * `coords` — Flat `Float64Array` `[lng0, lat0, lng1, lat1, …]`
 * * `target_bounds` — Optional `Float64Array` `[minLng, minLat, maxLng, maxLat]`.
 *   If not provided, bounds are computed automatically from the data.
 *
 * # Returns
 *
 * New `Float64Array` with coordinates mapped to [0,1].
 */
export function normalizeCoords(coords: Float64Array, target_bounds: Float64Array): Float64Array;

/**
 * Parse a GeoJSON string and return **all** coordinate pairs as a flat
 * `Float64Array` — `[lng0, lat0, lng1, lat1, …]`.
 *
 * This is designed for bulk ingestion of large datasets; the flat layout
 * allows direct upload to a GPU vertex buffer with no further processing.
 *
 * # Errors
 *
 * Returns a `SpatialErrorDetail` if the input is not valid GeoJSON.
 */
export function parseGeoJsonCoords(input: string): Float64Array;

/**
 * Parse a GeoJSON string and return structured per-feature results including
 * coordinates, offsets, counts, and geometry types.
 *
 * This is useful when you need to iterate features individually while still
 * benefitting from a single-pass parse.
 *
 * # Errors
 *
 * Returns a `SpatialErrorDetail` if the input is not valid GeoJSON.
 */
export function parseGeoJsonFeatures(input: string): GeoJsonFeaturesResult;

/**
 * Create a lazy GeoJSON FeatureCollection iterator.
 *
 * Accepts a `&str` (one-shot input) but uses a manual JSON state machine
 * internally to parse features one at a time. Memory peak is O(single feature)
 * rather than O(all features).
 *
 * ## Parameters
 *
 * - `input` — A GeoJSON FeatureCollection string.
 *
 * ## Returns
 *
 * A `LazyGeoJsonIter` that you can call `.nextFeature()` on repeatedly.
 *
 * ## Error
 *
 * Returns `JsValue` error if the input is not valid JSON or not a FeatureCollection.
 */
export function parseGeoJsonLazy(input: string): LazyGeoJsonIter;

/**
 * Parse a GeoJSON FeatureCollection and return coordinates in separate
 * per-feature arrays, useful when you need to map coordinates back to
 * individual features.
 *
 * Returns a `js_sys::Array` where each element is a `Float64Array`
 * containing the coordinates for one feature.
 */
export function parseGeoJsonPerFeature(input: string): Array<any>;

/**
 * Extract all feature properties from a GeoJSON string as a JSON string.
 *
 * Returns a JSON array of property objects. Features without properties
 * are represented as `null` entries.
 *
 * # Example
 * ```js
 * const props = JSON.parse(core.parseGeoJsonProperties(geojsonStr));
 * // props = [{ name: "Beijing", population: 21540000 }, { ... }]
 * ```
 *
 * # Errors
 *
 * Returns a `SpatialErrorDetail` if the input is not valid GeoJSON.
 */
export function parseGeoJsonProperties(input: string): string;

/**
 * Parse a GeoJSON FeatureCollection in chunks, calling `on_chunk` with
 * each batch of coordinate data and progress information.
 *
 * ## Parameters
 *
 * - `input` — The full GeoJSON string (must be a FeatureCollection).
 * - `chunk_size` — Number of features to process per chunk (e.g. 1000).
 *   Larger chunks = fewer JS↔WASM transitions but longer UI blocking.
 * - `on_chunk` — A JS callback: `(coords: Float64Array, processed: u32, total: u32) => void`
 *
 * ## Usage (JS)
 *
 * ```js
 * parseGeoJsonStream(hugeGeoJson, 500, (coords, processed, total) => {
 *   // Upload coords to GPU, update progress bar
 *   progressBar.value = processed / total;
 *   gl.bufferSubData(gl.ARRAY_BUFFER, offset, coords);
 * });
 * ```
 *
 * ## Design Rationale
 *
 * Standard JSON parsers (serde_json) build the full DOM in memory.
 * For a 200 MB FeatureCollection this costs ~400 MB WASM heap.
 *
 * This function first parses the full FeatureCollection (unavoidable with
 * the `geojson` crate), but then processes and emits features in chunks,
 * allowing the JS side to consume and discard coordinate data incrementally
 * rather than holding all coordinates in memory at once.
 *
 * For true streaming (constant memory), a custom tokeniser would be needed.
 * This is planned for a future release using `serde_json::StreamDeserializer`
 * over raw bytes.
 */
export function parseGeoJsonStream(input: string, chunk_size: number, on_chunk: Function): number;

/**
 * Parse IFC-SPF text and extract mesh geometry from IFCEXTRUDEDAREASOLID entities.
 *
 * This is an **experimental** feature that extracts a practical subset of IFC geometry:
 * - `IFCEXTRUDEDAREASOLID` entities are triangulated into indexed meshes
 * - Associated `IFCPOLYLINE` profiles provide the cross-section
 * - `IFCDIRECTION` and `IFCAXIS2PLACEMENT3D` define extrusion direction and position
 *
 * Returns an `IfcGeometryResult` containing all extracted meshes.
 *
 * # Arguments
 *
 * * `text` - The full IFC-SPF file content as a UTF-8 string.
 *
 * # Example
 *
 * ```ignore
 * let result = parse_ifc_geometry(ifc_text);
 * console.log(`Extracted ${result.meshCount()} meshes`);
 * ```
 */
export function parseIfcGeometry(text: string): IfcGeometryResult;

/**
 * Parse Well-Known Binary (WKB) data into a flat `Float64Array`.
 *
 * Supports 2D POINT, LINESTRING, POLYGON, MULTIPOINT.
 * Byte order is auto-detected (little-endian or big-endian).
 *
 * # Arguments
 * - `bytes`: `Uint8Array` containing WKB data.
 *
 * # Example
 * ```js
 * const coords = parseWkb(new Uint8Array(wkbBuffer));
 * ```
 */
export function parseWkb(bytes: Uint8Array): Float64Array;

/**
 * Parse a Well-Known Text (WKT) string into a flat `Float64Array`.
 *
 * Supports: POINT, LINESTRING, POLYGON, MULTIPOINT.
 *
 * # Arguments
 * - `input`: WKT string (case-insensitive).
 *
 * # Returns
 * Flat `[lng0, lat0, lng1, lat1, ...]` coordinates.
 *
 * # Example
 * ```js
 * const coords = parseWkt("LINESTRING(0 0, 10 10, 20 0)");
 * ```
 */
export function parseWkt(input: string): Float64Array;

/**
 * Calculate the area of a polygon in square meters using the spherical
 * excess formula.
 *
 * # Arguments
 * - `coords`: Flat `Float64Array` of a closed ring `[lng0,lat0, lng1,lat1, ..., lng0,lat0]`.
 *
 * For polygons with holes, use `areaWithHoles` instead.
 */
export function polygonArea(coords: Float64Array): number;

/**
 * Compute the intersection of two simple polygons.
 *
 * # Arguments
 *
 * * `ring1` — First polygon as flat closed ring `[lng0,lat0, ..., lng0,lat0]`
 * * `ring2` — Second polygon as flat closed ring `[lng0,lat0, ..., lng0,lat0]`
 *
 * # Returns
 *
 * A `Float64Array` with the intersection ring(s). Empty if polygons don't intersect.
 */
export function polygonIntersection(ring1: Float64Array, ring2: Float64Array): Float64Array;

/**
 * Check if two polygons intersect (share any point).
 *
 * # Arguments
 *
 * * `ring1` — First polygon as flat closed ring
 * * `ring2` — Second polygon as flat closed ring
 */
export function polygonIntersects(ring1: Float64Array, ring2: Float64Array): boolean;

/**
 * Compute the union of two simple polygons.
 *
 * # Arguments
 *
 * * `ring1` — First polygon as flat closed ring `[lng0,lat0, ..., lng0,lat0]`
 * * `ring2` — Second polygon as flat closed ring `[lng0,lat0, ..., lng0,lat0]`
 *
 * # Returns
 *
 * A `Float64Array` with the union ring(s).
 */
export function polygonUnion(ring1: Float64Array, ring2: Float64Array): Float64Array;

/**
 * Calculate the total length of a line string or polygon perimeter in meters
 * using the Haversine formula.
 *
 * # Arguments
 * - `coords`: Flat `Float64Array` `[lng0,lat0, lng1,lat1, ...]`.
 */
export function polylineLength(coords: Float64Array): number;

/**
 * Remove a property key from all features of a GeoJSON FeatureCollection.
 *
 * If a feature doesn't have the key, it's silently skipped.
 *
 * # Arguments
 *
 * * `input` — GeoJSON string.
 * * `key` — Property key to remove.
 *
 * # Returns
 *
 * Modified GeoJSON string with the property removed.
 */
export function removeProperty(input: string, key: string): string;

/**
 * Rename a property key in all features of a GeoJSON FeatureCollection.
 *
 * If a feature doesn't have the old key, it's silently skipped.
 *
 * # Arguments
 *
 * * `input` — GeoJSON string.
 * * `old_key` — Current property key name.
 * * `new_key` — New property key name.
 *
 * # Returns
 *
 * Modified GeoJSON string with the property renamed.
 */
export function renameProperty(input: string, old_key: string, new_key: string): string;

/**
 * Rhumb (constant-bearing) bearing from point 1 to point 2.
 *
 * # Arguments
 * - `lng1`, `lat1`: Point 1 in degrees.
 * - `lng2`, `lat2`: Point 2 in degrees.
 *
 * # Returns
 * Bearing in degrees [0, 360), where 0 = North, 90 = East.
 */
export function rhumbBearing(lng1: number, lat1: number, lng2: number, lat2: number): number;

/**
 * Rhumb (loxodrome/constant-bearing) distance between two WGS-84 points.
 *
 * Used in maritime and aviation navigation.
 *
 * # Arguments
 * - `lng1`, `lat1`: Point 1 in degrees.
 * - `lng2`, `lat2`: Point 2 in degrees.
 *
 * # Returns
 * Distance in meters.
 */
export function rhumbDistance(lng1: number, lat1: number, lng2: number, lat2: number): number;

/**
 * Dynamically set the maximum allowed input size in bytes.
 *
 * Default is 100 MB. Set to 0 to disable the limit.
 *
 * # Example (JS)
 * ```js
 * core.setInputSizeLimit(50 * 1024 * 1024); // 50 MB
 * ```
 */
export function setInputSizeLimit(bytes: number): void;

/**
 * Simplify a line string using the Douglas-Peucker algorithm.
 *
 * # Arguments
 * - `coords`: Flat `Float64Array` `[lng0,lat0, lng1,lat1, ...]`.
 * - `tolerance`: Simplification tolerance in **radians**.
 *   For typical geographic data, `0.0001` ≈ ~11 m at the equator.
 *
 * Returns simplified `Float64Array` `[lng0,lat0, ...]` preserving the first
 * and last points.
 */
export function simplifyDouglasPeucker(coords: Float64Array, tolerance: number): Float64Array;

/**
 * Sort coordinate pairs by latitude (keeping lng,lat pairs together).
 *
 * # Arguments
 *
 * * `coords` — Flat `Float64Array` `[lng0, lat0, lng1, lat1, …]`
 *
 * # Returns
 *
 * New sorted `Float64Array`.
 */
export function sortCoordsByLat(coords: Float64Array): Float64Array;

/**
 * Sort coordinate pairs by longitude (keeping lng,lat pairs together).
 *
 * # Arguments
 *
 * * `coords` — Flat `Float64Array` `[lng0, lat0, lng1, lat1, …]`
 *
 * # Returns
 *
 * New sorted `Float64Array`.
 */
export function sortCoordsByLng(coords: Float64Array): Float64Array;

/**
 * Interpolate a Z value on a TIN surface at (x, y) using barycentric interpolation.
 *
 * Finds the triangle containing (x, y) and interpolates Z.
 * If the point is outside the TIN convex hull, returns the Z of the nearest vertex.
 */
export function tinInterpolate(tin: TinResult, x: number, y: number): number;

/**
 * Generate Well-Known Binary (WKB) from coordinates.
 *
 * Produces little-endian WKB (byte order = 1).
 *
 * # Arguments
 * - `coords`: Flat `Float64Array` `[lng0, lat0, lng1, lat1, ...]`.
 * - `geometry_type`: `"POINT"`, `"LINESTRING"`, `"POLYGON"`, `"MULTIPOINT"`.
 *
 * # Example
 * ```js
 * const wkb = toWkb(coords, "LINESTRING");
 * ```
 */
export function toWkb(coords: Float64Array, geometry_type: string): Uint8Array;

/**
 * Generate a Well-Known Text (WKT) string from coordinates.
 *
 * # Arguments
 * - `coords`: Flat `Float64Array` `[lng0, lat0, lng1, lat1, ...]`.
 * - `geometry_type`: Geometry type string: `"POINT"`, `"LINESTRING"`,
 *   `"POLYGON"`, `"MULTIPOINT"`.
 *
 * # Example
 * ```js
 * const wkt = toWkt(coords, "LINESTRING");
 * ```
 */
export function toWkt(coords: Float64Array, geometry_type: string): string;

/**
 * Check if two polygons touch (share boundary but not interior).
 *
 * # Arguments
 *
 * * `ring1` — First polygon as flat closed ring
 * * `ring2` — Second polygon as flat closed ring
 */
export function touches(ring1: Float64Array, ring2: Float64Array): boolean;

/**
 * Convert a single UTM coordinate to WGS84.
 *
 * # Arguments
 *
 * * `zone` — UTM zone number (1-60).
 * * `easting` — Easting in meters.
 * * `northing` — Northing in meters.
 * * `is_north` — `true` for northern hemisphere, `false` for southern.
 *
 * # Returns
 *
 * `Float64Array` with `[longitude, latitude]` in degrees.
 */
export function utmToWgs84(zone: number, easting: number, northing: number, is_north: boolean): Float64Array;

/**
 * Validate coordinate values against the expected range for a given CRS.
 *
 * # Arguments
 *
 * * `coords` — Flat `Float64Array` `[lng0, lat0, lng1, lat1, …]`
 * * `crs` — One of: `"WGS84"`, `"GCJ02"`, `"BD09"`, `"Mercator"`
 *
 * # Returns
 *
 * A `ValidationResult` with valid count, invalid count, and indices of invalid pairs.
 */
export function validateCoords(coords: Float64Array, crs: string): ValidationResult;

/**
 * Return the library version string.
 */
export function version(): string;

/**
 * Vincenty inverse formula — geodesic distance between two points on the WGS-84 ellipsoid.
 *
 * More accurate than Haversine for long distances (sub-millimeter accuracy).
 *
 * # Arguments
 * - `lng1`, `lat1`: Point 1 in degrees.
 * - `lng2`, `lat2`: Point 2 in degrees.
 *
 * # Returns
 * Distance in meters. Returns `f64::NAN` if the points are antipodal (no convergence).
 */
export function vincentyDistance(lng1: number, lat1: number, lng2: number, lat2: number): number;

/**
 * Convert a single WGS84 coordinate to UTM.
 *
 * # Arguments
 *
 * * `lng` — Longitude in degrees.
 * * `lat` — Latitude in degrees.
 *
 * # Returns
 *
 * `Float64Array` with `[zone_number, easting, northing, is_north]`.
 * - `zone_number`: UTM zone (1-60)
 * - `easting`: Easting in meters (false easting + 500,000 applied)
 * - `northing`: Northing in meters
 * - `is_north`: 1.0 for northern hemisphere, 0.0 for southern
 */
export function wgs84ToUtm(lng: number, lat: number): Float64Array;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_cesium3dtile_free: (a: number, b: number) => void;
    readonly __wbg_cesiummeshgeometry_free: (a: number, b: number) => void;
    readonly __wbg_geojsonfeaturesresult_free: (a: number, b: number) => void;
    readonly __wbg_get_vectortileoptions_buffer: (a: number) => number;
    readonly __wbg_get_vectortileoptions_extent: (a: number) => number;
    readonly __wbg_get_vectortileoptions_generate_id: (a: number) => number;
    readonly __wbg_get_vectortileoptions_index_max_points: (a: number) => number;
    readonly __wbg_get_vectortileoptions_index_max_zoom: (a: number) => number;
    readonly __wbg_get_vectortileoptions_line_metrics: (a: number) => number;
    readonly __wbg_get_vectortileoptions_max_zoom: (a: number) => number;
    readonly __wbg_get_vectortileoptions_tolerance: (a: number) => number;
    readonly __wbg_gltfbuilder_free: (a: number, b: number) => void;
    readonly __wbg_ifcgeometryresult_free: (a: number, b: number) => void;
    readonly __wbg_lazygeojsoniter_free: (a: number, b: number) => void;
    readonly __wbg_memoryinfo_free: (a: number, b: number) => void;
    readonly __wbg_mvtfeature_free: (a: number, b: number) => void;
    readonly __wbg_mvtlayer_free: (a: number, b: number) => void;
    readonly __wbg_octree_free: (a: number, b: number) => void;
    readonly __wbg_set_vectortileoptions_buffer: (a: number, b: number) => void;
    readonly __wbg_set_vectortileoptions_extent: (a: number, b: number) => void;
    readonly __wbg_set_vectortileoptions_generate_id: (a: number, b: number) => void;
    readonly __wbg_set_vectortileoptions_index_max_points: (a: number, b: number) => void;
    readonly __wbg_set_vectortileoptions_index_max_zoom: (a: number, b: number) => void;
    readonly __wbg_set_vectortileoptions_line_metrics: (a: number, b: number) => void;
    readonly __wbg_set_vectortileoptions_max_zoom: (a: number, b: number) => void;
    readonly __wbg_set_vectortileoptions_tolerance: (a: number, b: number) => void;
    readonly __wbg_spatialedgeindex_free: (a: number, b: number) => void;
    readonly __wbg_spatialindex_free: (a: number, b: number) => void;
    readonly __wbg_tilesetresult_free: (a: number, b: number) => void;
    readonly __wbg_validationresult_free: (a: number, b: number) => void;
    readonly __wbg_vectortileengine_free: (a: number, b: number) => void;
    readonly __wbg_vectortileoptions_free: (a: number, b: number) => void;
    readonly addProperty: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly areaWithHoles: (a: number, b: number, c: number) => void;
    readonly batchBd09ToGcj02: (a: number) => number;
    readonly batchBd09ToGcj02InPlace: (a: number, b: number, c: number) => void;
    readonly batchBd09ToWgs84: (a: number) => number;
    readonly batchBd09ToWgs84InPlace: (a: number, b: number, c: number) => void;
    readonly batchGcj02ToBd09: (a: number) => number;
    readonly batchGcj02ToBd09InPlace: (a: number, b: number, c: number) => void;
    readonly batchGcj02ToWgs84: (a: number) => number;
    readonly batchGcj02ToWgs84InPlace: (a: number, b: number, c: number) => void;
    readonly batchMercatorToWgs84: (a: number) => number;
    readonly batchMercatorToWgs84InPlace: (a: number, b: number, c: number) => void;
    readonly batchUtmToWgs84: (a: number) => number;
    readonly batchUtmToWgs84InPlace: (a: number) => void;
    readonly batchWgs84ToBd09: (a: number) => number;
    readonly batchWgs84ToBd09InPlace: (a: number, b: number, c: number) => void;
    readonly batchWgs84ToBd09Mercator: (a: number) => number;
    readonly batchWgs84ToBd09MercatorInPlace: (a: number, b: number, c: number) => void;
    readonly batchWgs84ToCartesian3: (a: number, b: number) => number;
    readonly batchWgs84ToCgcs2000: (a: number) => number;
    readonly batchWgs84ToCgcs2000InPlace: (a: number, b: number, c: number) => void;
    readonly batchWgs84ToGcj02: (a: number) => number;
    readonly batchWgs84ToGcj02InPlace: (a: number, b: number, c: number) => void;
    readonly batchWgs84ToGcj02Mercator: (a: number) => number;
    readonly batchWgs84ToGcj02MercatorInPlace: (a: number, b: number, c: number) => void;
    readonly batchWgs84ToMercator: (a: number) => number;
    readonly batchWgs84ToMercatorInPlace: (a: number, b: number, c: number) => void;
    readonly batchWgs84ToUtm: (a: number) => number;
    readonly batchWgs84ToUtmInPlace: (a: number) => void;
    readonly bearing: (a: number, b: number, c: number, d: number) => number;
    readonly bestCrsForRegion: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly boundingBox: (a: number) => number;
    readonly bufferLineString: (a: number, b: number, c: number) => number;
    readonly bufferPoint: (a: number, b: number, c: number, d: number) => number;
    readonly buildOctree: (a: number, b: number, c: number, d: number) => number;
    readonly buildTin: (a: number, b: number) => void;
    readonly centroid: (a: number) => number;
    readonly cesium3dtile_batchTableJson: (a: number, b: number) => void;
    readonly cesium3dtile_featureBatchIds: (a: number) => number;
    readonly cesium3dtile_toBytes: (a: number) => number;
    readonly cesiummeshgeometry_indices: (a: number) => number;
    readonly cesiummeshgeometry_positions: (a: number) => number;
    readonly cgcs2000IsWgs84Compatible: () => number;
    readonly cleanCoords: (a: number, b: number, c: number, d: number) => void;
    readonly clusterByDensity: (a: number, b: number, c: number) => number;
    readonly clusterByGrid: (a: number, b: number, c: number) => number;
    readonly computeBounds: (a: number) => number;
    readonly computeBoundsMulti: (a: number) => number;
    readonly concaveHull: (a: number, b: number) => number;
    readonly contains: (a: number, b: number, c: number) => number;
    readonly convexHull: (a: number) => number;
    readonly countGeoJsonByProperty: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly countGeoJsonFeatures: (a: number, b: number, c: number) => void;
    readonly crsInfo: (a: number, b: number, c: number) => void;
    readonly decodeMvt: (a: number, b: number) => void;
    readonly decodeMvtToGeoJson: (a: number, b: number) => void;
    readonly deduplicateCoords: (a: number, b: number) => number;
    readonly denormalizeCoords: (a: number, b: number) => number;
    readonly destination: (a: number, b: number, c: number, d: number) => number;
    readonly disjoint: (a: number, b: number) => number;
    readonly encodePntsTile: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => void;
    readonly filterGeoJsonByBBox: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly filterGeoJsonByProperty: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly generate3DTile: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly generateCesiumGeometry: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly generateTileset: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly geoJsonFeatureCollection: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly geoJsonFromCoords: (a: number, b: number, c: number, d: number) => void;
    readonly geohashDecode: (a: number, b: number) => number;
    readonly geohashEncode: (a: number, b: number, c: number, d: number) => void;
    readonly geohashNeighbors: (a: number, b: number) => number;
    readonly geojsonfeaturesresult_coordinates: (a: number) => number;
    readonly geojsonfeaturesresult_counts: (a: number) => number;
    readonly geojsonfeaturesresult_offsets: (a: number) => number;
    readonly geojsonfeaturesresult_types: (a: number, b: number) => void;
    readonly getAllocatedBytes: () => number;
    readonly getInputSizeLimit: () => number;
    readonly getSupportedCrs: (a: number) => void;
    readonly gltfbuilder_addMaterial: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly gltfbuilder_addMesh: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly gltfbuilder_new: () => number;
    readonly gltfbuilder_toGlb: (a: number) => number;
    readonly gltfbuilder_toGltfJson: (a: number, b: number) => void;
    readonly gridIndex: (a: number, b: number) => number;
    readonly haversineDistance: (a: number, b: number, c: number, d: number) => number;
    readonly ifcgeometryresult_meshCount: (a: number) => number;
    readonly ifcgeometryresult_meshes: (a: number) => number;
    readonly ifcmesh_triangleCount: (a: number) => number;
    readonly ifcmesh_vertexCount: (a: number) => number;
    readonly init: () => void;
    readonly isInChina: (a: number, b: number) => number;
    readonly isPointInRing: (a: number, b: number, c: number) => number;
    readonly lazygeojsoniter_nextFeature: (a: number) => number;
    readonly lazygeojsoniter_remaining: (a: number) => number;
    readonly lazygeojsoniter_total: (a: number) => number;
    readonly memoryInfo: () => number;
    readonly memoryinfo_remaining: (a: number) => number;
    readonly memoryinfo_total: (a: number) => number;
    readonly memoryinfo_used: (a: number) => number;
    readonly midpoint: (a: number, b: number, c: number, d: number) => number;
    readonly mvtfeaturedecoded_geometry: (a: number) => number;
    readonly mvtfeaturedecoded_geometry_type: (a: number) => number;
    readonly mvtfeaturedecoded_id: (a: number) => number;
    readonly mvtfeaturedecoded_tagCount: (a: number) => number;
    readonly mvtfeaturedecoded_tagKey: (a: number, b: number, c: number) => void;
    readonly mvtfeaturedecoded_tagValue: (a: number, b: number, c: number) => void;
    readonly mvtlayerdecoded_extent: (a: number) => number;
    readonly mvtlayerdecoded_featureAt: (a: number, b: number) => number;
    readonly mvtlayerdecoded_featureCount: (a: number) => number;
    readonly mvtlayerdecoded_name: (a: number, b: number) => void;
    readonly normalizeCoords: (a: number, b: number) => number;
    readonly octree_depth: (a: number) => number;
    readonly octree_nodeBounds: (a: number, b: number) => number;
    readonly octree_nodeChildren: (a: number, b: number) => number;
    readonly octree_nodeCount: (a: number) => number;
    readonly octree_nodeLevel: (a: number, b: number) => number;
    readonly octree_nodePointCount: (a: number, b: number) => number;
    readonly octree_rootBounds: (a: number) => number;
    readonly octree_total_points: (a: number) => number;
    readonly parseGeoJsonCoords: (a: number, b: number, c: number) => void;
    readonly parseGeoJsonFeatures: (a: number, b: number, c: number) => void;
    readonly parseGeoJsonLazy: (a: number, b: number, c: number) => void;
    readonly parseGeoJsonPerFeature: (a: number, b: number, c: number) => void;
    readonly parseGeoJsonProperties: (a: number, b: number, c: number) => void;
    readonly parseGeoJsonStream: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly parseIfcGeometry: (a: number, b: number) => number;
    readonly parseWkb: (a: number, b: number) => void;
    readonly parseWkt: (a: number, b: number, c: number) => void;
    readonly polygonArea: (a: number, b: number) => void;
    readonly polygonIntersection: (a: number, b: number) => number;
    readonly polygonIntersects: (a: number, b: number) => number;
    readonly polygonUnion: (a: number, b: number) => number;
    readonly polylineLength: (a: number, b: number) => void;
    readonly removeProperty: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly renameProperty: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly rhumbBearing: (a: number, b: number, c: number, d: number) => number;
    readonly rhumbDistance: (a: number, b: number, c: number, d: number) => number;
    readonly setInputSizeLimit: (a: number) => void;
    readonly simplifyDouglasPeucker: (a: number, b: number) => number;
    readonly sortCoordsByLat: (a: number) => number;
    readonly sortCoordsByLng: (a: number) => number;
    readonly spatialedgeindex_nearestNeighbor: (a: number, b: number, c: number) => number;
    readonly spatialedgeindex_new: (a: number) => number;
    readonly spatialedgeindex_searchBBox: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly spatialedgeindex_size: (a: number) => number;
    readonly spatialindex_kNearestNeighbors: (a: number, b: number, c: number, d: number) => number;
    readonly spatialindex_nearestNeighbor: (a: number, b: number, c: number) => number;
    readonly spatialindex_new: (a: number) => number;
    readonly spatialindex_searchBBox: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly spatialindex_size: (a: number) => number;
    readonly tilesetresult_tile: (a: number, b: number) => number;
    readonly tilesetresult_tileBounds: (a: number, b: number) => number;
    readonly tilesetresult_tileUri: (a: number, b: number, c: number) => void;
    readonly tilesetresult_tile_count: (a: number) => number;
    readonly tilesetresult_tilesetJson: (a: number, b: number) => void;
    readonly tilesetresult_total_bytes: (a: number) => number;
    readonly tinInterpolate: (a: number, b: number, c: number) => number;
    readonly toWkb: (a: number, b: number, c: number, d: number) => void;
    readonly toWkt: (a: number, b: number, c: number, d: number) => void;
    readonly touches: (a: number, b: number) => number;
    readonly utmToWgs84: (a: number, b: number, c: number, d: number) => number;
    readonly validateCoords: (a: number, b: number, c: number, d: number) => void;
    readonly validationresult_invalid_count: (a: number) => number;
    readonly validationresult_invalid_indices: (a: number) => number;
    readonly validationresult_valid_count: (a: number) => number;
    readonly vectortileengine_cacheSize: (a: number) => number;
    readonly vectortileengine_clearTileCache: (a: number) => void;
    readonly vectortileengine_getTile: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly vectortileengine_getTileCached: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly vectortileengine_layer_name: (a: number, b: number) => void;
    readonly vectortileengine_new: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly vectortileengine_set_layer_name: (a: number, b: number, c: number) => void;
    readonly vectortileoptions_new: () => number;
    readonly version: (a: number) => void;
    readonly vincentyDistance: (a: number, b: number, c: number, d: number) => number;
    readonly wgs84ToUtm: (a: number, b: number) => number;
    readonly __wbg_tinresult_free: (a: number, b: number) => void;
    readonly __wbg_ifcmesh_free: (a: number, b: number) => void;
    readonly tinresult_vertexCount: (a: number) => number;
    readonly tinresult_triangleCount: (a: number) => number;
    readonly tinresult_indices: (a: number) => number;
    readonly tinresult_positions: (a: number) => number;
    readonly ifcmesh_positions: (a: number) => number;
    readonly ifcmesh_indices: (a: number) => number;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number) => void;
    readonly __wbindgen_export4: (a: number, b: number, c: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
