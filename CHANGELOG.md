# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `scripts/build-demo-site.sh` — static site for GitHub Pages / Vercel (`examples/` + `pkg/` layout).
- [docs/DEMO_SITE.md](./docs/DEMO_SITE.md) — 在线演示部署说明（GitHub Pages + Vercel）。
- `vercel.json` — optional Vercel static deploy.

### Fixed

- GitHub Pages 部署路径：保留 `examples/` 前缀，修复 `../pkg` 与 `data/china_cities.json` 加载失败问题。

### Added

- Root `package.json` dev scripts: `npm run build:pkg`, `npm run demo`, `npm run check`.
- `examples/README.md` with demo URLs and build instructions.

### Added (prior)

- `npm/package-lock.json` for reproducible `npm/` wrapper installs.

### Fixed

- WASM error paths now use structured `SpatialError` objects (`parse_js`, `tile_js`, …) instead of plain strings across GeoJSON, MVT, WKT/WKB, streaming, and validation APIs.

- CI and GitHub Pages now trigger on the `master` default branch (was `main`).
- `cargo fmt` drift in `spatial_analysis.rs`.
- Browser test `tests/web.rs` version assertion now tracks `CARGO_PKG_VERSION`.
- Documentation version strings updated to `0.2.0`; README feature list deduplicated.

### Changed

- Stop tracking `pkg/` in git (build via `wasm-pack` or CI artifacts).
- Declare `rust-version = "1.90"` in `Cargo.toml`.
- CI runs `wasm-pack test --node --release -- --test web` (wasm32 harness + version smoke test).
- Removed unused `transform_slice_in_place_simd` dead code.
- `SECURITY.md` updated for 0.2.x support and accurate `unsafe` policy.

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
