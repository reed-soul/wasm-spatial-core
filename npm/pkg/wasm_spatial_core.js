/* @ts-self-types="./wasm_spatial_core.d.ts" */
import { startWorkers } from './snippets/wasm-bindgen-rayon-38edf6e439f6d70d/src/workerHelpers.js';


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
 * A high-performance vector tile engine.
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
     * Request a tile by `z, x, y` coordinates.
     * Returns a raw `Uint8Array` representing the MVT (PBF) protobuf.
     * If the tile is empty or out of bounds, returns an empty array.
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
     * Create a new VectorTileEngine from a GeoJSON string.
     * @param {string} geojson_str
     * @param {VectorTileOptions} options
     */
    constructor(geojson_str, options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(geojson_str, wasm.__wbindgen_export3, wasm.__wbindgen_export4);
            const len0 = WASM_VECTOR_LEN;
            _assertClass(options, VectorTileOptions);
            var ptr1 = options.__destroy_into_raw();
            wasm.vectortileengine_new(retptr, ptr0, len0, ptr1);
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
    var ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export3);
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
    var ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export3);
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
    var ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export3);
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
    var ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export3);
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
    var ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export3);
    var len0 = WASM_VECTOR_LEN;
    wasm.batchMercatorToWgs84InPlace(ptr0, len0, addHeapObject(coords));
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
    var ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export3);
    var len0 = WASM_VECTOR_LEN;
    wasm.batchWgs84ToBd09InPlace(ptr0, len0, addHeapObject(coords));
}

/**
 * Batch convert a flat array of `[lng, lat, ...]` into `[x, y, z, ...]`.
 * @param {Float64Array} coords
 * @returns {Float64Array}
 */
export function batchWgs84ToCartesian3(coords) {
    const ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export3);
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
    var ptr0 = passArrayF64ToWasm0(_coords, wasm.__wbindgen_export3);
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
    var ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export3);
    var len0 = WASM_VECTOR_LEN;
    wasm.batchWgs84ToGcj02InPlace(ptr0, len0, addHeapObject(coords));
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
    var ptr0 = passArrayF64ToWasm0(coords, wasm.__wbindgen_export3);
    var len0 = WASM_VECTOR_LEN;
    wasm.batchWgs84ToMercatorInPlace(ptr0, len0, addHeapObject(coords));
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
 * Return the total number of features in a GeoJSON string.
 *
 * Useful for progress reporting before parsing a very large file.
 * @param {string} input
 * @returns {number}
 */
export function countGeoJsonFeatures(input) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export3, wasm.__wbindgen_export4);
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
 * Generate triangulated mesh from GeoJSON Polygons/MultiPolygons.
 * @param {string} geojson_str
 * @param {string | null} [height_property]
 * @returns {CesiumMeshGeometry}
 */
export function generateCesiumGeometry(geojson_str, height_property) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(geojson_str, wasm.__wbindgen_export3, wasm.__wbindgen_export4);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(height_property) ? 0 : passStringToWasm0(height_property, wasm.__wbindgen_export3, wasm.__wbindgen_export4);
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
 * Initialize the WASM module. Call this once before any other function.
 *
 * Sets up the panic hook for better error messages in the browser console.
 */
export function init() {
    wasm.init();
}

/**
 * @param {number} num_threads
 * @returns {Promise<any>}
 */
export function initThreadPool(num_threads) {
    const ret = wasm.initThreadPool(num_threads);
    return takeObject(ret);
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
 * Returns a `JsValue` error if the input is not valid GeoJSON.
 * @param {string} input
 * @returns {Float64Array}
 */
export function parseGeoJsonCoords(input) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export3, wasm.__wbindgen_export4);
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
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export3, wasm.__wbindgen_export4);
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
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export3, wasm.__wbindgen_export4);
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
        wasm.__wbindgen_export2(deferred1_0, deferred1_1, 1);
    }
}

export class wbg_rayon_PoolBuilder {
    static __wrap(ptr) {
        const obj = Object.create(wbg_rayon_PoolBuilder.prototype);
        obj.__wbg_ptr = ptr;
        wbg_rayon_PoolBuilderFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        wbg_rayon_PoolBuilderFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wbg_rayon_poolbuilder_free(ptr, 0);
    }
    build() {
        wasm.wbg_rayon_poolbuilder_build(this.__wbg_ptr);
    }
    /**
     * @returns {number}
     */
    numThreads() {
        const ret = wasm.wbg_rayon_poolbuilder_numThreads(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    receiver() {
        const ret = wasm.wbg_rayon_poolbuilder_receiver(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) wbg_rayon_PoolBuilder.prototype[Symbol.dispose] = wbg_rayon_PoolBuilder.prototype.free;

/**
 * @param {number} receiver
 */
export function wbg_rayon_start_worker(receiver) {
    wasm.wbg_rayon_start_worker(receiver);
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_copy_to_typed_array_126bf2bedf877cd8: function(arg0, arg1, arg2) {
            new Uint8Array(getObject(arg2).buffer, getObject(arg2).byteOffset, getObject(arg2).byteLength).set(getArrayU8FromWasm0(arg0, arg1));
        },
        __wbg___wbindgen_is_undefined_244a92c34d3b6ec0: function(arg0) {
            const ret = getObject(arg0) === undefined;
            return ret;
        },
        __wbg___wbindgen_memory_c2356dd1a089dfbd: function() {
            const ret = wasm.memory;
            return addHeapObject(ret);
        },
        __wbg___wbindgen_module_df704393dfd1853c: function() {
            const ret = wasmModule;
            return addHeapObject(ret);
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
                wasm.__wbindgen_export2(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_instanceof_Window_4153c1818a1c0c0b: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_length_223a59fdabd2e386: function(arg0) {
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
        __wbg_prototypesetcall_442370bc228f2c6b: function(arg0, arg1, arg2) {
            Float64Array.prototype.set.call(getArrayF64FromWasm0(arg0, arg1), getObject(arg2));
        },
        __wbg_set_1f222978a13c32ed: function(arg0, arg1, arg2) {
            getObject(arg0).set(getArrayU32FromWasm0(arg1, arg2));
        },
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
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export3, wasm.__wbindgen_export4);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_startWorkers_8b582d57e92bd2d4: function(arg0, arg1, arg2) {
            const ret = startWorkers(takeObject(arg0), takeObject(arg1), wbg_rayon_PoolBuilder.__wrap(arg2));
            return addHeapObject(ret);
        },
        __wbg_static_accessor_GLOBAL_THIS_1c7f1bd6c6941fdb: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_static_accessor_GLOBAL_e039bc914f83e74e: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_static_accessor_SELF_8bf8c48c28420ad5: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_static_accessor_WINDOW_6aeee9b51652ee0f: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
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

const CesiumMeshGeometryFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_cesiummeshgeometry_free(ptr, 1));
const SpatialIndexFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_spatialindex_free(ptr, 1));
const VectorTileEngineFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_vectortileengine_free(ptr, 1));
const VectorTileOptionsFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_vectortileoptions_free(ptr, 1));
const wbg_rayon_PoolBuilderFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wbg_rayon_poolbuilder_free(ptr, 1));

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

function dropObject(idx) {
    if (idx < 1028) return;
    heap[idx] = heap_next;
    heap_next = idx;
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
        wasm.__wbindgen_export(addHeapObject(e));
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
