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

- [ ] Unit cube vs axis-aligned OBB half-space test
- [ ] 100% correct on tetrahedron fixture

---

## W5.2 — Mesh split phase 1 (no interpolation)

**Title:** `feat(mesh-edit): export inside/outside submeshes without plane cut`

### Acceptance

- [ ] Two GLB outputs from single input
- [ ] Triangle count sum ≥ original (boundary duplication OK)

---

## W5.3 — Plane clip phase 2

**Title:** `feat(mesh-edit): clip mesh by plane with attribute interpolation`

### Acceptance

- [ ] New vertices on cut plane
- [ ] UV and normal interpolated linearly
- [ ] Golden mesh on single-triangle cut

---

## W5.4 — Cap holes (ear clipping)

**Title:** `feat(mesh-edit): cap planar boundary loops after clip`

### Acceptance

- [ ] Closed box cut in half → caps produce watertight mesh (Euler check on fixture)

---

## W5.5 — QEM edge collapse decimation

**Title:** `feat(mesh-edit): simplifyMeshQem target ratio`

### Proposal

- Garland–Heckbert quadrics in Rust (feature `mesh-edit`)
- `simplifyMesh(positions, indices, targetIndexCount) -> (pos, idx)`

### Acceptance

- [ ] Bunny or public sample mesh 100k → 10k triangles
- [ ] Benchmark in `tests/benchmark_suite.rs`
- [ ] Max error metric logged

---

## W5.6 — UV seam preservation

**Title:** `feat(mesh-edit): penalize QEM collapses across UV seams`

### Acceptance

- [ ] Textured fixture does not collapse seam edges in test config

---

## W5.7 — GPU-accelerated QEM (optional)

**Title:** `feat(webgpu): GPU assist for QEM on meshes > 1M triangles`

### Acceptance

- [ ] Behind `webgpu` + `mesh-edit`
- [ ] Document when CPU path is still default
