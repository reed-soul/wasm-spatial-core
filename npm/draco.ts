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

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Internal: parse a .pnts binary to extract raw positions and colors
// ---------------------------------------------------------------------------

interface PntsParsedData {
  positions: Float32Array;
  colors: Uint8Array | null;
  center: [number, number, number];
}

/**
 * Parse a 3D Tiles `.pnts` binary and extract raw position/color arrays.
 *
 * Layout: [28-byte header][feature table JSON (padded)][feature table binary]
 */
function parsePntsTile(buf: Uint8Array): PntsParsedData {
  // Header (28 bytes)
  const magic = String.fromCharCode(buf[0], buf[1], buf[2], buf[3]);
  if (magic !== "pnts") {
    throw new Error(`Invalid pnts magic: ${magic}`);
  }
  const view = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
  const version = view.getUint32(4, true);
  const byteLength = view.getUint32(8, true);
  const ftJsonByteLen = view.getUint32(12, true);
  const ftBinaryByteLen = view.getUint32(16, true);
  const btJsonByteLen = view.getUint32(20, true);
  const btBinaryByteLen = view.getUint32(24, true);

  // Feature Table JSON
  const ftJsonOffset = 28;
  const ftJsonRaw = new TextDecoder().decode(
    buf.slice(ftJsonOffset, ftJsonOffset + ftJsonByteLen)
  );
  const ft = JSON.parse(ftJsonRaw) as Record<string, unknown>;

  // Feature Table Binary offset
  const ftBinaryOffset = ftJsonOffset + padTo4(ftJsonByteLen);

  // Extract positions
  const posEntry = ft["POSITION"] as Record<string, number> | undefined;
  if (!posEntry || posEntry["byteOffset"] === undefined) {
    throw new Error("pnts tile has no POSITION attribute");
  }
  const posOffset = ftBinaryOffset + posEntry["byteOffset"];

  // Determine point count from POSITION size.
  // Without explicit POINTS_LENGTH, infer from available data.
  const pointsLength = (ft["POINTS_LENGTH"] as number | undefined) ?? null;

  let numPoints: number;
  if (pointsLength !== null) {
    numPoints = pointsLength;
  } else {
    // Estimate: remaining bytes / 12 (3 × float32)
    const remainingAfterPos = buf.byteLength - posOffset - (ftBinaryByteLen - posEntry["byteOffset"]);
    numPoints = Math.floor(remainingAfterPos / 12);
  }

  const positions = new Float32Array(
    buf.buffer,
    buf.byteOffset + posOffset,
    numPoints * 3
  );

  // Extract colors (RGB, 3 bytes per point)
  const colorEntry = ft["RGB"] as Record<string, number> | undefined;
  let colors: Uint8Array | null = null;
  if (colorEntry && colorEntry["byteOffset"] !== undefined) {
    const colorOffset = ftBinaryOffset + colorEntry["byteOffset"];
    colors = new Uint8Array(buf.buffer, buf.byteOffset + colorOffset, numPoints * 3);
  }

  // Extract RTC_CENTER if present
  const rtcCenter = ft["RTC_CENTER"] as [number, number, number] | undefined;
  const center: [number, number, number] = rtcCenter ?? [0, 0, 0];

  // Copy to avoid detached buffers
  return {
    positions: new Float32Array(positions),
    colors: colors ? new Uint8Array(colors) : null,
    center,
  };
}

// ---------------------------------------------------------------------------
// Internal: build a Draco-compressed .pnts binary
// ---------------------------------------------------------------------------

function padTo4(n: number): number {
  return (n + 3) & ~3;
}

function padBuf(arr: Uint8Array): Uint8Array {
  const padded = padTo4(arr.byteLength);
  if (padded === arr.byteLength) return arr;
  const out = new Uint8Array(padded);
  out.set(arr);
  return out;
}

/**
 * Encode positions via Draco point cloud encoder.
 * Returns compressed Draco buffer.
 */
