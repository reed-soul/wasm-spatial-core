# wasm-spatial-core

> High-performance WebAssembly spatial data engine — runs entirely in the browser. Zero server, zero upload.

[![npm](https://img.shields.io/npm/v/wasm-spatial-core)](https://www.npmjs.com/package/wasm-spatial-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)

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

The killer feature: **drag a LAS/LAZ file into the browser, get a Cesium-ready 3D Tiles tileset** — no server needed.

```typescript
import { loadSpatialCore } from "wasm-spatial-core";

const core = await loadSpatialCore();

// 1. Parse point cloud (LAS or LAZ)
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
| `parseGeoJsonStream(input, size, cb)` | Streaming parser |
| `parseGeoJsonLazy(input)` | O(1 feature) memory iterator |
| `geoJsonFromCoords(coords, type)` | Generate GeoJSON |
| `filterGeoJsonByProperty(input, k, v)` | Filter features |
| `filterGeoJsonByBBox(input, ...)` | Spatial filter |

### Point Cloud (LAS/LAZ)

| Function | Description |
|----------|-------------|
| `parseLasHeader(bytes)` | Parse LAS header |
| `parseLasPoints(bytes)` | Parse all points |
| `parseLasPointsWithProgress(bytes, cb)` | Parse with progress |
| `parsePointCloudAuto(bytes)` | Auto-detect LAS/LAZ |
| `decimateVoxelGrid(pos, col, size)` | Voxel decimation |
| `decimateRandom(pos, col, count)` | Random sampling |
| `colorizeByHeight(pos, minZ, maxZ)` | Height-based coloring |
| `estimateNormals(pos, k)` | kNN normal estimation |

### Point Cloud Streaming (COPC)

| Function | Description |
|----------|-------------|
| `new PointCloudStreamer(url)` | Create streamer |
| `.parseHeader()` | Parse header |
| `.readPoints(offset, count)` | Read points by offset |
| `.readRegion(min, max)` | Spatial range read |
| `computeRegionByteRange(...)` | Compute byte range |
| `supportsLaz()` | Check LAZ support |
| `lazStatus()` | Runtime LAZ info |

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

## 🌐 Live Demo

[https://reed-soul.github.io/wasm-spatial-core/examples/index.html](https://reed-soul.github.io/wasm-spatial-core/examples/index.html)

## 📄 License

MIT © 2026 Zhiqi Weilai
