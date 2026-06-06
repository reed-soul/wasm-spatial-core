# Wave 1 — Issue Templates (Core Runtime & Incremental Output)

Labels: `roadmap-v2`, `wave-1`, `engine`.

**Scope:** Infrastructure every pipeline needs — not product-specific “twin” APIs.

---

## W1.1 — Tile patch protocol

**Title:** `feat(tiles): incremental tileset patch instead of full rebuild`

### Problem

Small scene edits invalidate entire `tileset.json` + all tile blobs.

### Proposal

- `TilesetPatch { replacedTiles: Map<uri, bytes>, jsonDiff? }`
- `applyPatch(tileset, patch) -> TilesetResult`
- Document URI stability rules

### Acceptance

- [ ] Integration test: replace one leaf tile without changing unrelated URIs
- [ ] Patch size ≪ full tileset for single-tile edit

---

## W1.2 — AbortSignal through WASM jobs

**Title:** `feat(runtime): cancellable long-running WASM/Worker pipelines`

### Problem

Large parses and tile generation cannot be cancelled.

### Proposal

- Worker + WASM check abort flag between chunks
- `parseLasPointsWithProgress`, `generateTileset`, terrain encode honour optional cancel token
- JS adapter for standard `AbortSignal`

### Acceptance

- [ ] Abort mid-parse on large synthetic LAS → prompt return, no panic
- [ ] Document pattern in AGENTS.md

---

## W1.3 — Memory arena / job budget (optional)

**Title:** `feat(runtime): reusable buffer arena and per-job byte budget`

### Problem

Repeated allocations in multi-step pipelines cause WASM memory growth.

### Proposal

- `ProcessingContext` with `reserve(cap)` and reuse across steps in one job
- `estimateJobBytes(op, inputMeta) -> usize` for UI warnings

### Acceptance

- [ ] Octree + tileset job reuses single position buffer where safe
- [ ] Estimate within 2× of actual for LAS parse benchmark

**Note:** Defer if W1.1–W1.2 ship first; not blocking IR work.