function dracoEncodePointCloud(
  encoderModule: any,
  positions: Float32Array,
  colors: Uint8Array | null,
  options: DracoCompressOptions
): Uint8Array {
  const encoder = new encoderModule.Encoder();
  const pcBuilder = new encoderModule.PointCloudBuilder();
  const pointCloud = new encoderModule.PointCloud();

  const numPoints = positions.length / 3;

  // Add positions
  const posAttrId = pcBuilder.AddFloatAttribute(
    pointCloud,
    encoderModule.POSITION,
    numPoints,
    3,
    positions
  );

  // Add colors if requested
  let colorAttrId = -1;
  if (colors && options.compressColors) {
    // Convert RGB uint8 to float32 array for Draco
    const colorFloats = new Float32Array(numPoints * 3);
    for (let i = 0; i < numPoints * 3; i++) {
      colorFloats[i] = colors[i] / 255.0;
    }
    colorAttrId = pcBuilder.AddFloatAttribute(
      pointCloud,
      encoderModule.COLOR,
      numPoints,
      3,
      colorFloats
    );
  }

  // Set encoding options
  encoder.SetSpeedOptions(
    options.encodeSpeed ?? 5,
    options.decodeSpeed ?? 5
  );
  encoder.SetAttributeQuantization(
    encoderModule.POSITION,
    options.quantizationBits ?? 11
  );
  if (colorAttrId >= 0) {
    encoder.SetAttributeQuantization(
      encoderModule.COLOR,
      options.colorQuantizationBits ?? 8
    );
  }
  // Note: For point clouds, the default encoding method is sequential.
  // No need to call SetEncodingMethod — POINT_CLOUD_SEQUENTIAL_ENCODING
  // doesn't exist in the draco3d JS bindings.

  // Encode
  const encodedData = new encoderModule.DracoInt8Array();
  const encodedLen = encoder.EncodePointCloudToDracoBuffer(
    pointCloud,
    true, // preserve order
    encodedData
  );

  if (encodedLen <= 0) {
    encoderModule.destroy(encodedData);
    encoderModule.destroy(encoder);
    encoderModule.destroy(pcBuilder);
    encoderModule.destroy(pointCloud);
    throw new Error("Draco encoding failed");
  }

  // Copy to Uint8Array
  const result = new Uint8Array(encodedLen);
  for (let i = 0; i < encodedLen; i++) {
    result[i] = encodedData.GetValue(i);
  }

  encoderModule.destroy(encodedData);
  encoderModule.destroy(encoder);
  encoderModule.destroy(pcBuilder);
  encoderModule.destroy(pointCloud);

  return result;
}

/**
 * Build a .pnts binary with EXT_draco_point_cloud extension.
 *
 * Feature Table JSON:
 * ```json
 * {
 *   "POINTS_LENGTH": N,
 *   "POSITION": { "byteOffset": 0 },
 *   "extensions": {
 *     "EXT_draco_point_cloud": {
 *       "properties": [
 *         { "attribute": "POSITION", "byteOffset": 0, "elementCount": N, "componentType": 5126, "count": 3 }
 *       ]
 *     },
 *     ...
 *   },
 *   "extensions": {
 *     "EXT_structural_metadata": { ... }
 *   }
 * }
 * ```
 */
