# Performance Benchmarks

**Published, reproducible claims** (the 1.0 documentation bar):

- Head-to-head LAS → 3D Tiles vs py3dtiles: [`benchmarks/index.md`](./benchmarks/index.md)
  (generated 2026-07-08, Apple M4, wasm-spatial-core 0.8.2: **36 ms** vs py3dtiles
  2.86 s on the canonical 500k LAS). Reproduce with the commands in that file.
- WebGPU vs WASM heightfield / point transform: `tests/webgpu-bench.spec.mjs`
  (see [ROADMAP_V2.md](./ROADMAP_V2.md) W4). Point transform is memory-bound on
  integrated GPUs; heightfield flatten measured **1.64×** GPU on Apple M4 Metal.

The tables below are **historical native `debug` timings** (Apple M2, ~May 2025,
crate v0.6 era). They are useful for relative scale (10K vs 1M) but are **not**
the numbers to quote in README or the marketing site. Re-run with `--release`:

```bash
cargo test --test perf_test --features point-cloud --release -- --nocapture
```

## Point Cloud Pipeline (LAS → Octree → 3D Tiles)

| Scale | Octree Build | Tileset Gen | Octree Nodes | Tiles | Output Size |
|-------|-------------|-------------|-------------|-------|-------------|
| 10K   | 2.11 ms     | 0.55 ms     | 41          | 32    | 119 KB      |
| 100K  | 23.74 ms    | 4.29 ms     | 137         | 97    | 1.15 MB     |
| 1M    | 270 ms      | 41 ms       | 649         | 401   | 11.47 MB    |
| 10M   | 2,944 ms    | —           | 2,769       | —     | —           |

- 10M only runs octree build (tileset generation on 10M exceeds typical browser memory budgets).
- Octree is the dominant cost — expected for an O(n log n) spatial partition.
- Tileset generation scales roughly linearly with octree node count.

## LAZ Decompression (100K points)

| Metric | Value |
|--------|-------|
| Raw size | 2.0 MB |
| Compressed | 117 KB (5.8% ratio) |
| Decompress time | <1 ms |
| Compress time | 78.8 ms (one-time, server-side) |

LAZ decompression is near-instant at this scale — the `laz` crate uses vectorized bitstream decoding.

## WASM Binary Size

Measured after `wasm-pack build --target web --release --out-dir pkg
-- --features $(cat scripts/npm-wasm-features.txt)` (wasm-pack 0.13.1,
`wasm-opt = false` as in Cargo.toml, 2026-10-27 Asia/Shanghai):

| Metric | Value |
|--------|-------|
| Features | `point-cloud,geotiff,laz-support,mesh-ingest,terrain-edit,mesh-edit,webgpu` |
| WASM binary | 2,048,406 bytes (1.95 MB) |
| gzip | 709,039 bytes (693 KB) |
| JS glue | 362,045 bytes |
| TypeScript d.ts | 4,375 lines |
| Exported functions | 236 |
| Exported classes | 45 |

Re-measure:

```bash
stat -c%s pkg/wasm_spatial_core_bg.wasm
gzip -c pkg/wasm_spatial_core_bg.wasm | wc -c
```

Historical v0.6.0 snapshot (`point-cloud,geotiff` only): **1.2 MB** WASM, 173
exports. 0.11 adds `laz-support`, `mesh-ingest`, `terrain-edit`, `mesh-edit`,
and `webgpu` — the uncompressed binary is larger; gzip transfer is ~693 KB.

> WASM streaming compilation means the browser can start executing before the full download completes.
> `multi-thread` feature requires `atomics` + `bulk-memory` and COOP/COEP headers — build separately with `RUSTFLAGS='-C target-feature=+atomics,+bulk-memory'`.

## Point Spacing Estimation (Grid-Indexed)

| Scale | Grid-Indexed | Brute Force | Speedup |
|-------|-------------|-------------|---------|
| 1K    | 0.2 ms      | 0.3 ms      | 1.5×    |
| 10K   | 0.5 ms      | 8.2 ms      | 16×     |
| 100K  | 3.1 ms      | 820 ms      | 264×    |
| 500K  | 18 ms       | ~20 s       | ~1100×  |

- Grid-indexed uses spatial hashing with progressive ring expansion.
- Brute force is O(n × sample_size); grid-indexed is O(n + sample × k).
- Results agree within 2× for non-uniform point distributions.

## Comparison with JavaScript Libraries

| Library | 100K Octree | 1M Octree | WASM? | Notes |
|---------|------------|-----------|-------|-------|
| **wasm-spatial-core** | 24 ms | 270 ms | ✅ | This crate, single-thread |
| potree (JS) | ~200 ms | ~3 s | ❌ | Server-side, WebWorker recommended |
| three.js Octree | ~150 ms | ~2.5 s | ❌ | No built-in 3D Tiles support |
| las-js | ~500 ms parse | — | ❌ | LAS-only, no octree/tiles |

> Note: JS benchmarks are approximate from published reports and may vary by engine/device.
> WASM consistently achieves 5–10× speedup on compute-heavy operations.

## WASM Build (wasm-pack 0.13.1; installer may be newer)

| Metric | Value |
|--------|-------|
| Target | `wasm32-unknown-unknown` (web) |
| Features | see `scripts/npm-wasm-features.txt` |
| WASM binary | 1.95 MB (gzip 693 KB) |
| JS glue | 354 KB |
| TypeScript d.ts | 4,375 lines |
| Exported functions | 236 |
| Exported classes | 45 |

### Build Commands

```bash
# Web target (bundler / ESM)
wasm-pack build --target web --features $(cat scripts/npm-wasm-features.txt)

# Node.js target (for smoke tests)
wasm-pack build --target nodejs --features $(cat scripts/npm-wasm-features.txt)
```

### Smoke Test

```bash
node tests/wasm_smoke_test.mjs  # 47 API tests, all pass in <1s
```

### Demo Server

```bash
./scripts/serve_demo.sh          # http://localhost:8080
```

> `pkg/` is git-ignored (build artifact). Rebuild after `git clone`.

## Running Benchmarks

```bash
# Performance tests (print timing to stderr)
cargo test --test perf_test --all-features -- --nocapture

# Full test suite
cargo test --all-features

# With release optimizations
cargo test --test perf_test --all-features --release -- --nocapture
```

## Methodology

- Synthetic scan-line point patterns with configurable spread.
- Octree max depth: 10–16 depending on scale; max points per node: 1,000–20,000.
- All timing uses `std::time::Instant` (wall-clock, single thread).
- Tests run on Apple M2 ARM64, macOS 14, Rust 1.90.
