# 🚀 V1.0 Roadmap — 点云→3D Tiles 浏览器端完整管线

> **目标**：拖一个 LAZ/LAS 文件到浏览器，30 秒内看到可交互的 3D 点云。
> 零上传、零服务器、零安装。
>
> 这是 wasm-spatial-core 从"大而全的空间工具库"转型为
> "浏览器端 3D 数据引擎"的核心战役。

---

## 为什么做这个

| 现有方案 | 问题 |
|---------|------|
| **Cesium ion** | 付费云服务，数据要上传（隐私 + 延迟 + 依赖网络） |
| **PDAL + py3dtiles** | 需要 Python + GDAL 环境，部署复杂 |
| **Potree** | 最流行的点云查看器，但大点云必须有后端 |
| **deck.gl** | 前端渲染，但假设数据已处理 |
| **纯 JS 方案** | LAZ 解压不可能，大文件八叉树太慢 |

**没有人做到"浏览器端单文件→3D Tiles"。这是真正的空白地带。**

---

## 技术架构

```
用户拖入 LAZ/LAS 文件
        │
        ▼
  ┌─────────────┐
  │ LAZ 解压引擎  │  ← LazPerf 算法移植到 Rust/WASM
  └──────┬──────┘
         │ 原始点数据 (x, y, z, [r,g,b])
         ▼
  ┌─────────────┐
  │  八叉树构建   │  ← 空间分区，内存感知
  └──────┬──────┘
         │ 8 叉树节点，每个节点包含点的子集
         ▼
  ┌─────────────┐
  │  LOD 裁剪    │  ← geometricError + 屏幕空间误差
  └──────┬──────┘
         │ 多级细节层级
         ▼
  ┌─────────────┐
  │ pnts 编码器   │  ← 3D Tiles Point Cloud 格式
  └──────┬──────┘  ← 支持 Draco 压缩（可选）
         │ 二进制 tile 数据
         ▼
  ┌─────────────┐
  │tileset.json  │  ← 树结构 + boundingVolume
  └──────┬──────┘
         │ 完整 tileset
         ▼
  ┌─────────────┐
  │ Cesium 渲染  │  ← 加载 tileset.json → 交互式 3D
  └─────────────┘
```

---

## Phase A — 点云核心管线（先做这个）

### A1: LAZ 解压引擎

**目标**: 在 WASM 中解压 LAZ 格式，让用户能加载真实的压缩点云数据。

**现状**: 当前只支持未压缩的 LAS（Format 0/2）。LAZ 是实际生产中最常用的格式（10x 压缩），不支持 LAZ 就等于不能用真实数据。

**技术方案**:
- LAZ 依赖 LazPerf 压缩算法，包含多层编码（point-wise + channel-wise）
- Rust 生态有 `las-rs` crate 支持 LAZ（通过 `laz-rs`）
- 但 `las-rs` 的 LAZ 支持依赖 native code，可能不编译到 wasm32
- **方案 A**: 移植 laz-perf 核心算法（~5000 行 C++），纯 Rust 重写
- **方案 B**: 尝试 `laz-rs` 在 wasm32 下编译，不行就方案 A
- **方案 C（折中）**: 先支持 LAS，提供 LAZ→LAS 转换工具（用户预处理），但主线追踪 LAZ 支持

**优先方案 C + 追踪方案 B**。先用 LAS 快速出 Demo，同时尝试让 laz-rs 编译到 wasm。

**产出**:
- `parseLaz(bytes) -> LasPointCloud` — 或 `parseLazStream(reader, progress)`
- 性能目标：100MB LAZ 解压 < 5 秒（WASM）

**测试**: 用标准 LAZ 测试文件（ASPRS 官方样本）验证解压正确性。

---

### A2: 流式点云加载器

**目标**: 支持 > 内存大小的点云文件，不需要一次性全部加载。

**技术方案**:
- Web Fetch API + Range requests 加载文件片段
- LAZ/LAS header 先解析（227 bytes），获取点数/格式/bounds
- 然后按需加载指定 offset 的点数据
- 与 COPC (Cloud Optimized Point Cloud) 概念对接：已知偏移量后按需加载

**产出**:
- `PointCloudStreamer` class
  - `new(file_or_url)` — 接受 File 对象或 URL
  - `header() -> LasHeader`
  - `readRegion(offset, count) -> Float32Array` — 按偏移读取
  - `readBounds(min_x, min_y, min_z, max_x, max_y, max_z) -> Float32Array` — 空间范围读取
- `totalPoints() -> u32`
- `onProgress(callback)` — 进度回调

---

### A3: 八叉树空间分区

**目标**: 将点云组织为八叉树，为 LOD 和空间查询提供基础。

**技术方案**:
- 经典八叉树：递归将空间 8 等分
- 每个节点存储：
  - bounding box [min_x, min_y, min_z, max_x, max_y, max_z]
  - 点索引范围（offset + count）
  - 子节点引用（8 个）
- 终止条件：
  - 节点内点数 < max_points_per_node（默认 50000）
  - 或达到最大深度（默认 21，即全球→1米精度）
- **内存优化**: 不复制点数据，只存索引范围，引用原始 buffer

**产出**:
- `Octree` class
  - `build(positions, max_points_per_node?, max_depth?) -> Octree`
  - `root() -> OctreeNode`
  - `nodeCount() -> u32`
  - `depth() -> u32`
- `OctreeNode`
  - `boundingBox() -> Float64Array`
  - `pointCount() -> u32`
  - `children() -> Array<OctreeNode>` (0 or 8)
  - `isLeaf() -> bool`
  - `level() -> u32`
- 性能目标：1 亿点八叉树构建 < 10 秒

---

### A4: pnts Tile 编码器

