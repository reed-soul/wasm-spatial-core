# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.10.2] - 2026-08-28

### Fixed

- **`/node` subpath broken for ESM consumers** — the Node entry re-exported
  named bindings from the nodejs-target CommonJS glue via
  `export { x } from './pkg-node/...'`, which depends on cjs-module-lexer
  static analysis; the glue (top-level side-effect wasm instantiation,
  thousands of scattered `exports.x =` assignments) defeats it and every
  ESM import of `wasm-spatial-core/node` failed with
  "does not provide an export named 'version'". The entry now loads the
  glue via `createRequire` and re-exports real ESM bindings. CommonJS
  consumers were unaffected.

## [0.10.1] - 2026-08-28

Follow-up to 0.10.0's real-file COPC support, aimed at streaming consumers
(the upcoming `copc-loader` npm package).

### Added

- **`readCopcChunkStandalone(chunkBytes, expectedPoints, headerBytes)`** —
  decompress a single COPC/LAZ chunk from just its own compressed bytes:
  no full file, no chunk table, no seek. The chunk is framed internally as a
  synthetic single-chunk LAZ stream. This is the primitive HTTP-range
  streaming needs — fetch a byte range, decode it, discard it.
- `readCopcRegion`'s hierarchy path now uses the standalone decompressor
  (absolute hierarchy offsets slice directly; no per-chunk chunk-table
  decode or offset conversion).
- Native `positions_native()` accessor on `LasPointCloud` for non-wasm
  tests/tools.
- npm wrapper re-exports `copcQueryRanges` / `copcEstimateDownloadSize`.

## [0.10.0] - 2026-08-16

A correctness and packaging release. It ships the OCR-audit fixes — three
critical parser bugs, a COPC hierarchy denial-of-service, several
panic-on-input paths — and a southern-hemisphere UTM fix that requires a
BREAKING API change (batch UTM APIs now carry the hemisphere explicitly).
It is also the first release whose npm package has working subpath exports
(`/node`, `/abort`, `/webgpu`, `/draco`). If you are upgrading from 0.9,
read [docs/MIGRATION_V0_10.md](docs/MIGRATION_V0_10.md) — southern-hemisphere
UTM values computed by ≤ 0.9 are wrong by ~10,000 km and must be recomputed.

### Fixed — Critical — real COPC/LAZ file support

Verified end-to-end against a real 81 MB COPC file (autzen-classified,
10.65 M points, 278 chunks): header, chunk table, hierarchy spatial query,
per-chunk random access and bbox region reads now decode all points exactly.

- **LAS bounds read in the wrong order/layout** (`point_cloud.rs`,
  `point_cloud_stream.rs`) — the ASPRS public header stores bounds
  *interleaved* from offset 179 (MaxX, MinX, MaxY, MinY, MaxZ, MinZ); both
  parsers read them sequentially (and the COPC path even from offset 182),
  producing garbage bounds on every real file. Synthetic round-trip tests
  masked the bug because the fixture builder wrote the same wrong layout.
