# Performance Benchmarks

Benchmarks measured on **Apple M2 (arm64)**, Rust `debug` profile (native), May 2025.

> For release-optimized numbers, build with `--release`. Debug builds have ~5–10× overhead
> on hot loops due to bounds checks and lack of inlining.

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

| Build | Size | Notes |
|-------|------|-------|
| Default (GeoJSON + coords) | **1.0 MB** | No point cloud features |
| point-cloud | **1.1 MB** | +LAS parsing, octree, pnts, tileset |
| laz-support | **1.2 MB** | +LAZ/COPC decompression (~200KB overhead) |

> After gzip/brotli compression, typical transfer size is 200–400 KB.
> WASM streaming compilation means the browser can start executing before the full download completes.

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
