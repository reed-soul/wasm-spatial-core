/* tslint:disable */
/* eslint-disable */

/**
 * A high-performance spatial index using an R-Tree.
 */
export class SpatialIndex {
    free(): void;
    [Symbol.dispose](): void;
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
 * Batch WGS-84 → BD-09. Returns a **new** `Float64Array`.
 */
export function batchWgs84ToBd09(coords: Float64Array): Float64Array;

/**
 * **[Zero-Copy]** In-place WGS-84 → BD-09.
 */
export function batchWgs84ToBd09InPlace(coords: Float64Array): void;

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
 * Batch WGS-84 → Web Mercator (EPSG:3857). Returns a **new** `Float64Array`.
 */
export function batchWgs84ToMercator(coords: Float64Array): Float64Array;

/**
 * **[Zero-Copy]** In-place WGS-84 → Web Mercator (EPSG:3857).
 */
export function batchWgs84ToMercatorInPlace(coords: Float64Array): void;

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
 * Return the total number of features in a GeoJSON string.
 *
 * Useful for progress reporting before parsing a very large file.
 */
export function countGeoJsonFeatures(input: string): number;

/**
 * Initialize the WASM module. Call this once before any other function.
 *
 * Sets up the panic hook for better error messages in the browser console.
 */
export function init(): void;

export function initThreadPool(num_threads: number): Promise<any>;

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
 */
export function parseGeoJsonCoords(input: string): Float64Array;

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
 * Return the library version string.
 */
export function version(): string;

export class wbg_rayon_PoolBuilder {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    build(): void;
    numThreads(): number;
    receiver(): number;
}

export function wbg_rayon_start_worker(receiver: number): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_spatialindex_free: (a: number, b: number) => void;
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
    readonly batchWgs84ToBd09: (a: number) => number;
    readonly batchWgs84ToBd09InPlace: (a: number, b: number, c: number) => void;
    readonly batchWgs84ToCgcs2000: (a: number) => number;
    readonly batchWgs84ToCgcs2000InPlace: (a: number, b: number, c: number) => void;
    readonly batchWgs84ToGcj02: (a: number) => number;
    readonly batchWgs84ToGcj02InPlace: (a: number, b: number, c: number) => void;
    readonly batchWgs84ToMercator: (a: number) => number;
    readonly batchWgs84ToMercatorInPlace: (a: number, b: number, c: number) => void;
    readonly cgcs2000IsWgs84Compatible: () => number;
    readonly countGeoJsonFeatures: (a: number, b: number, c: number) => void;
    readonly init: () => void;
    readonly parseGeoJsonCoords: (a: number, b: number, c: number) => void;
    readonly parseGeoJsonPerFeature: (a: number, b: number, c: number) => void;
    readonly parseGeoJsonStream: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly spatialindex_new: (a: number) => number;
    readonly spatialindex_searchBBox: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly spatialindex_size: (a: number) => number;
    readonly version: (a: number) => void;
    readonly __wbg_wbg_rayon_poolbuilder_free: (a: number, b: number) => void;
    readonly initThreadPool: (a: number) => number;
    readonly wbg_rayon_poolbuilder_build: (a: number) => void;
    readonly wbg_rayon_poolbuilder_numThreads: (a: number) => number;
    readonly wbg_rayon_poolbuilder_receiver: (a: number) => number;
    readonly wbg_rayon_start_worker: (a: number) => void;
    readonly __wbindgen_export: (a: number) => void;
    readonly __wbindgen_export2: (a: number, b: number, c: number) => void;
    readonly __wbindgen_export3: (a: number, b: number) => number;
    readonly __wbindgen_export4: (a: number, b: number, c: number, d: number) => number;
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
