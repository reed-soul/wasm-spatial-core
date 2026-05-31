# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0] - 2026-05-31

### Added
- **glTF/GLB Writer enhancements** — `meshToGlb()` one-shot API for generic indexed meshes with optional normals. Multiple material support per builder instance.
- **Terrain styling pipeline** — Color ramp application (`applyTerrainColorRamp()`), hillshade generation (`hillshade()`), and contour line extraction (`contourLines()`) for GeoTIFF elevation grids.
- **b3dm 3D Tiles encoder** — `encodeB3dmTile()` encodes glTF/GLB geometry into 3D Tiles Batched 3D Model format with batch table JSON support.
- **i3dm 3D Tiles encoder** — `encodeI3dmTile()` encodes instanced 3D Tiles with positions, orientations (quaternions), and per-instance scales. `createInstancedTileset()` / `createInstancedTilesetI3dm()` generate complete tileset trees.
- **Mesh tileset generator** — `createMeshTileset()` generates tileset.json trees from pre-encoded b3dm tile data with bounding volumes and geometric error.
- **Cesium geometry adapter** — `generateCesiumGeometry()` converts GeoJSON polygons to Cesium `MeshGeometry` with indexed triangles. `generate3DTile()` wraps geometry into `Cesium3DTile` with batch IDs.
- **Worker terrain pipeline** — `WorkerHandle` with `processTerrain()` for streaming GeoTIFF → quantized-mesh processing in Web Workers. Progress callbacks (`onProgress`, `onComplete`, `onError`), cancellation, and chunked processing support.
- **MVT GeoJSON projection** — `decodeMvtToGeoJson()` and `mvtToGeoJson()` now project tile-space coordinates back to WGS-84 geographic coordinates.
- **MVT layer info** — `mvtLayerInfo()` returns per-layer metadata (feature count, extent, name) from MVT tiles.
- **Point cloud classification coloring** — `colorizeByClassification()` applies ASPRS standard classification colors. `colorizeByHeatmap()` for density-based heat coloring.
- **Build color ramp** — `buildColorRamp()` creates gradient color ramps from key-value pairs for reusable colorization.
- **Point cloud statistics & bounds** — `pointCloudStats()` computes min/max/mean/stddev for XYZ + intensity. `pointCloudBounds()` returns axis-aligned bounding box.
- **CesiumJS complete demo** — Full-featured demo page with point cloud rendering, terrain visualization, and 3D Tiles display on Cesium globe.
- **npm publish readiness** — `npm/` package with TypeScript re-exports, typed bindings, and build scripts for all feature combinations.
- **IFC geometry parser** — Extract `IFCEXTRUDEDAREASOLID` mesh geometry from IFC-SPF text files.
- **Spatial edge index** — `SpatialEdgeIndex` for bounding box search and nearest-neighbor on line segment collections.

### Changed
- Test count: 520 → 529 (added boundary condition and edge case tests across gltf_writer, spatial_analysis)
- Source lines: ~30,029 lines (26 modules)
- WASM binary: 1.2 MB (point-cloud + geotiff), 1.5 MB (all features, single-thread)
- WASM build now uses `wasm-pack --target web` with `--` separator for cargo features
- d.ts generation: 3,343 lines (core), 3,470 lines (all features)
- Exported functions: 173 (core), 182 (all features)
- `multi-thread` feature documented as requiring `atomics` + `bulk-memory` RUSTFLAGS

### Security
- All WASM exports consistently use camelCase (JS) / PascalCase (structs)
- Error returns: `Result<T, JsValue>` for WASM boundary, `SpatialErrorDetail` for internal — both auto-convert to JS Error

## [0.5.0] - 2026-06-01

### Added
- **GeoTIFF terrain decoder** (`src/geotiff.rs`) — Hand-written TIFF/GeoTIFF parser with zero external TIFF dependencies. Supports:
  - Float32, Uint16, Uint8 elevation grids
  - Strip-organized and tile-organized layouts
  - Uncompressed and DEFLATE/ZLib compression (LZW marked as TODO)
  - GeoKey metadata parsing (GTModelType, GeographicType, etc.)
  - Geographic bounds, resolution, and CRS extraction
