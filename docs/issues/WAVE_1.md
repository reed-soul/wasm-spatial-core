# Wave 1 — Issue Templates (Scene Instance Layer)

Copy a section below into a new GitHub issue. Labels: `roadmap-v2`, `wave-1`, `engine`.

**Scope:** Fixed or slot-based twins (parking, chargers, equipment) + incremental scene updates.  
**Out of scope for the engine:** inspection path replay, trajectory buffers, geofencing — handle in the upper application (Cesium/Three polylines + your MQTT layer).

---

## W1.1 — InstanceLayer API

**Title:** `feat(live-twin): InstanceLayer API over i3dm semantics`

### Problem

i3dm encoding exists but there is no runtime-friendly API for hundreds/thousands of instanced objects (vehicles, chargers, equipment) with stable IDs.

### Proposal

- `InstanceLayer.create({ templateGlb, slots: Slot[] })`
- `Slot { id, transform: Float32Array(16), visible, userData? }`
- Serialize to i3dm tileset OR hold mutable buffer for hot updates

### Acceptance

- [ ] Create layer with 100 slots from unit test GLB
- [ ] Export tileset.json + i3dm blob loadable in Cesium workflow demo
- [ ] TypeScript types in `npm/index.ts`

### Notes

- Build on `encode_i3dm_tile`, `create_instanced_tileset_i3dm`
- No UI in engine

---

## W1.2 — In-place instance pose update

**Title:** `feat(live-twin): updateInstance(id, matrix) without full tileset rebuild`

### Problem

Moving one object today implies regenerating tiles — unusable for real-time twins.

### Proposal

- Maintain slot index → transform map
- `updateInstance(id, columnMajorMat4)` updates internal buffer
- `flush()` optionally re-encodes i3dm or emits binary patch

### Acceptance

- [ ] 1000 sequential updates < 16 ms total on M2 (benchmark test)
- [ ] No memory leak across 10k updates (audit test)

---

## W1.3 — Occupancy and visibility flags

**Title:** `feat(live-twin): setVisible / setOccupied for instance slots`

### Problem

Parking-style twins need “slot occupied → show car model” without duplicate geometry.

### Proposal

- `setVisible(instanceId, bool)`
- `setOccupied(slotId, bool)` — shorthand: visible + optional scale 0

### Acceptance

- [ ] Unit test: 20 slots, 10 occupied → export reflects 10 visible instances
- [ ] Document pattern in VISION.md / npm README

---

## W1.4 — Tile patch protocol

**Title:** `feat(tiles): incremental tileset patch instead of full rebuild`

### Problem

Small scene edits invalidate entire tileset.json + all pnts blobs.

### Proposal

- `TilesetPatch { replacedTiles: Map<uri, bytes>, jsonDiff? }`
- `applyPatch(tileset, patch) -> new TilesetResult`
- Document URI stability rules

### Acceptance

- [ ] Integration test: add one leaf tile without changing unrelated URIs
- [ ] Patch size << full tileset for single-tile edit

---

## W1.5 — AbortSignal through WASM jobs

**Title:** `feat(runtime): cancellable long-running WASM/Worker pipelines`

### Problem

Large parses cannot be cancelled — bad UX and wasted memory.

### Proposal

- Worker + WASM check `abort` flag between chunks
- `parseLasPointsWithProgress` and tileset generation accept optional cancel token
- JS: standard `AbortSignal` adapter

### Acceptance

- [ ] Abort mid-parse on 10M synthetic LAS → returns promptly
- [ ] No UB / panic on abort path
