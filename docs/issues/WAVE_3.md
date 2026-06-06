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

- [ ] Unit test on 16×16 grid with square polygon
- [ ] Edge cells consistent with ray casting

---

## W3.2 — Excavate (cut)

**Title:** `feat(terrain-edit): excavateInside polygon by depth or target elevation`

### Parameters

- `mode: ByDepth(f32) | ToElevation(f32)`
- Only mutate cells where mask == inside

### Acceptance

- [ ] Golden raster compare on 32×32 fixture
- [ ] WASM export `excavateTerrain(...)`

---

## W3.3 — Flatten

**Title:** `feat(terrain-edit): flattenInside polygon to target height`

### Acceptance

- [ ] Inside cells equal target ± epsilon
- [ ] Outside cells unchanged

---

## W3.4 — Fill (raise)

**Title:** `feat(terrain-edit): fillInside polygon to target height`

### Acceptance

- [ ] Only raises cells below target; does not lower

---

## W3.5 — Edge feathering

**Title:** `feat(terrain-edit): blend ramp at polygon boundary (N cells)`

### Acceptance

- [ ] No visible cliff at 1-cell-wide ramp on test grid
- [ ] `feather_cells` parameter documented

---

## W3.6 — Re-encode quantized-mesh pyramid

**Title:** `feat(terrain-edit): deformed heightfield → TerrainTilesetResult`

### Acceptance

- [ ] Pipeline: parseGeotiff → deform → encodeTerrainTileset
- [ ] Integration test tileset.json valid JSON
- [ ] Cesium workflow demo optional toggle

---

## W3.7 — Golden raster tests

**Title:** `test(terrain-edit): golden height grids for cut/flatten/fill`

### Acceptance

- [ ] Checked-in small `tests/fixtures/terrain_edit/*.bin` or hex literals
- [ ] CI runs on all platforms
