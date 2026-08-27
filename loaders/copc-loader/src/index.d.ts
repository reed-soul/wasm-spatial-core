/**
 * copc-loader — COPC loader for loaders.gl
 * @packageDocumentation
 */

/** [minX, minY, minZ, maxX, maxY, maxZ] */
export type BBox6 = [number, number, number, number, number, number];

export interface CopcInfo {
  version: string;
  pointFormatId: number;
  pointCount: number;
  totalBytes: number;
  fileSize: number;
  pointDataOffset: number;
  xScale: number;
  yScale: number;
  zScale: number;
  xOffset: number;
  yOffset: number;
  zOffset: number;
  /** [minX, minY, minZ, maxX, maxY, maxZ] */
  bounds: number[];
  chunkTable: Array<{ offset: number; count: number; size: number }>;
  hasHierarchy: boolean;
}

export interface PointCloudAttribute {
  value: Float32Array | Uint8Array;
  size: number;
  normalized?: boolean;
}

export interface CopcData {
  loaderData: { copcInfo: CopcInfo };
  header: { vertexCount: number; boundingBox: number[] | null };
  schema: null;
  mode: 0;
  attributes: {
    POSITION: PointCloudAttribute;
    COLOR_0?: PointCloudAttribute;
  };
}

export interface CopcParseOptions {
  copc?: {
    /** Spatial subset; null = whole file. */
    bbox?: BBox6 | null;
  };
}

/** loaders.gl `test` — LASF magic. */
export function isCOPCFile(data: unknown): boolean;

/** Provide an already-initialized wasm-spatial-core API. */
export function setCore(core: object | null): void;

/** Initialize the WASM core (idempotent). */
export function init(): Promise<object>;

/** Parse a full COPC/LAZ file already in memory. */
export function parse(data: ArrayBuffer | Uint8Array, options?: CopcParseOptions): Promise<CopcData>;

/** Synchronous parse — core must be initialized first. */
export function parseSync(data: ArrayBuffer | Uint8Array, options?: CopcParseOptions): CopcData;

export interface LoadCOPCOptions {
  /** Post-decompression spatial filter. */
  bbox?: BBox6 | null;
  onProgress?: (info: { chunksDone: number; chunksTotal: number; points: number }) => void;
  signal?: AbortSignal;
  /** Custom fetch implementation (tests). */
  fetch?: typeof fetch;
}

/** Stream a COPC file over HTTP without downloading it whole. */
export function loadCOPC(url: string, options?: LoadCOPCOptions): Promise<CopcData>;

/** loaders.gl-compatible loader object. */
export const COPCLoader: {
  id: 'copc';
  module: 'copc';
  name: string;
  category: 'pointcloud';
  extensions: string[];
  mimeTypes: string[];
  text: false;
  test: typeof isCOPCFile;
  options: { copc: { bbox: BBox6 | null } };
  parse: typeof parse;
  parseSync: typeof parseSync;
};

export default typeof COPCLoader;
