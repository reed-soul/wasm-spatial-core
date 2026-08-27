/**
 * copc-loader — COPC (Cloud Optimized Point Cloud) loader for loaders.gl
 *
 * Wraps the wasm-spatial-core engine's COPC/LAZ decoder as a loaders.gl
 * loader object, plus `loadCOPC()` for HTTP-range streaming off any static
 * file host that supports Range requests.
 *
 * @packageDocumentation
 */

// Static import is side-effect free (the wasm core initializes lazily);
// gives parseSync access to the raw bindings after any init path has run.
import * as wasmSpatialCore from 'wasm-spatial-core';

// ─── WASM core loading ──────────────────────────────────────────────

let corePromise = null;
let coreOverride = null;
let syncCore = null;

/**
 * Provide an already-initialized wasm-spatial-core API (e.g. one obtained
 * from `loadSpatialCore()` / `loadSpatialCoreNode()`) instead of letting
 * copc-loader initialize its own instance.
 *
 * @param {object|null} core
 */
export function setCore(core) {
  coreOverride = core;
  corePromise = null;
}

function detectNode() {
  return (
    typeof process !== 'undefined' &&
    process.versions != null &&
    process.versions.node != null &&
    // Bundlers may shim `process`; require the real versions object.
    typeof process.versions.node === 'string'
  );
}

async function getCore() {
  if (coreOverride) return coreOverride;
  if (!corePromise) {
    corePromise = detectNode()
      ? import('wasm-spatial-core/node').then((m) => m.loadSpatialCoreNode())
      : import('wasm-spatial-core').then((m) => m.loadSpatialCore());
    corePromise.then((core) => {
      syncCore = core;
    });
  }
  return corePromise;
}

/** @private */
export async function _getCore() {
  return getCore();
}

/** Initialize the WASM core (idempotent). */
export async function init() {
  return getCore();
}

// ─── Byte helpers ───────────────────────────────────────────────────

function readU32LE(bytes, offset) {
  return (bytes[offset] | (bytes[offset + 1] << 8) | (bytes[offset + 2] << 16) | (bytes[offset + 3] << 24)) >>> 0;
}

function readU64LE(bytes, offset) {
  let lo = readU32LE(bytes, offset);
  let hi = readU32LE(bytes, offset + 4);
  return hi * 0x100000000 + lo;
}

/**
 * Minimal LAS 1.4 header fields needed before the VLR region is fetched.
 * @private
 */
function readLasHeaderPrefix(bytes) {
  return {
    magic: String.fromCharCode(...bytes.slice(0, 4)),
    versionMajor: bytes[24],
    versionMinor: bytes[25],
    pointDataOffset: readU32LE(bytes, 96),
  };
}

/** loaders.gl `test` — LASF magic (LAS/LAZ/COPC all share it). */
export function isCOPCFile(data) {
  if (!(data instanceof Uint8Array) && !(data instanceof ArrayBuffer) && !(data instanceof DataView)) {
    return false;
  }
  const bytes = toBytes(data);
  return (
    bytes.length > 375 &&
    bytes[0] === 0x4c && // L
    bytes[1] === 0x41 && // A
    bytes[2] === 0x53 && // S
    bytes[3] === 0x46 //   F
  );
}

function toBytes(data) {
  if (data instanceof Uint8Array) return data;
  if (data instanceof ArrayBuffer) return new Uint8Array(data);
  if (data instanceof DataView) return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
  return data;
}

// ─── Output shaping ─────────────────────────────────────────────────

/**
 * Shape a wasm `LasPointCloud` into the loaders.gl point-cloud result
 * (deck.gl `PointCloudLayer` consumes `attributes` directly).
 */
function toLoaderData(cloud, copcInfo) {
  const bounds = copcInfo.bounds || null;
  const attributes = {
    POSITION: { value: cloud.positions, size: 3 },
  };
  if (cloud.colors && cloud.colors.length === cloud.pointCount * 3) {
    attributes.COLOR_0 = { value: cloud.colors, size: 3, normalized: false };
  }
  return {
    loaderData: { copcInfo },
    header: {
      vertexCount: cloud.pointCount,
      boundingBox: bounds,
    },
    schema: null,
    mode: 0, // GL_POINTS
    attributes,
  };
}

// ─── loaders.gl loader object ───────────────────────────────────────