- **Quantized-mesh encoder** — Cesium terrain tile binary format. Encodes height matrices into quantized-mesh tiles with:
  - 88-byte header (center ECEF, min/max height, oct-normal, water mask)
  - Quantized vertex coordinates (uint16)
  - Triangle indices (uint16 or uint32 based on vertex count)
  - Edge indices (west, south, east, north borders)
- **Terrain tileset generator** — `encodeTerrainTileset()` generates tileset.json + quantized-mesh tile pyramid with LOD levels. Each level downsamples 2× automatically.
- **Terrain demo** (`examples/terrain-demo/index.html`) — Three.js-based GeoTIFF terrain viewer with:
  - Drag-and-drop file loading
  - Height gradient coloring (blue → green → yellow → red → white)
  - Interactive OrbitControls (rotate, zoom, pan)
  - Height scale slider and color mode selection
  - Built-in demo terrain generator (128×128 procedural terrain)
- **WASM exports**: `parseGeotiff()`, `parseGeotiffTile()`, `encodeQuantizedMesh()`, `encodeTerrainTileset()`, `supportsGeotiff()`, `geotiffStatus()`
- **New dependency**: `flate2` 1.1 for DEFLATE/ZLib decompression (pure Rust, WASM-compatible)
- **New feature flag**: `geotiff`

### Changed
- Test count: 460 → 455 (refactored GeoTIFF tests to use core functions for native targets)
- Total lines: ~24737 → ~27000

## [0.4.0] - 2026-05-31

### Added

- **Draco compression status** — `supportsDraco()` and `dracoStatus()` runtime checks. Draco encoding is not supported in WASM due to `draco-oxide`'s transitive dependency on `getrandom@0.3` which requires the `wasm_js` configuration flag (not expressible in Cargo.toml). Server-side or build-pipeline Draco encoding with client-side Google Draco WASM decoder is recommended as a workaround.
- **COPC HTTP Range query** — `copcQueryRanges(copcInfo, bbox)` returns JSON with HTTP `Range` headers needed to fetch COPC chunks. `copcEstimateDownloadSize(copcInfo)` estimates total bytes for a full download.
- **Grid-indexed point spacing** — `estimatePointSpacing()` optimized from O(n×sample) brute force to O(n + sample×k) using a spatial grid index with progressive ring expansion. Fallback to brute force for small or degenerate point sets.

### Changed

- Point spacing algorithm: grid-based spatial index replaces brute-force nearest-neighbor search. ~10x faster for large point clouds (100K+ points).
- Test count: 422 → 432 (Draco status tests, COPC Range tests, grid-indexed spacing tests).

## [0.3.1] - 2026-05-31

### Fixed

- **LAS header offset bug** — Point data was read from incorrect offset when `header.point_data_offset` differed from the default 227 bytes. Now correctly uses the offset value from the LAS header, fixing parsing failures on files with custom VLRs or non-standard header sizes.

### Changed

- Extracted duplicated `build_test_las_blob` helper into the shared `test_helpers` module, reducing code duplication across `point_cloud.rs` and `point_cloud_stream.rs`.

### Added

- `PERFORMANCE.md` — Benchmark data for octree build, tileset generation, LAZ decompression, and WASM binary sizes.
- `tests/pipeline_integration_test.rs` — End-to-end pipeline integration test using real `sample.las` fixture.
- README badges for npm version, CI status, license, WASM size, and test count.

## [0.3.0] - 2026-05-31

### Added

