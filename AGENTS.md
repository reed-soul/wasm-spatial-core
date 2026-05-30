# AGENTS.md

Guidance for AI coding agents working in this repository.

## Cursor Cloud specific instructions

### Product

**wasm-spatial-core** is a Rust → WebAssembly spatial engine. No backend: **Cargo + wasm-pack** for CI; static HTTP for browser demos.

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

### Gotchas

- Default git branch is **`master`** (CI watches `master`, not `main`).
- `examples/cesium-demo/` does not load WASM.
- Prefer `SpatialError` for new APIs; some modules still use `JsValue::from_str` (see `CONTRIBUTING.md`).
