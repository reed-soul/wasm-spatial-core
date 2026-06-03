/**
 * @module draco
 *
 * Draco compression for 3D Tiles point cloud tiles.
 *
 * Uses Google's `draco3d` npm package (Apache-2.0) to compress
 * per-tile point positions, then reassembles the 3D Tiles `.pnts`
 * binary with the `EXT_draco_point_cloud` extension.
 *
 * ## Installation
 *
 * ```bash
 * npm install draco3d
 * ```
 *
 * ## Quick start
 *
 * ```ts
 * import { loadSpatialCore, compressTilesetWithDraco } from "wasm-spatial-core/draco";
 * import { createEncoderModule } from "draco3d";
 *
 * const wasm = await loadSpatialCore();
 * const tileset = wasm.generateTileset(positions, 50000, 21);
 *
 * const encoderModule = await createEncoderModule({});
 * const compressed = compressTilesetWithDraco(tileset, encoderModule, {
 *   quantizationBits: 12,
 * });
 * // compressed.tiles[i] is a Draco-compressed .pnts Uint8Array
 * ```
 *
 * @packageDocumentation
 */
/**
 * Options for Draco compression.
 */
export interface DracoCompressOptions {
    /**
     * Quantization bits for POSITION attribute.
     * Higher = better quality, larger output.
     * Typical range: 8–18. Default: 11.
     */
    quantizationBits?: number;
    /**
     * Encoding speed (0–10). Higher = faster encoding, lower compression.
     * Default: 5.
     */
    encodeSpeed?: number;
    /**
     * Decode speed (0–10). Higher = faster decoding, lower compression.
     * Default: 5.
     */
    decodeSpeed?: number;
    /**
     * Also compress RGB colors via Draco (if tile has colors).
     * Default: false — colors are left uncompressed for smaller gains.
     */
    compressColors?: boolean;
    /**
     * Quantization bits for RGB attribute (only when `compressColors` is true).
     * Default: 8.
     */
    colorQuantizationBits?: number;
    /**
     * Called after each tile is compressed with progress info.
     */
    onProgress?: (index: number, total: number, original: number, compressed: number) => void;
}
/** Result of Draco-compressing a single tile. */
export interface DracoTileResult {
    /** The Draco-compressed `.pnts` binary. */
    data: Uint8Array;
    /** Original uncompressed size. */
    originalSize: number;
    /** Compressed size. */
    compressedSize: number;
    /** Compression ratio (compressed / original). */
    ratio: number;
}
/**
 * A WASM `TilesetResult`-like object (duck-typed for flexibility).
 *
 * Any object that exposes `.tileCount`, `.tile(i)`, and `.tilesetJson()` works.
 */
export interface TilesetLike {
    tileCount(): number;
    tile(index: number): Uint8Array;
    tilesetJson(): string;
}
/**
 * Compress a single `.pnts` tile with Draco.
 *
 * @param tileData  — Raw `.pnts` binary.
 * @param encoderModule — Initialized `draco3d.createEncoderModule()` result.
 * @param options   — Compression options.
 * @returns Compressed tile info.
 *
 * @example
 * ```ts
 * const encoderModule = await createEncoderModule({});
 * const result = compressPntsTileWithDraco(tileset.tile(0), encoderModule);
 * console.log(`Compression ratio: ${(result.ratio * 100).toFixed(1)}%`);
 * ```
 */
export declare function compressPntsTileWithDraco(tileData: Uint8Array, encoderModule: any, options?: DracoCompressOptions): DracoTileResult;
/**
 * Compress all tiles in a `TilesetResult` with Draco.
 *
 * Returns an array of compressed tiles in the same order. You can
 * combine these with the original `tilesetJson()` to serve a
 * Draco-compressed 3D Tiles tileset.
 *
 * @param tileset        — WASM `TilesetResult` or compatible object.
 * @param encoderModule  — Initialized `draco3d.createEncoderModule()` result.
 * @param options        — Compression options.
 * @returns Array of per-tile compression results.
 *
 * @example
 * ```ts
 * const encoderModule = await createEncoderModule({});
 * const results = compressTilesetWithDraco(tileset, encoderModule, {
 *   quantizationBits: 12,
 *   onProgress: (i, total, orig, comp) => {
 *     console.log(`Tile ${i + 1}/${total}: ${(comp / orig * 100).toFixed(0)}%`);
 *   },
 * });
 * const totalOrig = results.reduce((s, r) => s + r.originalSize, 0);
 * const totalComp = results.reduce((s, r) => s + r.compressedSize, 0);
 * console.log(`Overall: ${(totalComp / totalOrig * 100).toFixed(1)}% of original`);
 * ```
 */
export declare function compressTilesetWithDraco(tileset: TilesetLike, encoderModule: any, options?: DracoCompressOptions): DracoTileResult[];
/**
 * Build a ready-to-serve Draco-compressed tileset.
 *
 * Returns an object with:
 * - `tilesetJson` — the tileset.json string (same as input)
 * - `tiles` — map of `{ [uri]: Uint8Array }` (compressed tile data)
 * - `totalOriginalSize` / `totalCompressedSize` — aggregate stats
 *
 * @example
 * ```ts
 * const { tiles, tilesetJson, totalCompressedSize, totalOriginalSize } =
 *   await buildDracoTileset(tileset, encoderModule);
 *
 * for (const [uri, data] of Object.entries(tiles)) {
 *   response = new Response(data, {
 *     headers: { "Content-Type": "application/octet-stream" },
 *   });
 * }
 * ```
 */
export declare function buildDracoTileset(tileset: TilesetLike, encoderModule: any, options?: DracoCompressOptions): {
    tilesetJson: string;
    tiles: Record<string, Uint8Array>;
    totalOriginalSize: number;
    totalCompressedSize: number;
    compressionRatio: number;
};
