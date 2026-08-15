/**
 * Server-side batch processing helpers for wasm-spatial-core (Node.js).
 *
 * @packageDocumentation
 */
/** Options for point cloud → 3D Tiles batch conversion. */
export interface PointCloudBatchOptions {
    /** Max points per octree leaf (default: 50_000). */
    maxPointsPerNode?: number;
    /** Max octree depth (default: 21). */
    maxDepth?: number;
    /** Override auto-estimated average point spacing (meters). */
    avgSpacing?: number;
    /** Multiplier applied to spacing-based geometric error (default: 1.0). */
    spacingFactor?: number;
}
/** Result of a point cloud → 3D Tiles batch conversion. */
export interface PointCloudBatchResult {
    pointCount: number;
    positions: Float32Array;
    colors: Uint8Array | null;
    tilesetJson: string;
    tiles: Uint8Array[];
    tileUris: string[];
    tileCount: number;
    totalBytes: number;
    estimatedSpacing: number;
}
/** Options for GeoTIFF → terrain tileset batch conversion. */
export interface GeotiffBatchOptions {
    /** Minimum zoom level (default: 0). */
    minZoom?: number;
    /** Maximum zoom level (default: auto from image size). */
    maxZoom?: number;
}
/** Result of a GeoTIFF → terrain tileset batch conversion. */
export interface GeotiffBatchResult {
    width: number;
    height: number;
    bounds: Float64Array;
    tilesetJson: string;
    tileCount: number;
}
type SpatialCoreApi = Awaited<ReturnType<typeof import("./node.js").loadSpatialCoreNode>>;
/**
 * Parse a point cloud buffer and generate a complete 3D Tiles tileset.
 *
 * Suitable for server-side batch jobs: read a point cloud file from disk
 * (formats depend on WASM build features — default npm: LAS/PLY/OBJ/PCD),
 * pass the bytes here, then write `tilesetJson` and tile binaries to an output directory.
 */
export declare function batchPointCloudToTileset(core: SpatialCoreApi, bytes: Uint8Array, options?: PointCloudBatchOptions): Promise<PointCloudBatchResult>;
/**
 * Parse a GeoTIFF elevation grid and generate a quantized-mesh terrain tileset.
 */
export declare function batchGeotiffToTerrain(core: SpatialCoreApi, bytes: Uint8Array, options?: GeotiffBatchOptions): Promise<GeotiffBatchResult>;
export {};
