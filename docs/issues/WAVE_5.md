# Wave 5 — Issue Templates (Advanced Mesh Edit)

Labels: `roadmap-v2`, `wave-5`, `engine`, `mesh-edit`.

**Depends on:** W2 (MeshChunk), W4 recommended for QEM at scale.

---

## W5.1 — OBB triangle classifier

**Title:** `feat(mesh-edit): classify triangles inside/outside OBB`

### Input

- `positions, indices`, `obb: Mat4` (box from -0.5..0.5 transformed)

### Output

- `inside_indices`, `outside_indices` (may share boundary triangles in phase 1)

### Acceptance

- [x] Unit cube vs axis-aligned OBB half-space test — `mesh_edit.rs::tests::test_unit_cube_obb_classify`
- [x] 100% correct on tetrahedron fixture — `test_tetrahedron_obb_classify_all_inside` + `..._all_outside`

---

## W5.2 — Mesh split phase 1 (no interpolation)

**Title:** `feat(mesh-edit): export inside/outside submeshes without plane cut`

### Acceptance

- [x] Two GLB outputs from single input — `mesh_edit.rs::tests::test_split_exports_glb_bytes` (insideGlb + outsideGlb)
- [x] Triangle count sum ≥ original (boundary duplication OK) — `test_split_mesh_by_obb_triangle_count`

---

## W5.3 — Plane clip phase 2

**Title:** `feat(mesh-edit): clip mesh by plane with attribute interpolation`

### Acceptance

- [x] New vertices on cut plane — `mesh_clip.rs::clip_mesh_by_plane` interpolates intersection vertices
- [x] UV and normal interpolated linearly — `clip_mesh_by_plane` attribute interpolation path
- [x] Golden mesh on single-triangle cut — covered by `mesh_clip.rs` unit tests + cap integration

---

## W5.4 — Cap holes (ear clipping)

**Title:** `feat(mesh-edit): cap planar boundary loops after clip`

### Acceptance

- [x] Closed box cut in half → caps produce watertight mesh (Euler check on fixture) — `mesh_cap.rs::tests::test_box_clip_and_cap_is_watertight` (Euler χ == 2)

---

## W5.5 — QEM edge collapse decimation

**Title:** `feat(mesh-edit): simplifyMeshQem target ratio`

### Proposal

- Garland–Heckbert quadrics in Rust (feature `mesh-edit`)
- `simplifyMesh(positions, indices, targetIndexCount) -> (pos, idx)`

### Acceptance

- [x] Bunny or public sample mesh 100k → 10k triangles — `tests/benchmark_suite.rs::benchmark_mesh_qem_100k_to_10k`: 99458 → 10000 tris
- [x] Benchmark in `tests/benchmark_suite.rs` — same test (JSON timing output)
- [x] Max error metric logged — measured max_error = 0.058650 (2026-06-27, native release)

---

## W5.6 — UV seam preservation

**Title:** `feat(mesh-edit): penalize QEM collapses across UV seams`

### Acceptance

- [x] Textured fixture does not collapse seam edges in test config — `mesh_qem.rs::tests::test_qem_preserves_uv_seam_vertices` + `test_uv_seam_collapse_blocked_on_coincident_vertices`

---

## W5.7 — GPU QEM

**Title:** `feat(webgpu): GPU quadric accumulation + edge cost kernels for QEM`

### Acceptance

- [x] WGSL `mesh_quadrics_v1` + `mesh_edge_costs_v1` (shader bundle `1.1.0`)
- [x] `GpuContext.accumulateQuadrics`, `evaluateEdgeCosts`, `simplifyMeshQem` with WASM fallback
- [x] CPU reference parity tests (`tests/qem_gpu_parity_test.rs`)
- [x] Hybrid GPU quadric/cost eval + CPU collapse loop (UV seam preservation on GPU path)
