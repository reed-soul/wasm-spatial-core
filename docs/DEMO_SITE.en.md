# Demo Site

> **中文版:** [DEMO_SITE.md](./DEMO_SITE.md)

Lets users try **wasm-spatial-core** directly in the browser, with no local
Rust install required.

## Source material

| Resource | Contents |
|----------|----------|
| [README.md](../README.md) | Product intro, API examples, building from source |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | Dev environment and test commands |
| [examples/README.md](../examples/README.md) | How to run the demos locally |
| [ROADMAP_V2.md](../ROADMAP_V2.md) | Roadmap |
| [CHANGELOG.md](../CHANGELOG.md) | Version history |

## Demo pages

| Page | Capability |
|------|------------|
| **Demo hub** `examples/index.html` | Multi-tab: quick start, coordinate conversion, GeoJSON pipeline, **interactive 3D point-cloud preview** (WASM voxel decimation), WASM-vs-JS benchmark, China cities map |
| **Full interactive** `examples/demo/index.html` | GeoJSON parse + GCJ-02 conversion + R-tree spatial index + canvas |
| **Performance comparison** `bench/browser/index.html` | Million-scale coordinates, WASM vs pure JS |
| **Worker multi-threading** `examples/worker-demo/` | Requires COOP/COEP (see notes below) |
| **Three.js point cloud** `examples/point-cloud-demo/` | Drag LAS/PLY (LAZ needs `laz-support` build), orbit controls, octree tiling |
| **WebGL point cloud** `examples/webgl-pointcloud/` | Shared WebGL viewer module, WASM LAS parsing, terrain point cloud demo |
| **Cesium 3D Tiles point cloud** `examples/point-cloud-cesium/` | Full 3D Tiles pipeline on the globe |
| **Cesium styled canvas** `examples/cesium-demo/` | Pure-JS visualization (does not load WASM) |
| **Head-to-head benchmark** `/benchmarks/index.md` | wasm-spatial-core vs py3dtiles vs loaders.gl (auto-published by CI) |

## GitHub Pages (recommended)

The repo CI automatically, on every **push to `master`**:

1. Builds WASM → `pkg/`
2. Runs `scripts/build-demo-site.sh` to assemble `_site/`
3. Deploys to GitHub Pages

### First-time setup (repo settings — otherwise you get a 404)

The site is published from the **`gh-pages` branch** (CI keeps it updated).

1. Open **https://github.com/reed-soul/wasm-spatial-core/settings/pages**
2. **Build and deployment** → Source: **Deploy from a branch**
3. Branch: **`gh-pages`**, directory: **`/ (root)`** → **Save**
4. Wait 1–3 minutes

### Temporary mirror (try it even before Pages is enabled)

jsDelivr mirrors the `gh-pages` branch (handy for a first look; different
domain):

- Demo hub: https://cdn.jsdelivr.net/gh/reed-soul/wasm-spatial-core@gh-pages/examples/index.html

### URLs

For this repo (`wasm-spatial-core`):

| Page | URL |
|------|-----|
| Home (redirect) | `https://<user>.github.io/wasm-spatial-core/` |
| **Demo hub** | `https://<user>.github.io/wasm-spatial-core/examples/index.html` |
| Full interactive demo | `https://<user>.github.io/wasm-spatial-core/examples/demo/index.html` |
| Performance benchmark | `https://<user>.github.io/wasm-spatial-core/bench/browser/index.html` |
| Head-to-head benchmark | `https://<user>.github.io/wasm-spatial-core/benchmarks/index.md` |

Replace `<user>` with your org/username (this repo: `reed-soul`).

Local preview matches production paths exactly:

```bash
bash scripts/build-demo-site.sh
npx http-server _site -p 8080 -c-1
# http://127.0.0.1:8080/examples/index.html
```

## Vercel (optional)

The repo root includes `vercel.json`: on build it installs Rust + wasm-pack and
runs `scripts/build-demo-site.sh`.

1. Import this GitHub repo at [vercel.com](https://vercel.com)
2. Use the default config (Framework: Other)
3. After deploy, visit the assigned `*.vercel.app` domain

Note: Vercel builds download the Rust toolchain, so the first deploy is slow;
for routine updates prefer GitHub Pages.

**Worker multi-threading demo:** on Vercel you can enable COOP/COEP headers
for `worker-demo` via `vercel.json`; GitHub Pages does not support custom
response headers, so that demo may be limited on Pages.

## Local development (paths match production)

```bash
npm run build:pkg
npx http-server . -p 8080 -c-1
```

Open `http://127.0.0.1:8080/examples/index.html` (do not use the old flat
`../pkg` deploy path).
