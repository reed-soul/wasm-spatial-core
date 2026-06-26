# Wave 4 — Issue Templates (WebGPU Compute Core)

Labels: `roadmap-v2`, `wave-4`, `engine`, `webgpu`.

**Prerequisite:** Latest Chrome with `navigator.gpu`.

**Out of scope:** frustum culling — Cesium/Three handle visibility.

---

## W4.1 — GpuContext bootstrap

**Title:** `feat(webgpu): GpuContext init from navigator.gpu`

### Acceptance

- [x] Returns null gracefully when GPU unavailable — `npm/webgpu.ts` GpuContext handles missing `navigator.gpu`; `src/webgpu.rs::supports_webgpu`
- [x] Demo `examples/webgpu-smoke/` (minimal compute) — `app.mjs` + `kernels.mjs` + `index.html`
- [x] Feature flag `webgpu`, default off — `Cargo.toml` feature `webgpu = ["point-cloud"]`, not in `default`

---

## W4.2 — Buffer layout contract

**Title:** `feat(webgpu): document GPU buffer layouts for positions/indices/heights`

### Acceptance

- [x] Shared layout doc + TS types — `src/webgpu.rs` layout constants + `shaders/README.md`; `tests/webgpu_layout_test.rs::test_gpu_layout_matches_npm_constants` asserts WASM↔npm parity
- [x] Minimize CPU↔GPU copies where possible — storage-buffer in/out bindings in WGSL kernels

---

## W4.3 — Point transform compute kernel

**Title:** `feat(webgpu): WGSL Mat4 × vec3 batch transform`

### Acceptance

- [ ] 10M points: GPU faster than WASM SIMD on discrete GPU (benchmark) — **open**: requires discrete GPU + Chrome to measure; kernel exists in `shaders/transform_points_v1.wgsl`
- [x] Matches CPU reference on 1k points — `tests/webgpu_layout_test.rs::test_transform_cpu_reference_for_gpu_parity`

---

## W4.4 — Heightfield deform kernel

**Title:** `feat(webgpu): parallel flatten/cut on GPU grid`

### Acceptance

- [x] Matches W3 CPU on 512×512 grid — `tests/heightfield_flatten_parity_test.rs` (CPU reference == `flatten_inside`)
- [ ] 2048×2048 faster than WASM-only — **open**: requires discrete GPU + Chrome to measure; CPU path measured ~40 ms (W3 gate)

---

## W4.5 — WASM fallback policy

**Title:** `feat(webgpu): unified API with automatic WASM fallback`

### Acceptance

- [x] Same API with `preferGpu: false` forces CPU path (test) — `tests/heightfield_gpu_fallback_test.mjs` + `npm/webgpu.ts` fallback branch

---

## W4.6 — WGSL shader versioning

**Title:** `chore(webgpu): versioned shaders/ + subgroup feature detection`

### Acceptance

- [x] `shaders/` directory with README — `shaders/{transform_points,heightfield_flatten,mesh_quadrics,mesh_edge_costs}_v1.wgsl` + `shaders/README.md`
- [x] Runtime checks `adapter.features` before subgroups — `npm/webgpu.ts` feature detection + `SHADER_BUNDLE_VERSION`
