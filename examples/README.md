# Browser demos

## 在线地址（GitHub Pages）

部署成功后（见 [docs/DEMO_SITE.md](../docs/DEMO_SITE.md)）：

- 演示中心：https://reed-soul.github.io/wasm-spatial-core/examples/index.html
- 完整交互：https://reed-soul.github.io/wasm-spatial-core/examples/demo/index.html

## 本地运行

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

One-liner（与 GitHub Pages 相同目录结构）：

```bash
npm run demo
# 打开 http://127.0.0.1:8080/examples/index.html
```

仅本地快速调试（直接 serve 仓库根目录）：

```bash
npm run demo:dev
```

`examples/cesium-demo/` is a **pure JavaScript** canvas demo and does not load the WASM `pkg/`.
