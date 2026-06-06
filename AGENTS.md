# AGENTS.md

Guidance for AI coding agents working in this repository.

## Cursor Cloud specific instructions

### Product

**wasm-spatial-core** is a Rust → WebAssembly spatial engine. No backend: **Cargo + wasm-pack** for CI; static HTTP for browser demos.

**Vision & V2 roadmap:** [VISION.md](./VISION.md) · [ROADMAP_V2.md](./ROADMAP_V2.md) · [docs/ENGINE_BOUNDARY.md](./docs/ENGINE_BOUNDARY.md). V1 is complete ([ROADMAP_V1.md](./ROADMAP_V1.md)). New work must be **engine core** (formats, IR, geometry, tiles, GPU) — no product plugins (parking, trajectories, geofence, timelines). Default start: **W2 Spatial IR**.

### Toolchain

Rust stable **≥ 1.90** (`rust-version` in `Cargo.toml`). If `mvt` / `edition2024` errors appear, run `rustup default stable && rustup update stable`.

```bash
rustup target add wasm32-unknown-unknown
rustup component add clippy rustfmt
# wasm-pack: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

### CI parity

| Step | Command |
|------|---------|
| Format | `cargo fmt --all -- --check` |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` |
| Test | `cargo test --verbose` |
| WASM build | `wasm-pack build --target web --release --out-dir pkg` |
| WASM bindgen tests | `wasm-pack test --node --release -- --test web` |

### `pkg/` directory

**Not in git.** Run `wasm-pack build --target web --release --out-dir pkg` before `examples/*` demos. CI uploads `pkg/` as an artifact for Pages.

### Browser demos

```bash
npm run demo
# or: npm run build:pkg && npx http-server . -p 8080 -c-1
# http://127.0.0.1:8080/examples/demo/index.html → Run Analysis
```

See `examples/README.md`.

Worker demo needs COOP/COEP headers (see `CONTRIBUTING.md`).

### Node smoke test

```bash
wasm-pack build --target nodejs --release --out-dir pkg-node
node -e "const w=require('./pkg-node/wasm_spatial_core.js'); console.log(w.version());"
```

### Cancellable pipelines (W1)

Long-running WASM jobs accept an optional `shouldAbort: () => boolean` callback (or JS `Function` returning truthy). When aborted, APIs return `SpatialError::Cancelled` (`code === 'CANCELLED'`).

```js
import { createAbortChecker, linkAbortSignalToWorker } from 'wasm-spatial-core/abort';

const controller = new AbortController();
const shouldAbort = createAbortChecker(controller.signal);
linkAbortSignalToWorker(worker, controller.signal);

// WASM: parseLasPointsWithProgressAndAbort(bytes, onProgress, shouldAbort)
// Worker: worker.cancel() or controller.abort()
```

Incremental tile edits: build `TilesetPatch`, call `setTile(uri, bytes)`, then `applyTilesetPatch(base, patch)` — unrelated tile URIs are preserved.

### Gotchas

- Default git branch is **`master`** (CI watches `master`, not `main`).
- `examples/cesium-demo/` does not load WASM.
- Prefer `SpatialError` for new APIs; some modules still use `JsValue::from_str` (see `CONTRIBUTING.md`).
