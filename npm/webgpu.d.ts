/**
 * @module webgpu
 *
 * WebGPU compute kernels for wasm-spatial-core (Wave 4).
 *
 * GPU paths use WGSL shaders from `shaders/`. CPU fallback uses WASM
 * (`transformPointCloud`, `flattenTerrain`) when GPU is unavailable or
 * `preferGpu: false`.
 *
 * @packageDocumentation
 */
/** @version 1.0.0 — keep in sync with shaders/transform_points_v1.wgsl */
export declare const TRANSFORM_POINTS_WGSL_V1 = "struct TransformParams {\n  matrix: mat4x4<f32>,\n  point_count: u32,\n}\n@group(0) @binding(0) var<uniform> params: TransformParams;\n@group(0) @binding(1) var<storage, read> positions_in: array<f32>;\n@group(0) @binding(2) var<storage, read_write> positions_out: array<f32>;\n@compute @workgroup_size(256)\nfn main(@builtin(global_invocation_id) gid: vec3<u32>) {\n  let i = gid.x;\n  if (i >= params.point_count) { return; }\n  let base = i * 3u;\n  let v = vec4<f32>(positions_in[base], positions_in[base+1u], positions_in[base+2u], 1.0);\n  let out = params.matrix * v;\n  positions_out[base] = out.x;\n  positions_out[base+1u] = out.y;\n  positions_out[base+2u] = out.z;\n}";
/** @version 1.0.1 — keep in sync with shaders/heightfield_flatten_v1.wgsl
 *
 * v1.0.1: renamed `target` → `target_height`. `target` is a reserved WGSL
 * keyword, so v1.0.0 failed to compile in Chrome. Caught by
 * tests/webgpu-bench.spec.mjs.
 */
