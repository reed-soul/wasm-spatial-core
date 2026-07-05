/**
 * Helpers for wasm-bindgen point-cloud types (getter-based API as of v0.7+).
 * Copies WASM-backed buffers into owned typed arrays before the source is freed.
 */

export function readLasHeader(header) {
  return {
    numPoints: header.numPoints,
    versionMajor: header.versionMajor,
    versionMinor: header.versionMinor,
    pointFormatId: header.pointFormatId,
    pointDataRecordLength: header.pointDataRecordLength,
    boundsMinX: header.boundsMinX,
    boundsMinY: header.boundsMinY,
    boundsMinZ: header.boundsMinZ,
    boundsMaxX: header.boundsMaxX,
    boundsMaxY: header.boundsMaxY,
    boundsMaxZ: header.boundsMaxZ,
  };
}

export function copyPointCloud(cloud) {
  const positions = new Float32Array(cloud.positions);
  const rawColors = cloud.colors;
  const colors = rawColors ? new Uint8Array(rawColors) : null;
  return {
    positions,
    positionsArray: Array.from(positions),
    colors,
    colorsArray: colors ? Array.from(colors) : null,
  };
}

export function copyPlyResult(ply) {
  const { positions, positionsArray, colors, colorsArray } = copyPointCloud(ply);
  return {
    vertexCount: ply.vertexCount,
    hasColors: ply.hasColors(),
    positions,
    positionsArray,
    colors,
    colorsArray,
  };
}

export function readTilesetResult(tileset) {
  return {
    tileCount: tileset.tileCount,
    totalBytes: tileset.totalBytes,
    tilesetJson: tileset.tilesetJson(),
  };
}

/** Rewrite terrain_*.cmpt (or other) URIs in tileset JSON to blob URLs. */
export function bindTerrainTilesetUrls(tilesetResult) {
  const { tileCount, tilesetJson } = readTilesetResult(tilesetResult);
  let json = tilesetJson;
  for (let i = 0; i < tileCount; i++) {
    const uri = tilesetResult.tileUri(i);
    const tileData = tilesetResult.tile(i);
    const blob = new Blob([tileData], { type: 'application/octet-stream' });
    const url = URL.createObjectURL(blob);
    json = json.replaceAll(`"${uri}"`, `"${url}"`);
  }
  return json;
}

export function centerSquarePolygon(bounds, fraction = 0.5) {
  const [west, south, east, north] = bounds;
  const halfW = ((east - west) * fraction) / 2;
  const halfH = ((north - south) * fraction) / 2;
  const cx = (west + east) / 2;
  const cy = (south + north) / 2;
  return [
    cx - halfW, cy - halfH,
    cx + halfW, cy - halfH,
    cx + halfW, cy + halfH,
    cx - halfW, cy + halfH,
  ];
}

export function memoryUsedBytes(mem) {
  return mem.used;
}

/**
 * Try to parse a COPC header. Returns null when the file is plain LAS/LAZ.
 */
export function tryParseCopcHeader(wasm, bytes) {
  if (!wasm?.parseCopcHeader) return null;
  try {
    const info = wasm.parseCopcHeader(bytes);
    const chunkTable = info.chunkTable;
    if (!chunkTable || chunkTable.length === 0) return null;
    return info;
  } catch {
    return null;
  }
}

/**
 * Stream COPC chunks into flat position/color buffers (no octree build).
 */