**目标**: 将八叉树节点编码为 3D Tiles Point Cloud (pnts) 格式。

**pnts 格式规范** (3D Tiles 1.0):
```
Byte Length | Description
          28 | Header (magic "pnts", version, byteLength, featureTableJSONByteLength,
               |  featureTableBinaryByteLength, batchTableJSONByteLength,
               |  batchTableBinaryByteLength)
   variable | Feature Table JSON: {"POSITION": {"byteOffset":0}, "RGB": {"byteOffset":12}, ...}
   variable | Feature Table Binary: raw position/color data (Float32 positions, Uint8 colors)
   variable | Batch Table JSON (optional)
   variable | Batch Table Binary (optional)
```

**技术方案**:
- Position 编码：Float32 (x, y, z) 相对于 tile center偏移（减少数值精度损失）
- Color 编码：Uint8 (r, g, b)
- 法线编码：可选，用 Oct16 编码（2 bytes → xyz 法线方向）
- **Draco 压缩**: 暂不实现（需要 draco-rs 编译到 wasm，复杂度太高）
  - 第一版先输出未压缩 pnts
  - Draco 作为 Phase B 的优化项

**产出**:
- `encodePntsTile(positions, center, [colors]) -> Uint8Array`
  - positions: Float32Array [x,y,z,...]
  - center: [cx, cy, cz] tile 中心（用于相对编码）
  - colors: 可选 Uint8Array [r,g,b,...]
  - 返回完整的 pnts 二进制数据

---

### A5: tileset.json 生成器

**目标**: 从八叉树生成完整的 3D Tiles tileset 树结构。

**tileset.json 结构**:
```json
{
  "asset": { "version": "1.0" },
  "geometricError": 500,
  "root": {
    "boundingVolume": { "region": [...] },
    "geometricError": 500,
    "refine": "ADD",
    "content": { "uri": "root.pnts" },
    "children": [
      {
        "boundingVolume": { "region": [...] },
        "geometricError": 250,
        "refine": "ADD",
        "content": { "uri": "child_0.pnts" },
        "children": [...]
      }
    ]
  }
}
```

**技术方案**:
- 从八叉树根开始递归生成
- 每个节点 = 一个 tile
- geometricError = 节点包围盒对角线 / 层级因子
- boundingVolume 用 box 或 region
- refine 策略：ADD（叠加细化）
- 输出格式：一个 JSON (tileset.json) + 多个二进制 pnts

**产出**:
- `generateTileset(octree, positions, [colors]) -> TilesetResult`
  - `TilesetResult`
    - `tilesetJson() -> String` — tileset.json 内容
    - `tile(index) -> Uint8Array` — 获取指定 tile 的二进制数据
    - `tileCount() -> u32`
    - `tileUri(index) -> String` — tile 的 URI（如 "0.pnts"）
    - `totalBytes() -> usize` — 所有 tile 总大小

---

### A6: Demo — 拖拽→Cesium 渲染

**目标**: 一个 HTML 页面，拖 LAZ 文件进去就能看到 3D 点云。

**技术方案**:
1. 拖拽区域接收 File 对象
2. WASM 解析 LAS/LAZ
3. 构建八叉树
4. 生成 tileset（内存中）
5. 创建 Blob URLs 给每个 pnts tile
6. Cesium.C3DTileset 加载 tileset.json
7. 地球上显示点云

**Demo 特性**:
- 拖拽上传区
- 解析进度条
- 点数/文件大小显示
- LOD 级别显示
- 飞行到点云位置（自动）
- 颜色模式切换（原始色/高度着色/强度着色）

**文件**: `examples/point-cloud-demo/index.html`

---

## Phase B — LOD 优化 + 性能（Phase A 完成后）

### B1: 视点驱动的动态加载
- Cesium 的 camera 变化 → 计算当前视锥 → 只加载可见 tiles
- 内存管理：卸载远离视点的 tiles
- 需要与 Cesium 的 tile loading 机制对接

### B2: 几何误差自动校准
- 根据点间距计算 geometricError
- 确保视觉无缝过渡（没有明显的"弹出"）

### B3: WebWorker 并行处理
- 八叉树构建放到 Worker
- pnts 编码放到 Worker
- 主线程只负责 UI 和 Cesium 渲染

### B4: Draco 压缩（可选）
- 尝试 `draco-rs` 编译到 wasm32
- 如果不行，暂时跳过（pnts 不压缩也能用）

---

## Phase C — 扩展数据源

### C1: E57 格式
- 建筑扫描主流格式
- 二进制格式 + XML 元数据
- 比 LAS 更复杂

### C2: PLY/OBJ
- 通用点云/网格格式
- 相对简单

### C3: COPC 完整支持
- Cloud Optimized Point Cloud
- LAZ + COPC header
- 已有基础，需要完善

---

## 实施顺序

```
Week 1:
  ├── A1 (LAZ) — 尝试 laz-rs 编译 wasm，不行就走 LAS-only + 转换工具
  ├── A3 (八叉树) — 核心数据结构
  └── A4 (pnts) — 编码器

Week 2:
  ├── A5 (tileset.json) — 从八叉树生成
  ├── A2 (流式加载) — 如果需要
  └── A6 (Demo) — 端到端打通

Week 3:
  ├── B1-B4 — 性能优化
  └── 文档 + README 重写
```

---

## 成功标准

1. **拖一个 100MB LAS 文件到浏览器 → 30 秒内看到 3D 点云**
2. **1 亿点（~2GB LAS）的文件能在 5 分钟内处理完毕并可交互**
3. **Demo 在 GitHub Pages 上可以在线体验**（不需要本地部署）
4. **npm install wasm-spatial-core → 3 行代码就能用**

做到这四点，这个项目就不再是"锦上添花"，而是"浏览器端点云处理的唯一选择"。
