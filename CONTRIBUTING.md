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
wasm-pack build --target web --release --out-dir pkg   # required for examples/ demos
```

The `pkg/` directory is listed in `.gitignore` and is **not** stored in git. CI builds it on every run; locally you must run `wasm-pack build` before opening browser examples.

---

## Error handling (API)

New code should return `Result<_, SpatialErrorDetail>` (see `src/errors.rs`) so JavaScript can read `e.code` / `e.message`.

Some legacy paths still throw plain `JsValue` strings (`geojson_streaming`, `vector_tile`, `wkb_wkt`, etc.). When touching those modules, prefer migrating to `SpatialError` rather than adding new string errors.

---

## Build Commands

```bash
# Native debug build
cargo build

# Native release build
cargo build --release

# WASM package (single-threaded, outputs to pkg/)
wasm-pack build --target web --release --out-dir pkg

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

# Optional: real browser (requires Chrome + chromedriver)
wasm-pack test --chrome --headless --release -- --test web

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
| `single-thread` | ✅ | Zero-config mode, works everywhere |
| `multi-thread` | ❌ | Web Workers + SharedArrayBuffer (requires nightly) |
| `point-cloud` | ❌ | LAS/LAZ parsing + voxel grid decimation |

---

## Code Style

We enforce strict formatting and linting via CI:

- **rustfmt**: `cargo fmt --all -- --check` — all code must be formatted
- **clippy**: `cargo clippy --all-targets --all-features -- -D warnings` — no warnings allowed

### Rust Conventions

- Prefer `#[inline]` on hot-path functions (coordinate transforms, parsers)
- Use `Float64Array` for coordinates (WGS-84 precision), `Float32Array` for GPU buffers
- Validate input sizes with `validate_input_size()` — max 100 MB
- All public WASM functions must accept typed arrays, not JS objects

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
5. **Open a PR** against `main` — use the [PR template](.github/PULL_REQUEST_TEMPLATE.md)
6. **Address review feedback** — CI must pass before merge

---

## Testing Requirements

- **New features must include tests.** At minimum: happy path + edge cases.
- **Bug fixes must include a regression test** that reproduces the original issue.
- **All PRs must pass CI** (fmt, clippy, test, wasm-pack build).
- Tests live in two places:
  - `#[cfg(test)] mod tests` blocks inside source files (unit tests)
  - `tests/` directory (integration tests)

---

## Issue Templates

Use the appropriate template when opening an issue:

- 🐛 **Bug Report** — [`.github/ISSUE_TEMPLATE/bug_report.md`](.github/ISSUE_TEMPLATE/bug_report.md)
- 💡 **Feature Request** — [`.github/ISSUE_TEMPLATE/feature_request.md`](.github/ISSUE_TEMPLATE/feature_request.md)

---

## Architecture Overview

```
src/
├── lib.rs               # WASM entry point, memory management, input validation
├── coordinate.rs        # Batch CRS projections (WGS84/GCJ02/BD09/Mercator/CGCS2000)
├── geojson_parser.rs    # GeoJSON → flat Float64Array coordinate extraction
├── geojson_streaming.rs # Chunked GeoJSON parser with progress callbacks
├── spatial_index.rs     # R-Tree spatial index + edge index (bbox/kNN queries)
├── vector_tile.rs       # MVT vector tile generation from GeoJSON
├── cesium_adapter.rs    # WGS84→Cartesian3, polygon triangulation, b3dm 3D Tiles
├── point_cloud.rs       # LAS/LAZ/PCD parsing + voxel/random decimation
├── ifc_reader.rs        # IFC/BIM geometry extraction (experimental)
├── gltf_writer.rs       # glTF 2.0 / GLB binary scene builder
├── spatial_analysis.rs  # Buffer, bounding box, centroid on WGS-84
└── utils.rs             # Panic hook setup
```

---

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
