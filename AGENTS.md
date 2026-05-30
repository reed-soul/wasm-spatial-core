# AGENTS.md

Guidance for AI coding agents working in this repository.

## Cursor Cloud specific instructions

### Product

**wasm-spatial-core** is a Rust → WebAssembly spatial engine (GeoJSON, CRS transforms, R-tree, MVT, point clouds, etc.). There is no backend service: CI and most development use **Cargo + wasm-pack** only. Browser demos need a **static HTTP server** after building `pkg/`.

### Toolchain (one-time on a fresh VM)

The crate depends on **Rust stable ≥ 1.90** (transitive `mvt` needs a recent Cargo). If `cargo clippy` fails with `edition2024` / `mvt` parse errors, run `rustup default stable` and `rustup update stable`.

```bash
rustup default stable
rustup target add wasm32-unknown-unknown
rustup component add clippy rustfmt
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

Optional: Node.js ≥ 18 (benchmarks, `npm/` wrapper). Optional nightly only for `multi-thread` WASM builds (see `CONTRIBUTING.md`).

### CI parity (lint / test / build)

Match `.github/workflows/ci.yml`:

| Step | Command |
|------|---------|
| Format | `cargo fmt --all -- --check` |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` |
| Test | `cargo test --verbose` |
| WASM | `wasm-pack build --target web --release --out-dir pkg` |

Stress tests: `cargo test -- --ignored`. Browser WASM tests (`wasm-pack test --headless chrome`) exist but are **not** in CI.

**Note:** As of setup, `src/spatial_analysis.rs` may fail `cargo fmt --check` until formatted; clippy and tests pass on stable 1.96+.

### Running browser demos

1. Build WASM: `wasm-pack build --target web --release --out-dir pkg`
2. Serve repo root (WASM MIME + ES modules):  
   `npx http-server /workspace -p 8080 -c-1`
3. Open `http://127.0.0.1:8080/examples/demo/index.html` and click **Run Analysis** (WGS-84 → GCJ-02 on sample China GeoJSON).

Worker / SharedArrayBuffer demo (`examples/worker-demo/`) needs COOP/COEP:  
`npx http-server . --cors --coop --coep`

### Node smoke test (no browser)

```bash
wasm-pack build --target nodejs --release --out-dir pkg-node
node -e "const w=require('./pkg-node/wasm_spatial_core.js'); console.log(w.version()); console.log(w.batchWgs84ToGcj02(new Float64Array([116.397,39.909])))"
```

Node `pkg-node` loads WASM synchronously (CommonJS `exports`); do not call `init()`.

### npm wrapper

`cd npm && npm install && npm run build` — copies/bundles from `pkg/` per `npm/package.json`.

### Gotchas

- Demos import `../../pkg/wasm_spatial_core.js` relative to `examples/`; rebuild `pkg/` after Rust API changes.
- `examples/cesium-demo/` is pure JS canvas; it does not load the WASM `pkg/`.
- Prebuilt `pkg/` in the tree may be stale; prefer a fresh `wasm-pack build` when testing changes.
