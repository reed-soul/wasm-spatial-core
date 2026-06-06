# Wave 2 — Issue Templates (Spatial IR + GLB Ingest)

Labels: `roadmap-v2`, `wave-2`, `engine`.

---

## W2.1 — SpatialChunk IR enum

**Title:** `feat(ir): SpatialChunk unified internal representation`

### Proposal

```rust
enum SpatialChunk {
    PointCloud(PointCloudChunk),
    Mesh(MeshChunk),
    Heightfield(HeightfieldChunk),
}
```

Instance transforms for **export** use existing i3dm encoders — not a separate IR variant.

Each variant: `metadata: ChunkMeta`, `data: ...`, `version: u64`.

### Acceptance

- [ ] Native unit tests construct each variant
- [ ] `ChunkMeta` includes CRS id, AABB, byte_size estimate
- [ ] Feature flag `mesh-ingest` gates mesh variants

---

## W2.2 — Chunk metadata and versioning

**Title:** `feat(ir): ChunkMeta CRS, AABB, version, provenance`

### Acceptance

- [ ] JSON serialize metadata for debugging (not hot path)
- [ ] Version increments on edit operations (stub mutators OK)

---

## W2.3 — GLB/glTF reader

**Title:** `feat(mesh-ingest): parse GLB into MeshChunk`

### Proposal

- Mirror subset of `gltf_writer` schema (TRIANGLES, POSITION, NORMAL, TEXCOORD_0)
- `parseGlb(bytes) -> MeshChunk`
- Reject unsupported extensions gracefully

### Acceptance

- [ ] Round-trip: `meshToGlb` → `parseGlb` preserves vertex/index counts
- [ ] WASM export `parseGlb`
- [ ] Max size respects `get_current_input_limit()`

---

## W2.4 — Region select

**Title:** `feat(ir): selectByAabb / selectByPolygon on MeshChunk and PointCloudChunk`

### Acceptance

- [ ] AABB select returns new chunk with subset triangles/points
- [ ] Empty selection returns error `SpatialError`

---

## W2.5 — Chunk export paths

**Title:** `feat(ir): export MeshChunk/PointCloudChunk to glTF and 3D Tiles subset`

### Acceptance

- [ ] Selected mesh → standalone GLB bytes
- [ ] Selected points → pnts tile + minimal tileset.json

---

## W2.6 — ENU / local tangent frame

**Title:** `feat(crs): local ENU frame from anchor lng/lat/alt`

### Proposal

- `createEnuFrame(anchor: [lng, lat, alt]) -> EnuFrame`
- `wgs84ToEnu(coords, frame)` / `enuToWgs84`
- f64 precision for geo; f32 for rendering offsets relative to anchor

### Acceptance

- [ ] Round-trip error < 1 mm at 1 km from anchor (test)
- [ ] WASM exports

---

## W2.7 — SVD 3D alignment (backlog — not core path)

**Status:** Deferred. File only if photogrammetry↔GIS registration blocks real users.

Use per-point CRS transforms + ENU (W2.6) for the common case.