/**
 * loaders.gl-compatible COPC loader.
 *
 * Works with `parse()` / `load()` from `@loaders.gl/core`:
 *
 * ```js
 * import { load } from '@loaders.gl/core';
 * import { COPCLoader } from 'copc-loader';
 * const data = await load(url, COPCLoader, { copc: { bbox: [x0,y0,z0,x1,y1,z1] } });
 * ```
 */
export const COPCLoader = {
  id: 'copc',
  module: 'copc',
  name: 'COPC (Cloud Optimized Point Cloud)',
  category: 'pointcloud',
  extensions: ['copc', 'laz'],
  mimeTypes: [],
  text: false,
  test: isCOPCFile,
  options: {
    copc: {
      /** Spatial subset [minX,minY,minZ,maxX,maxY,maxZ]; null = whole file. */
      bbox: null,
    },
  },
  parse,
  parseSync,
};

/**
 * Parse a full COPC/LAZ file already in memory.
 *
 * @param {ArrayBuffer|Uint8Array} data full file bytes
 * @param {object} [options]
 * @param {number[]|null} [options.copc.bbox] spatial subset
 * @returns loaders.gl point-cloud data (`attributes.POSITION`, `attributes.COLOR_0`)
 */
export async function parse(data, options) {
  const core = await getCore();
  return parseWithCore(core, data, options);
}

/** @private */
export function parseWithCore(core, data, options) {
  const bytes = toBytes(data);
  const bbox = options && options.copc && options.copc.bbox;
  const info = core.parseCopcHeader(bytes);

  let cloud;
  if (bbox) {
    cloud = core.readCopcRegion(bytes, bbox[0], bbox[1], bbox[2], bbox[3], bbox[4], bbox[5]);
  } else if (info.hasHierarchy) {
    // Full read: expand the header bounds by more than a float32 rounding
    // step so points exactly on the boundary aren't dropped (positions are
    // cast to f32 and compared against f64 bounds). At UTM-scale magnitudes
    // (~1e6) one f32 ulp is ~0.06 m.
    const b = info.bounds;
    const mag = Math.max(
      Math.abs(b[0]),
      Math.abs(b[1]),
      Math.abs(b[2]),
      Math.abs(b[3]),
      Math.abs(b[4]),
      Math.abs(b[5])
    );
    const eps = mag * 1e-6;
    cloud = core.readCopcRegion(
      bytes,
      b[0] - eps,
      b[1] - eps,
      b[2] - eps,
      b[3] + eps,
      b[4] + eps,
      b[5] + eps
    );
  } else {
    cloud = core.parseLazPoints(bytes);
  }
  return toLoaderData(cloud, info);
}

/**
 * Synchronous parse — requires the WASM core to be initialized first
 * (`await init()`, `setCore(...)`, or any prior `await parse(...)`).
 *
 * @param {ArrayBuffer|Uint8Array} data
 * @param {object} [options]
 */
export function parseSync(data, options) {
  // Use the instance the async init path loaded (web-target root import is
  // a *different* module with its own uninitialized wasm state in Node).
  const core = coreOverride || syncCore || wasmSpatialCore;
  return parseWithCore(core, data, options);
}

// ─── HTTP-range streaming ───────────────────────────────────────────

class RangeFetcher {
  constructor(url, fetchImpl) {
    this.url = url;
    this.fetch = fetchImpl || fetch;
    this.fileSize = 0;
    this.supportsRanges = true;
  }

  async range(start, end) {
    // end inclusive
    const res = await this.fetch(this.url, {
      headers: { Range: `bytes=${start}-${end}` },
    });
    if (!res.ok && res.status !== 206) {
      throw new Error(`copc-loader: fetch failed (${res.status}) for ${this.url}`);
    }
    if (res.status === 200) {
      this.supportsRanges = false;
      const buf = new Uint8Array(await res.arrayBuffer());
      this.fileSize = buf.length;
      return { bytes: buf.slice(start, end + 1), full: buf };
    }
    const cr = res.headers.get('content-range'); // bytes s-e/total
    if (cr) {
      const total = Number(cr.split('/')[1]);
      if (Number.isFinite(total)) this.fileSize = total;
    }
    return { bytes: new Uint8Array(await res.arrayBuffer()), full: null };
  }
}

/**
 * Stream a COPC file over HTTP without downloading it whole.
 *
 * Fetches header + VLRs + chunk table first (~KB), then fetches chunk byte
 * ranges and decompresses each with WASM as it arrives.
 *
 * @param {string} url
 * @param {object} [options]
 * @param {number[]|null} [options.bbox] post-decompression spatial filter
 * @param {function} [options.onProgress] `({chunksDone, chunksTotal, points}) => void`
 * @param {AbortSignal} [options.signal]
 * @param {function} [options.fetch] custom fetch (tests)
 * @returns loaders.gl point-cloud data
 */
