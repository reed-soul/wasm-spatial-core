# Browser demos

Demos import WASM from `../pkg/` (repo root). **`pkg/` is not in git** — build it first:

```bash
# from repository root
npm run build:pkg
# or: wasm-pack build --target web --release --out-dir pkg
```

Serve the repo root and open:

| Demo | URL |
|------|-----|
| Hub | http://127.0.0.1:8080/examples/index.html |
| Interactive (GeoJSON + CRS + R-tree) | http://127.0.0.1:8080/examples/demo/index.html |
| Worker / multi-thread (needs COOP/COEP) | http://127.0.0.1:8080/examples/worker-demo/index.html |

One-liner from repo root:

```bash
npm run demo
```

`examples/cesium-demo/` is a **pure JavaScript** canvas demo and does not load the WASM `pkg/`.
