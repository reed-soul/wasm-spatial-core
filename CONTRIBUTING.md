# Contributing to wasm-spatial-core

Thanks for your interest in improving this project! This guide covers everything you need to get started.

## Design Philosophy

`wasm-spatial-core` is built on one idea: **bring server-grade spatial computing to the browser via WebAssembly, compiled from Rust**. Every API is designed for zero-copy memory sharing, streaming processing of large datasets, and direct GPU pipeline feeding. We prioritise performance, correctness, and a clean JS/WASM interop boundary over feature count.

---

## Development Environment

### Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| [Rust](https://rustup.rs/) | stable **≥ 1.90** (`rust-version` in `Cargo.toml`) | `rustup default stable` |
| [wasm-pack](https://rustwasm.github.io/wasm-pack/) | latest | `curl https://rustwasm.github.io/wasm-pack/installer/init.sh \| sSf \| sh` |
| [Node.js](https://nodejs.org/) | ≥ 18 | [nodejs.org](https://nodejs.org/) |

For multi-threading support (optional):
```bash
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
```

### Setup

```bash
git clone https://github.com/reed-soul/wasm-spatial-core.git
cd wasm-spatial-core
cargo build
wasm-pack build --target web --release --out-dir pkg -- --features point-cloud
```

The `pkg/` directory is listed in `.gitignore` and is **not** stored in git. CI builds it on every run; locally you must run `wasm-pack build` before opening browser examples.

---

## Error handling (API)

WASM exports that return `Result<_, JsValue>` should use helpers in `src/errors.rs` (`parse_js`, `invalid_input_js`, `tile_js`, etc.) so JavaScript receives `{ name: "SpatialError", code, message }`.

Prefer `Result<_, SpatialErrorDetail>` in new Rust-only APIs; convert with `.map_err(Into::into)` at the wasm boundary.

---

## Build Commands

```bash
# Native debug build
cargo build

# Native release build
cargo build --release

# WASM package (single-threaded, outputs to pkg/)
wasm-pack build --target web --release --out-dir pkg

# WASM package with point cloud support
wasm-pack build --target web --release --out-dir pkg -- --features point-cloud

# WASM package with LAZ support (+ ~400KB)
wasm-pack build --target web --release --out-dir pkg -- --features laz-support

# WASM package (multi-threaded, requires nightly)
RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals' \
  cargo +nightly build -Z build-std=panic_abort,std \
  --target wasm32-unknown-unknown --release --features multi-thread

# Run unit & integration tests
cargo test --all-features

# Large-scale stress tests (ignored in CI; run before releases)
cargo test -- --ignored

# WASM bindgen tests (Node.js harness; CI uses this)
wasm-pack test --node --release -- --test web

# Run benchmarks
cargo bench

# Check formatting
cargo fmt --all -- --check

# Lint
cargo clippy --all-targets --all-features -- -D warnings
```

### Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `single-thread` | ✅ | Zero-config mode, works everywhere, no COOP/COEP needed |
| `multi-thread` | ❌ | Web Workers + SharedArrayBuffer via Rayon (requires nightly) |
| `point-cloud` | ❌ | LAS/PCD/PLY/OBJ parsing + voxel grid decimation + octree + 3D Tiles |
| `laz-support` | ❌ | LAZ/COPC decompression via `laz` crate (implies `point-cloud`, adds ~400KB) |
| `e57-support` | ❌ | E57 architectural/industrial scan format via `e57` crate |

### Feature Flag Combinations

```bash
# Minimal (coordinate + GeoJSON only)
cargo build                           # default features

# Point cloud pipeline
cargo build --features point-cloud    # LAS + octree + 3D Tiles

# Full point cloud (LAS + LAZ + COPC)
cargo build --features laz-support    # implies point-cloud

# All features
cargo build --all-features            # everything
```

---

## Adding a New Point Cloud Format

To add support for a new point cloud format (e.g., XYZ, PTX, RIEGL):

### 1. Create a parser module

Add a new file `src/<format>.rs`:

```rust
use wasm_bindgen::prelude::*;

/// Parse <FORMAT> point cloud file
#[wasm_bindgen]
pub fn parse<Format>Points(bytes: &[u8]) -> Result<LasPointCloud, JsValue> {
    // 1. Parse header/structure
    // 2. Extract positions (Float32Array)
    // 3. Extract optional colors (Uint8Array)
    // 4. Return as LasPointCloud (or create a new result type)
    todo!()
}
```

### 2. Register in `src/lib.rs`

Add the module declaration and WASM exports:

```rust
#[cfg(feature = "point-cloud")]
mod <format>;
```

### 3. Add to auto-detection

Update `parsePointCloudAuto()` to detect the new format by magic bytes/extension.

### 4. Add tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_<format>_header() {
        // Test header parsing
    }

    #[test]
    fn test_parse_<format>_points() {
        // Test point extraction
    }
}
```

### 5. Update `npm/index.ts`

Add the new exports to the TypeScript convenience wrapper.

### Guidelines

- **Reuse `LasPointCloud`** for result types when the format has similar attributes (positions, colors, normals).
- **Zero-copy**: Return `Float32Array` / `Uint8Array` views into WASM memory, not serialized JSON.
- **Progress callbacks**: For large files, accept an `on_progress: Function` parameter.
- **Streaming**: If the format supports random access (like COPC), implement a `PointCloudStreamer`-like interface.

---

## Performance Testing

### Benchmarking with Criterion

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench -- spatial_benchmarks

# Generate HTML report
cargo bench -- --save-baseline main
open target/criterion/report/index.html
```

### Browser Performance

The `bench/browser/` directory contains browser-side benchmarks comparing WASM vs pure JS implementations:

```bash
# Build and serve benchmarks
npm run demo
# Open http://127.0.0.1:8080/bench/browser/index.html
```

