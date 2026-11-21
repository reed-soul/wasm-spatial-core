# Head-to-head: LAS → 3D Tiles

End-to-end wall-clock comparison: **wasm-spatial-core** vs **py3dtiles** vs
**loaders.gl** on the same LAS bytes. Methodology lives in
[`benchmarks/index.md`](../../benchmarks/index.md).

## Fixture

Do **not** commit the LAS. CI (and the reproduce steps) generate it:

```bash
mkdir -p bench/head-to-head/fixtures
cargo run --example gen_test_las --features point-cloud -- \
  500000 bench/head-to-head/fixtures/canonical_500k.las
```

## Run

```bash
wasm-pack build --target nodejs --release --out-dir pkg-node -- --features point-cloud
pip install 'py3dtiles[las]'   # optional locally; required in CI
npm install --no-save @loaders.gl/las   # optional; documents the capability gap

mkdir -p bench/head-to-head
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
```