- **Point Cloud → 3D Tiles pipeline** — Full browser-side pipeline: LAS/LAZ/COPC → parse → octree build → pnts tile encoding → tileset.json generation. Zero server, zero upload.
- **Octree spatial partitioning** (`src/octree.rs`) — Recursive 8-way subdivision. Two-pass build (index permutation + reorder). Degenerate case handling (coincident points). WASM export: `buildOctree()` → `Octree` class.
- **pnts tile encoder** (`src/pnts.rs`) — Full 3D Tiles Point Cloud binary format. 28-byte header, feature table (JSON + binary with POSITION + optional RGB), batch table. WASM export: `encodePntsTile()`.
- **tileset.json generator** — Recursive tileset tree from octree hierarchy. Box boundingVolume, level-scaled geometricError, per-leaf tile content URIs. WASM export: `generateTileset()` → `TilesetResult` class.
- **View-dependent LOD** — `computeScreenSpaceError()` and `getVisibleTiles()` for screen-space error driven dynamic tile selection. Recursive octree traversal with configurable SSE threshold.
- **LAZ decompression** — `laz` crate (v0.12.1) integrated. `parseLazPoints()`, `parseLazPointsStream()`, `parsePointCloudAuto()` auto-detection. `supportsLaz()` and `lazStatus()` runtime checks.
- **COPC support** — Cloud Optimized Point Cloud header parsing, chunk table access, region-based byte range computation.
- **Point cloud statistics** — `octreeMemoryUsage()` for Rust-side octree memory estimation.
- **Point cloud coloring** — `colorizeByHeight()`, `colorizeByIntensity()`, `applyColorRamp()` for height-gradient and intensity-based RGBA coloring.
- **Point cloud normals** — `estimateNormals()` (kNN) and `flipNormals()` for consistent orientation toward centroid.
- **Three.js point cloud demo** — Zero-dependency 3D point cloud viewer (no tokens required).
- **Cesium 3D Tiles demo** — Point cloud rendered on Cesium globe via 3D Tiles.
- **PLY/OBJ parsing** — ASCII + binary PCD parsing, PLY ASCII + binary, OBJ vertex/normal extraction.
- **WKB/WKT support** — `parseWkb()`, `parseWkt()`, `toWkb()`, `toWkt()` for OGC Well-Known Binary/Text formats.
- **TopoJSON support** — `parseTopoJson()` for TopoJSON format parsing.
- **GPX support** — `parseGpx()` for GPS Exchange format parsing.
- **Convex/Concave hull** — `convexHull()` and `concaveHull()` for point set geometry.
- **Density/Grid clustering** — `clusterByDensity()` (DBSCAN-style) and `clusterByGrid()` for spatial point clustering.
- **CRS utilities** — `crsInfo()`, `getSupportedCrs()`, `bestCrsForRegion()`, `isInChina()` for CRS metadata and region detection.
- **Rhumb navigation** — `rhumbDistance()` and `rhumbBearing()` for constant-bearing calculations.
- **Vincenty distance** — `vincentyDistance()` for high-precision geodesic distance on the WGS-84 ellipsoid.
- **Error handling enhancement** — Structured `SpatialError` objects instead of plain strings across all APIs.
- **End-to-end pipeline tests** (`tests/point_cloud_pipeline.rs`) — 1000-point synthetic cloud → octree → pnts tiles → tileset.json validation (3 tests).
- **Sample data guide** (`examples/sample-data/README.md`) — Links to ASPRS, OpenTopography, Potree test data sources.
- **GitHub Pages demo site** — `scripts/build-demo-site.sh`, `vercel.json`, docs/DEMO_SITE.md.
- **npm package** — `npm/` wrapper with `npm/index.ts` TypeScript re-exports, `npm/package.json`, quick start README.

### Changed

- Module count: 17 → 25 (added `octree`, `pnts`, `ply`, `obj`, `e57`, `wkb_wkt`, `topojson`, `gpx`).
- Test count: 344 → 400.
- Source lines: ~20K → ~23K.
- WASM binary: ~1.2 MB (with point-cloud features including laz).
- Stop tracking `pkg/` in git (build via `wasm-pack` or CI artifacts).
- Declare `rust-version = "1.90"` in `Cargo.toml`.
- CI runs `wasm-pack test --node --release -- --test web` (wasm32 harness + version smoke test).

### Fixed

- GitHub Pages 部署路径：保留 `examples/` 前缀，修复 `../pkg` 与 `data/china_cities.json` 加载失败问题。
- WASM error paths now use structured `SpatialError` objects.
- CI and GitHub Pages now trigger on the `master` default branch (was `main`).
- Browser test `tests/web.rs` version assertion now tracks `CARGO_PKG_VERSION`.

## [0.2.0] - 2026-05-30

### Added