export async function loadCOPC(url, options) {
  options = options || {};
  const fetcher = new RangeFetcher(url, options.fetch);

  // 1. Fixed header prefix → pointDataOffset
  const { bytes: prefix, full } = await fetcher.range(0, 374);
  const header = readLasHeaderPrefix(prefix);
  if (header.magic !== 'LASF') {
    throw new Error(`copc-loader: not a LAS/COPC file (magic ${header.magic})`);
  }
  const pdo = header.pointDataOffset;

  if (full && !fetcher.supportsRanges) {
    // Server ignored Range → we already have the whole file.
    return parse(full, options);
  }

  // 2. Header + VLRs + the u64 chunk-table-offset field (one range).
  const { bytes: headerVlrWithField } = await fetcher.range(0, pdo + 7);
  const headerVlr = headerVlrWithField.slice(0, pdo);
  const ctOffset = readU64LE(headerVlrWithField, pdo);
  if (!fetcher.fileSize) {
    throw new Error('copc-loader: server did not report file size (Content-Range)');
  }

  const core = await getCore();
  if (options.signal && options.signal.aborted) throw new Error('aborted');

  // 3. Chunk table (trailing EVLR bytes after it are ignored by the decoder).
  const { bytes: tableTail } = await fetcher.range(ctOffset, fetcher.fileSize - 1);

  // Synthesize: [header+VLRs][u64 → pdo+8][chunk table]. The engine frames
  // and decodes the table from this exact layout.
  const synth = new Uint8Array(pdo + 8 + tableTail.length);
  synth.set(headerVlr, 0);
  const dv = new DataView(synth.buffer);
  dv.setUint32(pdo, pdo + 8, true);
  dv.setUint32(pdo + 4, 0, true);
  synth.set(tableTail, pdo + 8);

  const info = core.parseCopcHeader(synth);
  const chunks = info.chunkTable || [];
  if (chunks.length === 0) {
    throw new Error('copc-loader: no chunk table found (streamed LAZ without chunk table is not streamable)');
  }

  // 4. Stream chunk ranges, decompress standalone.
  const dataStart = pdo + 8;
  const bbox = options.bbox || null;
  let positions = [];
  let colors = [];
  let hasColors = false;

  let chunksDone = 0;
  const CONCURRENCY = 6;
  let next = 0;
  async function worker() {
    while (next < chunks.length) {
      if (options.signal && options.signal.aborted) throw new Error('aborted');
      const i = next++;
      const c = chunks[i];
      const absStart = dataStart + c.offset;
      const { bytes: chunkBytes } = await fetcher.range(absStart, absStart + c.size - 1);
      const cloud = core.readCopcChunkStandalone(chunkBytes, c.count, headerVlr);
      appendCloud(cloud);
      chunksDone++;
      if (options.onProgress) {
        options.onProgress({ chunksDone, chunksTotal: chunks.length, points: positions.length / 3 });
      }
    }
  }
  function appendCloud(cloud) {
    const pos = cloud.positions;
    const col = cloud.colors;
    for (let i = 0; i < cloud.pointCount; i++) {
      const x = pos[i * 3], y = pos[i * 3 + 1], z = pos[i * 3 + 2];
      if (bbox) {
        if (x < bbox[0] || x > bbox[3] || y < bbox[1] || y > bbox[4] || z < bbox[2] || z > bbox[5]) continue;
      }
      positions.push(x, y, z);
      if (col) {
        hasColors = true;
        colors.push(col[i * 3], col[i * 3 + 1], col[i * 3 + 2]);
      }
    }
  }
  await Promise.all(Array.from({ length: Math.min(CONCURRENCY, chunks.length) }, worker));

  const pointCount = positions.length / 3;
  const attributes = {
    POSITION: { value: new Float32Array(positions), size: 3 },
  };
  if (hasColors && colors.length === pointCount * 3) {
    attributes.COLOR_0 = { value: new Uint8Array(colors), size: 3, normalized: false };
  }
  return {
    loaderData: { copcInfo: { ...info, fileSize: fetcher.fileSize } },
    header: { vertexCount: pointCount, boundingBox: info.bounds },
    schema: null,
    mode: 0,
    attributes,
  };
}

export default COPCLoader;
