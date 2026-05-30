/* @ts-self-types="./wasm_spatial_core.d.ts" */

/**
 * A Cesium 3D Tiles b3dm tile containing a triangulated batched mesh.
 */
export class Cesium3DTile {
    static __wrap(ptr) {
        const obj = Object.create(Cesium3DTile.prototype);
        obj.__wbg_ptr = ptr;
        Cesium3DTileFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        Cesium3DTileFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_cesium3dtile_free(ptr, 0);
    }
    /**
     * @returns {string}
     */
    get batchTableJson() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.cesium3dtile_batchTableJson(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_export4(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {Uint32Array}
     */
    get featureBatchIds() {
        const ret = wasm.cesium3dtile_featureBatchIds(this.__wbg_ptr);
        return takeObject(ret);
    }
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
     * @returns {Uint8Array}
     */
    toBytes() {
        const ret = wasm.cesium3dtile_toBytes(this.__wbg_ptr);
        return takeObject(ret);
    }
}
if (Symbol.dispose) Cesium3DTile.prototype[Symbol.dispose] = Cesium3DTile.prototype.free;

/**
 * Contains triangulated mesh data ready for Cesium.Geometry
 */
export class CesiumMeshGeometry {
    static __wrap(ptr) {
        const obj = Object.create(CesiumMeshGeometry.prototype);
        obj.__wbg_ptr = ptr;
        CesiumMeshGeometryFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        CesiumMeshGeometryFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_cesiummeshgeometry_free(ptr, 0);
    }
    /**
     * @returns {Uint32Array}
     */
    get indices() {
        const ret = wasm.cesiummeshgeometry_indices(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
     * @returns {Float64Array}
     */
    get positions() {
        const ret = wasm.cesiummeshgeometry_positions(this.__wbg_ptr);
        return takeObject(ret);
    }
}
if (Symbol.dispose) CesiumMeshGeometry.prototype[Symbol.dispose] = CesiumMeshGeometry.prototype.free;

/**
 * Result of structured GeoJSON feature parsing.
 *
 * Contains per-feature coordinate buffers, offsets, counts, and geometry types.
 */
export class GeoJsonFeaturesResult {
    static __wrap(ptr) {
        const obj = Object.create(GeoJsonFeaturesResult.prototype);
        obj.__wbg_ptr = ptr;
        GeoJsonFeaturesResultFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        GeoJsonFeaturesResultFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_geojsonfeaturesresult_free(ptr, 0);
    }
    /**
     * All coordinates as a flat `Float64Array`.
     * @returns {Float64Array}
     */
    get coordinates() {
        const ret = wasm.geojsonfeaturesresult_coordinates(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
     * Per-feature coordinate pair count.
     * @returns {Uint32Array}
     */
    get counts() {
        const ret = wasm.geojsonfeaturesresult_counts(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
     * Per-feature starting offset into the coordinate buffer.
     * @returns {Uint32Array}
     */
    get offsets() {
        const ret = wasm.geojsonfeaturesresult_offsets(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
     * Comma-separated geometry type for each feature.
     * @returns {string}
     */
    get types() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.geojsonfeaturesresult_types(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_export4(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) GeoJsonFeaturesResult.prototype[Symbol.dispose] = GeoJsonFeaturesResult.prototype.free;

/**
 * glTF 2.0 builder — collect meshes and materials, then export as GLB or JSON.
 */
export class GltfBuilder {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        GltfBuilderFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_gltfbuilder_free(ptr, 0);
    }
    /**
     * Add a material with base color (RGBA 0–1 range).
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     * @returns {number}
     */
    addMaterial(r, g, b, a) {
        const ret = wasm.gltfbuilder_addMaterial(this.__wbg_ptr, r, g, b, a);
        return ret >>> 0;
    }
    /**
     * Add a mesh with positions, indices, and optional normals.
     *
     * - `positions`: Flat `Float32Array` `[x0, y0, z0, x1, y1, z1, ...]`
     * - `indices`: Flat `Uint32Array` `[i0, i1, i2, ...]`
     * - `normals`: Optional flat `Float32Array` `[nx0, ny0, nz0, ...]` (may be `null`)
     * - `material_index`: Material index (0-based), or `-1` for no material.
     * @param {Float32Array} positions
     * @param {Uint32Array} indices
     * @param {Float32Array} normals
     * @param {number} material_index
     */
    addMesh(positions, indices, normals, material_index) {
        try {
            wasm.gltfbuilder_addMesh(this.__wbg_ptr, addBorrowedObject(positions), addBorrowedObject(indices), addBorrowedObject(normals), material_index);
        } finally {
            heap[stack_pointer++] = undefined;
            heap[stack_pointer++] = undefined;
            heap[stack_pointer++] = undefined;
        }
    }
    /**
     * Create a new empty glTF builder.
     */
    constructor() {
        const ret = wasm.gltfbuilder_new();
        this.__wbg_ptr = ret;
        GltfBuilderFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Export as binary GLB (`Uint8Array`).
     * @returns {Uint8Array}
     */
    toGlb() {
        const ret = wasm.gltfbuilder_toGlb(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
     * Export as glTF JSON string (no binary — positions/indices as base64).
     * @returns {string}
     */
    toGltfJson() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.gltfbuilder_toGltfJson(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_export4(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) GltfBuilder.prototype[Symbol.dispose] = GltfBuilder.prototype.free;

/**
 * Result of parsing IFC geometry.
 */
export class IfcGeometryResult {
    static __wrap(ptr) {
        const obj = Object.create(IfcGeometryResult.prototype);
        obj.__wbg_ptr = ptr;
        IfcGeometryResultFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IfcGeometryResultFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_ifcgeometryresult_free(ptr, 0);
    }
    /**
     * Total number of meshes extracted (JS getter delegates to impl method).
     * @returns {number}
     */
    get meshCount() {
        const ret = wasm.ifcgeometryresult_meshCount(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Array of extracted meshes.
     * @returns {Array<any>}
     */
    get meshes() {
        const ret = wasm.ifcgeometryresult_meshes(this.__wbg_ptr);
        return takeObject(ret);
    }
}
if (Symbol.dispose) IfcGeometryResult.prototype[Symbol.dispose] = IfcGeometryResult.prototype.free;

/**
 * A single mesh extracted from an IFC entity.
 */
export class IfcMesh {
    static __wrap(ptr) {
        const obj = Object.create(IfcMesh.prototype);
        obj.__wbg_ptr = ptr;
        IfcMeshFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IfcMeshFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_ifcmesh_free(ptr, 0);
    }
    /**
     * Triangle indices as `Uint32Array` `[i0, i1, i2, ...]`.
     * @returns {Uint32Array}
     */
    get indices() {
        const ret = wasm.ifcmesh_indices(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
     * Vertex positions as `Float64Array` `[x0, y0, z0, x1, y1, z1, ...]`.
     * @returns {Float64Array}
     */
    get positions() {
        const ret = wasm.ifcmesh_positions(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
     * Number of triangles.
     * @returns {number}
     */
    get triangleCount() {
        const ret = wasm.ifcmesh_triangleCount(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Number of vertices.
     * @returns {number}
     */
    get vertexCount() {
        const ret = wasm.ifcmesh_vertexCount(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) IfcMesh.prototype[Symbol.dispose] = IfcMesh.prototype.free;

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
    static __wrap(ptr) {
        const obj = Object.create(LazyGeoJsonIter.prototype);
        obj.__wbg_ptr = ptr;
        LazyGeoJsonIterFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        LazyGeoJsonIterFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_lazygeojsoniter_free(ptr, 0);
    }
    /**
     * Advance to the next feature and return its coordinates as a `Float64Array`.
     *
     * Returns `null` (JS undefined) when all features have been consumed.
     * @returns {Float64Array | undefined}
     */
    nextFeature() {
        const ret = wasm.lazygeojsoniter_nextFeature(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
     * Get the remaining unconsumed feature count.
     * @returns {number}
     */
    remaining() {
        const ret = wasm.lazygeojsoniter_remaining(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get the total feature count.
     * @returns {number}
     */
    get total() {
        const ret = wasm.lazygeojsoniter_total(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) LazyGeoJsonIter.prototype[Symbol.dispose] = LazyGeoJsonIter.prototype.free;

/**
 * WASM linear memory usage info.
 */
export class MemoryInfo {
    static __wrap(ptr) {
        const obj = Object.create(MemoryInfo.prototype);
        obj.__wbg_ptr = ptr;
        MemoryInfoFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        MemoryInfoFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_memoryinfo_free(ptr, 0);
    }
    /**
     * Remaining free memory (in bytes).
     * @returns {number}
     */
    get remaining() {
        const ret = wasm.memoryinfo_remaining(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Total WASM linear memory allocated (in bytes).
     * @returns {number}
     */
    get total() {
        const ret = wasm.memoryinfo_total(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Approximate used memory (in bytes).
     * @returns {number}
     */
    get used() {
        const ret = wasm.memoryinfo_used(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) MemoryInfo.prototype[Symbol.dispose] = MemoryInfo.prototype.free;

/**
 * A decoded MVT feature with geometry, type, and tags.
 */
export class MvtFeature {
    static __wrap(ptr) {
        const obj = Object.create(MvtFeature.prototype);
        obj.__wbg_ptr = ptr;
        MvtFeatureFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        MvtFeatureFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_mvtfeature_free(ptr, 0);
    }
    /**
     * Flat tile-space coordinates as `Float64Array`.
     * @returns {Float64Array}
     */
    get geometry() {
        const ret = wasm.mvtfeaturedecoded_geometry(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
     * Geometry type: 0=Unknown, 1=Point, 2=LineString, 3=Polygon.
     * @returns {number}
     */
    get geometry_type() {
        const ret = wasm.mvtfeaturedecoded_geometry_type(this.__wbg_ptr);
        return ret;
    }
    /**
     * Feature ID (0 if not set).
     * @returns {number}
     */
    get id() {
        const ret = wasm.mvtfeaturedecoded_id(this.__wbg_ptr);
        return ret;
    }
    /**
     * Tag count.
     * @returns {number}
     */
    tagCount() {
        const ret = wasm.mvtfeaturedecoded_tagCount(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get a tag key by index.
     * @param {number} index
     * @returns {string}
     */
    tagKey(index) {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.mvtfeaturedecoded_tagKey(retptr, this.__wbg_ptr, index);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_export4(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get a tag value by index.
     * @param {number} index
     * @returns {string}
     */
    tagValue(index) {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.mvtfeaturedecoded_tagValue(retptr, this.__wbg_ptr, index);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_export4(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) MvtFeature.prototype[Symbol.dispose] = MvtFeature.prototype.free;

/**
 * A decoded MVT layer with structured feature data.
 */
export class MvtLayer {
    static __wrap(ptr) {
        const obj = Object.create(MvtLayer.prototype);
        obj.__wbg_ptr = ptr;
        MvtLayerFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        MvtLayerFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_mvtlayer_free(ptr, 0);
    }
    /**
     * Layer extent (typically 4096).
     * @returns {number}
     */
    get extent() {
        const ret = wasm.mvtlayerdecoded_extent(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get feature by index.
     * @param {number} index
     * @returns {MvtFeature}
     */
    featureAt(index) {
        const ret = wasm.mvtlayerdecoded_featureAt(this.__wbg_ptr, index);
        return MvtFeature.__wrap(ret);
    }
    /**
     * Number of features in this layer.
     * @returns {number}
     */
    featureCount() {
        const ret = wasm.mvtlayerdecoded_featureCount(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Layer name.
     * @returns {string}
     */
    get name() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.mvtlayerdecoded_name(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_export4(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) MvtLayer.prototype[Symbol.dispose] = MvtLayer.prototype.free;

/**
 * A spatial index for 2D line segments using an R-Tree.
 *
 * Indexes individual edges (line segments) from LineString geometries.
 * Supports bounding box queries to find all edges that intersect with
 * a given rectangular area. Useful for viewport-based progressive loading
 * of road networks, pipelines, and other linear features.
 */
export class SpatialEdgeIndex {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        SpatialEdgeIndexFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_spatialedgeindex_free(ptr, 0);
    }
    /**
     * Find the nearest edge to a given query coordinate.
     * Returns the ID of the nearest edge, or `null` if the index is empty.
     *
     * Distance is measured as the minimum Euclidean distance from the
     * query point to any point on the edge.
     * @param {number} query_x
     * @param {number} query_y
     * @returns {number | undefined}
     */
    nearestNeighbor(query_x, query_y) {
        const ret = wasm.spatialedgeindex_nearestNeighbor(this.__wbg_ptr, query_x, query_y);
        return ret === Number.MAX_SAFE_INTEGER ? undefined : ret;
    }
    /**
     * Build a spatial edge index from line segments.
     *
     * Input format: a flat `Float64Array` of line segment endpoints
     * `[x0, y0, x1, y1, x2, y2, x3, y3, ...]` where each consecutive
     * pair of 2D points forms an edge (line segment).
     *
     * Each edge is assigned an ID equal to its sequential index
     * (0 for the first edge, 1 for the second, etc.).
     * @param {Float64Array} segments
     */
    constructor(segments) {
        try {
            const ret = wasm.spatialedgeindex_new(addBorrowedObject(segments));
            this.__wbg_ptr = ret;
            SpatialEdgeIndexFinalization.register(this, this.__wbg_ptr, this);
            return this;
        } finally {
            heap[stack_pointer++] = undefined;
        }
    }
    /**
     * Search for all edges within a given bounding box.
     * Returns a `Uint32Array` containing the IDs of matching edges.
     *
     * An edge matches if its bounding box intersects the query envelope.
     * @param {number} min_x
     * @param {number} min_y
     * @param {number} max_x
     * @param {number} max_y
     * @returns {Uint32Array}
     */
    searchBBox(min_x, min_y, max_x, max_y) {
        const ret = wasm.spatialedgeindex_searchBBox(this.__wbg_ptr, min_x, min_y, max_x, max_y);
        return takeObject(ret);
    }
    /**
     * Get the total number of edges in the index.
     * @returns {number}
     */
    size() {
        const ret = wasm.spatialedgeindex_size(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) SpatialEdgeIndex.prototype[Symbol.dispose] = SpatialEdgeIndex.prototype.free;

/**
 * A high-performance spatial index using an R-Tree.
 */
export class SpatialIndex {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        SpatialIndexFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_spatialindex_free(ptr, 0);
    }
    /**
     * Find the K nearest neighbors to a given query coordinate.
     * Returns a `Uint32Array` containing the IDs, ordered by distance (nearest first).
     * If `k` is greater than the number of points, returns all points.
     * @param {number} query_x
     * @param {number} query_y
     * @param {number} k
     * @returns {Uint32Array}
     */
    kNearestNeighbors(query_x, query_y, k) {
        const ret = wasm.spatialindex_kNearestNeighbors(this.__wbg_ptr, query_x, query_y, k);
        return takeObject(ret);
    }
    /**
     * Find the nearest point to a given query coordinate.
     * Returns the ID of the nearest point, or `null` if the index is empty.
     * @param {number} query_x
     * @param {number} query_y
     * @returns {number | undefined}
     */
    nearestNeighbor(query_x, query_y) {
        const ret = wasm.spatialindex_nearestNeighbor(this.__wbg_ptr, query_x, query_y);
        return ret === Number.MAX_SAFE_INTEGER ? undefined : ret;
    }
    /**
     * Build a spatial index from a flat Float64Array of coordinates `[lng0, lat0, lng1, lat1, ...]`.
     * Each coordinate pair is assigned an ID equal to its index (i.e. `0` for the first pair, `1` for the second).
     * @param {Float64Array} coords
     */
    constructor(coords) {
        try {
            const ret = wasm.spatialindex_new(addBorrowedObject(coords));
            this.__wbg_ptr = ret;
            SpatialIndexFinalization.register(this, this.__wbg_ptr, this);
            return this;
        } finally {
            heap[stack_pointer++] = undefined;
        }
    }
    /**
     * Search for all points within a given bounding box.
     * Returns a `Uint32Array` containing the IDs of the points.
     * @param {number} min_x
     * @param {number} min_y
     * @param {number} max_x
     * @param {number} max_y
     * @returns {Uint32Array}
     */
    searchBBox(min_x, min_y, max_x, max_y) {
        const ret = wasm.spatialindex_searchBBox(this.__wbg_ptr, min_x, min_y, max_x, max_y);
        return takeObject(ret);
    }
    /**
     * Get the total number of points in the index.
     * @returns {number}
     */
    size() {
        const ret = wasm.spatialindex_size(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) SpatialIndex.prototype[Symbol.dispose] = SpatialIndex.prototype.free;

/**
 * Result of building a TIN from scattered 3D points.
 */
export class TinResult {
    static __wrap(ptr) {
        const obj = Object.create(TinResult.prototype);
        obj.__wbg_ptr = ptr;
        TinResultFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        TinResultFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_tinresult_free(ptr, 0);
    }
    /**
     * Triangle indices `[i0,i1,i2, i3,i4,i5, ...]`.
     * @returns {Uint32Array}
     */
    get indices() {
        const ret = wasm.tinresult_indices(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
     * Flat vertex positions `[x0,y0,z0, x1,y1,z1, ...]`.
     * @returns {Float64Array}
     */
    get positions() {
        const ret = wasm.tinresult_positions(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
     * Number of triangles.
     * @returns {number}
     */
    get triangleCount() {
        const ret = wasm.tinresult_triangleCount(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Number of vertices.
     * @returns {number}
     */
    get vertexCount() {
        const ret = wasm.tinresult_vertexCount(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) TinResult.prototype[Symbol.dispose] = TinResult.prototype.free;

/**
 * Result of coordinate validation.
 */
export class ValidationResult {
    static __wrap(ptr) {
        const obj = Object.create(ValidationResult.prototype);
        obj.__wbg_ptr = ptr;
        ValidationResultFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ValidationResultFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_validationresult_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    get invalid_count() {
        const ret = wasm.validationresult_invalid_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {Uint32Array}
     */
    get invalid_indices() {
        const ret = wasm.validationresult_invalid_indices(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
     * @returns {number}
     */
    get valid_count() {
        const ret = wasm.validationresult_valid_count(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) ValidationResult.prototype[Symbol.dispose] = ValidationResult.prototype.free;

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
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        VectorTileEngineFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_vectortileengine_free(ptr, 0);
    }
    /**
     * Get the number of cached tiles.
     * @returns {number}
     */
    cacheSize() {
        const ret = wasm.vectortileengine_cacheSize(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Clear the tile LRU cache.
     */
    clearTileCache() {
        wasm.vectortileengine_clearTileCache(this.__wbg_ptr);
    }
    /**
     * Request a tile by `z, x, y` coordinates.
     * Returns a raw `Uint8Array` representing the MVT (PBF) protobuf.
     * If the tile is empty or out of bounds, returns an empty array.
     *
     * Feature properties (`name`, `id`, `class`, and any other fields)
     * from the original GeoJSON are automatically encoded as MVT tags.
     * @param {number} z
     * @param {number} x
     * @param {number} y
     * @returns {Uint8Array}
     */
    getTile(z, x, y) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.vectortileengine_getTile(retptr, this.__wbg_ptr, z, x, y);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
     * Request a tile with LRU caching (max 64 tiles).
     *
     * If the tile was previously requested, returns the cached result
     * without re-computing. Otherwise, generates the tile, caches it,
     * and returns it.
     *
     * Use `clearTileCache()` to evict all cached tiles.
     * @param {number} z
     * @param {number} x
     * @param {number} y
     * @returns {Uint8Array}
     */
    getTileCached(z, x, y) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.vectortileengine_getTileCached(retptr, this.__wbg_ptr, z, x, y);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
     * Get the layer name used by this engine.
     * @returns {string}
     */
    get layerName() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.vectortileengine_layer_name(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_export4(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Create a new VectorTileEngine from a GeoJSON string.
     *
     * The `layer_name` parameter controls the layer name embedded in the
     * MVT protobuf output. Defaults to `"default"`.
     * @param {string} geojson_str
     * @param {VectorTileOptions} options
     * @param {string | null} [layer_name]
     */
    constructor(geojson_str, options, layer_name) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(geojson_str, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len0 = WASM_VECTOR_LEN;
            _assertClass(options, VectorTileOptions);
            var ptr1 = options.__destroy_into_raw();
            var ptr2 = isLikeNone(layer_name) ? 0 : passStringToWasm0(layer_name, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            var len2 = WASM_VECTOR_LEN;
            wasm.vectortileengine_new(retptr, ptr0, len0, ptr1, ptr2, len2);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
            if (r2) {
                throw takeObject(r1);
            }
            this.__wbg_ptr = r0;
            VectorTileEngineFinalization.register(this, this.__wbg_ptr, this);
            return this;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
     * Set a new layer name for subsequent tile requests.
     * @param {string} name
     */
    set layerName(name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.vectortileengine_set_layer_name(this.__wbg_ptr, ptr0, len0);
    }
}
if (Symbol.dispose) VectorTileEngine.prototype[Symbol.dispose] = VectorTileEngine.prototype.free;

/**
 * Options for vector tile generation.
 */
export class VectorTileOptions {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        VectorTileOptionsFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_vectortileoptions_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    get buffer() {
        const ret = wasm.__wbg_get_vectortileoptions_buffer(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get extent() {
        const ret = wasm.__wbg_get_vectortileoptions_extent(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {boolean}
     */
    get generate_id() {
        const ret = wasm.__wbg_get_vectortileoptions_generate_id(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {number}
     */
    get index_max_points() {
        const ret = wasm.__wbg_get_vectortileoptions_index_max_points(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get index_max_zoom() {
        const ret = wasm.__wbg_get_vectortileoptions_index_max_zoom(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {boolean}
     */
    get line_metrics() {
        const ret = wasm.__wbg_get_vectortileoptions_line_metrics(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {number}
     */
    get max_zoom() {
        const ret = wasm.__wbg_get_vectortileoptions_max_zoom(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get tolerance() {
        const ret = wasm.__wbg_get_vectortileoptions_tolerance(this.__wbg_ptr);
        return ret;
    }
    /**
     * @param {number} arg0
     */
    set buffer(arg0) {
        wasm.__wbg_set_vectortileoptions_buffer(this.__wbg_ptr, arg0);
    }
    /**
     * @param {number} arg0
     */
    set extent(arg0) {
        wasm.__wbg_set_vectortileoptions_extent(this.__wbg_ptr, arg0);
    }
    /**
     * @param {boolean} arg0
     */
    set generate_id(arg0) {
        wasm.__wbg_set_vectortileoptions_generate_id(this.__wbg_ptr, arg0);
    }
    /**
     * @param {number} arg0
     */
    set index_max_points(arg0) {
        wasm.__wbg_set_vectortileoptions_index_max_points(this.__wbg_ptr, arg0);
    }
    /**
     * @param {number} arg0
     */
    set index_max_zoom(arg0) {
        wasm.__wbg_set_vectortileoptions_index_max_zoom(this.__wbg_ptr, arg0);
    }
    /**
     * @param {boolean} arg0
     */
    set line_metrics(arg0) {
        wasm.__wbg_set_vectortileoptions_line_metrics(this.__wbg_ptr, arg0);
    }
    /**
     * @param {number} arg0
     */
    set max_zoom(arg0) {
        wasm.__wbg_set_vectortileoptions_max_zoom(this.__wbg_ptr, arg0);
    }
    /**
     * @param {number} arg0
     */
    set tolerance(arg0) {
        wasm.__wbg_set_vectortileoptions_tolerance(this.__wbg_ptr, arg0);
    }
    constructor() {
        const ret = wasm.vectortileoptions_new();
        this.__wbg_ptr = ret;
        VectorTileOptionsFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
}
if (Symbol.dispose) VectorTileOptions.prototype[Symbol.dispose] = VectorTileOptions.prototype.free;

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
 * @param {string} input
 * @param {string} key
 * @param {string} value
 * @returns {string}
 */
export function addProperty(input, key, value) {
    let deferred5_0;
    let deferred5_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(key, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(value, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len2 = WASM_VECTOR_LEN;
        wasm.addProperty(retptr, ptr0, len0, ptr1, len1, ptr2, len2);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        var ptr4 = r0;
        var len4 = r1;
        if (r3) {
            ptr4 = 0; len4 = 0;
            throw takeObject(r2);
        }
        deferred5_0 = ptr4;
        deferred5_1 = len4;
        return getStringFromWasm0(ptr4, len4);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export4(deferred5_0, deferred5_1, 1);
    }
}

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
 * @param {Float64Array} rings
 * @param {Uint32Array} ring_sizes
 * @returns {number}
 */
export function areaWithHoles(rings, ring_sizes) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.areaWithHoles(retptr, addBorrowedObject(rings), addBorrowedObject(ring_sizes));
        var r0 = getDataViewMemory0().getFloat64(retptr + 8 * 0, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        if (r3) {
            throw takeObject(r2);
        }
        return r0;
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        heap[stack_pointer++] = undefined;
        heap[stack_pointer++] = undefined;
    }
}

/**
 * Batch BD-09 → GCJ-02. Returns a **new** `Float64Array`.
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function batchBd09ToGcj02(coords) {
    try {
        const ret = wasm.batchBd09ToGcj02(addBorrowedObject(coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * **[Zero-Copy]** In-place BD-09 → GCJ-02.
 * @param {Float64Array} coords
 */
export function batchBd09ToGcj02InPlace(coords) {
    var ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export);
    var len0 = WASM_VECTOR_LEN;
    wasm.batchBd09ToGcj02InPlace(ptr0, len0, addHeapObject(coords));
}

/**
 * Batch BD-09 → WGS-84. Returns a **new** `Float64Array`.
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function batchBd09ToWgs84(coords) {
    try {
        const ret = wasm.batchBd09ToWgs84(addBorrowedObject(coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * **[Zero-Copy]** In-place BD-09 → WGS-84.
 * @param {Float64Array} coords
 */
export function batchBd09ToWgs84InPlace(coords) {
    var ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export);
    var len0 = WASM_VECTOR_LEN;
    wasm.batchBd09ToWgs84InPlace(ptr0, len0, addHeapObject(coords));
}

/**
 * Batch GCJ-02 → BD-09. Returns a **new** `Float64Array`.
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function batchGcj02ToBd09(coords) {
    try {
        const ret = wasm.batchGcj02ToBd09(addBorrowedObject(coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * **[Zero-Copy]** In-place GCJ-02 → BD-09.
 * @param {Float64Array} coords
 */
export function batchGcj02ToBd09InPlace(coords) {
    var ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export);
    var len0 = WASM_VECTOR_LEN;
    wasm.batchGcj02ToBd09InPlace(ptr0, len0, addHeapObject(coords));
}

/**
 * Batch GCJ-02 → WGS-84. Returns a **new** `Float64Array`.
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function batchGcj02ToWgs84(coords) {
    try {
        const ret = wasm.batchGcj02ToWgs84(addBorrowedObject(coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * **[Zero-Copy]** In-place GCJ-02 → WGS-84.
 * @param {Float64Array} coords
 */
export function batchGcj02ToWgs84InPlace(coords) {
    var ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export);
    var len0 = WASM_VECTOR_LEN;
    wasm.batchGcj02ToWgs84InPlace(ptr0, len0, addHeapObject(coords));
}

/**
 * Batch Web Mercator (EPSG:3857) → WGS-84. Returns a **new** `Float64Array`.
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function batchMercatorToWgs84(coords) {
    try {
        const ret = wasm.batchMercatorToWgs84(addBorrowedObject(coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * **[Zero-Copy]** In-place Web Mercator (EPSG:3857) → WGS-84.
 * @param {Float64Array} coords
 */
export function batchMercatorToWgs84InPlace(coords) {
    var ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export);
    var len0 = WASM_VECTOR_LEN;
    wasm.batchMercatorToWgs84InPlace(ptr0, len0, addHeapObject(coords));
}

/**
 * Convert batch UTM coordinates to WGS84.
 *
 * Input: flat `[zone, easting, northing, zone, easting, northing, ...]`.
 * Output: flat `[lng, lat, lng, lat, ...]`.
 * @param {Float64Array} utm_coords
 * @returns {Float64Array}
 */
export function batchUtmToWgs84(utm_coords) {
    try {
        const ret = wasm.batchUtmToWgs84(addBorrowedObject(utm_coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * Convert batch UTM to WGS84 in-place.
 *
 * Input layout: `[zone, easting, northing, ...]`.
 * Output layout: `[lng, lat, 0, ...]` (third component zeroed).
 * @param {Float64Array} coords
 */
export function batchUtmToWgs84InPlace(coords) {
    try {
        wasm.batchUtmToWgs84InPlace(addBorrowedObject(coords));
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * Batch WGS-84 → BD-09. Returns a **new** `Float64Array`.
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function batchWgs84ToBd09(coords) {
    try {
        const ret = wasm.batchWgs84ToBd09(addBorrowedObject(coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * **[Zero-Copy]** In-place WGS-84 → BD-09.
 * @param {Float64Array} coords
 */
export function batchWgs84ToBd09InPlace(coords) {
    var ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export);
    var len0 = WASM_VECTOR_LEN;
    wasm.batchWgs84ToBd09InPlace(ptr0, len0, addHeapObject(coords));
}

/**
 * Batch WGS-84 → BD-09 → Web Mercator. Returns a **new** `Float64Array`.
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function batchWgs84ToBd09Mercator(coords) {
    try {
        const ret = wasm.batchWgs84ToBd09Mercator(addBorrowedObject(coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * **[Zero-Copy]** In-place WGS-84 → BD-09 → Web Mercator.
 * @param {Float64Array} coords
 */
export function batchWgs84ToBd09MercatorInPlace(coords) {
    var ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export);
    var len0 = WASM_VECTOR_LEN;
    wasm.batchWgs84ToBd09MercatorInPlace(ptr0, len0, addHeapObject(coords));
}

/**
 * Batch convert a flat array of `[lng, lat, ...]` into `[x, y, z, ...]`.
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function batchWgs84ToCartesian3(coords) {
    const ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.batchWgs84ToCartesian3(ptr0, len0);
    return takeObject(ret);
}

/**
 * Batch "WGS-84 → CGCS2000" — identity transform. Returns a copy.
 *
 * See [`cgcs2000_is_wgs84_compatible`] for precision details.
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function batchWgs84ToCgcs2000(coords) {
    try {
        const ret = wasm.batchWgs84ToCgcs2000(addBorrowedObject(coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * **[Zero-Copy]** In-place "WGS-84 → CGCS2000" — identity transform.
 *
 * Provided for API completeness. Since CGCS2000 ≈ WGS-84 (< 1 cm difference),
 * this is a no-op. The buffer is returned unchanged.
 *
 * If your pipeline requires an explicit CGCS2000 step, call this to make the
 * intent clear in code without incurring any runtime cost.
 * @param {Float64Array} _coords
 */
export function batchWgs84ToCgcs2000InPlace(_coords) {
    var ptr0 = passArrayF64ToWasm0(_coords, wasm.__wbindgen_export);
    var len0 = WASM_VECTOR_LEN;
    wasm.batchWgs84ToCgcs2000InPlace(ptr0, len0, addHeapObject(_coords));
}

/**
 * Batch WGS-84 → GCJ-02. Returns a **new** `Float64Array`.
 *
 * For large datasets, prefer the `InPlace` variant to avoid copies.
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function batchWgs84ToGcj02(coords) {
    try {
        const ret = wasm.batchWgs84ToGcj02(addBorrowedObject(coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * **[Zero-Copy]** In-place WGS-84 → GCJ-02.
 *
 * Mutates the input `[lng, lat, …]` buffer directly in WASM linear memory.
 * ```js
 * const buf = new Float64Array(wasmMemory.buffer, ptr, len);
 * wasm.batchWgs84ToGcj02InPlace(buf);
 * // buf is now in GCJ-02 — no copy occurred
 * ```
 * @param {Float64Array} coords
 */
export function batchWgs84ToGcj02InPlace(coords) {
    var ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export);
    var len0 = WASM_VECTOR_LEN;
    wasm.batchWgs84ToGcj02InPlace(ptr0, len0, addHeapObject(coords));
}

/**
 * Batch WGS-84 → GCJ-02 → Web Mercator. Returns a **new** `Float64Array`.
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function batchWgs84ToGcj02Mercator(coords) {
    try {
        const ret = wasm.batchWgs84ToGcj02Mercator(addBorrowedObject(coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * **[Zero-Copy]** In-place WGS-84 → GCJ-02 → Web Mercator.
 *
 * Most common pipeline for Chinese web map applications.
 * @param {Float64Array} coords
 */
export function batchWgs84ToGcj02MercatorInPlace(coords) {
    var ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export);
    var len0 = WASM_VECTOR_LEN;
    wasm.batchWgs84ToGcj02MercatorInPlace(ptr0, len0, addHeapObject(coords));
}

/**
 * Batch WGS-84 → Web Mercator (EPSG:3857). Returns a **new** `Float64Array`.
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function batchWgs84ToMercator(coords) {
    try {
        const ret = wasm.batchWgs84ToMercator(addBorrowedObject(coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * **[Zero-Copy]** In-place WGS-84 → Web Mercator (EPSG:3857).
 * @param {Float64Array} coords
 */
export function batchWgs84ToMercatorInPlace(coords) {
    var ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export);
    var len0 = WASM_VECTOR_LEN;
    wasm.batchWgs84ToMercatorInPlace(ptr0, len0, addHeapObject(coords));
}

/**
 * Convert batch WGS84 coordinates to UTM.
 *
 * Input: flat `[lng0, lat0, lng1, lat1, ...]`.
 * Output: flat `[zone, easting, northing, zone, easting, northing, ...]`.
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function batchWgs84ToUtm(coords) {
    try {
        const ret = wasm.batchWgs84ToUtm(addBorrowedObject(coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * Convert batch WGS84 to UTM in-place.
 *
 * The input buffer must be pre-allocated with 3 values per point (same as output).
 * Input layout: `[lng, lat, 0, lng, lat, 0, ...]`.
 * Output layout: `[zone, easting, northing, zone, easting, northing, ...]`.
 * @param {Float64Array} coords
 */
export function batchWgs84ToUtmInPlace(coords) {
    try {
        wasm.batchWgs84ToUtmInPlace(addBorrowedObject(coords));
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {number} lng1
 * @param {number} lat1
 * @param {number} lng2
 * @param {number} lat2
 * @returns {number}
 */
export function bearing(lng1, lat1, lng2, lat2) {
    const ret = wasm.bearing(lng1, lat1, lng2, lat2);
    return ret;
}

/**
 * Recommend the best CRS for a geographic region.
 *
 * # Arguments
 * - `min_lng`, `min_lat`, `max_lng`, `max_lat`: Bounding box in degrees.
 *
 * # Returns
 * JSON string with `crs` (recommended CRS code) and `reason`.
 * @param {number} min_lng
 * @param {number} min_lat
 * @param {number} max_lng
 * @param {number} max_lat
 * @returns {string}
 */
export function bestCrsForRegion(min_lng, min_lat, max_lng, max_lat) {
    let deferred1_0;
    let deferred1_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.bestCrsForRegion(retptr, min_lng, min_lat, max_lng, max_lat);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        deferred1_0 = r0;
        deferred1_1 = r1;
        return getStringFromWasm0(r0, r1);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export4(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Compute the axis-aligned bounding box of a set of coordinates.
 *
 * Returns `[minLng, minLat, maxLng, maxLat]`.
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function boundingBox(coords) {
    try {
        const ret = wasm.boundingBox(addBorrowedObject(coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * Generate a buffer polygon around a line string (union of point buffers).
 *
 * Returns a flat `Float64Array` of polygon vertices `[lng0, lat0, ...]`.
 * Note: this is a simplified implementation that produces a convex hull of
 * all circle vertices around each line point. For production use with
 * concave results, consider `geo`'s `BooleanOps` union.
 * @param {Float64Array} coords
 * @param {number} radius_meters
 * @param {number | null} [segments]
 * @returns {Float64Array}
 */
export function bufferLineString(coords, radius_meters, segments) {
    try {
        const ret = wasm.bufferLineString(addBorrowedObject(coords), radius_meters, isLikeNone(segments) ? Number.MAX_SAFE_INTEGER : (segments) >>> 0);
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * Generate a buffer polygon around a point.
 *
 * Returns a flat `Float64Array` of polygon vertices `[lng0, lat0, lng1, lat1, ...]`
 * forming a circle approximation around the given point.
 * @param {number} lng
 * @param {number} lat
 * @param {number} radius_meters
 * @param {number | null} [segments]
 * @returns {Float64Array}
 */
export function bufferPoint(lng, lat, radius_meters, segments) {
    const ret = wasm.bufferPoint(lng, lat, radius_meters, isLikeNone(segments) ? Number.MAX_SAFE_INTEGER : (segments) >>> 0);
    return takeObject(ret);
}

/**
 * Build a TIN from scattered 3D points using the Bowyer-Watson algorithm.
 *
 * # Arguments
 * - `points`: Flat `Float64Array` `[x0,y0,z0, x1,y1,z1, ...]`
 *
 * # Returns
 * `TinResult` with deduplicated positions and triangle indices.
 * @param {Float64Array} points
 * @returns {TinResult}
 */
export function buildTin(points) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.buildTin(retptr, addBorrowedObject(points));
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        if (r2) {
            throw takeObject(r1);
        }
        return TinResult.__wrap(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        heap[stack_pointer++] = undefined;
    }
}

/**
 * Compute the centroid (mean center) of a set of coordinates.
 *
 * Returns `[lng, lat]`.
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function centroid(coords) {
    try {
        const ret = wasm.centroid(addBorrowedObject(coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

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
 * @returns {boolean}
 */
export function cgcs2000IsWgs84Compatible() {
    const ret = wasm.cgcs2000IsWgs84Compatible();
    return ret !== 0;
}

/**
 * Clean coordinate data by removing, clamping, or snapping invalid values.
 *
 * # Arguments
 *
 * * `coords` — Flat `Float64Array` `[lng0, lat0, lng1, lat1, …]`
 * * `strategy` — One of: `"remove"`, `"clamp"`, `"snap"`
 * @param {Float64Array} coords
 * @param {string} strategy
 * @returns {Float64Array}
 */
export function cleanCoords(coords, strategy) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(strategy, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.cleanCoords(retptr, addBorrowedObject(coords), ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        if (r2) {
            throw takeObject(r1);
        }
        return takeObject(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {Float64Array} coords
 * @param {number} epsilon
 * @param {number} min_points
 * @returns {Float64Array}
 */
export function clusterByDensity(coords, epsilon, min_points) {
    try {
        const ret = wasm.clusterByDensity(addBorrowedObject(coords), epsilon, min_points);
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {Float64Array} coords
 * @param {number} cell_size
 * @param {number} min_points
 * @returns {Float64Array}
 */
export function clusterByGrid(coords, cell_size, min_points) {
    try {
        const ret = wasm.clusterByGrid(addBorrowedObject(coords), cell_size, min_points);
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * Compute the axis-aligned bounding box of a set of 2D coordinates.
 *
 * Input: flat `Float64Array` of `[lng0, lat0, lng1, lat1, ...]`.
 * Output: `Float64Array` of `[minLng, minLat, maxLng, maxLat]`.
 *
 * Uses a manual 4-wide f64 comparison pattern for efficient vectorization
 * hints to the LLVM backend (effectively SIMD-style without explicit SIMD intrinsics).
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function computeBounds(coords) {
    try {
        const ret = wasm.computeBounds(addBorrowedObject(coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * Compute the merged bounding box of multiple coordinate buffers.
 *
 * Input: a JS `Array` of `Float64Array` coordinate buffers.
 * Output: `Float64Array` of `[minLng, minLat, maxLng, maxLat]`.
 *
 * Equivalent to calling `computeBounds` on each buffer individually
 * and then merging the results, but processes all buffers in a single pass
 * for better cache locality.
 * @param {Array<any>} buffers
 * @returns {Float64Array}
 */
export function computeBoundsMulti(buffers) {
    try {
        const ret = wasm.computeBoundsMulti(addBorrowedObject(buffers));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {Float64Array} coords
 * @param {number} alpha
 * @returns {Float64Array}
 */
export function concaveHull(coords, alpha) {
    try {
        const ret = wasm.concaveHull(addBorrowedObject(coords), alpha);
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {Float64Array} outer_ring
 * @param {number} point_x
 * @param {number} point_y
 * @returns {boolean}
 */
export function contains(outer_ring, point_x, point_y) {
    try {
        const ret = wasm.contains(addBorrowedObject(outer_ring), point_x, point_y);
        return ret !== 0;
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * Compute the convex hull of a set of 2D points using Andrew's monotone chain algorithm.
 *
 * # Arguments
 * - `coords`: Flat `Float64Array` `[lng0, lat0, lng1, lat1, ...]`.
 *
 * # Returns
 * Flat `Float64Array` of convex hull vertices (closed: first == last).
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function convexHull(coords) {
    try {
        const ret = wasm.convexHull(addBorrowedObject(coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {string} input
 * @param {string} key
 * @returns {string}
 */
export function countGeoJsonByProperty(input, key) {
    let deferred4_0;
    let deferred4_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(key, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len1 = WASM_VECTOR_LEN;
        wasm.countGeoJsonByProperty(retptr, ptr0, len0, ptr1, len1);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        var ptr3 = r0;
        var len3 = r1;
        if (r3) {
            ptr3 = 0; len3 = 0;
            throw takeObject(r2);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export4(deferred4_0, deferred4_1, 1);
    }
}

/**
 * Return the total number of features in a GeoJSON string.
 *
 * Useful for progress reporting before parsing a very large file.
 * @param {string} input
 * @returns {number}
 */
export function countGeoJsonFeatures(input) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.countGeoJsonFeatures(retptr, ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        if (r2) {
            throw takeObject(r1);
        }
        return r0 >>> 0;
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
 * Return JSON info for a specific CRS code.
 *
 * # Arguments
 * - `code`: CRS code string, e.g. `"EPSG:4326"`, `"GCJ-02"`, `"BD-09"`.
 *
 * # Returns
 * JSON object with `name`, `description`, `bounds`, `unit`.
 * @param {string} code
 * @returns {string}
 */
export function crsInfo(code) {
    let deferred2_0;
    let deferred2_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(code, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.crsInfo(retptr, ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        deferred2_0 = r0;
        deferred2_1 = r1;
        return getStringFromWasm0(r0, r1);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export4(deferred2_0, deferred2_1, 1);
    }
}

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
 * @param {Uint8Array} bytes
 * @returns {MvtLayer}
 */
export function decodeMvt(bytes) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.decodeMvt(retptr, addHeapObject(bytes));
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        if (r2) {
            throw takeObject(r1);
        }
        return MvtLayer.__wrap(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

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
 * @param {Uint8Array} bytes
 * @returns {string}
 */
export function decodeMvtToGeoJson(bytes) {
    let deferred2_0;
    let deferred2_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.decodeMvtToGeoJson(retptr, addHeapObject(bytes));
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        var ptr1 = r0;
        var len1 = r1;
        if (r3) {
            ptr1 = 0; len1 = 0;
            throw takeObject(r2);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export4(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Deduplicate coordinates within a tolerance.
 *
 * Keeps the first occurrence of each coordinate pair within `tolerance` distance.
 *
 * # Arguments
 *
 * * `coords` — Flat `Float64Array` `[lng0, lat0, lng1, lat1, …]`
 * * `tolerance` — Maximum distance (in coordinate units) for two points to be considered duplicates
 * @param {Float64Array} coords
 * @param {number} tolerance
 * @returns {Float64Array}
 */
export function deduplicateCoords(coords, tolerance) {
    try {
        const ret = wasm.deduplicateCoords(addBorrowedObject(coords), tolerance);
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {Float64Array} normals
 * @param {Float64Array} source_bounds
 * @returns {Float64Array}
 */
export function denormalizeCoords(normals, source_bounds) {
    try {
        const ret = wasm.denormalizeCoords(addBorrowedObject(normals), addBorrowedObject(source_bounds));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {number} lng
 * @param {number} lat
 * @param {number} bearing_deg
 * @param {number} distance_m
 * @returns {Float64Array}
 */
export function destination(lng, lat, bearing_deg, distance_m) {
    const ret = wasm.destination(lng, lat, bearing_deg, distance_m);
    return takeObject(ret);
}

/**
 * Check if two polygons are disjoint (share no points at all).
 *
 * # Arguments
 *
 * * `ring1` — First polygon as flat closed ring
 * * `ring2` — Second polygon as flat closed ring
 * @param {Float64Array} ring1
 * @param {Float64Array} ring2
 * @returns {boolean}
 */
export function disjoint(ring1, ring2) {
    try {
        const ret = wasm.disjoint(addBorrowedObject(ring1), addBorrowedObject(ring2));
        return ret !== 0;
    } finally {
        heap[stack_pointer++] = undefined;
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {string} input
 * @param {number} min_lng
 * @param {number} min_lat
 * @param {number} max_lng
 * @param {number} max_lat
 * @returns {string}
 */
export function filterGeoJsonByBBox(input, min_lng, min_lat, max_lng, max_lat) {
    let deferred3_0;
    let deferred3_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.filterGeoJsonByBBox(retptr, ptr0, len0, min_lng, min_lat, max_lng, max_lat);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        var ptr2 = r0;
        var len2 = r1;
        if (r3) {
            ptr2 = 0; len2 = 0;
            throw takeObject(r2);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export4(deferred3_0, deferred3_1, 1);
    }
}

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
 * @param {string} input
 * @param {string} key
 * @param {string} value
 * @returns {string}
 */
export function filterGeoJsonByProperty(input, key, value) {
    let deferred5_0;
    let deferred5_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(key, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(value, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len2 = WASM_VECTOR_LEN;
        wasm.filterGeoJsonByProperty(retptr, ptr0, len0, ptr1, len1, ptr2, len2);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        var ptr4 = r0;
        var len4 = r1;
        if (r3) {
            ptr4 = 0; len4 = 0;
            throw takeObject(r2);
        }
        deferred5_0 = ptr4;
        deferred5_1 = len4;
        return getStringFromWasm0(ptr4, len4);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export4(deferred5_0, deferred5_1, 1);
    }
}

/**
 * Generate a complete b3dm 3D Tile from GeoJSON polygons/multipolygons.
 *
 * Reuses `generate_cesium_geometry` internally for triangulation, then
 * wraps the result in the b3dm binary envelope suitable for Cesium's
 * `Cesium3DTileset`.
 * @param {string} geojson_str
 * @param {string | null} [height_property]
 * @returns {Cesium3DTile}
 */
export function generate3DTile(geojson_str, height_property) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(geojson_str, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(height_property) ? 0 : passStringToWasm0(height_property, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        var len1 = WASM_VECTOR_LEN;
        wasm.generate3DTile(retptr, ptr0, len0, ptr1, len1);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        if (r2) {
            throw takeObject(r1);
        }
        return Cesium3DTile.__wrap(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
 * Generate triangulated mesh from GeoJSON Polygons/MultiPolygons.
 * @param {string} geojson_str
 * @param {string | null} [height_property]
 * @returns {CesiumMeshGeometry}
 */
export function generateCesiumGeometry(geojson_str, height_property) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(geojson_str, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(height_property) ? 0 : passStringToWasm0(height_property, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        var len1 = WASM_VECTOR_LEN;
        wasm.generateCesiumGeometry(retptr, ptr0, len0, ptr1, len1);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        if (r2) {
            throw takeObject(r1);
        }
        return CesiumMeshGeometry.__wrap(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

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
 * @param {Float64Array} coords
 * @param {string} types
 * @param {string} properties_json
 * @returns {string}
 */
export function geoJsonFeatureCollection(coords, types, properties_json) {
    let deferred4_0;
    let deferred4_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(types, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(properties_json, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len1 = WASM_VECTOR_LEN;
        wasm.geoJsonFeatureCollection(retptr, addBorrowedObject(coords), ptr0, len0, ptr1, len1);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        var ptr3 = r0;
        var len3 = r1;
        if (r3) {
            ptr3 = 0; len3 = 0;
            throw takeObject(r2);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        heap[stack_pointer++] = undefined;
        wasm.__wbindgen_export4(deferred4_0, deferred4_1, 1);
    }
}

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
 * @param {Float64Array} coords
 * @param {string} geometry_type
 * @returns {string}
 */
export function geoJsonFromCoords(coords, geometry_type) {
    let deferred3_0;
    let deferred3_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(geometry_type, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.geoJsonFromCoords(retptr, addBorrowedObject(coords), ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        var ptr2 = r0;
        var len2 = r1;
        if (r3) {
            ptr2 = 0; len2 = 0;
            throw takeObject(r2);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        heap[stack_pointer++] = undefined;
        wasm.__wbindgen_export4(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Decode a Geohash string into `[longitude, latitude, width, height]`.
 *
 * Returns a `Float64Array` with:
 * - `[0]` center longitude
 * - `[1]` center latitude
 * - `[2]` bounding box width in degrees
 * - `[3]` bounding box height in degrees
 * @param {string} hash
 * @returns {Float64Array}
 */
export function geohashDecode(hash) {
    const ptr0 = passStringToWasm0(hash, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.geohashDecode(ptr0, len0);
    return takeObject(ret);
}

/**
 * Encode (longitude, latitude) to a Geohash string with given precision (1-12).
 * @param {number} lng
 * @param {number} lat
 * @param {number} precision
 * @returns {string}
 */
export function geohashEncode(lng, lat, precision) {
    let deferred1_0;
    let deferred1_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.geohashEncode(retptr, lng, lat, precision);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        deferred1_0 = r0;
        deferred1_1 = r1;
        return getStringFromWasm0(r0, r1);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export4(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Get the 8 neighboring Geohash cells (N, NE, E, SE, S, SW, W, NW).
 *
 * Returns a `JsValue` (Array) of 8 Geohash strings.
 * @param {string} hash
 * @returns {Array<any>}
 */
export function geohashNeighbors(hash) {
    const ptr0 = passStringToWasm0(hash, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.geohashNeighbors(ptr0, len0);
    return takeObject(ret);
}

/**
 * Get the approximate number of allocated bytes in WASM linear memory.
 *
 * This reads the current `memory.buffer.byteLength`. Note that WASM memory
 * only grows (never shrinks), so this value is the peak allocation size.
 *
 * Returns 0 on non-WASM targets.
 * @returns {number}
 */
export function getAllocatedBytes() {
    const ret = wasm.getAllocatedBytes();
    return ret >>> 0;
}

/**
 * Get the current input size limit in bytes.
 *
 * Returns 100 MB (104,857,600) if not changed.
 * @returns {number}
 */
export function getInputSizeLimit() {
    const ret = wasm.getInputSizeLimit();
    return ret >>> 0;
}

/**
 * Return a JSON array of supported coordinate reference systems.
 *
 * Each entry contains `code`, `name`, `description`.
 * @returns {string}
 */
export function getSupportedCrs() {
    let deferred1_0;
    let deferred1_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.getSupportedCrs(retptr);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        deferred1_0 = r0;
        deferred1_1 = r1;
        return getStringFromWasm0(r0, r1);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export4(deferred1_0, deferred1_1, 1);
    }
}

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
 * @param {Float64Array} coords
 * @param {number} cell_size_deg
 * @returns {Float64Array}
 */
export function gridIndex(coords, cell_size_deg) {
    try {
        const ret = wasm.gridIndex(addBorrowedObject(coords), cell_size_deg);
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {number} lng1
 * @param {number} lat1
 * @param {number} lng2
 * @param {number} lat2
 * @returns {number}
 */
export function haversineDistance(lng1, lat1, lng2, lat2) {
    const ret = wasm.haversineDistance(lng1, lat1, lng2, lat2);
    return ret;
}

/**
 * Initialize the WASM module. Call this once before any other function.
 *
 * Sets up the panic hook for better error messages in the browser console.
 */
export function init() {
    wasm.init();
}

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
 * @param {number} lng
 * @param {number} lat
 * @returns {boolean}
 */
export function isInChina(lng, lat) {
    const ret = wasm.isInChina(lng, lat);
    return ret !== 0;
}

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
 * @param {number} point_x
 * @param {number} point_y
 * @param {Float64Array} ring_coords
 * @returns {boolean}
 */
export function isPointInRing(point_x, point_y, ring_coords) {
    try {
        const ret = wasm.isPointInRing(point_x, point_y, addBorrowedObject(ring_coords));
        return ret !== 0;
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 *
 * Provides insight into WASM linear memory allocation, useful for monitoring
 * large spatial data processing workloads.
 *
 * **Note:** Only available in WASM runtime. On native, returns zeros.
 * @returns {MemoryInfo}
 */
export function memoryInfo() {
    const ret = wasm.memoryInfo();
    return MemoryInfo.__wrap(ret);
}

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
 * @param {number} lng1
 * @param {number} lat1
 * @param {number} lng2
 * @param {number} lat2
 * @returns {Float64Array}
 */
export function midpoint(lng1, lat1, lng2, lat2) {
    const ret = wasm.midpoint(lng1, lat1, lng2, lat2);
    return takeObject(ret);
}

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
 * @param {Float64Array} coords
 * @param {Float64Array} target_bounds
 * @returns {Float64Array}
 */
export function normalizeCoords(coords, target_bounds) {
    try {
        const ret = wasm.normalizeCoords(addBorrowedObject(coords), addBorrowedObject(target_bounds));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {string} input
 * @returns {Float64Array}
 */
export function parseGeoJsonCoords(input) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.parseGeoJsonCoords(retptr, ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        if (r2) {
            throw takeObject(r1);
        }
        return takeObject(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

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
 * @param {string} input
 * @returns {GeoJsonFeaturesResult}
 */
export function parseGeoJsonFeatures(input) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.parseGeoJsonFeatures(retptr, ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        if (r2) {
            throw takeObject(r1);
        }
        return GeoJsonFeaturesResult.__wrap(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

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
 * @param {string} input
 * @returns {LazyGeoJsonIter}
 */
export function parseGeoJsonLazy(input) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.parseGeoJsonLazy(retptr, ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        if (r2) {
            throw takeObject(r1);
        }
        return LazyGeoJsonIter.__wrap(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
 * Parse a GeoJSON FeatureCollection and return coordinates in separate
 * per-feature arrays, useful when you need to map coordinates back to
 * individual features.
 *
 * Returns a `js_sys::Array` where each element is a `Float64Array`
 * containing the coordinates for one feature.
 * @param {string} input
 * @returns {Array<any>}
 */
export function parseGeoJsonPerFeature(input) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.parseGeoJsonPerFeature(retptr, ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        if (r2) {
            throw takeObject(r1);
        }
        return takeObject(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

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
 * @param {string} input
 * @returns {string}
 */
export function parseGeoJsonProperties(input) {
    let deferred3_0;
    let deferred3_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.parseGeoJsonProperties(retptr, ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        var ptr2 = r0;
        var len2 = r1;
        if (r3) {
            ptr2 = 0; len2 = 0;
            throw takeObject(r2);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export4(deferred3_0, deferred3_1, 1);
    }
}

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
 * @param {string} input
 * @param {number} chunk_size
 * @param {Function} on_chunk
 * @returns {number}
 */
export function parseGeoJsonStream(input, chunk_size, on_chunk) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.parseGeoJsonStream(retptr, ptr0, len0, chunk_size, addBorrowedObject(on_chunk));
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        if (r2) {
            throw takeObject(r1);
        }
        return r0 >>> 0;
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {string} text
 * @returns {IfcGeometryResult}
 */
export function parseIfcGeometry(text) {
    const ptr0 = passStringToWasm0(text, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.parseIfcGeometry(ptr0, len0);
    return IfcGeometryResult.__wrap(ret);
}

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
 * @param {Uint8Array} bytes
 * @returns {Float64Array}
 */
export function parseWkb(bytes) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.parseWkb(retptr, addBorrowedObject(bytes));
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        if (r2) {
            throw takeObject(r1);
        }
        return takeObject(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {string} input
 * @returns {Float64Array}
 */
export function parseWkt(input) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.parseWkt(retptr, ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        if (r2) {
            throw takeObject(r1);
        }
        return takeObject(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
 * Calculate the area of a polygon in square meters using the spherical
 * excess formula.
 *
 * # Arguments
 * - `coords`: Flat `Float64Array` of a closed ring `[lng0,lat0, lng1,lat1, ..., lng0,lat0]`.
 *
 * For polygons with holes, use `areaWithHoles` instead.
 * @param {Float64Array} coords
 * @returns {number}
 */
export function polygonArea(coords) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.polygonArea(retptr, addBorrowedObject(coords));
        var r0 = getDataViewMemory0().getFloat64(retptr + 8 * 0, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        if (r3) {
            throw takeObject(r2);
        }
        return r0;
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {Float64Array} ring1
 * @param {Float64Array} ring2
 * @returns {Float64Array}
 */
export function polygonIntersection(ring1, ring2) {
    try {
        const ret = wasm.polygonIntersection(addBorrowedObject(ring1), addBorrowedObject(ring2));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
        heap[stack_pointer++] = undefined;
    }
}

/**
 * Check if two polygons intersect (share any point).
 *
 * # Arguments
 *
 * * `ring1` — First polygon as flat closed ring
 * * `ring2` — Second polygon as flat closed ring
 * @param {Float64Array} ring1
 * @param {Float64Array} ring2
 * @returns {boolean}
 */
export function polygonIntersects(ring1, ring2) {
    try {
        const ret = wasm.polygonIntersects(addBorrowedObject(ring1), addBorrowedObject(ring2));
        return ret !== 0;
    } finally {
        heap[stack_pointer++] = undefined;
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {Float64Array} ring1
 * @param {Float64Array} ring2
 * @returns {Float64Array}
 */
export function polygonUnion(ring1, ring2) {
    try {
        const ret = wasm.polygonUnion(addBorrowedObject(ring1), addBorrowedObject(ring2));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
        heap[stack_pointer++] = undefined;
    }
}

/**
 * Calculate the total length of a line string or polygon perimeter in meters
 * using the Haversine formula.
 *
 * # Arguments
 * - `coords`: Flat `Float64Array` `[lng0,lat0, lng1,lat1, ...]`.
 * @param {Float64Array} coords
 * @returns {number}
 */
export function polylineLength(coords) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.polylineLength(retptr, addBorrowedObject(coords));
        var r0 = getDataViewMemory0().getFloat64(retptr + 8 * 0, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        if (r3) {
            throw takeObject(r2);
        }
        return r0;
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {string} input
 * @param {string} key
 * @returns {string}
 */
export function removeProperty(input, key) {
    let deferred4_0;
    let deferred4_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(key, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len1 = WASM_VECTOR_LEN;
        wasm.removeProperty(retptr, ptr0, len0, ptr1, len1);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        var ptr3 = r0;
        var len3 = r1;
        if (r3) {
            ptr3 = 0; len3 = 0;
            throw takeObject(r2);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export4(deferred4_0, deferred4_1, 1);
    }
}

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
 * @param {string} input
 * @param {string} old_key
 * @param {string} new_key
 * @returns {string}
 */
export function renameProperty(input, old_key, new_key) {
    let deferred5_0;
    let deferred5_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(old_key, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(new_key, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len2 = WASM_VECTOR_LEN;
        wasm.renameProperty(retptr, ptr0, len0, ptr1, len1, ptr2, len2);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        var ptr4 = r0;
        var len4 = r1;
        if (r3) {
            ptr4 = 0; len4 = 0;
            throw takeObject(r2);
        }
        deferred5_0 = ptr4;
        deferred5_1 = len4;
        return getStringFromWasm0(ptr4, len4);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export4(deferred5_0, deferred5_1, 1);
    }
}

/**
 * Rhumb (constant-bearing) bearing from point 1 to point 2.
 *
 * # Arguments
 * - `lng1`, `lat1`: Point 1 in degrees.
 * - `lng2`, `lat2`: Point 2 in degrees.
 *
 * # Returns
 * Bearing in degrees [0, 360), where 0 = North, 90 = East.
 * @param {number} lng1
 * @param {number} lat1
 * @param {number} lng2
 * @param {number} lat2
 * @returns {number}
 */
export function rhumbBearing(lng1, lat1, lng2, lat2) {
    const ret = wasm.rhumbBearing(lng1, lat1, lng2, lat2);
    return ret;
}

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
 * @param {number} lng1
 * @param {number} lat1
 * @param {number} lng2
 * @param {number} lat2
 * @returns {number}
 */
export function rhumbDistance(lng1, lat1, lng2, lat2) {
    const ret = wasm.rhumbDistance(lng1, lat1, lng2, lat2);
    return ret;
}

/**
 * Dynamically set the maximum allowed input size in bytes.
 *
 * Default is 100 MB. Set to 0 to disable the limit.
 *
 * # Example (JS)
 * ```js
 * core.setInputSizeLimit(50 * 1024 * 1024); // 50 MB
 * ```
 * @param {number} bytes
 */
export function setInputSizeLimit(bytes) {
    wasm.setInputSizeLimit(bytes);
}

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
 * @param {Float64Array} coords
 * @param {number} tolerance
 * @returns {Float64Array}
 */
export function simplifyDouglasPeucker(coords, tolerance) {
    try {
        const ret = wasm.simplifyDouglasPeucker(addBorrowedObject(coords), tolerance);
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function sortCoordsByLat(coords) {
    try {
        const ret = wasm.sortCoordsByLat(addBorrowedObject(coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function sortCoordsByLng(coords) {
    try {
        const ret = wasm.sortCoordsByLng(addBorrowedObject(coords));
        return takeObject(ret);
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

/**
 * Interpolate a Z value on a TIN surface at (x, y) using barycentric interpolation.
 *
 * Finds the triangle containing (x, y) and interpolates Z.
 * If the point is outside the TIN convex hull, returns the Z of the nearest vertex.
 * @param {TinResult} tin
 * @param {number} x
 * @param {number} y
 * @returns {number}
 */
export function tinInterpolate(tin, x, y) {
    _assertClass(tin, TinResult);
    const ret = wasm.tinInterpolate(tin.__wbg_ptr, x, y);
    return ret;
}

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
 * @param {Float64Array} coords
 * @param {string} geometry_type
 * @returns {Uint8Array}
 */
export function toWkb(coords, geometry_type) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(geometry_type, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.toWkb(retptr, addBorrowedObject(coords), ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        if (r2) {
            throw takeObject(r1);
        }
        return takeObject(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {Float64Array} coords
 * @param {string} geometry_type
 * @returns {string}
 */
export function toWkt(coords, geometry_type) {
    let deferred3_0;
    let deferred3_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(geometry_type, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.toWkt(retptr, addBorrowedObject(coords), ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        var ptr2 = r0;
        var len2 = r1;
        if (r3) {
            ptr2 = 0; len2 = 0;
            throw takeObject(r2);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        heap[stack_pointer++] = undefined;
        wasm.__wbindgen_export4(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Check if two polygons touch (share boundary but not interior).
 *
 * # Arguments
 *
 * * `ring1` — First polygon as flat closed ring
 * * `ring2` — Second polygon as flat closed ring
 * @param {Float64Array} ring1
 * @param {Float64Array} ring2
 * @returns {boolean}
 */
export function touches(ring1, ring2) {
    try {
        const ret = wasm.touches(addBorrowedObject(ring1), addBorrowedObject(ring2));
        return ret !== 0;
    } finally {
        heap[stack_pointer++] = undefined;
        heap[stack_pointer++] = undefined;
    }
}

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
 * @param {number} zone
 * @param {number} easting
 * @param {number} northing
 * @param {boolean} is_north
 * @returns {Float64Array}
 */
export function utmToWgs84(zone, easting, northing, is_north) {
    const ret = wasm.utmToWgs84(zone, easting, northing, is_north);
    return takeObject(ret);
}

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
 * @param {Float64Array} coords
 * @param {string} crs
 * @returns {ValidationResult}
 */
export function validateCoords(coords, crs) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(crs, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.validateCoords(retptr, addBorrowedObject(coords), ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        if (r2) {
            throw takeObject(r1);
        }
        return ValidationResult.__wrap(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        heap[stack_pointer++] = undefined;
    }
}

/**
 * Return the library version string.
 * @returns {string}
 */
export function version() {
    let deferred1_0;
    let deferred1_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.version(retptr);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        deferred1_0 = r0;
        deferred1_1 = r1;
        return getStringFromWasm0(r0, r1);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export4(deferred1_0, deferred1_1, 1);
    }
}

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
 * @param {number} lng1
 * @param {number} lat1
 * @param {number} lng2
 * @param {number} lat2
 * @returns {number}
 */
export function vincentyDistance(lng1, lat1, lng2, lat2) {
    const ret = wasm.vincentyDistance(lng1, lat1, lng2, lat2);
    return ret;
}

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
 * @param {number} lng
 * @param {number} lat
 * @returns {Float64Array}
 */
export function wgs84ToUtm(lng, lat) {
    const ret = wasm.wgs84ToUtm(lng, lat);
    return takeObject(ret);
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_copy_to_typed_array_126bf2bedf877cd8: function(arg0, arg1, arg2) {
            new Uint8Array(getObject(arg2).buffer, getObject(arg2).byteOffset, getObject(arg2).byteLength).set(getArrayU8FromWasm0(arg0, arg1));
        },
        __wbg___wbindgen_debug_string_07cb72cfcc952e2b: function(arg0, arg1) {
            const ret = debugString(getObject(arg1));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_is_null_066086be3abe9bb3: function(arg0) {
            const ret = getObject(arg0) === null;
            return ret;
        },
        __wbg___wbindgen_is_undefined_244a92c34d3b6ec0: function(arg0) {
            const ret = getObject(arg0) === undefined;
            return ret;
        },
        __wbg___wbindgen_memory_c2356dd1a089dfbd: function() {
            const ret = wasm.memory;
            return addHeapObject(ret);
        },
        __wbg___wbindgen_number_get_dd6d69a6079f26f1: function(arg0, arg1) {
            const obj = getObject(arg1);
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_throw_9c75d47bf9e7731e: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_call_761cb61423a6f121: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            const ret = getObject(arg0).call(getObject(arg1), getObject(arg2), getObject(arg3), getObject(arg4));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_error_a6fa202b58aa1cd3: function(arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.error(getStringFromWasm0(arg0, arg1));
            } finally {
                wasm.__wbindgen_export4(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_get_41476db20fef99a8: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(getObject(arg0), getObject(arg1));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_get_652f640b3b0b6e3e: function(arg0, arg1) {
            const ret = getObject(arg0)[arg1 >>> 0];
            return addHeapObject(ret);
        },
        __wbg_ifcmesh_new: function(arg0) {
            const ret = IfcMesh.__wrap(arg0);
            return addHeapObject(ret);
        },
        __wbg_length_0a6ce016dc1460b0: function(arg0) {
            const ret = getObject(arg0).length;
            return ret;
        },
        __wbg_length_223a59fdabd2e386: function(arg0) {
            const ret = getObject(arg0).length;
            return ret;
        },
        __wbg_length_5693120f2a64a00d: function(arg0) {
            const ret = getObject(arg0).length;
            return ret;
        },
        __wbg_length_ba3c032602efe310: function(arg0) {
            const ret = getObject(arg0).length;
            return ret;
        },
        __wbg_length_eaf0f4c1173c0a9f: function(arg0) {
            const ret = getObject(arg0).length;
            return ret;
        },
        __wbg_new_227d7c05414eb861: function() {
            const ret = new Error();
            return addHeapObject(ret);
        },
        __wbg_new_2fad8ca02fd00684: function() {
            const ret = new Object();
            return addHeapObject(ret);
        },
        __wbg_new_with_length_2a29aa33411ddc89: function(arg0) {
            const ret = new Float64Array(arg0 >>> 0);
            return addHeapObject(ret);
        },
        __wbg_new_with_length_9011f5da794bf5d9: function(arg0) {
            const ret = new Uint8Array(arg0 >>> 0);
            return addHeapObject(ret);
        },
        __wbg_new_with_length_95e51bab415f3ca8: function(arg0) {
            const ret = new Array(arg0 >>> 0);
            return addHeapObject(ret);
        },
        __wbg_new_with_length_b91f070a091394cc: function(arg0) {
            const ret = new Uint32Array(arg0 >>> 0);
            return addHeapObject(ret);
        },
        __wbg_prototypesetcall_05223d3fcba7faf9: function(arg0, arg1, arg2) {
            Uint32Array.prototype.set.call(getArrayU32FromWasm0(arg0, arg1), getObject(arg2));
        },
        __wbg_prototypesetcall_442370bc228f2c6b: function(arg0, arg1, arg2) {
            Float64Array.prototype.set.call(getArrayF64FromWasm0(arg0, arg1), getObject(arg2));
        },
        __wbg_prototypesetcall_f2b501ba26592df2: function(arg0, arg1, arg2) {
            Float32Array.prototype.set.call(getArrayF32FromWasm0(arg0, arg1), getObject(arg2));
        },
        __wbg_prototypesetcall_fd4050e806e1d519: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), getObject(arg2));
        },
        __wbg_set_1f222978a13c32ed: function(arg0, arg1, arg2) {
            getObject(arg0).set(getArrayU32FromWasm0(arg1, arg2));
        },
        __wbg_set_5337f8ac82364a3f: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
            return ret;
        }, arguments); },
        __wbg_set_b0d9dc239ecdb765: function(arg0, arg1, arg2) {
            getObject(arg0).set(getArrayU8FromWasm0(arg1, arg2));
        },
        __wbg_set_e307b0b9eac6f966: function(arg0, arg1, arg2) {
            getObject(arg0).set(getArrayF64FromWasm0(arg1, arg2));
        },
        __wbg_set_f614f6a0608d1d1d: function(arg0, arg1, arg2) {
            getObject(arg0)[arg1 >>> 0] = takeObject(arg2);
        },
        __wbg_stack_3b0d974bbf31e44f: function(arg0, arg1) {
            const ret = getObject(arg1).stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbindgen_cast_0000000000000001: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return addHeapObject(ret);
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return addHeapObject(ret);
        },
        __wbindgen_object_clone_ref: function(arg0) {
            const ret = getObject(arg0);
            return addHeapObject(ret);
        },
        __wbindgen_object_drop_ref: function(arg0) {
            takeObject(arg0);
        },
    };
    return {
        __proto__: null,
        "./wasm_spatial_core_bg.js": import0,
    };
}

const Cesium3DTileFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_cesium3dtile_free(ptr, 1));
const CesiumMeshGeometryFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_cesiummeshgeometry_free(ptr, 1));
const GeoJsonFeaturesResultFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_geojsonfeaturesresult_free(ptr, 1));
const GltfBuilderFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_gltfbuilder_free(ptr, 1));
const IfcGeometryResultFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_ifcgeometryresult_free(ptr, 1));
const IfcMeshFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_ifcmesh_free(ptr, 1));
const LazyGeoJsonIterFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_lazygeojsoniter_free(ptr, 1));
const MemoryInfoFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_memoryinfo_free(ptr, 1));
const MvtFeatureFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_mvtfeature_free(ptr, 1));
const MvtLayerFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_mvtlayer_free(ptr, 1));
const SpatialEdgeIndexFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_spatialedgeindex_free(ptr, 1));
const SpatialIndexFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_spatialindex_free(ptr, 1));
const TinResultFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_tinresult_free(ptr, 1));
const ValidationResultFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_validationresult_free(ptr, 1));
const VectorTileEngineFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_vectortileengine_free(ptr, 1));
const VectorTileOptionsFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_vectortileoptions_free(ptr, 1));

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
}

function addBorrowedObject(obj) {
    if (stack_pointer == 1) throw new Error('out of js stack');
    heap[--stack_pointer] = obj;
    return stack_pointer;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function dropObject(idx) {
    if (idx < 1028) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayF64FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat64ArrayMemory0().subarray(ptr / 8, ptr / 8 + len);
}

function getArrayU32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

let cachedFloat64ArrayMemory0 = null;
function getFloat64ArrayMemory0() {
    if (cachedFloat64ArrayMemory0 === null || cachedFloat64ArrayMemory0.byteLength === 0) {
        cachedFloat64ArrayMemory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function getObject(idx) { return heap[idx]; }

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_export3(addHeapObject(e));
    }
}

let heap = new Array(1024).fill(undefined);
heap.push(undefined, null, true, false);

let heap_next = heap.length;

function isLikeNone(x) {
    return x === undefined || x === null;
}

function passArrayF64ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 8, 8) >>> 0;
    getFloat64ArrayMemory0().set(arg, ptr / 8);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

let stack_pointer = 1024;

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedFloat32ArrayMemory0 = null;
    cachedFloat64ArrayMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('wasm_spatial_core_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