- **GeoJSON Write (Serialization)** (`src/geojson_parser.rs`) — `geoJsonFromCoords(coords, geometry_type)` generates a GeoJSON Feature from flat coordinate buffer. `geoJsonFeatureCollection(coords, types, properties_json)` generates complete FeatureCollections. Supports Point, LineString, Polygon, MultiPoint. Properties separated by unit separator (0x01). 7 tests.
- **GeoJSON Property Filtering** (`src/geojson_parser.rs`) — `filterGeoJsonByProperty(input, key, value)` filters features by property value. `filterGeoJsonByBBox(input, minLng, minLat, maxLng, maxLat)` filters features by bounding box. `countGeoJsonByProperty(input, key)` returns property value → count mapping (COUNT GROUP BY). 5 tests.
- **Coordinate Validation & Cleaning** (`src/utils.rs`) — `validateCoords(coords, crs)` validates against CRS-specific ranges (WGS84, GCJ02, BD09, Mercator). `cleanCoords(coords, strategy)` with remove/clamp/snap strategies. `deduplicateCoords(coords, tolerance)` removes near-duplicate points. 11 tests.
- **Coordinate Pipeline Transforms** (`src/coordinate.rs`) — `batchWgs84ToGcj02Mercator(coords)` and `batchWgs84ToBd09Mercator(coords)` — single-step pipeline transforms (WGS84→GCJ02→Mercator, WGS84→BD09→Mercator) for Chinese web map applications. In-place variants included. 4 tests.
- **Coordinate Normalization** (`src/coordinate.rs`) — `normalizeCoords(coords, bounds)` normalizes coordinates to [0,1] range. `denormalizeCoords(normals, bounds)` reverses the normalization. Auto-computes bounds if not provided. 3 tests.
- **Polygon Boolean Operations** (`src/topology.rs`) — `polygonIntersection(ring1, ring2)` and `polygonUnion(ring1, ring2)` using `geo::BooleanOps`. Returns empty array for non-intersecting polygons. 5 tests.
- **Spatial Relationship Predicates** (`src/topology.rs`) — `contains(outer_ring, point_x, point_y)` point-in-polygon via `geo::Contains`. `touches(ring1, ring2)` adjacency detection. `polygonIntersects(ring1, ring2)` intersection test. `disjoint(ring1, ring2)` disjoint test. All using `geo` crate algorithms with DE-9IM topology. 8 tests.
- **Point Cloud Colorization** (`src/point_cloud.rs`) — `colorizeByHeight(positions, min_z, max_z, low_color, high_color)` height-gradient RGBA coloring. `colorizeByIntensity(positions, intensities)` grayscale intensity mapping. `applyColorRamp(positions, colors)` discrete color application. All return Float32Array RGBA (0.0–1.0). 4 tests.
- **Coordinate Sorting & Gridding** (`src/coordinate.rs`) — `sortCoordsByLng(coords)` and `sortCoordsByLat(coords)` sort coordinate pairs. `gridIndex(coords, cell_size_deg)` assigns spatial hash grid IDs. 5 tests.
- **Dynamic Memory Management** (`src/lib.rs`) — `setInputSizeLimit(bytes)` dynamically adjusts the input size limit (default 100 MB). `getInputSizeLimit()` queries the current limit. `getAllocatedBytes()` reads WASM linear memory size. 4 tests.
- **End-to-End Stress Tests** (`tests/stress_test.rs`) — 6 large-scale stress tests (100K features, 10M points, 1K polygon pairs, 1M point dedup). All marked `#[ignore]` for CI; run locally with `cargo test -- --ignored`.
- **Lazy GeoJSON Parser** (`src/geojson_streaming.rs`) — `parseGeoJsonLazy(input)` returns a `LazyGeoJsonIter` with `nextFeature()`, `remaining()`, `total()`. Uses a manual JSON state machine to extract coordinates one feature at a time — O(single feature) memory peak instead of O(all features). Skips properties, only extracts coordinates. 11 tests.
- **Bounds Computation** (`src/spatial_index.rs`) — `computeBounds(coords)` computes `[minLng, minLat, maxLng, maxLat]` with SIMD-style 4-wide f64 comparison. `computeBoundsMulti(buffers)` merges bounds from multiple coordinate arrays. 6 tests.
- **MVT Decoder** (`src/vector_tile.rs`) — `decodeMvt(bytes)` decodes protobuf MVT tiles into structured `MvtLayer`/`MvtFeature` objects with geometry types, tile-space coordinates, tags, and feature IDs. `decodeMvtToGeoJson(bytes)` converts MVT to GeoJSON FeatureCollection. Includes ZigZag geometry command decoding. 5 tests.
- **Performance Benchmark** (`bench/comparison/`) — Node.js script comparing `wasm-spatial-core` vs `proj4js` for WGS84→GCJ02, WGS84→Mercator, and GeoJSON parsing at 10K/100K/1M point scales.
- **Topology Analysis** (`src/topology.rs`) — Polygon area (spherical excess formula), polyline/polygon length (Haversine), Douglas-Peucker simplification, point-in-ring (ray-casting), area with holes support, TIN interpolation, polygon boolean operations (intersection/union).
- **GeoJSON Feature Properties** — `parseGeoJsonProperties()` extracts all feature properties as JSON array. `parseGeoJsonFeatures()` returns structured per-feature result with coordinates, offsets, counts, and geometry types.
- **Geodesic Calculations** — `haversineDistance()` (public), `bearing()` (forward azimuth), `destination()` (direct geodesic problem), `midpoint()` (great-circle midpoint) in `spatial_analysis.rs`.
- **Geohash Encoding/Decoding** — `geohashEncode(lng, lat, precision)` and `geohashDecode(hash)` with neighbor computation.
- **prost 0.14** dependency (matching geozero/mvt versions for MVT protobuf support).

