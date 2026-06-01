# WebGL Point Cloud Viewer

A lightweight, zero-dependency point cloud viewer powered by native WebGL. No Cesium, no Three.js, no Potree — just raw WebGL and WASM-accelerated point cloud processing.

## Features

- **Native WebGL rendering** — Custom vertex/fragment shaders with circular points and Eye-Dome Lighting
- **Adaptive point size** — Distance-based sizing: closer points are smaller, distant points are larger
- **Trackball camera** — Left-drag rotate, right-drag pan, scroll zoom, double-click reset
- **WASM integration** — Loads `wasm-spatial-core` for high-performance LAS/LAZ parsing (with JS fallback)
- **Color modes** — Original colors, height gradient, ASPRS classification, density heatmap
- **Point size control** — Real-time adjustable base point size
- **FPS counter** — Live rendering performance monitoring
- **Touch support** — Pinch-to-zoom and touch drag on mobile

## Supported Formats

| Format | Extension | WASM Required |
|--------|-----------|---------------|
| LAS    | .las      | No (JS fallback) |
| LAZ    | .laz      | Yes |
| PLY    | .ply      | No |
| XYZ    | .xyz      | No |

## Try It

Open [examples/webgl-pointcloud/index.html](../examples/webgl-pointcloud/index.html) in a browser, or try the [demo on GitHub Pages](https://reed-soul.github.io/wasm-spatial-core/webgl-pointcloud/).

## Architecture

```
File Input → Format Detection → Parse (WASM/JS) → Center → WebGL Buffer → Render Loop
                                                          ↓
                                              Color Mode Compute
```

### WebGL Pipeline

1. **Vertex Shader**: Receives position + color attributes, applies MVP matrix, computes distance-adaptive point size
2. **Fragment Shader**: Circular point shape (discards non-circular pixels), EDL edge darkening, anti-aliased edges
3. **Matrix Math**: Hand-written (no gl-matrix dependency) — perspective, lookAt, multiply, translate, rotateX/Y
4. **LOD**: Simplified distance-based point size (no full 3D Tiles LOD — keeps it lightweight)

## Integration with wasm-spatial-core

```js
import init, { parsePointCloudAuto, pointCloudStats }
  from 'wasm-spatial-core';

await init();

const cloud = parsePointCloudAuto(lasBytes);
const stats = JSON.parse(pointCloudStats(cloud.positions()));

// cloud.positions() → Float32Array → WebGL buffer
// cloud.colors()    → Uint8Array   → WebGL buffer
```

The viewer includes a complete JS fallback parser (LAS format 0/2) so it works even without the WASM module loaded.