function buildDracoPnts(
  numPoints: number,
  dracoBuffer: Uint8Array,
  colors: Uint8Array | null,
  compressColors: boolean
): Uint8Array {
  // Feature Table JSON
  const hasColors = colors !== null && !compressColors;
  const dracoDataOffset = 0;

  let ftJson: Record<string, unknown>;
  if (hasColors) {
    // Colors stay uncompressed, Draco data at offset 0
    const colorByteOffset = dracoBuffer.length;
    ftJson = {
      POINTS_LENGTH: numPoints,
      POSITION: { byteOffset: 0 },
      RGB: { byteOffset: colorByteOffset },
      extensions: {
        EXT_draco_point_cloud: {
          properties: [
            {
              attribute: "POSITION",
              byteOffset: dracoDataOffset,
              elementCount: numPoints,
              componentType: 5126, // FLOAT
              count: 3,
            },
          ],
        },
      },
    };
  } else {
    ftJson = {
      POINTS_LENGTH: numPoints,
      POSITION: { byteOffset: 0 },
      extensions: {
        EXT_draco_point_cloud: {
          properties: [
            {
              attribute: "POSITION",
              byteOffset: dracoDataOffset,
              elementCount: numPoints,
              componentType: 5126, // FLOAT
              count: 3,
            },
          ],
        },
      },
    };
  }

  const ftJsonStr = JSON.stringify(ftJson);
  const ftJsonPadded = padBuf(new TextEncoder().encode(ftJsonStr));

  // Feature Table Binary = draco buffer + optional colors
  let ftBinary: Uint8Array;
  if (hasColors) {
    const colorPadded = padBuf(colors!);
    ftBinary = new Uint8Array(dracoBuffer.length + colorPadded.length);
    ftBinary.set(dracoBuffer, 0);
    ftBinary.set(colorPadded, dracoBuffer.length);
  } else {
    ftBinary = padBuf(dracoBuffer);
  }

  // Batch Table (empty)
  const btJson = padBuf(new TextEncoder().encode("{}"));

  // Header (28 bytes)
  const headerLen = 28;
  const byteLength = headerLen + ftJsonPadded.length + ftBinary.length + btJson.length;
  const header = new ArrayBuffer(headerLen);
  const hv = new DataView(header);

  // Magic
  hv.setUint8(0, 0x70); // p
  hv.setUint8(1, 0x6e); // n
  hv.setUint8(2, 0x74); // t
  hv.setUint8(3, 0x73); // s
  // Version
  hv.setUint32(4, 1, true);
  // Byte length
  hv.setUint32(8, byteLength, true);
  // Feature Table JSON byte length
  hv.setUint32(12, ftJsonPadded.length, true);
  // Feature Table Binary byte length
  hv.setUint32(16, ftBinary.length, true);
  // Batch Table JSON byte length
  hv.setUint32(20, btJson.length, true);
  // Batch Table Binary byte length
  hv.setUint32(24, 0, true);

  // Assemble
  const result = new Uint8Array(byteLength);
  result.set(new Uint8Array(header), 0);
  result.set(ftJsonPadded, headerLen);
  result.set(ftBinary, headerLen + ftJsonPadded.length);
  result.set(btJson, headerLen + ftJsonPadded.length + ftBinary.length);

  return result;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

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
export function compressPntsTileWithDraco(
  tileData: Uint8Array,
  encoderModule: any,
  options: DracoCompressOptions = {}
): DracoTileResult {
  const parsed = parsePntsTile(tileData);
  const numPoints = parsed.positions.length / 3;

  const dracoBuffer = dracoEncodePointCloud(
    encoderModule,
    parsed.positions,
    parsed.colors,
    options
  );

  const compressed = buildDracoPnts(
    numPoints,
    dracoBuffer,
    parsed.colors,
    options.compressColors ?? false
  );

  return {
    data: compressed,
    originalSize: tileData.byteLength,
    compressedSize: compressed.byteLength,
    ratio: compressed.byteLength / tileData.byteLength,
  };
}

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
export function compressTilesetWithDraco(
  tileset: TilesetLike,
  encoderModule: any,
  options: DracoCompressOptions = {}
): DracoTileResult[] {
  const count = tileset.tileCount();
  const results: DracoTileResult[] = [];

  for (let i = 0; i < count; i++) {
    const tileData = tileset.tile(i);
    const result = compressPntsTileWithDraco(tileData, encoderModule, options);
    results.push(result);
    options.onProgress?.(i, count, result.originalSize, result.compressedSize);
  }

  return results;
}

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
export function buildDracoTileset(
  tileset: TilesetLike,
  encoderModule: any,
  options: DracoCompressOptions = {}
): {
  tilesetJson: string;
  tiles: Record<string, Uint8Array>;
  totalOriginalSize: number;
  totalCompressedSize: number;
  compressionRatio: number;
} {
  const results = compressTilesetWithDraco(tileset, encoderModule, options);
  const tiles: Record<string, Uint8Array> = {};
  let totalOriginalSize = 0;
  let totalCompressedSize = 0;

  for (let i = 0; i < results.length; i++) {
    // Use original tile URI from tileset
    const uri = `tile_${i}.pnts`;
    tiles[uri] = results[i].data;
    totalOriginalSize += results[i].originalSize;
    totalCompressedSize += results[i].compressedSize;
  }

  return {
    tilesetJson: tileset.tilesetJson(),
    tiles,
    totalOriginalSize,
    totalCompressedSize,
    compressionRatio: totalCompressedSize / totalOriginalSize,
  };
}