### Changed

- Version bumped to `0.2.0` (stable release).
- Input size limit is now dynamically adjustable via `setInputSizeLimit()` (was hardcoded 100 MB constant).
- `geo::Coordinate` updated to `geo::Coord` (geo 0.29 API change).
- 220 tests (up from 158 in v0.1.0). ~11.3K lines of source code.

## [0.1.0] - 2026-05-30

### Added

- **Coordinate Projection** — Batch conversion between WGS-84, GCJ-02, BD-09, Web Mercator (EPSG:3857), and CGCS2000. Zero-copy in-place variants for all transforms.
- **GeoJSON Parser** — Parse FeatureCollections into flat `Float64Array` coordinate buffers; feature counting for progress reporting.
- **GeoJSON Streaming Parser** — Chunked processing with JS progress callbacks for large files.
- **Spatial Index (R-Tree)** — Bounding box search, nearest-neighbor, and K-nearest-neighbor queries. Point index and edge (line segment) index.
- **Vector Tile Slicing** — Frontend MVT (PBF) tile generation from GeoJSON via `geojsonvt`, with configurable tile parameters.
- **Cesium Adapter** — WGS-84 → Cartesian3 (ECEF) batch conversion, polygon triangulation via earcut, 3D Tiles (b3dm) generation.
- **LAS Point Cloud** — Hand-written LAS header + point parser, COPC range-based access (header-only parse + point offset computation), voxel grid and random decimation, PCD ASCII/binary parsing.
- **PCD Point Cloud** — Parse ASCII and binary PCD files into coordinate arrays.
- **IFC/BIM Geometry** (experimental) — Extract `IFCEXTRUDEDAREASOLID` mesh geometry from IFC-SPF text.
- **glTF / GLB Writer** — Build glTF 2.0 scenes in WASM with materials and multiple meshes, export as GLB binary or JSON.
- **Spatial Analysis** — Point buffering, line buffering, axis-aligned bounding box, centroid computation on WGS-84.
- **GPU-Ready Output** — Interleaved vertex buffers and indexed geometry generation for WebGL2/WebGPU direct consumption.
- **Streaming API** — Chunked GeoJSON parsing with per-feature coordinate arrays.
- **Memory Management** — `memoryInfo()` API for WASM linear memory monitoring; 100 MB input size limit.
- **Multi-threading** (optional) — Web Workers + SharedArrayBuffer via Rayon (`multi-thread` feature flag).

### Performance

- SIMD-hinted inner loops for coordinate transform hot paths.
- `#[inline]` annotations on all public WASM entry points.
- Rayon-based parallel processing for multi-threaded WASM builds.
- LTO + single codegen unit in release profile for optimal codegen.

### Demos

- Interactive demo with coordinate projection, GeoJSON parsing, spatial index, and Cesium geometry.
- Benchmark comparison page (Pure JS vs WASM).
- Phase 2 pipeline demo (streaming + index + tile).
- Web Worker multi-threading demo.

## [0.1.0]: https://github.com/reed-soul/wasm-spatial-core/releases/tag/v0.1.0