### Performance Checklist

- [ ] `cargo bench` — native criterion benchmarks
- [ ] Browser benchmarks — WASM vs JS comparison
- [ ] Memory profiling — check `memoryInfo()` output for large inputs
- [ ] WASM binary size — `ls -la pkg/wasm_spatial_core_bg.wasm`
- [ ] Load time — measure WASM init time in browser

### Known Performance Characteristics

- **Coordinate transforms**: ~27× faster than proj4js (SIMD-hinted loops)
- **GeoJSON parsing**: ~11× faster than JSON.parse + manual extraction
- **Octree build**: 1M points < 3s in single-threaded WASM
- **3D Tiles generation**: ~1s for 1M points (octree + pnts encoding)
- **WASM binary**: ~1.2 MB with point-cloud + LAZ features

---

## Code Style

We enforce strict formatting and linting via CI:

- **rustfmt**: `cargo fmt --all -- --check` — all code must be formatted
- **clippy**: `cargo clippy --all-targets --all-features -- -D warnings` — no warnings allowed

### Rust Conventions

- Prefer `#[inline]` on hot-path functions (coordinate transforms, parsers)
- Use `Float64Array` for coordinates (WGS-84 precision), `Float32Array` for GPU buffers
- Validate input sizes with `validate_input_size()` — max 100 MB (adjustable via `setInputSizeLimit()`)
- All public WASM functions must accept typed arrays, not JS objects
- Point cloud functions should support progress callbacks for large files
- Octree and spatial index operations must be O(n log n) or better

---

## Commit Message Convention

We follow a simplified [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>: <short description>

<optional body with context>
```

**Types:**

| Type | Usage |
|------|-------|
| `feat` | New feature or capability |
| `fix` | Bug fix |
| `docs` | Documentation changes |
| `refactor` | Code restructuring without behavior change |
| `test` | Adding or updating tests |
| `perf` | Performance improvement |
| `chore` | Build, CI, dependency updates |

**Examples:**
```
feat: add UTM projection support
feat: implement E57 point cloud parser
fix: handle empty GeoJSON FeatureCollection gracefully
perf: SIMD-accelerated coordinate transform inner loop
docs: update API reference for SpatialEdgeIndex
test: add edge cases for boundary polygon buffer
chore: upgrade wasm-bindgen to 0.2.97
```

---

## Pull Request Process

1. **Fork** the repository
2. **Create a feature branch**: `git checkout -b feat/your-feature`
3. **Make changes** with clear commit messages
4. **Ensure all checks pass**:
   - `cargo fmt --all -- --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all-features`
5. **Open a PR** against `master` — use the [PR template](.github/PULL_REQUEST_TEMPLATE.md)
6. **Address review feedback** — CI must pass before merge

---

## Testing Requirements

- **New features must include tests.** At minimum: happy path + edge cases.
- **Bug fixes must include a regression test** that reproduces the original issue.
- **All PRs must pass CI** (fmt, clippy, test, wasm-pack build).
- Tests live in two places:
  - `#[cfg(test)] mod tests` blocks inside source files (unit tests)
  - `tests/` directory (integration tests)

### Current Test Coverage

- **400 tests** across all modules
- **Integration tests**: `tests/integration_test.rs`, `tests/point_cloud_pipeline.rs`
- **Stress tests**: `tests/stress_test.rs` (marked `#[ignore]`, run with `--ignored`)
- **WASM tests**: `tests/web.rs` (version smoke test via wasm-bindgen-test)

---

## Architecture Overview

```
src/
├── lib.rs               # WASM entry point, memory management, input validation
├── coordinate.rs        # Batch CRS projections (WGS84/GCJ02/BD09/Mercator/CGCS2000/UTM)
├── geojson_parser.rs    # GeoJSON → flat Float64Array coordinate extraction
├── geojson_streaming.rs # Chunked GeoJSON parser with progress callbacks
├── spatial_index.rs     # R-Tree spatial index + edge index (bbox/kNN queries)
├── vector_tile.rs       # MVT vector tile generation from GeoJSON
├── cesium_adapter.rs    # WGS84→Cartesian3, polygon triangulation, b3dm 3D Tiles
├── point_cloud.rs       # LAS/LAZ/PCD parsing + voxel/random decimation
├── point_cloud_stream.rs # Streaming point cloud loader (range-based access)
├── octree.rs            # 8-way octree spatial partitioning for point clouds
├── pnts.rs              # 3D Tiles Point Cloud (pnts) binary encoder
├── ply.rs               # PLY format parser (ASCII + binary)
├── obj.rs               # OBJ mesh parser
├── e57.rs               # E57 format support (via e57 crate)
├── wkb_wkt.rs           # OGC Well-Known Binary / Text format
├── topojson.rs          # TopoJSON format parser
├── gpx.rs               # GPS Exchange format parser
├── ifc_reader.rs        # IFC/BIM geometry extraction (experimental)
├── gltf_writer.rs       # glTF 2.0 / GLB binary scene builder
├── spatial_analysis.rs  # Buffer, bounding box, centroid, hull, clustering on WGS-84
├── topology.rs          # Polygon boolean ops, predicates, area, length
├── errors.rs            # Structured error types (SpatialError)
├── utils.rs             # Panic hook setup, coordinate normalization
└── worker.rs            # WebWorker thread pool initialization
```

---

## Issue Templates

Use the appropriate template when opening an issue:

- 🐛 **Bug Report** — [`.github/ISSUE_TEMPLATE/bug_report.md`](.github/ISSUE_TEMPLATE/bug_report.md)
- 💡 **Feature Request** — [`.github/ISSUE_TEMPLATE/feature_request.md`](.github/ISSUE_TEMPLATE/feature_request.md)

---

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
