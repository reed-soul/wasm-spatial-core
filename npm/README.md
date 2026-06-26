# wasm-spatial-core

> A Web3D spatial compute engine for the browser — ingest, edit geometry, emit 3D Tiles / glTF. Zero server, zero upload.

[![npm](https://img.shields.io/npm/v/wasm-spatial-core)](https://www.npmjs.com/package/wasm-spatial-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)

## What you get from `npm install`

The published package is a **prebuilt WASM binary** (`point-cloud` + `geotiff`):

| Included | Requires custom build |
|----------|----------------------|
| LAS, PLY, OBJ, PCD | LAZ / COPC (`laz-support`) |
| Octree + 3D Tiles (pnts) | E57 (`e57-support`) |
| GeoTIFF → quantized-mesh | Terrain deformation (`terrain-edit`) |
| Coordinates, GeoJSON, MVT | Spatial IR + GLB ingest (`mesh-ingest`) |
| | Mesh QEM / clip / OBB split (`mesh-edit`, needs `mesh-ingest`) |
| | WebGPU compute kernels (`webgpu`) |

**Formats:** **10+** in the default npm build · **15+** with optional format features (LAZ/COPC, E57, GLB ingest).

Check at runtime: `supportsLaz()`, `supportsGeotiff()`, `lazStatus()`, `supportsWebGpu()`, `supportsMeshEdit()`.

Full matrix: [Feature flags in the repo README](../README.md#feature-flags).

## 🚀 Quick Start

```bash
npm install wasm-spatial-core
```

```typescript
import { loadSpatialCore } from "wasm-spatial-core";

const core = await loadSpatialCore();

// Coordinate conversion
const wgs84 = new Float64Array([116.404, 39.915, 121.474, 31.230]);
const gcj02 = core.batchWgs84ToGcj02(wgs84);

// GeoJSON parsing
const coords = core.parseGeoJsonCoords(geojsonStr);
```

## ☁️ Point Cloud → 3D Tiles

**Drag a LAS file into the browser, get a Cesium-ready 3D Tiles tileset** — no server needed.

> **LAZ / COPC:** not included in the default npm binary. Build with
> `--features laz-support` or check `supportsLaz()` before calling
> `parsePointCloudAuto` on compressed files.

```typescript
import { loadSpatialCore } from "wasm-spatial-core";

const core = await loadSpatialCore();

// 1. Parse point cloud (LAS — default npm build)
const lasBuffer = await fetch("scan.las").then(r => r.arrayBuffer());
const points = core.parseLasPoints(new Uint8Array(lasBuffer));

// 2. Decimate if needed (voxel grid → uniform density)
const decimated = core.decimateVoxelGrid(
  points.positions(),
  points.colors(),
  1.0  // 1-meter grid
);

// 3. Build spatial index (octree)
const octree = core.buildOctree(decimated.positions, 50000, 10);

// 4. Generate 3D Tiles tileset
const tileset = core.generateTileset(
  decimated.positions,
  50000,           // max points per tile
  10,              // max tree depth
  decimated.colors
);

// 5. Use with Cesium
console.log(tileset.tilesetJson());       // tileset.json
console.log(tileset.tileCount());        // number of .pnts tiles
const tile0 = tileset.tile(0);           // Uint8Array of first tile
const bounds0 = tileset.tileBounds(0);  // Float64Array [minX..maxZ]

// 6. LOD: get visible tiles for current camera
const fov = Math.PI / 3; // 60° vertical FOV
const visible = core.getVisibleTiles(
  decimated.positions,
  camera.x, camera.y, camera.z,
  fov, 1920, 1080
);
// → Uint32Array of node indices to load
```

## 📋 API Reference

### Coordinate Projection

| Function | Description |
|----------|-------------|
| `batchWgs84ToGcj02(coords)` | WGS-84 → GCJ-02 |
| `batchGcj02ToWgs84(coords)` | GCJ-02 → WGS-84 |
| `batchWgs84ToBd09(coords)` | WGS-84 → BD-09 |
| `batchBd09ToWgs84(coords)` | BD-09 → WGS-84 |
| `batchWgs84ToMercator(coords)` | WGS-84 → EPSG:3857 |
| `batchMercatorToWgs84(coords)` | EPSG:3857 → WGS-84 |
| `batchWgs84ToGcj02Mercator(coords)` | WGS-84 → GCJ-02 → Mercator |
| `*InPlace` variants | Zero-copy in-place for all above |
| `wgs84ToUtm(lng, lat)` | WGS-84 → UTM |
| `utmToWgs84(zone, e, n, isN)` | UTM → WGS-84 |

### GeoJSON

| Function | Description |
|----------|-------------|
| `parseGeoJsonCoords(input)` | Extract coordinates → Float64Array |
| `countGeoJsonFeatures(input)` | Count features |
| `parseGeoJsonStream(input, size, cb)` | Chunked parser (full JSON parse, batched coord output) |
| `parseGeoJsonLazy(input)` | One-feature-at-a-time iterator (input string required) |
| `geoJsonFromCoords(coords, type)` | Generate GeoJSON |
| `filterGeoJsonByProperty(input, k, v)` | Filter features |
| `filterGeoJsonByBBox(input, ...)` | Spatial filter |

### Point Cloud (LAS — default npm)

| Function | Description |
|----------|-------------|
| `parseLasHeader(bytes)` | Parse LAS header |
| `parseLasPoints(bytes)` | Parse all points |
| `parseLasPointsWithProgress(bytes, cb)` | Parse with progress |
| `parsePointCloudAuto(bytes)` | Auto-detect format (LAZ/COPC only with `laz-support` build) |
| `decimateVoxelGrid(pos, col, size)` | Voxel decimation |
| `decimateRandom(pos, col, count)` | Random sampling |
| `colorizeByHeight(pos, minZ, maxZ)` | Height-based coloring |
| `estimateNormals(pos, k)` | kNN normal estimation |

### Point Cloud — LAZ / COPC (custom build only)

| Function | Description |
|----------|-------------|
| `new PointCloudStreamer(url)` | Create streamer |
| `.parseHeader()` | Parse header |
| `.readPoints(offset, count)` | Read points by offset |
| `.readRegion(min, max)` | Spatial range read |
| `computeRegionByteRange(...)` | Compute byte range |
| `supportsLaz()` | `false` in default npm; `true` with `laz-support` |
| `lazStatus()` | Runtime LAZ capability string |

### Octree

| Function | Description |
|----------|-------------|
| `buildOctree(positions, maxPts?, maxDepth?)` | Build spatial octree |
| `Octree` | Octree class |
| `.nodeCount()` / `.depth()` / `.totalPoints()` | Tree stats |
| `.rootBounds()` / `.nodeBounds(i)` | Bounding boxes |
| `.leafCount()` | Number of leaf nodes |
| `octreeMemoryUsage(n, internal, pts)` | Memory estimate |

### 3D Tiles (pnts)

| Function | Description |
|----------|-------------|
| `encodePntsTile(pos, cx, cy, cz, colors?)` | Encode pnts binary |
| `generateTileset(pos, maxPts?, maxDepth?, colors?)` | Full tileset |
| `TilesetResult` | Tileset class |
| `.tilesetJson()` / `.tileCount()` / `.tile(i)` | Access tiles |
| `.tileBounds(i)` / `.tileUri(i)` | Tile metadata |

### LOD

| Function | Description |
|----------|-------------|
| `computeScreenSpaceError(geoErr, dist, fov, h)` | SSE in pixels |
| `getVisibleTiles(pos, cam, fov, w, h, ...)` | Visible tile indices |

### Spatial Analysis

| Function | Description |
|----------|-------------|
| `haversineDistance(lng1, lat1, lng2, lat2)` | Great-circle distance |
| `bearing()` / `destination()` / `midpoint()` | Geodesic |
| `bufferPoint()` / `bufferLineString()` | Buffer geometry |
| `polygonArea()` / `polylineLength()` | Measurements |
| `simplifyDouglasPeucker(coords, tol)` | Line simplification |
| `polygonIntersection()` / `polygonUnion()` | Boolean ops |

### Cesium Integration

| Function | Description |
|----------|-------------|
| `batchWgs84ToCartesian3(coords)` | WGS84 → ECEF |
| `generateCesiumGeometry(geojson, h)` | Triangulate → mesh |
| `generate3DTile(geojson, h)` | Build b3dm tile |

### Memory

| Function | Description |
|----------|-------------|
| `memoryInfo()` | WASM memory → MemoryInfo |
| `getAllocatedBytes()` | Peak allocation |
| `setInputSizeLimit(bytes)` | Set max input size |

## 🖥️ Node.js Batch Processing

Server-side pipelines via the `nodejs` WASM target — no browser or COOP/COEP headers required.

```typescript
import { loadSpatialCoreNode, batchPointCloudToTileset } from "wasm-spatial-core/node";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";

const core = await loadSpatialCoreNode();
const las = readFileSync("scan.las");
const result = await batchPointCloudToTileset(core, las);

mkdirSync("output/tiles", { recursive: true });
writeFileSync("output/tileset.json", result.tilesetJson);
result.tiles.forEach((data, i) => {
  writeFileSync(join("output/tiles", result.tileUris[i]), data);
});
```

Build the Node.js WASM package: `npm run build:wasm:node` (outputs to `npm/pkg-node/`).

## 🌐 Live Demo

[https://reed-soul.github.io/wasm-spatial-core/examples/index.html](https://reed-soul.github.io/wasm-spatial-core/examples/index.html)

## 📄 License

MIT © 2026 Zhiqi Weilai