export declare const HEIGHTFIELD_FLATTEN_WGSL_V1 = "struct FlattenParams {\n  width: u32,\n  height: u32,\n  target_height: f32,\n  _pad: u32,\n}\n@group(0) @binding(0) var<uniform> params: FlattenParams;\n@group(0) @binding(1) var<storage, read> mask: array<u32>;\n@group(0) @binding(2) var<storage, read_write> heights: array<f32>;\n@compute @workgroup_size(8, 8)\nfn main(@builtin(global_invocation_id) gid: vec3<u32>) {\n  let col = gid.x;\n  let row = gid.y;\n  if (col >= params.width || row >= params.height) { return; }\n  let idx = row * params.width + col;\n  if (mask[idx] == 1u) { heights[idx] = params.target_height; }\n}";
/** @version 1.1.0 — keep in sync with shaders/mesh_quadrics_v1.wgsl */
export declare const MESH_QUADRICS_WGSL_V1 = "struct QuadricParams {\n  tri_count: u32,\n  vertex_count: u32,\n}\n@group(0) @binding(0) var<uniform> params: QuadricParams;\n@group(0) @binding(1) var<storage, read> positions: array<f32>;\n@group(0) @binding(2) var<storage, read> indices: array<u32>;\n@group(0) @binding(3) var<storage, read_write> quadrics: array<atomic<u32>>;\nfn atomic_add_f32(ptr: ptr<storage, atomic<u32>>, val: f32) {\n  loop {\n    let old_bits = atomicLoad(ptr);\n    let new_bits = bitcast<u32>(bitcast<f32>(old_bits) + val);\n    let result = atomicCompareExchangeWeak(ptr, old_bits, new_bits);\n    if result.exchanged { break; }\n  }\n}\nfn add_quadric_to_vertex(vertex: u32, q: array<f32, 10>) {\n  let base = vertex * 10u;\n  for (var i = 0u; i < 10u; i = i + 1u) {\n    atomic_add_f32(&quadrics[base + i], q[i]);\n  }\n}\nfn plane_quadric(p0: vec3<f32>, p1: vec3<f32>, p2: vec3<f32>) -> array<f32, 10> {\n  let ab = p1 - p0;\n  let ac = p2 - p0;\n  let n = cross(ab, ac);\n  let len = length(n);\n  var q: array<f32, 10>;\n  if (len < 1e-12) {\n    for (var i = 0u; i < 10u; i = i + 1u) { q[i] = 0.0; }\n    return q;\n  }\n  let nn = n / len;\n  let d = -dot(nn, p0);\n  let a = nn.x; let b = nn.y; let c = nn.z;\n  q[0] = a * a; q[1] = a * b; q[2] = a * c; q[3] = a * d;\n  q[4] = b * b; q[5] = b * c; q[6] = b * d;\n  q[7] = c * c; q[8] = c * d; q[9] = d * d;\n  return q;\n}\n@compute @workgroup_size(256)\nfn main(@builtin(global_invocation_id) gid: vec3<u32>) {\n  let tri = gid.x;\n  if (tri >= params.tri_count) { return; }\n  let base = tri * 3u;\n  let i0 = indices[base]; let i1 = indices[base + 1u]; let i2 = indices[base + 2u];\n  if (i0 >= params.vertex_count || i1 >= params.vertex_count || i2 >= params.vertex_count) { return; }\n  let p0 = vec3<f32>(positions[i0 * 3u], positions[i0 * 3u + 1u], positions[i0 * 3u + 2u]);\n  let p1 = vec3<f32>(positions[i1 * 3u], positions[i1 * 3u + 1u], positions[i1 * 3u + 2u]);\n  let p2 = vec3<f32>(positions[i2 * 3u], positions[i2 * 3u + 1u], positions[i2 * 3u + 2u]);\n  let q = plane_quadric(p0, p1, p2);\n  add_quadric_to_vertex(i0, q);\n  add_quadric_to_vertex(i1, q);\n  add_quadric_to_vertex(i2, q);\n}";
/** @version 1.1.0 — keep in sync with shaders/mesh_edge_costs_v1.wgsl */
export declare const MESH_EDGE_COSTS_WGSL_V1 = "struct EdgeCostParams {\n  edge_count: u32,\n  vertex_count: u32,\n}\n@group(0) @binding(0) var<uniform> params: EdgeCostParams;\n@group(0) @binding(1) var<storage, read> positions: array<f32>;\n@group(0) @binding(2) var<storage, read> quadrics: array<f32>;\n@group(0) @binding(3) var<storage, read> edges: array<u32>;\n@group(0) @binding(4) var<storage, read_write> costs: array<f32>;\nfn load_quadric(vertex: u32) -> array<f32, 10> {\n  var q: array<f32, 10>;\n  let base = vertex * 10u;\n  for (var i = 0u; i < 10u; i = i + 1u) { q[i] = quadrics[base + i]; }\n  return q;\n}\nfn add_quadric(a: array<f32, 10>, b: array<f32, 10>) -> array<f32, 10> {\n  var out: array<f32, 10>;\n  for (var i = 0u; i < 10u; i = i + 1u) { out[i] = a[i] + b[i]; }\n  return out;\n}\nfn eval_quadric(q: array<f32, 10>, p: vec3<f32>) -> f32 {\n  let x = p.x; let y = p.y; let z = p.z;\n  return q[0] * x * x + q[4] * y * y + q[7] * z * z\n    + 2.0 * (q[1] * x * y + q[2] * x * z + q[5] * y * z)\n    + 2.0 * (q[3] * x + q[6] * y + q[8] * z) + q[9];\n}\nfn optimal_position(q: array<f32, 10>) -> vec3<f32> {\n  let m0 = q[0]; let m1 = q[1]; let m2 = q[2]; let m3 = q[3];\n  let m4 = q[4]; let m5 = q[5]; let m6 = q[6]; let m7 = q[7]; let m8 = q[8];\n  let det = m0 * (m4 * m7 - m5 * m5) - m1 * (m1 * m7 - m2 * m5) + m2 * (m1 * m5 - m2 * m4);\n  if (abs(det) < 1e-12) { return vec3<f32>(0.0); }\n  let x = (-m3 * (m4 * m7 - m5 * m5) + m6 * (m1 * m7 - m2 * m5) - m8 * (m1 * m5 - m2 * m4)) / det;\n  let y = (m0 * (-m6 * m7 + m5 * m8) - m1 * (-m3 * m7 + m2 * m8) + m2 * (-m3 * m5 + m2 * m6)) / det;\n  let z = (m0 * (m4 * m8 - m5 * m6) - m1 * (m1 * m8 - m2 * m6) + m2 * (m1 * m6 - m2 * m4)) / det;\n  return vec3<f32>(x, y, z);\n}\n@compute @workgroup_size(256)\nfn main(@builtin(global_invocation_id) gid: vec3<u32>) {\n  let edge_idx = gid.x;\n  if (edge_idx >= params.edge_count) { return; }\n  let ebase = edge_idx * 2u;\n  let a = edges[ebase]; let b = edges[ebase + 1u];\n  if (a >= params.vertex_count || b >= params.vertex_count) {\n    costs[edge_idx] = 1e30; return;\n  }\n  let q = add_quadric(load_quadric(a), load_quadric(b));\n  var pos = optimal_position(q);\n  if (all(pos == vec3<f32>(0.0))) {\n    let pa = vec3<f32>(positions[a * 3u], positions[a * 3u + 1u], positions[a * 3u + 2u]);\n    let pb = vec3<f32>(positions[b * 3u], positions[b * 3u + 1u], positions[b * 3u + 2u]);\n    pos = (pa + pb) * 0.5;\n  }\n  costs[edge_idx] = max(eval_quadric(q, pos), 0.0);\n}";
export declare const SHADER_BUNDLE_VERSION = "1.1.0";
export declare const GPU_BUFFER_LAYOUT: {
    readonly POSITION_STRIDE_BYTES: 12;
    readonly MATRIX_FLOAT_COUNT: 16;
    readonly HEIGHT_STRIDE_BYTES: 4;
    readonly MASK_STRIDE_BYTES: 1;
    readonly INDEX_STRIDE_BYTES: 4;
    readonly QUADRIC_FLOAT_COUNT: 10;
};
export type GpuBufferLayout = typeof GPU_BUFFER_LAYOUT;
export interface GpuContextOptions {
    powerPreference?: GPUPowerPreference;
    label?: string;
}
/** Minimal WASM module surface for CPU fallback. */
export interface WasmQemResult {
    mesh: {
        positions: Float32Array;
        indices: Uint32Array;
        texcoords: Float32Array;
        hasTexcoords: boolean;
    };
    maxError: number;
    trianglesBefore: number;
    trianglesAfter: number;
}
export interface WasmSpatialCore {
    transformPointCloud(positions: Float32Array, matrix: Float32Array): Float32Array;
    flattenTerrain?(heights: Float32Array, width: number, height: number, bounds: Float64Array, polygon: Float64Array, target: number, featherCells: number): void;
    simplifyMeshQem?(mesh: unknown, targetTriangles: number, preserveUvSeams?: boolean): WasmQemResult;
}
export interface TransformPointsOptions {
    /** When false, always use WASM CPU path (W4.5). Default: true */
    preferGpu?: boolean;
    wasm: WasmSpatialCore;
}
export interface FlattenHeightfieldOptions {
    preferGpu?: boolean;
    wasm: WasmSpatialCore;
    bounds: Float64Array;
    polygon: Float64Array;
    featherCells?: number;
}
export interface SimplifyMeshQemOptions {
    /** When false, always use WASM CPU path. Default: true */
    preferGpu?: boolean;
    /** Block collapses across UV seams (W5.6). Default: true */
    preserveUvSeams?: boolean;
    wasm: WasmSpatialCore;
}
export interface QemSimplifyResult {
    positions: Float32Array;
    indices: Uint32Array;
    texcoords: Float32Array | null;
    maxError: number;
    trianglesBefore: number;
    trianglesAfter: number;
}
export declare class GpuContext {
    readonly adapter: GPUAdapter;
    readonly device: GPUDevice;
    readonly hasSubgroups: boolean;
    readonly shaderVersion = "1.1.0";
    private transformPipeline;
    private flattenPipeline;
    private quadricsPipeline;
    private edgeCostsPipeline;
    private constructor();
    /** Create a GPU context, or `null` when WebGPU is unavailable. */
    static create(options?: GpuContextOptions): Promise<GpuContext | null>;
    private getTransformPipeline;
    private getFlattenPipeline;
    private getQuadricsPipeline;
    private getEdgeCostsPipeline;
    /**
     * Accumulate per-vertex quadrics from indexed triangles (W5.7).
     */
    accumulateQuadrics(positions: Float32Array, indices: Uint32Array): Promise<Float32Array>;
    /**
     * Evaluate QEM collapse cost per undirected edge (W5.7).
     * `edges` is flat `[a0, b0, a1, b1, …]`.
     */
    evaluateEdgeCosts(positions: Float32Array, quadrics: Float32Array, edges: Uint32Array): Promise<Float32Array>;
    /**
     * Batch Mat4 × vec3 transform on GPU (W4.3).
     * Matrix is column-major (WebGL convention).
     */
    transformPoints(positions: Float32Array, matrix: Float32Array): Promise<Float32Array>;
    /**
     * Flatten masked heightfield cells on GPU (W4.4).
     * `mask` is Uint8Array (0/1), row-major.
     */
    flattenHeightfield(heights: Float32Array, width: number, height: number, mask: Uint8Array, target: number): Promise<Float32Array>;
}
/**
 * Transform point positions — GPU when available, else WASM CPU.
 */
export declare function transformPoints(ctx: GpuContext | null, positions: Float32Array, matrix: Float32Array, options: TransformPointsOptions): Promise<Float32Array>;
/**
 * Flatten heightfield inside polygon — GPU mask path or WASM flattenTerrain.
 */
export declare function flattenHeightfield(ctx: GpuContext | null, heights: Float32Array, width: number, height: number, mask: Uint8Array, target: number, options: FlattenHeightfieldOptions): Promise<Float32Array>;
/** Check whether subgroup features are available on an adapter. */
export declare function detectSubgroupFeatures(adapter: GPUAdapter): boolean;
/**
 * QEM mesh decimation — GPU-assisted when available, else WASM CPU (W5.7).
 *
 * `wasmMesh` must expose `positions`, `indices`, and optional `texcoords` /
 * `hasTexcoords` getters (e.g. `WasmMeshChunk`).
 */
export declare function simplifyMeshQem(ctx: GpuContext | null, wasmMesh: {
    positions: Float32Array;
    indices: Uint32Array;
    texcoords?: Float32Array;
    hasTexcoords?: boolean;
}, targetTriangles: number, options: SimplifyMeshQemOptions): Promise<QemSimplifyResult>;