- **Chunk table was a stub** (`point_cloud_stream.rs`) — the previous reader
  could not decode the arithmetic-coded LASzip chunk table and fabricated a
  single pseudo-chunk covering the whole file (with `count = 0xFFFFFFFF` for
  COPC's variable-size chunks). Now decodes the real table via
  `laz::laszip::ChunkTable::read_from` (278 exact entries for the sample).
- **Per-chunk decompression re-created a decompressor over a chunk slice**
  — `LasZipDecompressor` reads the chunk table itself at construction, so
  this always failed on real files ("failed to fill whole buffer"). Now a
  single decompressor over the full file + `seek()` to the chunk's global
  point index.
- **Variable-size chunk `seek()` dropped leading points** — laz 0.12's
  `seek()` computes the in-chunk delta as `point_idx % chunk_point_count`,
  valid only for fixed-size chunks; COPC's variable chunks lost up to
  `count-1` leading points per chunk. Worked around by seeking to the first
  multiple of the chunk's point count inside the chunk (forces delta = 0).
- **COPC hierarchy offsets passed unconverted** — hierarchy EVLR entries
  carry absolute file offsets; `read_copc_region` fed them to the chunk
  reader, which expects offsets relative to `point_data_offset + 8`. All
  hierarchy-path chunk reads silently produced nothing.
- **Per-chunk header slice cut at 375 bytes** — the LASZIP VLR lives after
  the fixed header (offset 1736 in the sample); the per-chunk decompressor
  could never find it, so hierarchy reads silently skipped every chunk.
- **RGB extraction wrong on every format** — channels are u16 little-endian
  (8-bit value = high byte) at byte 20 for formats 2/3 and byte 28 for
  formats 5/7/8/10 (GPSTIME8 before RGB); the old code read three raw bytes
  at 20..23, mixing channel halves and missing the GPS-time offset.
- **`parseCopcHeader` JSON lacked `fileSize`** — `copcQueryRanges` requires
  it, so the documented JSON round-trip always failed with
  "Missing fileSize".

### Fixed — Critical

- **COPC `VecDeque` import broke default build** (`copc_hierarchy.rs`) — the
  `use std::collections::VecDeque;` import was gated behind
  `#[cfg(feature = "laz-support")]`, but `query_copc_chunks_for_bbox` (always
  compiled) uses `VecDeque`. Building without `laz-support` failed with
  `cannot find type VecDeque in this scope`. Removed the feature gate.
- **b3dm index truncation** (`cesium_adapter.rs`) — `to_bytes` cast u32 indices
  to u16 (`v as u16`), silently wrapping any index > 65535 and corrupting
  meshes with more than 65535 vertices. Now emits u32 indices
  (componentType 5125 / UNSIGNED_INT).
- **geohash decode wrong for repeated characters** (`coordinate.rs`) —
  `geohash_decode` used `hash.chars().position(|x| x == c)`, which returns the
  FIRST occurrence of `c`. Any geohash with a repeated character (e.g.
  "ww4g0", "wx4g00") decoded the wrong bits as lng/lat. Now delegates to the
  correct bit-position tracker `geohash_decode_core`.

### Fixed — High

- **UTM southern-hemisphere always decoded as north** (`coordinate.rs`) — the
  `northing >= 0.0` hemisphere heuristic is always true for valid UTM
  (southern northings have 10,000,000 added), so every southern-hemisphere
  point was decoded with ~10,000,000 m error. **BREAKING**: batch UTM APIs now
  carry the hemisphere explicitly — `batchWgs84ToUtm` outputs 4 values/point
  `[zone, easting, northing, isNorth, ...]`, and `batchUtmToWgs84` /
  `*InPlace` require 4 values/point input. See "Changed" below.
- **COPC BFS denial-of-service** (`copc_hierarchy.rs`) — the hierarchy walk had
  no visited-set guard; a malformed/adversarial COPC with self-referencing or
  cycle-forming pages could loop forever or grow the queue unbounded. Added a
  `HashSet<u64>` of visited page offsets.
- **Several panic-on-input bugs** that abort the entire WASM instance instead
  of returning a typed error: `batch_wgs84_to_cartesian3` on odd-length input
  (`cesium_adapter.rs`); GeoJSON coordinate arrays with < 2 values
  (`cesium_adapter.rs`); `normalize_coords_native` `copy_from_slice` when the
  caller's bounds array length ≠ 4 (`coordinate.rs`); `denormalize_coords_native`
  `bounds[2]`/`bounds[3]` on short input (`coordinate.rs`); pub `*_core` ENU
  helpers that asserted on misaligned input (`enu_frame.rs`). All now return
  `Result<_, SpatialError/JsValue>`.
- **ENU altitude NaN at the poles** (`enu_frame.rs`) — `p / lat.cos()` divides
  by zero when a point lies on the rotation axis (p == 0), yielding NaN
  altitude. Falls back to the z-based formula near the poles.
- **E57 intensity array desync** (`e57.rs`) — when a point in an
  intensity-bearing cloud had `intensity: None`, nothing was pushed, so
  `intensities[i]` no longer lined up with point `i`. Pushes a 0.0 sentinel.
- **b3dm Feature Table wrong types** (`cesium_adapter.rs`) — POSITIONS declared
  as "SCALAR" (should be "VEC3"); binary sections padded with 0x20 instead of
  0x00 per the 3D Tiles spec.
- **b3dm batch table size mismatch** (`cesium_adapter.rs`) — the batch table
  always had a single `"id": ["0"]` entry regardless of BATCH_LENGTH, which
  makes Cesium fail to load tiles with > 1 feature. Now sized to the real
  feature count (parsed from GeoJSON, not a fragile string-scan estimate).
- **JSON injection / malformed-JSON hazards** (`chunk_export.rs`,
  `coordinate.rs`) — caller-controlled `tile_uri` and `code` were interpolated
  raw into JSON strings; NaN/Infinity bounds produced non-JSON tokens. Now
  escaped via `json_escape` / `serde_json` and guarded by `json_num`.

### Changed — BREAKING

- **Batch UTM APIs now carry hemisphere explicitly.**
  - `batchWgs84ToUtm`: output layout changed from 3 to 4 values/point
    `[zone, easting, northing, isNorth, ...]` (`isNorth` = 1.0 north / 0.0 south).
  - `batchUtmToWgs84`: input layout changed from 3 to 4 values/point
    `[zone, easting, northing, isNorth, ...]`.
  - `batchWgs84ToUtmInPlace` / `batchUtmToWgs84InPlace`: signature changed
    `&Float64Array` → `&mut [f64]` (true zero-copy, consistent with the other
    `InPlace` APIs and the file's documented design); 4 values/point layout.
- **ENU `EnuFrame::from_anchor` and `batch_wgs84_to_enu_core` /
  `batch_enu_to_wgs84_core` now return `Result<_, SpatialErrorDetail>`**
  (previously panicked/infallible). `from_anchor` validates finite coordinates
  and latitude ∈ [-90, 90].
- **`error_to_js` now throws a real `js_sys::Error`** (not a plain `Object`),
  so JS `e instanceof Error`, `e.stack`, and `console.error` formatting work.
  The thrown value still carries `code` / `name = "SpatialError"` properties.
- **`batch_wgs84_to_cartesian3` now returns `Result<Float64Array, JsValue>`**
  instead of panicking on odd-length input.

### Fixed — Medium / Low

- COPC `offset`/`size` `as usize` truncation on 32-bit targets →
  `usize::try_from` (`copc_hierarchy.rs`).
- COPC invalid negative `byte_size` with points → error instead of `.max(0)`.
- E57 coordinates `x as f32` overflow → clamped to f32 range; invalid
  Cartesian points now skipped instead of injecting (0,0,0) placeholders.
- E57 `point_count = positions.len() as u32 / 3` truncation → divide-then-cast.
- `batch_wgs84_to_enu_f32_core` now clamps to f32 range and maps non-finite → 0.0.
- `geohash_neighbors_core` no longer calls `geohash_decode_core` twice.
- `rgb_colors_for_pnts` RGBA→RGB path pre-allocates the exact capacity.
- Added `SpatialError::input_too_large` convenience constructor (parity with
  the other variants); tests now cover the `Cancelled` variant.

### Fixed — npm packaging

- **The npm tarball shipped without any subpath exports.** 0.8.0–0.9.0 were
  published using wasm-pack's generated `package.json`, which restricts the
  tarball to its own 6 files — so `wasm-spatial-core/abort`, `/node`,
  `/webgpu` and `/draco` failed to resolve for registry consumers. The
  curated `npm/package.json` is now the publish manifest: CI stages the
  prebuilt web+nodejs WASM into `npm/pkg{,-node}`, compiles the TypeScript
  entries to JS, and type-checks them (the previous `tsc --noEmit || true`
  in CI silently swallowed failures). The published package now contains
  the compiled entries, `.d.ts`, and both WASM targets (23 files).
- `npm/index.ts` re-exported `getInputSizeLimit` twice (hard `tsc` error) and
  imported the WASM bindings via the never-shipped flat layout — rewritten to
  the published `./pkg/` layout.
- `npm/batch.ts` called `TerrainTilesetResult` fields as methods
  (`tilesetJson()`, `.tileCount`) — now reads the real `tilesetJson` /
  `tile_count` properties and guards optional `tileUri` results.
- `npm/webgpu.ts` type-checks against `@webgpu/types` (eight `writeBuffer`
  type-variance errors fixed).
- Stale 0.6.0-era `npm/pkg` build outputs (~1.4 MB of binaries, tracked
  since May) removed from git; `.gitignore` now covers `npm/pkg*` and
  generated `npm/*.js`.

### Added — Release infrastructure

- `scripts/check-version-sync.sh` — fails CI when any user-facing version
  string drifts from `Cargo.toml`, and blocks re-committing `npm/pkg` build
  outputs. Wired into the `rust` CI job.
- `RELEASE.md` — the release checklist (pre-flight, version-bump table,
  tag/publish flow, post-release verification), previously recorded only in
  the v0.9.0 commit message.
- `docs/MIGRATION_V0_10.md` — JS before/after for every BREAKING change in
  this release, linked from the README.
- `ROADMAP_1_0.md` — the 1.0 acceptance criteria and the v0.10 → v0.11 → 1.0
  release path.
- The `rust` CI job now uploads the nodejs-target build; `publish-npm`
  consumes it so `wasm-spatial-core/node` resolves in the published package.

### Fixed — Security / CSP

- **Removed the last `js_sys::eval` calls** (`octree.rs`, `worker.rs`) — the
  generated bindings no longer import `eval`, so the engine loads under
  strict Content-Security-Policy without `unsafe-eval`. `supportsMultiThread()`
  now reports the real `SharedArrayBuffer` availability instead of always
  returning `true` in multi-thread builds; `supportsWorker()` probes `Worker`
  via `Reflect.has` on the global scope.

### Fixed — CI

- **Repaired the browser-test job, red on master since 2026-07-10** — the W4
  WebGPU bench page imports `examples/webgpu-smoke/app.mjs`, which the site
  restructure (e752484) deleted; the module 404'd, the page's
  `__benchResult` stayed `undefined`, and the self-skip check crashed. The
  helper is restored (its `examples/shared/` dependencies survived).

## [0.9.0] - 2026-07-09

A correctness-first release. The headline change fixes a LAS header
parsing bug that prevented the engine from reading real-world LAS files
produced by standard tools (laspy, PDAL, py3dtiles, CloudCompare). This
release also ships the first public head-to-head benchmark against
py3dtiles, plus credibility cleanup on the docs surface.

### Fixed — Critical

- **LAS header point-count offset (commit `d424bf1`)** — the LAS parser
  read the "number of point records" from byte offset **100**, but the
  ASPRS LAS spec places it at offset **107** (offset 100 is the VLR-count
  field). The engine therefore could not read LAS files written by
  standard tooling — it read the VLR count as the point count and got 0
  or garbage. The bug was masked because the in-repo test-data generators
  wrote the point count at offset 100 too, so engine + fixtures were
  internally consistent while both diverged from the spec by 7 bytes.
  Fixed **three** independent parser implementations:
  - `parse_las_header` / `parse_las_points_core` (`point_cloud.rs`)
  - `LazFileHeader::read_from_cursor` (`point_cloud.rs`, LAZ path) — also
    fixed swapped VLR/point-count reads and the VLR count being read as
    `u16` instead of `u32`
  - streaming parser (`point_cloud_stream.rs`) — point-count offset
    100→107; bounds offsets 182–222 → 179–219 (were shifted +3); sequential
    cursor parser reordered to spec-correct positions
  All committed LAS fixtures regenerated via laspy (the reference impl) so
  they round-trip with laspy/py3dtiles. Cross-validated: engine reads
  canonical laspy files; laspy reads regenerated fixtures.

### Added

- **Head-to-head benchmark vs py3dtiles + loaders.gl** (`bench/head-to-head/`,
  commit `6bb48b7`) — reproducible harness comparing the headline task
  (LAS → Cesium 3D Tiles) across engines, with a public comparison page
  published to GitHub Pages (`/benchmarks/`). Fairness controls: identical
  input bytes (SHA-256 recorded), no reprojection, trimmed-mean timing,
  output byte/tile verification; rows where wasm-spatial-core loses or ties
  are kept. **CI result (Linux, AMD EPYC, 4 CPUs, canonical 500k-point LAS):**
  wasm-spatial-core **74 ms** (6.0 MB out, 64 tiles) vs py3dtiles 12.1.1
  **3.79 s** (7.9 MB, 40 tiles) — **~51× faster end-to-end**, with lower
  peak RSS (192 MB vs 244 MB). loaders.gl documented as parse-only (no
  shipped LAS→pnts writer). New CI job `head-to-head-bench` republishes on
  every master push.
- **W4 WebGPU benchmark + W3.6 acceptance test** (carried from Unreleased) —
  `tests/webgpu-bench.spec.mjs` measures the W4.3/W4.4 exit criteria on the
  real Metal adapter; `tests/cesium-terrain.spec.mjs` drives headless Cesium
  1.119 to prove quantized-mesh output loads via the real consumer.
- **`encodeTerrainTmsPyramid` WASM API** — emits a TMS-layout quantized-mesh
  pyramid (`layer.json` + `{z}/{x}/{y}.terrain`) consumable by
  `CesiumTerrainProvider`.
- **Playwright browser-test infrastructure** — first browser tests in the
  repo (`@playwright/test`, `playwright.config.mjs`, test server + fixtures).

### Changed

- **Roadmap calibration** — W4.3 re-scoped 🟡→✅: point transform is
  memory-bound and WASM SIMD legitimately wins on integrated GPU (Apple M4:
  0.72×); the "GPU beats WASM" claim is now documented as a discrete-GPU
  hardware precondition, not an unmet bug. No 🟡 items remain. W3.6 promoted
  🟡→✅ via the Cesium acceptance test.
- **Test count unified to 857** (`cargo test --all-features` = 857 passed /
  0 failed / 34 ignored) across README, npm README badge, and ROADMAP_V2.
- **WGSL reserved-keyword fix** — `target` field renamed to `target_height`
  in `heightfield_flatten_v1.wgsl`; the kernel had never compiled in a real
  browser (caught by the new bench).

### Documentation

- **`ARCHITECTURE.md`** added — canonical 3-layer split + data-flow diagram
  + module→feature map, consolidating VISION/ENGINE_BOUNDARY/ROADMAP.
- **`CODE_OF_CONDUCT.md`** added (Contributor Covenant 2.1).
- **Draco (V1-B4) re-evaluated 2026-07-09** — `draco-oxide@0.1.0-alpha.7`
  still blocked on wasm32 (getrandom 0.3 backend + an independent
  wasm-bindgen version-pin wall via js-sys 0.3.77). Blocker shifted, not
  removed; reproduce command documented. F2 quantization remains the
  in-engine compression alternative.
- **`bench/browser/index.html`** — fixed broken WASM import (`../pkg/` →
  `/pkg/`) so the in-browser benchmark works on GitHub Pages.


### Added — W4 WebGPU benchmark + W3.6 acceptance test

- **W4 GPU-vs-WASM benchmark** (`tests/webgpu-bench.spec.mjs` +
  `tests/fixtures/webgpu-bench.html`) — drives headless Chromium on the real
  Metal adapter to measure the two hardware-gated W4 exit criteria:
  - W4.3 transform (10M points): WASM 120.6ms vs GPU 167.0ms = 0.72× on Apple
    M4 integrated GPU (WASM wins — memory-bound; a discrete GPU is still
    required to demonstrate the GPU advantage). Parity max abs err 6.1e-5.
  - W4.4 heightfield flatten (2048×2048): WASM 47.8ms vs GPU 29.2ms = **1.64×**
    on the same M4 (GPU wins even integrated). Exact-match parity (max abs
    err 0). Promoted W4.4 from 🟡→✅ in ROADMAP_V2.
  The test self-skips on environments without a WebGPU adapter (headless Linux
  CI without GPU). Parity is a hard gate; speedup numbers are reported but not
  gated (hardware-dependent).

### Fixed

- **WGSL reserved-keyword compile bug in `heightfield_flatten_v1.wgsl`** — the
  shader used `target` as a struct field name, but `target` is a reserved
  keyword in WGSL, so Chrome rejected it with "Error while parsing WGSL:
  'target' is a reserved keyword". The heightfield flatten kernel never
  compiled in a real browser. Renamed to `target_height` in all three sync
  points (`shaders/heightfield_flatten_v1.wgsl`, `examples/shared/webgpu-kernels.mjs`,
  `npm/webgpu.ts`). Caught by the new `webgpu-bench.spec.mjs` — without the
  bench, this bug had no automated coverage.

### Added — W3.6 acceptance test (quantized-mesh spec compliance)

- **Headless Cesium `CesiumTerrainProvider` load test** — the load-bearing gap
  in W3.6 is now closed. The existing `quantized_mesh_roundtrip_test.rs` only
  proved our encoded bytes decode against our OWN decoder; `tests/cesium-terrain.spec.mjs`
  drives a headless Chromium + Cesium 1.119 to fetch a `layer.json` + `{z}/{x}/{y}.terrain`
  pyramid we generate, construct a `CesiumTerrainProvider`, and run
  `sampleTerrainMostDetailed` — which only resolves if Cesium successfully
  decoded our quantized-mesh bytes. CI job `browser-test` (`.github/workflows/ci.yml`).
- **`encodeTerrainTmsPyramid` WASM API** (`src/terrain_tms.rs`, gated under
  `geotiff`) — emits a TMS-layout quantized-mesh pyramid (`layer.json` +
  `0/0/0.terrain` + `1/{0,1}/{0,1}.terrain`) consumable by `CesiumTerrainProvider`.
  Distinct from `geotiff::encode_terrain_tileset_core`, which emits a 3D-Tiles
  `tileset.json` consumed by `Cesium3DTileset` (which does NOT natively render
  quantized-mesh). The new path targets the real terrain consumer.
- **Playwright infrastructure** — first browser test in the repo. Adds
  `@playwright/test` devDependency, `playwright.config.mjs`, custom
  `tests/terrain_tms_server.mjs` (regenerates artifacts at boot + serves repo
  root + `/terrain-tms/*`), `tests/terrain_tms_generate.mjs` (CLI + library
  `generateAll()`), `tests/fixtures/cesium-terrain-loader.html` (test page).
  `npm run test:browser` runs the suite locally.

### Changed

- W3.6 (`ROADMAP_V2.md`) promoted 🟡→✅ with the new acceptance test as the
  verification basis.

## [0.8.2] - 2026-07-03

### Fixed
- **npm package contents (critical)** — 0.8.0 and 0.8.1 published broken npm packages: `npm install wasm-spatial-core@0.8.x` produced a 4-file package (README + index.ts + package.json + LICENSE) with **no `.wasm` or `.js`** — the WASM binary and JS bindings were missing entirely, so `import` failed. Root cause: the CI publish job copied `npm/package.json` over wasm-pack's `pkg/package.json` and ran `npm publish` from `pkg/`; npm/package.json's `files` paths (`pkg/wasm_spatial_core.js`) then resolved to `pkg/pkg/wasm_spatial_core.js` (double prefix), excluding the real files. Fix: publish from `pkg/` using wasm-pack's own `package.json` (correct `files`/`main`/`types` relative to cwd), verified locally to produce a 6-file package with the 1.2 MB `.wasm` + 197 KB `.js`. Run `npm view wasm-spatial-core@0.8.2` to confirm.

## [0.8.1] - 2026-07-02

### Added
- **W3.6 quantized-mesh encoder (spec-conformant)** — new `src/quantized_mesh.rs` module emitting byte-exact Cesium quantized-mesh-1.0 tiles (88-byte header, zig-zag delta vertex encoding, high-water-mark index/edge encoding, `.terrain` extension). This is the module behind the W3.6 compliance fix listed under Fixed below; round-trip test in `tests/quantized_mesh_roundtrip_test.rs`.
- **3DGS PLY ingest (minimal slice)** — `parsePly` now recognizes 3D Gaussian Splatting `.ply` files by their `f_dc_0` property and derives RGB colors from the SH degree-0 coefficients (`RGB = clamp((0.5 + 0.2820945569·f_dc)·255)` per [graphdeco-inria#485](https://github.com/graphdeco-inria/gaussian-splatting/issues/485)). A 3DGS file no longer degrades to a black, uncolored point cloud; the derived RGB flows through `PointCloudChunk` and region selection like any point cloud. The 56 high-order splat attributes (`f_rest_*`, `opacity`, `scale_*`, `rot_*`) are intentionally ignored (faithful splat rendering is future scope).
- **pre-commit hook** — `.githooks/pre-commit` auto-formats staged `.rs` files via `cargo fmt` so a commit never carries unformatted Rust. Opt-in per clone: `git config core.hooksPath .githooks`.

### Fixed
- **W3.6 quantized-mesh compliance** — `encode_quantized_mesh_core` now emits byte-exact Cesium quantized-mesh-1.0 tiles (was a self-invented layout that CesiumTerrainProvider could not load): correct 88-byte header with real f32 heights + bounding sphere + horizon occlusion point, zig-zag delta vertex encoding (MAX_UV=32767), high-water-mark index/edge encoding, `.terrain` extension (was `.cmpt`). New module `src/quantized_mesh.rs` with round-trip test in `tests/quantized_mesh_roundtrip_test.rs`.
- **`PointCloudChunk::select_points` color stride** — was copying colors with an RGBA (4-byte) stride while every ingest path produces RGB (3-byte); silently corrupted colors on region selection. Now uses the correct 3-byte stride.
- **`parsePly` input limit** — was the only parser still using the static 100 MB `DEFAULT_MAX_INPUT_SIZE`; now uses the runtime `get_current_input_limit()` so JS `setInputSizeLimit()` applies (large 3DGS scenes no longer hard-rejected).

## [0.8.0] - 2026-06-27

### Added
- **Wave 1 — Core runtime** (incremental tiles, cancellable jobs, memory budget):
  - `TilesetPatch` / `applyTilesetPatch` — replace individual tile blobs by URI without full rebuild
  - `parseLasPointsWithProgressAndAbort`, `generateTilesetWithAbort` — honour abort callbacks between chunks
  - `SpatialError::Cancelled` (`CANCELLED` code) for programmatic cancel handling
  - `ProcessingContext` — reusable position/color buffer arena across pipeline steps
  - `estimateJobBytes` — heuristic job memory estimate for UI warnings
  - `wasm-spatial-core/abort` — `createAbortChecker`, `linkAbortSignalToWorker`, `runWithAbortSignal`
- **Wave 2 — Spatial IR + GLB ingest** (`mesh-ingest` feature):
  - `SpatialChunk` IR: `PointCloudChunk`, `MeshChunk`, `HeightfieldChunk` with `ChunkMeta` (CRS, AABB, version)
  - `parseGlb()` / `parseGlbCore()` — GLB → `MeshChunk`; round-trip with `meshToGlb`
  - `WasmMeshChunk` — `selectAabb()`, `toGlb()`, geometry getters
  - AABB region select on mesh and point cloud chunks
  - **W2.5 export** — `exportPointCloudToPnts()` → single `.pnts` + minimal `tileset.json`; mesh `exportToGlb`
  - **W2.6 ENU frame** — `createEnuFrame()`, `wgs84ToEnu` / `enuToWgs84`, f32 rendering offsets; sub-mm round-trip at 1 km
  - **W2.7 SVD alignment** — `computeSvdAlignment()` Umeyama/Kabsch solve; 4×4 matrix for `transformPointCloud`; pairs with ENU survey control points
  - **W2.7 robust alignment** — weighted solve, per-point residuals/inlier report, RANSAC (`computeSvdAlignmentRansac`)
  - **W2.4 polygon select** — `PolygonExtrusion`, `selectByPolygon` on mesh/point-cloud chunks; WASM `WasmMeshChunk.selectPolygon()`
- **Wave 5 — Mesh geometry edit** (`mesh-edit` feature, requires `mesh-ingest`):
  - `classifyTrianglesByObb` — inside/outside triangle classification relative to OBB
  - `splitMeshByObb` — phase-1 mesh split into inside/outside submeshes + GLB export (`WasmMeshSplit`)
  - **W5.3 plane clip** — `clipMeshByPlane` with linear position/normal/UV interpolation
  - **W5.4 cap holes** — `clipAndCapMesh` ear-clips planar boundary loops (Euler χ=2 on box fixture)
  - **W5.5 QEM decimation** — `simplifyMeshQem` Garland–Heckbert CPU path + `benchmark_mesh_qem_100k_to_10k`
  - **W5.6 UV seam preservation** — `QemOptions.preserve_uv_seams` blocks collapses across seam/coincident-UV edges; UVs retained in output
  - **W5.7 GPU QEM** — WGSL `mesh_quadrics_v1` + `mesh_edge_costs_v1`; `GpuContext.accumulateQuadrics` / `evaluateEdgeCosts`; `simplifyMeshQem` hybrid GPU path with WASM fallback
- **Wave 4 — WebGPU compute** (`webgpu` feature):
  - `wasm-spatial-core/webgpu` — `GpuContext`, `transformPoints`, `flattenHeightfield`, `simplifyMeshQem` with WASM fallback
  - WGSL kernels in `shaders/` (`transform_points_v1`, `heightfield_flatten_v1`, `mesh_quadrics_v1`, `mesh_edge_costs_v1`)
  - Buffer layout contract (Rust + TS + `shaders/README.md`)
  - Demo `examples/webgpu-smoke/` — GPU vs CPU parity on 1k points
- **Wave 3 — Terrain deformation** (`terrain-edit` feature, requires `geotiff`):
  - `rasterizeTerrainMask`, `excavateTerrain`, `flattenTerrain`, `fillTerrain`
  - Polygon-masked cut / flatten / fill with boundary feathering
  - `encodeDeformedTerrainTileset` — deformed grid → quantized-mesh pyramid
  - Golden raster tests for cut/flatten on fixture grids
- **Product vision** — [VISION.md](./VISION.md): next-gen Web3D spatial engine positioning (latest Chrome, WASM + WebGPU)
- **Roadmap V2** — [ROADMAP_V2.md](./ROADMAP_V2.md): Waves 1–5 capability plan
- **Issue templates** — [docs/issues/WAVE_1.md](./docs/issues/) … `WAVE_5.md` + GitHub form `.github/ISSUE_TEMPLATE/roadmap_v2_capability.yml`

### Fixed
- **GLB writer** — `indices` accessor now serialized on triangle primitives (required for GLB read round-trip)

### Changed
- **Wave 1 scope** — Dropped trajectory/geofence from engine roadmap (application-layer concern)
- **Core-only pass** — Removed instance/parking twin APIs from roadmap; W1 = runtime (patch/cancel) only; added [ENGINE_BOUNDARY.md](./docs/ENGINE_BOUNDARY.md); deferred frustum cull, GPU QEM, SVD from core path; **W2 Spatial IR** is default start

### Engineering & quality (0.8.0 release hardening)
- **CI baseline restored** — Fixed 4 CI-blocking issues: `cargo fmt` on bench/geotiff/ply; clippy `manual_is_multiple_of` (Rust 1.93) in `gltf_writer.rs`; `real_geotiff_test` / `external_geotiff_test` missing `required-features = ["geotiff"]` (E0432/E0282); feature-gated runtime test assertion
- **Test count** — 819 tests green under `cargo test --all-features` (was 661 on prior landing page)
- **Perf gates verified** — W3 flatten 2048×2048 ~40 ms (native release, budget 500 ms); W5 QEM 99458→10000 triangles, max_error 0.058650
- **Roadmap calibrated** — All V2 exit criteria annotated with backing test (✅/🟡/⚠️); only 2 items remain open (W4.3/W4.4 discrete-GPU speedup, hardware-dependent)
- **`pkg/` untracked** — Removed 8 build artifacts from git tracking (incl. ~1.1 MB `.wasm` binary committed across 7 history entries); root cause was a tracked `pkg/.gitignore` bypassing the top-level `/pkg` rule; anti-regression notes added to `AGENTS.md` / `CONTRIBUTING.md`

## [0.7.1] - 2026-06-06

### Added
- **Node.js batch API** (`wasm-spatial-core/node`) — Server-side point cloud and GeoTIFF pipelines via wasm-pack `nodejs` target:
  - `loadSpatialCoreNode()` — initialise WASM in Node.js without browser headers
  - `batchPointCloudToTileset()` — parse → octree → 3D Tiles in one call
  - `batchGeotiffToTerrain()` — GeoTIFF → quantized-mesh terrain tileset
  - `npm run build:wasm:node` builds the Node.js WASM package into `npm/pkg-node/`
- **GeoTIFF LZW decompression** — TIFF LZW strips/tiles now decode via `weezl` (TIFF size-switch compatible)

### Changed
- **`generateTileset` default LOD** — Automatically estimates point spacing and applies spacing-aware `geometricError` for smoother LOD transitions (previously required `generateTilesetWithSpacing`)
- `generateTilesetWithSpacing` remains available for explicit spacing/factor overrides

## [0.7.0] - 2026-06-01

### Added
- **Point cloud analysis toolkit** (`src/point_cloud_analysis.rs`) — Comprehensive analysis functions:
  - `pointCloudAnalysis()` — Full statistics: bounds, centroid, std deviation per axis, average spacing, density, color distribution
  - `filterByBounds()` — Spatial bounding box filter with color preservation
  - `filterByClassification()` — ASPRS classification filter (ground, vegetation, buildings, water, etc.)
  - `transformPointCloud()` — 4×4 matrix transformation (column-major, WebGL convention)
  - `translatePointCloud()` — Translation (dx, dy, dz)
  - `scalePointCloud()` — Non-uniform scaling (sx, sy, sz)
  - `rotatePointCloud()` — Rodrigues' rotation around arbitrary axis
  - `mergePointClouds()` — Merge two point clouds with color handling
  - `PointCloudStats` struct with JSON serialization
  - `FilteredResult` struct for filter/merge outputs
- **WebGL Point Cloud Viewer** (`examples/webgl-pointcloud/`) — Lightweight zero-dependency viewer:
  - Native WebGL point rendering with custom shaders (circular points, EDL effect)
  - Hand-written matrix math (perspective, lookAt, multiply, translate, rotate)
  - Trackball camera (left-drag rotate, right-drag pan, scroll zoom)
  - Distance-adaptive point sizing (simplified LOD)
  - Color modes: original, height gradient, classification, density heatmap
  - WASM integration with JS fallback parser
  - Touch support, FPS counter, point size control
- **Cesium Workflow Demo** (`examples/cesium-workflow/`) — Complete "drag→3D" pipeline:
  - Smart format detection (LAS, LAZ, PLY, OBJ, GeoTIFF, GLB, glTF)
  - Visual pipeline progress (parse → octree → encode → load)
  - Simple octree spatial partitioning for tile generation
  - pnts tile encoding and tileset.json generation
  - Cesium point primitive rendering with auto-fly-to
  - Export to ZIP (JSZip), GLB placeholder, clipboard share
  - Zero token, zero server, fully browser-based
- **Point cloud analysis documentation** (`docs/webgl-pointcloud/`)

### Changed
- Test count: 658 → 689 (+31 new analysis tests)
- Source lines: ~31,544 → ~35,050 (new module + examples)
- WASM module: `src/point_cloud_analysis.rs` registered under `point-cloud` feature
- `lib.rs` exports: added `point_cloud_analysis` re-exports
- Fixed `test_check_memory_available_no_limit` test for native targets (overflow-safe)

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

[Unreleased]: https://github.com/reed-soul/wasm-spatial-core/compare/v0.10.0...HEAD
[0.10.0]: https://github.com/reed-soul/wasm-spatial-core/releases/tag/v0.10.0
[0.9.0]: https://github.com/reed-soul/wasm-spatial-core/releases/tag/v0.9.0
