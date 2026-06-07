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

export function memoryUsedBytes(mem) {
  return mem.used;
}
