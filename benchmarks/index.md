# Head-to-head benchmark: wasm-spatial-core vs alternatives

> Task: **LAS point cloud → Cesium 3D Tiles (.pnts + tileset.json)**, end-to-end, no upload, no server.

<!-- HEAD_TO_HEAD_RESULTS_START -->

**Generated:** 2026-11-21T18:00:03.709Z  
**Hardware:** INTEL(R) XEON(R) PLATINUM 8573C · 4 CPUs · linux/x64  
**Methodology:** same input bytes, no reprojection, trimmed mean of 5 timed runs after warmup (min/max dropped). Output bytes = sum of generated tile files + tileset.json.

## bench/head-to-head/fixtures/canonical_500k.las

| Engine | Wall time | Peak RSS | Output size | Tiles | Speedup vs wasm |
|--------|----------|----------|-------------|-------|-----------------|
| loaders.gl `not-installed` | ❌ @loaders.gl/las not installed: @loaders.gl/las n | — | — | — | — |
| py3dtiles `--version` | 2.60s | 247MB | 7.6MB | 39 | **31.6×** ⚡ |
| wasm-spatial-core `0.11.0` | 82ms | 177MB | 5.7MB | 64 | baseline |

<details><summary>Notes</summary>

- **loaders.gl:** @loaders.gl not installed in this environment. Documented capability gap: @loaders.gl is a reader framework with no shipped LAS→pnts/3D-Tiles writer.
- **py3dtiles:** py3dtiles convert, no reprojection (matches wasm no-op). RSS is GNU time max RSS of the child (Linux) or unavailable.
- **wasm-spatial-core:** parsePointCloudAuto → generateTileset (octree + pnts encode in one call)

</details>

<!-- HEAD_TO_HEAD_RESULTS_END -->

## 🏗️ Architecture comparison (not timed)

| | wasm-spatial-core | py3dtiles | loaders.gl | Cesium ion |
|---|---|---|---|---|
| Runs where | Browser (WASM) or Node | Python process | Browser/Node | Cloud service |
| Data leaves machine | **No** | No (local) | No (local) | **Yes** (upload) |
| Install needed | `npm i` (1.95MB WASM) | Python + pip + deps | npm | Account + token |
| LAS → 3D Tiles | ✅ full pipeline | ✅ full pipeline | ❌ read only | ✅ (cloud) |
| Offline / air-gapped | ✅ | ✅ | ✅ | ❌ |
| Cesium ion quota | not consumed | not consumed | not consumed | consumed |

## 🔁 Reproduce

```bash
git clone https://github.com/reed-soul/wasm-spatial-core.git && cd wasm-spatial-core
mkdir -p bench/head-to-head/fixtures
cargo run --example gen_test_las --features point-cloud -- \
  500000 bench/head-to-head/fixtures/canonical_500k.las
wasm-pack build --target nodejs --release --out-dir pkg-node -- --features point-cloud
pip install 'py3dtiles[las]'
npm install --no-save @loaders.gl/las   # optional; documents the capability gap
WASM_PKG_DIR=pkg-node node bench/head-to-head/run-wasm.mjs \
  bench/head-to-head/fixtures/canonical_500k.las \
  > bench/head-to-head/results-wasm.jsonl
python3 bench/head-to-head/run-py3dtiles.py \
  bench/head-to-head/fixtures/canonical_500k.las \
  > bench/head-to-head/results-py3dtiles.jsonl
node bench/head-to-head/run-loaders-gl.mjs \
  bench/head-to-head/fixtures/canonical_500k.las \
  > bench/head-to-head/results-loaders-gl.jsonl
node bench/head-to-head/aggregate.mjs
# → updates the results table in benchmarks/index.md
```
