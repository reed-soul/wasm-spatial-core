# Wave 3 — Issue Templates (Terrain Deformation)

Labels: `roadmap-v2`, `wave-3`, `engine`, `geotiff`.

---

## W3.1 — Polygon rasterization on heightfield

**Title:** `feat(terrain-edit): rasterize polygon mask on elevation grid`

### Proposal

- Input: `heights: &[f32], width, height, bounds, polygon: &[lng,lat,...]`
- Output: `mask: Vec<u8>` inside/outside per cell
- Use existing geo bounds mapping from GeoTIFF module

### Acceptance

- [x] Unit test on 16×16 grid with square polygon — `terrain_edit.rs::rasterize_polygon_mask` + golden test `test_golden_mask_4x4_fixture`
- [x] Edge cells consistent with ray casting — ray-cast impl in `rasterize_polygon_mask`

---

## W3.2 — Excavate (cut)

**Title:** `feat(terrain-edit): excavateInside polygon by depth or target elevation`

### Parameters

- `mode: ByDepth(f32) | ToElevation(f32)`
- Only mutate cells where mask == inside

### Acceptance

- [x] Golden raster compare on 32×32 fixture — `tests/terrain_edit_golden_test.rs::test_golden_excavate_32x32_fixture`
- [x] WASM export `excavateTerrain(...)` — `terrain_edit.rs::excavate_terrain` (WASM-bound)

---

## W3.3 — Flatten

**Title:** `feat(terrain-edit): flattenInside polygon to target height`

### Acceptance

- [x] Inside cells equal target ± epsilon — `flatten_inside` + golden `test_golden_flatten_4x4_fixture`
- [x] Outside cells unchanged — same golden test asserts outside preservation

---

## W3.4 — Fill (raise)

**Title:** `feat(terrain-edit): fillInside polygon to target height`

### Acceptance

- [x] Only raises cells below target; does not lower — `fill_inside` + golden `test_golden_fill_2x2`

---

## W3.5 — Edge feathering

**Title:** `feat(terrain-edit): blend ramp at polygon boundary (N cells)`

### Acceptance

- [x] No visible cliff at 1-cell-wide ramp on test grid — `terrain_edit.rs::feather_blend` (ramp blend at boundary)
- [x] `feather_cells` parameter documented — exposed on `flatten_polygon` / `excavate_terrain` WASM API

---

## W3.6 — Re-encode quantized-mesh pyramid

**Title:** `feat(terrain-edit): deformed heightfield → TerrainTilesetResult`

### Acceptance

- [x] Pipeline: parseGeotiff → deform → encodeTerrainTileset — `terrain_edit.rs::encode_deformed_terrain_tileset` (delegates to `geotiff.rs::encode_terrain_tileset_core`)
- [x] Integration test tileset.json valid JSON — `terrain_edit.rs::tests::test_encode_deformed_tileset_json` (parses JSON, checks `root`)
- [x] Cesium workflow demo optional toggle — `examples/point-cloud-cesium/` (loads `encodeDeformedTerrainTileset` output into Cesium); `examples/terrain-demo/` renders deformed grids via Three.js
- [x] Quantized-mesh bytes conform to CesiumGS/quantized-mesh-1.0 spec — 88-byte header (real f32 heights, bounding sphere, horizon point), zig-zag delta vertex encoding, high-water-mark index encoding; round-trip test in `tests/quantized_mesh_roundtrip_test.rs`
- [x] Tile URIs use `.terrain` extension (was `.cmpt`)

---

## W3.7 — Golden raster tests

**Title:** `test(terrain-edit): golden height grids for cut/flatten/fill`

### Acceptance

- [x] Checked-in small `tests/fixtures/terrain_edit/*.bin` or hex literals — inline hex literals in `tests/terrain_edit_golden_test.rs`
- [x] CI runs on all platforms — pure-Rust, no platform deps; runs under default `cargo test --all-features`