export async function streamCopcPositions(wasm, bytes, copcInfo, options = {}) {
  const onProgress = options.onProgress;
  const chunks = copcInfo.chunkTable;
  const headerBytes = bytes.subarray(0, Math.min(375, bytes.length));
  const totalChunks = chunks.length;

  const posParts = [];
  const colorParts = [];
  let hasColor = false;

  for (let i = 0; i < totalChunks; i++) {
    const entry = chunks[i];
    const chunkCloud = wasm.readCopcChunk(
      bytes,
      entry.offset,
      entry.size,
      entry.count,
      headerBytes,
    );
    const positions = chunkCloud.positions;
    posParts.push(new Float32Array(positions));
    const colors = chunkCloud.colors;
    if (colors?.length) {
      hasColor = true;
      colorParts.push(new Uint8Array(colors));
    }
    if (typeof chunkCloud.free === 'function') chunkCloud.free();
    onProgress?.(i + 1, totalChunks);
    if (i % 4 === 3) await new Promise((r) => setTimeout(r, 0));
  }

  const pointCount = posParts.reduce((n, p) => n + p.length, 0) / 3;
  const positions = new Float32Array(pointCount * 3);
  let offset = 0;
  for (const part of posParts) {
    positions.set(part, offset);
    offset += part.length;
  }

  let colorsOut = null;
  if (hasColor) {
    colorsOut = new Uint8Array(pointCount * 3);
    offset = 0;
    for (const part of colorParts) {
      colorsOut.set(part, offset);
      offset += part.length;
    }
  }

  return { positions, colors: colorsOut, pointCount, hasColor };
}

/**
 * Stream COPC chunks into OctreeChunkBuilder, then finish with reordered buffers.
 *
 * @param {object} wasm — initialized wasm module
 * @param {Uint8Array} bytes — full COPC file
 * @param {object} copcInfo — result of parseCopcHeader
 * @param {object} [options]
 * @param {number} [options.maxPointsPerNode=50000]
 * @param {number} [options.maxDepth=21]
 * @param {(done: number, total: number) => void} [options.onProgress]
 * @returns {{ octree, positions: Float32Array, colors: Uint8Array|null, pointCount: number, hasColor: boolean }}
 */
export async function streamCopcToOctree(wasm, bytes, copcInfo, options = {}) {
  const maxPointsPerNode = options.maxPointsPerNode ?? 50_000;
  const maxDepth = options.maxDepth ?? 21;
  const onProgress = options.onProgress;
  const pointCount = Math.min(Number(copcInfo.pointCount) || 0, 0xffffffff);
  const chunks = copcInfo.chunkTable;
  const headerBytes = bytes.subarray(0, Math.min(375, bytes.length));
  const builder = wasm.OctreeChunkBuilder.withCapacity(maxPointsPerNode, maxDepth, pointCount);

  let hasColor = false;
  const totalChunks = chunks.length;

  for (let i = 0; i < totalChunks; i++) {
    const entry = chunks[i];
    const offset = entry.offset;
    const size = entry.size;
    const count = entry.count;
    const chunkCloud = wasm.readCopcChunk(bytes, offset, size, count, headerBytes);
    const positions = chunkCloud.positions;
    const colors = chunkCloud.colors;
    if (colors?.length) {
      hasColor = true;
      builder.pushChunkWithColors(positions, colors);
    } else {
      builder.pushChunk(positions);
    }
    if (typeof chunkCloud.free === 'function') chunkCloud.free();
    onProgress?.(i + 1, totalChunks);
    if (i % 4 === 3) await new Promise((r) => setTimeout(r, 0));
  }

  const n = builder.pointCount;
  const positions = new Float32Array(n * 3);
  let octree;
  let colorsOut = null;

  if (hasColor) {
    const colors = new Uint8Array(n * 3);
    octree = builder.finishWithColors(positions, colors);
    colorsOut = colors;
  } else {
    octree = builder.finish(positions);
  }

  return {
    octree,
    positions,
    colors: colorsOut,
    pointCount: n,
    hasColor,
  };
}

/**
 * COPC → Spatial IR PointCloudChunk (W2 ingest path).
 * Returns null when mesh-ingest / pointCloudChunkFromBuffers is unavailable.
 */
export async function streamCopcToPointCloudChunk(wasm, bytes, copcInfo, options = {}) {
  if (!wasm?.pointCloudChunkFromBuffers) return null;
  const { positions, colors, pointCount, hasColor } = await streamCopcPositions(
    wasm,
    bytes,
    copcInfo,
    options,
  );
  const chunk = wasm.pointCloudChunkFromBuffers(
    positions,
    colors,
    options.sourceFormat ?? 'copc',
  );
  return { chunk, positions, colors, pointCount, hasColor };
}
