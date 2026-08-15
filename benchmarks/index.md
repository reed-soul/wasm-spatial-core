# Head-to-head benchmark: wasm-spatial-core vs alternatives

> Task: **LAS point cloud → Cesium 3D Tiles (.pnts + tileset.json)**, end-to-end, no upload, no server.

**Generated:** 2026-08-15T20:56:41.521Z  
**Hardware:** AMD EPYC 7763 64-Core Processor · 4 CPUs · linux/x64  
**Methodology:** same input bytes (SHA-256 verified), same no-op transform (no reprojection), trimmed mean of 5 timed runs after warmup. Output bytes = sum of all generated tile files + tileset.json.

## ⚖️ Fairness & honesty

- Every engine that produced a result is shown — including rows where wasm-spatial-core is **slower or ties**.
- **loaders.gl cannot do this task** (LAS read only, no pnts writer) — that row documents a capability gap, not a timeout. `output_bytes = 0`.
- **Cesium ion is not timed here** — it requires uploading private data to a cloud service and an API key, which contradicts the zero-upload thesis. It is a different category (cloud tiling service vs. in-browser engine). See the architecture comparison below.
- py3dtiles is a Python CLI; wasm-spatial-core runs in WASM/Node. The comparison is **wall-clock end-to-end** on the same machine, not a microbenchmark of equivalent algorithms.
- RSS for py3dtiles is child-process max (coarse); wasm RSS is Node process RSS.

## bench/head-to-head/fixtures/canonical_500k.las

| Engine | Wall time | Peak RSS | Output size | Tiles | Speedup vs wasm |
|--------|----------|----------|-------------|-------|-----------------|
| loaders.gl `4.x` | 3ms | 150MB | 0 (N/A) | 0 | 0.03× (wasm slower) |
| py3dtiles `12.1.1` | 3.72s | 246MB | 7.9MB | 40 | **49.0×** ⚡ |
| wasm-spatial-core `0.10.0` | 76ms | 189MB | 6.0MB | 64 | baseline |

<details><summary>Notes</summary>

- **loaders.gl:** PARSE-ONLY. @loaders.gl/tiles not installed. loaders.gl reads LAS but has no shipped LAS→pnts/3D-Tiles writer; output_bytes=0 is the documented capability gap, not a timeout. Tile generation would require a custom writer or a separate tool.
- **py3dtiles:** py3dtiles convert, no reprojection (matches wasm no-op). point_count=-1 means not readable from tileset.json; byte-count parity is the verification axis. RSS is child-process max across all runs (coarse).
- **wasm-spatial-core:** parsePointCloudAuto → generateTileset (octree + pnts encode in one call)

</details>

## 🏗️ Architecture comparison (not timed)

| | wasm-spatial-core | py3dtiles | loaders.gl | Cesium ion |
|---|---|---|---|---|
| Runs where | Browser (WASM) or Node | Python process | Browser/Node | Cloud service |
| Data leaves machine | **No** | No (local) | No (local) | **Yes** (upload) |
| Install needed | `npm i` (1.2MB WASM) | Python + pip + deps | npm | Account + token |
| LAS → 3D Tiles | ✅ full pipeline | ✅ full pipeline | ❌ read only | ✅ (cloud) |
| Offline / air-gapped | ✅ | ✅ | ✅ | ❌ |
| Cesium ion quota | not consumed | not consumed | not consumed | consumed |

## 🔁 Reproduce

```bash
git clone https://github.com/reed-soul/wasm-spatial-core.git && cd wasm-spatial-core
wasm-pack build --target nodejs --release --out-dir pkg-node -- --features point-cloud
pip install py3dtiles
npm install @loaders.gl/las   # optional; documents the capability gap
node bench/head-to-head/run-wasm.mjs test-data/large/synthetic_500k.las > bench/head-to-head/results-wasm.jsonl
python3 bench/head-to-head/run-py3dtiles.py test-data/large/synthetic_500k.las > bench/head-to-head/results-py3dtiles.jsonl
node bench/head-to-head/run-loaders-gl.mjs test-data/large/synthetic_500k.las > bench/head-to-head/results-loaders-gl.jsonl
node bench/head-to-head/aggregate.mjs
# → opens benchmarks/index.md
```
