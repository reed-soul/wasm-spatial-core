# Wave 4 — Issue Templates (WebGPU Compute Core)

Labels: `roadmap-v2`, `wave-4`, `engine`, `webgpu`.

**Prerequisite:** Latest Chrome with `navigator.gpu` (no Safari/Firefox requirement).

---

## W4.1 — GpuContext bootstrap

**Title:** `feat(webgpu): GpuContext init from navigator.gpu`

### Proposal

- TS: `createGpuContext() -> { device, queue } | null`
- Rust/WASM optional: only orchestration first; shaders in WGSL strings or `include_str!`
- Feature flag `webgpu` — default off

### Acceptance

- [ ] Returns null gracefully when GPU unavailable
- [ ] Demo page `examples/webgpu-smoke/` (minimal compute)

---

## W4.2 — Buffer layout contract

**Title:** `feat(webgpu): document GPU buffer layouts for positions/indices/heights`

### Proposal

- Shared struct layout doc + TS types
- `GpuBufferPool` reuse allocations

### Acceptance

- [ ] No CPU copy when passing same ArrayBuffer to WASM and GPU (where possible)

---

## W4.3 — Point transform compute kernel

**Title:** `feat(webgpu): WGSL Mat4 × vec3 batch transform`

### Acceptance

- [ ] Benchmark: 10M points GPU vs WASM SIMD — GPU wins on discrete GPU
- [ ] Correctness vs CPU reference on 1k points

---

## W4.4 — Heightfield deform kernel

**Title:** `feat(webgpu): parallel flatten/cut on GPU grid`

### Acceptance

- [ ] Matches W3 CPU result on 512×512 grid
- [ ] 2048×2048 faster than WASM-only path

---

## W4.5 — Frustum cull kernel

**Title:** `feat(webgpu): AABB vs frustum → visible instance indices`

### Acceptance

- [ ] Correct subset on synthetic 1000 AABBs
- [ ] Feeds W1 InstanceLayer culling path

---

## W4.6 — WASM fallback policy

**Title:** `feat(webgpu): unified API with automatic WASM fallback`

### Proposal

- `transformPointsAuto(buffer, matrix, { preferGpu: true })`
- Same signature whether GPU runs or not

### Acceptance

- [ ] Unit test forces CPU path when `preferGpu: false`

---

## W4.7 — WGSL shader versioning

**Title:** `chore(webgpu): versioned shaders/ directory + subgroup feature detection`

### Acceptance

- [ ] Shaders in `shaders/` with README
- [ ] Runtime checks `adapter.features` before using subgroups
