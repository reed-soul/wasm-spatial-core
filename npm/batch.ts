/**
 * Server-side batch processing helpers for wasm-spatial-core (Node.js).
 *
 * @packageDocumentation
 */

import type { TilesetResult } from "./wasm_spatial_core.js";

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

function collectTiles(tileset: TilesetResult): Uint8Array[] {
  const tiles: Uint8Array[] = [];
  for (let i = 0; i < tileset.tileCount; i++) {
    tiles.push(tileset.tile(i));
  }
  return tiles;
}

function collectTileUris(tileset: TilesetResult): string[] {
  const uris: string[] = [];
  for (let i = 0; i < tileset.tileCount; i++) {
    uris.push(tileset.tileUri(i));
  }
  return uris;
}

/**
 * Parse a point cloud buffer and generate a complete 3D Tiles tileset.
 *
 * Suitable for server-side batch jobs: read a point cloud file from disk
 * (formats depend on WASM build features — default npm: LAS/PLY/OBJ/PCD),
 * pass the bytes here, then write `tilesetJson` and tile binaries to an output directory.
 */
export async function batchPointCloudToTileset(
  core: SpatialCoreApi,
  bytes: Uint8Array,
  options: PointCloudBatchOptions = {},
): Promise<PointCloudBatchResult> {
  const cloud = core.parsePointCloudAuto(bytes);
  const pointCount = cloud.pointCount;
  const positions = cloud.positions;
  const colors = cloud.colors;

  const maxPointsPerNode = options.maxPointsPerNode ?? 50_000;
  const maxDepth = options.maxDepth ?? 21;

  const estimatedSpacing = core.estimatePointSpacing(positions, 1000);

  const tileset =
    options.avgSpacing !== undefined || options.spacingFactor !== undefined
      ? core.generateTilesetWithSpacing(
          positions,
          maxPointsPerNode,
          maxDepth,
          colors ?? undefined,
          options.avgSpacing ?? null,
          options.spacingFactor ?? null,
        )
      : core.generateTileset(positions, maxPointsPerNode, maxDepth, colors ?? undefined);

  const tiles = collectTiles(tileset);
  const tileUris = collectTileUris(tileset);

  return {
    pointCount,
    positions,
    colors,
    tilesetJson: tileset.tilesetJson(),
    tiles,
    tileUris,
    tileCount: tileset.tileCount,
    totalBytes: tileset.totalBytes,
    estimatedSpacing,
  };
}

/**
 * Parse a GeoTIFF elevation grid and generate a quantized-mesh terrain tileset.
 */
export async function batchGeotiffToTerrain(
  core: SpatialCoreApi,
  bytes: Uint8Array,
  options: GeotiffBatchOptions = {},
): Promise<GeotiffBatchResult> {
  const info = core.parseGeotiff(bytes);
  const width = info.width;
  const height = info.height;
  const bounds = info.bounds;
  const elevations = info.elevation;

  const center = new Float64Array([
    (bounds[0] + bounds[2]) * 0.5,
    (bounds[1] + bounds[3]) * 0.5,
    0,
  ]);

  const maxZoom =
    options.maxZoom ??
    Math.max(0, Math.ceil(Math.log2(Math.max(width, height))) - 4);

  const tileset = core.encodeTerrainTileset(
    elevations,
    width,
    height,
    bounds,
    center,
    maxZoom,
  );

  return {
    width,
    height,
    bounds,
    tilesetJson: tileset.tilesetJson(),
    tileCount: tileset.tileCount,
  };
}
