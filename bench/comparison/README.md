# Performance Benchmark: wasm-spatial-core vs proj4js

## Quick Start

```bash
# Install dependencies
npm install proj4

# Build WASM for Node.js target
wasm-pack build --target nodejs --release --out-dir pkg-node

# Run benchmark
node bench/comparison/bench_comparison.mjs
```

## What It Tests

| Benchmark | wasm-spatial-core | proj4js |
|-----------|-------------------|---------|
| WGS84 → GCJ02 | `batchWgs84ToGcj02InPlace()` | `proj4("EPSG:4326", "EPSG:4490", ...)` |
| WGS84 → Mercator | `batchWgs84ToMercatorInPlace()` | `proj4("EPSG:4326", "EPSG:3857", ...)` |
| GeoJSON Parse | `parseGeoJsonCoords()` | Native `JSON.parse()` |

## Data Volumes

- **10K** points — typical single-tile feature count
- **100K** points — dense urban tile
- **1M** points — stress test

## Output Format

```
═══════════════════════════════════════════════════════════════
  wasm-spatial-core  vs  proj4js  Performance Comparison
═══════════════════════════════════════════════════════════════

── 10K points ──
  WGS84→GCJ02:  WASM 0.45ms     22.2K/s    | proj4js 15.23ms    656.6/s   | 33.8x
  ...

═══════════════════════════════════════════════════════════════
  Summary
═══════════════════════════════════════════════════════════════
│ Size      │ WGS84→GCJ02 WASM │ ... │ Speedup │
├───────────┼──────────────────┼ ...─┼─────────┤
│ 10K       │ 0.45ms          │ ... │    33.8x │
│ 100K      │ 4.52ms          │ ... │    34.1x │
│ 1M        │ 45.1ms          │ ... │    33.9x │
└───────────┴──────────────────┴ ...─┴─────────┘
```

## Notes

- `proj4js` doesn't support GCJ02 directly; we use EPSG:4490 (CGCS2000) as a comparable projection.
- GeoJSON parsing only runs for ≤100K points to avoid excessive memory usage.
- Each benchmark runs 3 warmup iterations + 10 timed iterations (trimmed mean).
- Results are machine-dependent; run on your target hardware for accurate comparisons.
