# Wave 4 — Issue Templates (WebGPU Compute Core)

Labels: `roadmap-v2`, `wave-4`, `engine`, `webgpu`.

**Prerequisite:** Latest Chrome with `navigator.gpu`.

**Out of scope:** frustum culling — Cesium/Three handle visibility.

---

## W4.1 — GpuContext bootstrap

**Title:** `feat(webgpu): GpuContext init from navigator.gpu`

### Acceptance

- [ ] Returns null gracefully when GPU unavailable
- [ ] Demo `examples/webgpu-smoke/` (minimal compute)
- [ ] Feature flag `webgpu`, default off

---

## W4.2 — Buffer layout contract

**Title:** `feat(webgpu): document GPU buffer layouts for positions/indices/heights`

### Acceptance

- [ ] Shared layout doc + TS types
- [ ] Minimize CPU↔GPU copies where possible

---

## W4.3 — Point transform compute kernel

**Title:** `feat(webgpu): WGSL Mat4 × vec3 batch transform`

### Acceptance

- [ ] 10M points: GPU faster than WASM SIMD on discrete GPU (benchmark)
- [ ] Matches CPU reference on 1k points

---

## W4.4 — Heightfield deform kernel

**Title:** `feat(webgpu): parallel flatten/cut on GPU grid`

### Acceptance

- [ ] Matches W3 CPU on 512×512 grid
- [ ] 2048×2048 faster than WASM-only

---

## W4.5 — WASM fallback policy

**Title:** `feat(webgpu): unified API with automatic WASM fallback`

### Acceptance

- [ ] Same API with `preferGpu: false` forces CPU path (test)

---

## W4.6 — WGSL shader versioning

**Title:** `chore(webgpu): versioned shaders/ + subgroup feature detection`

### Acceptance

- [ ] `shaders/` directory with README
- [ ] Runtime checks `adapter.features` before subgroups
