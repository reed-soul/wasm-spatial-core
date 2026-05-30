# 在线演示站（Demo Site）

让用户在浏览器里直接试用 **wasm-spatial-core**，无需本地安装 Rust。

## 已有内容

| 资源 | 说明 |
|------|------|
| [README.md](../README.md) | 产品介绍、API 示例、从源码构建 |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | 开发环境与测试命令 |
| [examples/README.md](../examples/README.md) | 本地如何跑 demo |
| [PLAN.md](../PLAN.md) | 路线图 |
| [CHANGELOG.md](../CHANGELOG.md) | 版本变更 |

## 演示页面

| 页面 | 能力 |
|------|------|
| **演示中心** `examples/index.html` | 多 Tab：快速上手、坐标转换、GeoJSON 管线、点云说明、WASM vs JS 基准、中国城市群地图 |
| **完整交互** `examples/demo/index.html` | GeoJSON 解析 + GCJ-02 转换 + R-tree 空间索引 + 画布 |
| **性能对比** `bench/browser/index.html` | 百万级坐标 WASM vs 纯 JS |
| **Worker 多线程** `examples/worker-demo/` | 需 COOP/COEP（见下方说明） |
| **Cesium 风格画布** `examples/cesium-demo/` | 纯 JS 可视化（不加载 WASM） |

## GitHub Pages（推荐）

仓库 CI 在每次 **push 到 `master`** 时自动：

1. 构建 WASM → `pkg/`
2. 执行 `scripts/build-demo-site.sh` 生成 `_site/`
3. 部署到 GitHub Pages

### 首次启用（仓库设置，否则会 404）

站点发布在 **`gh-pages` 分支**（CI 自动更新）。

1. 打开 **https://github.com/reed-soul/wasm-spatial-core/settings/pages**
2. **Build and deployment** → Source: **Deploy from a branch**
3. Branch: **`gh-pages`**，目录: **`/ (root)`** → **Save**
4. 等待 1～3 分钟

### 临时镜像（未开 Pages 时也可试用）

jsDelivr 会同步 `gh-pages` 分支（适合先体验，域名不同）：

- 演示中心：https://cdn.jsdelivr.net/gh/reed-soul/wasm-spatial-core@gh-pages/examples/index.html

### 访问地址

项目站（仓库名 `wasm-spatial-core`）一般为：

| 页面 | URL |
|------|-----|
| 首页（跳转） | `https://<user>.github.io/wasm-spatial-core/` |
| **演示中心** | `https://<user>.github.io/wasm-spatial-core/examples/index.html` |
| 完整交互 demo | `https://<user>.github.io/wasm-spatial-core/examples/demo/index.html` |
| 性能基准 | `https://<user>.github.io/wasm-spatial-core/bench/browser/index.html` |

将 `<user>` 换为组织/用户名（本仓库为 `reed-soul`）。

本地预览与线上一致：

```bash
bash scripts/build-demo-site.sh
npx http-server _site -p 8080 -c-1
# http://127.0.0.1:8080/examples/index.html
```

## Vercel（可选）

根目录已包含 `vercel.json`：构建时安装 Rust + wasm-pack 并执行 `scripts/build-demo-site.sh`。

1. [vercel.com](https://vercel.com) 导入本 GitHub 仓库  
2. 使用默认配置（Framework: Other）  
3. 部署完成后访问分配的 `*.vercel.app` 域名  

说明：Vercel 构建需下载 Rust 工具链，首次部署较慢；日常更新可优先用 GitHub Pages。

**Worker 多线程 demo** 在 Vercel 上可通过 `vercel.json` 里对 `worker-demo` 的 COOP/COEP 头启用；GitHub Pages 默认不支持自定义响应头，该 demo 在 Pages 上可能受限。

## 本地开发（与线上路径一致）

```bash
npm run build:pkg
npx http-server . -p 8080 -c-1
```

打开 `http://127.0.0.1:8080/examples/index.html`（不要用错 `../pkg` 的旧扁平部署路径）。
