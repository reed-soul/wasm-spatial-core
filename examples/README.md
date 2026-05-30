# Browser demos

## Online (GitHub Pages)

| Demo | URL |
|------|-----|
| Hub | https://reed-soul.github.io/wasm-spatial-core/examples/index.html |
| Interactive | https://reed-soul.github.io/wasm-spatial-core/examples/demo/index.html |

See [docs/DEMO_SITE.md](../docs/DEMO_SITE.md) for deployment notes.

## Local

Demos import WASM from `../pkg/`. **`pkg/` is not in git** — build it first:

```bash
npm run build:pkg
# or: wasm-pack build --target web --release --out-dir pkg
```

| Demo | URL (after `npm run demo`) |
|------|----------------------------|
| Hub | http://127.0.0.1:8080/examples/index.html |
| Interactive | http://127.0.0.1:8080/examples/demo/index.html |
| Worker (COOP/COEP) | http://127.0.0.1:8080/examples/worker-demo/index.html |

```bash
npm run demo      # same layout as GitHub Pages (_site)
npm run demo:dev  # serve repo root directly
```

`examples/cesium-demo/` is pure JavaScript and does not load the WASM `pkg/`.
