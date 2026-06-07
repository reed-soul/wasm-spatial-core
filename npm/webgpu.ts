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

// Shader sources embedded for browser bundles. Edit `shaders/*.wgsl` first,
// then sync the string constants below (see shaders/README.md).

/** @version 1.0.0 — keep in sync with shaders/transform_points_v1.wgsl */
export const TRANSFORM_POINTS_WGSL_V1 = `struct TransformParams {
  matrix: mat4x4<f32>,
  point_count: u32,
}
@group(0) @binding(0) var<uniform> params: TransformParams;
@group(0) @binding(1) var<storage, read> positions_in: array<f32>;
@group(0) @binding(2) var<storage, read_write> positions_out: array<f32>;
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let i = gid.x;
  if (i >= params.point_count) { return; }
  let base = i * 3u;
  let v = vec4<f32>(positions_in[base], positions_in[base+1u], positions_in[base+2u], 1.0);
  let out = params.matrix * v;
  positions_out[base] = out.x;
  positions_out[base+1u] = out.y;
  positions_out[base+2u] = out.z;
}`;

/** @version 1.0.0 — keep in sync with shaders/heightfield_flatten_v1.wgsl */
export const HEIGHTFIELD_FLATTEN_WGSL_V1 = `struct FlattenParams {
  width: u32,
  height: u32,
  target: f32,
  _pad: u32,
}
@group(0) @binding(0) var<uniform> params: FlattenParams;
@group(0) @binding(1) var<storage, read> mask: array<u32>;
@group(0) @binding(2) var<storage, read_write> heights: array<f32>;
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let col = gid.x;
  let row = gid.y;
  if (col >= params.width || row >= params.height) { return; }
  let idx = row * params.width + col;
  if (mask[idx] == 1u) { heights[idx] = params.target; }
}`;

/** @version 1.1.0 — keep in sync with shaders/mesh_quadrics_v1.wgsl */
export const MESH_QUADRICS_WGSL_V1 = `struct QuadricParams {
  tri_count: u32,
  vertex_count: u32,
}
@group(0) @binding(0) var<uniform> params: QuadricParams;
@group(0) @binding(1) var<storage, read> positions: array<f32>;
@group(0) @binding(2) var<storage, read> indices: array<u32>;
@group(0) @binding(3) var<storage, read_write> quadrics: array<atomic<u32>>;
fn atomic_add_f32(ptr: ptr<storage, atomic<u32>>, val: f32) {
  loop {
    let old_bits = atomicLoad(ptr);
    let new_bits = bitcast<u32>(bitcast<f32>(old_bits) + val);
    let result = atomicCompareExchangeWeak(ptr, old_bits, new_bits);
    if result.exchanged { break; }
  }
}
fn add_quadric_to_vertex(vertex: u32, q: array<f32, 10>) {
  let base = vertex * 10u;
  for (var i = 0u; i < 10u; i = i + 1u) {
    atomic_add_f32(&quadrics[base + i], q[i]);
  }
}
fn plane_quadric(p0: vec3<f32>, p1: vec3<f32>, p2: vec3<f32>) -> array<f32, 10> {
  let ab = p1 - p0;
  let ac = p2 - p0;
  let n = cross(ab, ac);
  let len = length(n);
  var q: array<f32, 10>;
  if (len < 1e-12) {
    for (var i = 0u; i < 10u; i = i + 1u) { q[i] = 0.0; }
    return q;
  }
  let nn = n / len;
  let d = -dot(nn, p0);
  let a = nn.x; let b = nn.y; let c = nn.z;
  q[0] = a * a; q[1] = a * b; q[2] = a * c; q[3] = a * d;
  q[4] = b * b; q[5] = b * c; q[6] = b * d;
  q[7] = c * c; q[8] = c * d; q[9] = d * d;
  return q;
}
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let tri = gid.x;
  if (tri >= params.tri_count) { return; }
  let base = tri * 3u;
  let i0 = indices[base]; let i1 = indices[base + 1u]; let i2 = indices[base + 2u];
  if (i0 >= params.vertex_count || i1 >= params.vertex_count || i2 >= params.vertex_count) { return; }
  let p0 = vec3<f32>(positions[i0 * 3u], positions[i0 * 3u + 1u], positions[i0 * 3u + 2u]);
  let p1 = vec3<f32>(positions[i1 * 3u], positions[i1 * 3u + 1u], positions[i1 * 3u + 2u]);
  let p2 = vec3<f32>(positions[i2 * 3u], positions[i2 * 3u + 1u], positions[i2 * 3u + 2u]);
  let q = plane_quadric(p0, p1, p2);
  add_quadric_to_vertex(i0, q);
  add_quadric_to_vertex(i1, q);
  add_quadric_to_vertex(i2, q);
}`;

/** @version 1.1.0 — keep in sync with shaders/mesh_edge_costs_v1.wgsl */
export const MESH_EDGE_COSTS_WGSL_V1 = `struct EdgeCostParams {
  edge_count: u32,
  vertex_count: u32,
}
@group(0) @binding(0) var<uniform> params: EdgeCostParams;
@group(0) @binding(1) var<storage, read> positions: array<f32>;
@group(0) @binding(2) var<storage, read> quadrics: array<f32>;
@group(0) @binding(3) var<storage, read> edges: array<u32>;
@group(0) @binding(4) var<storage, read_write> costs: array<f32>;
fn load_quadric(vertex: u32) -> array<f32, 10> {
  var q: array<f32, 10>;
  let base = vertex * 10u;
  for (var i = 0u; i < 10u; i = i + 1u) { q[i] = quadrics[base + i]; }
  return q;
}
fn add_quadric(a: array<f32, 10>, b: array<f32, 10>) -> array<f32, 10> {
  var out: array<f32, 10>;
  for (var i = 0u; i < 10u; i = i + 1u) { out[i] = a[i] + b[i]; }
  return out;
}
fn eval_quadric(q: array<f32, 10>, p: vec3<f32>) -> f32 {
  let x = p.x; let y = p.y; let z = p.z;
  return q[0] * x * x + q[4] * y * y + q[7] * z * z
    + 2.0 * (q[1] * x * y + q[2] * x * z + q[5] * y * z)
    + 2.0 * (q[3] * x + q[6] * y + q[8] * z) + q[9];
}
fn optimal_position(q: array<f32, 10>) -> vec3<f32> {
  let m0 = q[0]; let m1 = q[1]; let m2 = q[2]; let m3 = q[3];
  let m4 = q[4]; let m5 = q[5]; let m6 = q[6]; let m7 = q[7]; let m8 = q[8];
  let det = m0 * (m4 * m7 - m5 * m5) - m1 * (m1 * m7 - m2 * m5) + m2 * (m1 * m5 - m2 * m4);
  if (abs(det) < 1e-12) { return vec3<f32>(0.0); }
  let x = (-m3 * (m4 * m7 - m5 * m5) + m6 * (m1 * m7 - m2 * m5) - m8 * (m1 * m5 - m2 * m4)) / det;
  let y = (m0 * (-m6 * m7 + m5 * m8) - m1 * (-m3 * m7 + m2 * m8) + m2 * (-m3 * m5 + m2 * m6)) / det;
  let z = (m0 * (m4 * m8 - m5 * m6) - m1 * (m1 * m8 - m2 * m6) + m2 * (m1 * m6 - m2 * m4)) / det;
  return vec3<f32>(x, y, z);
}
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let edge_idx = gid.x;
  if (edge_idx >= params.edge_count) { return; }
  let ebase = edge_idx * 2u;
  let a = edges[ebase]; let b = edges[ebase + 1u];
  if (a >= params.vertex_count || b >= params.vertex_count) {
    costs[edge_idx] = 1e30; return;
  }
  let q = add_quadric(load_quadric(a), load_quadric(b));
  var pos = optimal_position(q);
  if (all(pos == vec3<f32>(0.0))) {
    let pa = vec3<f32>(positions[a * 3u], positions[a * 3u + 1u], positions[a * 3u + 2u]);
    let pb = vec3<f32>(positions[b * 3u], positions[b * 3u + 1u], positions[b * 3u + 2u]);
    pos = (pa + pb) * 0.5;
  }
  costs[edge_idx] = max(eval_quadric(q, pos), 0.0);
}`;

export const SHADER_BUNDLE_VERSION = "1.1.0";

// ---------------------------------------------------------------------------
// Buffer layout contract (W4.2) — keep in sync with src/webgpu.rs
// ---------------------------------------------------------------------------

export const GPU_BUFFER_LAYOUT = {
  POSITION_STRIDE_BYTES: 12,
  MATRIX_FLOAT_COUNT: 16,
  HEIGHT_STRIDE_BYTES: 4,
  MASK_STRIDE_BYTES: 1,
  INDEX_STRIDE_BYTES: 4,
  QUADRIC_FLOAT_COUNT: 10,
} as const;

export type GpuBufferLayout = typeof GPU_BUFFER_LAYOUT;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

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
  flattenTerrain?(
    heights: Float32Array,
    width: number,
    height: number,
    bounds: Float64Array,
    polygon: Float64Array,
    target: number,
    featherCells: number,
  ): void;
  simplifyMeshQem?(
    mesh: unknown,
    targetTriangles: number,
    preserveUvSeams?: boolean,
  ): WasmQemResult;
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

// ---------------------------------------------------------------------------
// GpuContext (W4.1)
// ---------------------------------------------------------------------------

export class GpuContext {
  readonly adapter: GPUAdapter;
  readonly device: GPUDevice;
  readonly hasSubgroups: boolean;
  readonly shaderVersion = SHADER_BUNDLE_VERSION;

  private transformPipeline: GPUComputePipeline | null = null;
  private flattenPipeline: GPUComputePipeline | null = null;
  private quadricsPipeline: GPUComputePipeline | null = null;
  private edgeCostsPipeline: GPUComputePipeline | null = null;

  private constructor(adapter: GPUAdapter, device: GPUDevice) {
    this.adapter = adapter;
    this.device = device;
    this.hasSubgroups = adapter.features.has("subgroups" as GPUFeatureName);
  }

  /** Create a GPU context, or `null` when WebGPU is unavailable. */
  static async create(options: GpuContextOptions = {}): Promise<GpuContext | null> {
    if (typeof navigator === "undefined" || !navigator.gpu) {
      return null;
    }

    const adapter = await navigator.gpu.requestAdapter({
      powerPreference: options.powerPreference ?? "high-performance",
    });
    if (!adapter) {
      return null;
    }

    const device = await adapter.requestDevice({
      label: options.label ?? "wasm-spatial-core",
    });

    return new GpuContext(adapter, device);
  }

  private async getTransformPipeline(): Promise<GPUComputePipeline> {
    if (this.transformPipeline) {
      return this.transformPipeline;
    }
    const module = this.device.createShaderModule({
      label: "transform_points_v1",
      code: TRANSFORM_POINTS_WGSL_V1,
    });
    this.transformPipeline = await this.device.createComputePipelineAsync({
      label: "transform_points_v1",
      layout: "auto",
      compute: { module, entryPoint: "main" },
    });
    return this.transformPipeline;
  }

  private async getFlattenPipeline(): Promise<GPUComputePipeline> {
    if (this.flattenPipeline) {
      return this.flattenPipeline;
    }
    const module = this.device.createShaderModule({
      label: "heightfield_flatten_v1",
      code: HEIGHTFIELD_FLATTEN_WGSL_V1,
    });
    this.flattenPipeline = await this.device.createComputePipelineAsync({
      label: "heightfield_flatten_v1",
      layout: "auto",
      compute: { module, entryPoint: "main" },
    });
    return this.flattenPipeline;
  }

  private async getQuadricsPipeline(): Promise<GPUComputePipeline> {
    if (this.quadricsPipeline) {
      return this.quadricsPipeline;
    }
    const module = this.device.createShaderModule({
      label: "mesh_quadrics_v1",
      code: MESH_QUADRICS_WGSL_V1,
    });
    this.quadricsPipeline = await this.device.createComputePipelineAsync({
      label: "mesh_quadrics_v1",
      layout: "auto",
      compute: { module, entryPoint: "main" },
    });
    return this.quadricsPipeline;
  }

  private async getEdgeCostsPipeline(): Promise<GPUComputePipeline> {
    if (this.edgeCostsPipeline) {
      return this.edgeCostsPipeline;
    }
    const module = this.device.createShaderModule({
      label: "mesh_edge_costs_v1",
      code: MESH_EDGE_COSTS_WGSL_V1,
    });
    this.edgeCostsPipeline = await this.device.createComputePipelineAsync({
      label: "mesh_edge_costs_v1",
      layout: "auto",
      compute: { module, entryPoint: "main" },
    });
    return this.edgeCostsPipeline;
  }

  /**
   * Accumulate per-vertex quadrics from indexed triangles (W5.7).
   */
  async accumulateQuadrics(
    positions: Float32Array,
    indices: Uint32Array,
  ): Promise<Float32Array> {
    if (positions.length % 3 !== 0) {
      throw new Error("positions length must be a multiple of 3");
    }
    if (indices.length % 3 !== 0) {
      throw new Error("indices length must be a multiple of 3");
    }

    const vertexCount = positions.length / 3;
    const triCount = indices.length / 3;
    const quadricFloats = vertexCount * GPU_BUFFER_LAYOUT.QUADRIC_FLOAT_COUNT;
    const quadricBytes = quadricFloats * 4;

    const pipeline = await this.getQuadricsPipeline();
    const uniformData = new Uint32Array(8);
    uniformData[0] = triCount;
    uniformData[1] = vertexCount;

    const uniformBuffer = this.device.createBuffer({
      size: 32,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });
    const posBuffer = this.device.createBuffer({
      size: positions.byteLength,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
    });
    const idxBuffer = this.device.createBuffer({
      size: indices.byteLength,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
    });
    const quadricBuffer = this.device.createBuffer({
      size: quadricBytes,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC | GPUBufferUsage.COPY_DST,
    });
    const readBuffer = this.device.createBuffer({
      size: quadricBytes,
      usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    });

    this.device.queue.writeBuffer(uniformBuffer, 0, uniformData);
    this.device.queue.writeBuffer(posBuffer, 0, positions);
    this.device.queue.writeBuffer(idxBuffer, 0, indices);
    this.device.queue.writeBuffer(quadricBuffer, 0, new Uint8Array(quadricBytes));

    const bindGroup = this.device.createBindGroup({
      layout: pipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: uniformBuffer } },
        { binding: 1, resource: { buffer: posBuffer } },
        { binding: 2, resource: { buffer: idxBuffer } },
        { binding: 3, resource: { buffer: quadricBuffer } },
      ],
    });

    const encoder = this.device.createCommandEncoder();
    const pass = encoder.beginComputePass();
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.dispatchWorkgroups(Math.ceil(triCount / 256));
    pass.end();
    encoder.copyBufferToBuffer(quadricBuffer, 0, readBuffer, 0, quadricBytes);
    this.device.queue.submit([encoder.finish()]);

    await readBuffer.mapAsync(GPUMapMode.READ);
    const result = new Float32Array(readBuffer.getMappedRange().slice(0));
    readBuffer.unmap();

    uniformBuffer.destroy();
    posBuffer.destroy();
    idxBuffer.destroy();
    quadricBuffer.destroy();
    readBuffer.destroy();

    return result;
  }

  /**
   * Evaluate QEM collapse cost per undirected edge (W5.7).
   * `edges` is flat `[a0, b0, a1, b1, …]`.
   */
  async evaluateEdgeCosts(
    positions: Float32Array,
    quadrics: Float32Array,
    edges: Uint32Array,
  ): Promise<Float32Array> {
    if (edges.length % 2 !== 0) {
      throw new Error("edges length must be even");
    }
    const vertexCount = positions.length / 3;
    const edgeCount = edges.length / 2;
    const expectedQuadrics = vertexCount * GPU_BUFFER_LAYOUT.QUADRIC_FLOAT_COUNT;
    if (quadrics.length !== expectedQuadrics) {
      throw new Error(`quadrics length must be vertexCount × ${GPU_BUFFER_LAYOUT.QUADRIC_FLOAT_COUNT}`);
    }

    const pipeline = await this.getEdgeCostsPipeline();
    const uniformData = new Uint32Array(8);
    uniformData[0] = edgeCount;
    uniformData[1] = vertexCount;

    const uniformBuffer = this.device.createBuffer({
      size: 32,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });
    const posBuffer = this.device.createBuffer({
      size: positions.byteLength,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
    });
    const quadricBuffer = this.device.createBuffer({
      size: quadrics.byteLength,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
    });
    const edgeBuffer = this.device.createBuffer({
      size: edges.byteLength,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
    });
    const costBuffer = this.device.createBuffer({
      size: edgeCount * 4,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
    });
    const readBuffer = this.device.createBuffer({
      size: edgeCount * 4,
      usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    });

    this.device.queue.writeBuffer(uniformBuffer, 0, uniformData);
    this.device.queue.writeBuffer(posBuffer, 0, positions);
    this.device.queue.writeBuffer(quadricBuffer, 0, quadrics);
    this.device.queue.writeBuffer(edgeBuffer, 0, edges);

    const bindGroup = this.device.createBindGroup({
      layout: pipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: uniformBuffer } },
        { binding: 1, resource: { buffer: posBuffer } },
        { binding: 2, resource: { buffer: quadricBuffer } },
        { binding: 3, resource: { buffer: edgeBuffer } },
        { binding: 4, resource: { buffer: costBuffer } },
      ],
    });

    const encoder = this.device.createCommandEncoder();
    const pass = encoder.beginComputePass();
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.dispatchWorkgroups(Math.ceil(edgeCount / 256));
    pass.end();
    encoder.copyBufferToBuffer(costBuffer, 0, readBuffer, 0, edgeCount * 4);
    this.device.queue.submit([encoder.finish()]);

    await readBuffer.mapAsync(GPUMapMode.READ);
    const result = new Float32Array(readBuffer.getMappedRange().slice(0));
    readBuffer.unmap();

    uniformBuffer.destroy();
    posBuffer.destroy();
    quadricBuffer.destroy();
    edgeBuffer.destroy();
    costBuffer.destroy();
    readBuffer.destroy();

    return result;
  }

  /**
   * Batch Mat4 × vec3 transform on GPU (W4.3).
   * Matrix is column-major (WebGL convention).
   */
  async transformPoints(positions: Float32Array, matrix: Float32Array): Promise<Float32Array> {
    if (matrix.length !== GPU_BUFFER_LAYOUT.MATRIX_FLOAT_COUNT) {
      throw new Error(`matrix must have ${GPU_BUFFER_LAYOUT.MATRIX_FLOAT_COUNT} elements`);
    }
    if (positions.length % 3 !== 0) {
      throw new Error("positions length must be a multiple of 3");
    }

    const pointCount = positions.length / 3;
    const pipeline = await this.getTransformPipeline();

    const uniformData = new ArrayBuffer(256);
    const uniformF32 = new Float32Array(uniformData);
    uniformF32.set(matrix, 0);
    new Uint32Array(uniformData)[16] = pointCount;

    const uniformBuffer = this.device.createBuffer({
      size: 256,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });

    const inBuffer = this.device.createBuffer({
      size: positions.byteLength,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
    });

    const outBuffer = this.device.createBuffer({
      size: positions.byteLength,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
    });

    const readBuffer = this.device.createBuffer({
      size: positions.byteLength,
      usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    });

    this.device.queue.writeBuffer(uniformBuffer, 0, uniformData);
    this.device.queue.writeBuffer(inBuffer, 0, positions);

    const bindGroup = this.device.createBindGroup({
      layout: pipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: uniformBuffer } },
        { binding: 1, resource: { buffer: inBuffer } },
        { binding: 2, resource: { buffer: outBuffer } },
      ],
    });

    const encoder = this.device.createCommandEncoder();
    const pass = encoder.beginComputePass();
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.dispatchWorkgroups(Math.ceil(pointCount / 256));
    pass.end();

    encoder.copyBufferToBuffer(outBuffer, 0, readBuffer, 0, positions.byteLength);
    this.device.queue.submit([encoder.finish()]);

    await readBuffer.mapAsync(GPUMapMode.READ);
    const result = new Float32Array(readBuffer.getMappedRange().slice(0));
    readBuffer.unmap();

    uniformBuffer.destroy();
    inBuffer.destroy();
    outBuffer.destroy();
    readBuffer.destroy();

    return result;
  }

  /**
   * Flatten masked heightfield cells on GPU (W4.4).
   * `mask` is Uint8Array (0/1), row-major.
   */
  async flattenHeightfield(
    heights: Float32Array,
    width: number,
    height: number,
    mask: Uint8Array,
    target: number,
  ): Promise<Float32Array> {
    const count = width * height;
    if (heights.length !== count || mask.length !== count) {
      throw new Error("heights/mask size must equal width × height");
    }

    const pipeline = await this.getFlattenPipeline();
    const maskU32 = new Uint32Array(count);
    for (let i = 0; i < count; i++) {
      maskU32[i] = mask[i] ? 1 : 0;
    }

    const uniformData = new Uint32Array(4);
    uniformData[0] = width;
    uniformData[1] = height;
    new Float32Array(uniformData.buffer)[2] = target;

    const uniformBuffer = this.device.createBuffer({
      size: 16,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });

    const maskBuffer = this.device.createBuffer({
      size: maskU32.byteLength,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
    });

    const heightBuffer = this.device.createBuffer({
      size: heights.byteLength,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC | GPUBufferUsage.COPY_DST,
    });

    const readBuffer = this.device.createBuffer({
      size: heights.byteLength,
      usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    });

    this.device.queue.writeBuffer(uniformBuffer, 0, uniformData);
    this.device.queue.writeBuffer(maskBuffer, 0, maskU32);
    this.device.queue.writeBuffer(heightBuffer, 0, heights);

    const bindGroup = this.device.createBindGroup({
      layout: pipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: uniformBuffer } },
        { binding: 1, resource: { buffer: maskBuffer } },
        { binding: 2, resource: { buffer: heightBuffer } },
      ],
    });

    const encoder = this.device.createCommandEncoder();
    const pass = encoder.beginComputePass();
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.dispatchWorkgroups(Math.ceil(width / 8), Math.ceil(height / 8));
    pass.end();

    encoder.copyBufferToBuffer(heightBuffer, 0, readBuffer, 0, heights.byteLength);
    this.device.queue.submit([encoder.finish()]);

    await readBuffer.mapAsync(GPUMapMode.READ);
    const result = new Float32Array(readBuffer.getMappedRange().slice(0));
    readBuffer.unmap();

    uniformBuffer.destroy();
    maskBuffer.destroy();
    heightBuffer.destroy();
    readBuffer.destroy();

    return result;
  }
}

// ---------------------------------------------------------------------------
// Unified API with WASM fallback (W4.5)
// ---------------------------------------------------------------------------

/**
 * Transform point positions — GPU when available, else WASM CPU.
 */
export async function transformPoints(
  ctx: GpuContext | null,
  positions: Float32Array,
  matrix: Float32Array,
  options: TransformPointsOptions,
): Promise<Float32Array> {
  const preferGpu = options.preferGpu !== false;
  if (preferGpu && ctx) {
    return ctx.transformPoints(positions, matrix);
  }
  return options.wasm.transformPointCloud(positions, matrix);
}

/**
 * Flatten heightfield inside polygon — GPU mask path or WASM flattenTerrain.
 */
export async function flattenHeightfield(
  ctx: GpuContext | null,
  heights: Float32Array,
  width: number,
  height: number,
  mask: Uint8Array,
  target: number,
  options: FlattenHeightfieldOptions,
): Promise<Float32Array> {
  const preferGpu = options.preferGpu !== false;
  if (preferGpu && ctx) {
    return ctx.flattenHeightfield(heights, width, height, mask, target);
  }

  if (!options.wasm.flattenTerrain) {
    throw new Error("WASM flattenTerrain unavailable — build with terrain-edit feature");
  }

  const out = new Float32Array(heights);
  options.wasm.flattenTerrain(
    out,
    width,
    height,
    options.bounds,
    options.polygon,
    target,
    options.featherCells ?? 0,
  );
  return out;
}

/** Check whether subgroup features are available on an adapter. */
export function detectSubgroupFeatures(adapter: GPUAdapter): boolean {
  return adapter.features.has("subgroups" as GPUFeatureName);
}

// ---------------------------------------------------------------------------
// QEM GPU-assisted simplification (W5.7)
// ---------------------------------------------------------------------------

const QEM_EPS = 1e-12;
const UV_SEAM_EPS = 1e-4;
const POS_SEAM_EPS = 1e-5;

function edgeKey(a: number, b: number): string {
  return a < b ? `${a}:${b}` : `${b}:${a}`;
}

function buildUniqueEdges(indices: Uint32Array): Uint32Array {
  const seen = new Set<string>();
  const flat: number[] = [];
  for (let i = 0; i < indices.length; i += 3) {
    const tri = [indices[i], indices[i + 1], indices[i + 2]];
    for (let j = 0; j < 3; j++) {
      const a = tri[j];
      const b = tri[(j + 1) % 3];
      const key = edgeKey(a, b);
      if (!seen.has(key)) {
        seen.add(key);
        flat.push(a < b ? a : b, a < b ? b : a);
      }
    }
  }
  return new Uint32Array(flat);
}

function uvEqual(a0: number, a1: number, b0: number, b1: number): boolean {
  return Math.abs(a0 - b0) <= UV_SEAM_EPS && Math.abs(a1 - b1) <= UV_SEAM_EPS;
}

function detectUvSeamEdges(indices: Uint32Array, texcoords: Float32Array): Set<string> {
  const recorded = new Map<string, [number, number, number, number]>();
  const seams = new Set<string>();
  for (let i = 0; i < indices.length; i += 3) {
    const tri = [indices[i], indices[i + 1], indices[i + 2]];
    for (let j = 0; j < 3; j++) {
      const va = tri[j];
      const vb = tri[(j + 1) % 3];
      const key = edgeKey(va, vb);
      const lo = va < vb ? va : vb;
      const hi = va < vb ? vb : va;
      const uvPair: [number, number, number, number] = [
        texcoords[lo * 2],
        texcoords[lo * 2 + 1],
        texcoords[hi * 2],
        texcoords[hi * 2 + 1],
      ];
      const prev = recorded.get(key);
      if (
        prev &&
        (!uvEqual(prev[0], prev[1], uvPair[0], uvPair[1]) ||
          !uvEqual(prev[2], prev[3], uvPair[2], uvPair[3]))
      ) {
        seams.add(key);
      } else if (!prev) {
        recorded.set(key, uvPair);
      }
    }
  }
  return seams;
}

function findCoincidentUvSeamEdges(
  positions: Float32Array,
  texcoords: Float32Array,
): Set<string> {
  const seams = new Set<string>();
  const n = positions.length / 3;
  for (let i = 0; i < n; i++) {
    for (let j = i + 1; j < n; j++) {
      const bi = i * 3;
      const bj = j * 3;
      const dx = positions[bi] - positions[bj];
      const dy = positions[bi + 1] - positions[bj + 1];
      const dz = positions[bi + 2] - positions[bj + 2];
      if (dx * dx + dy * dy + dz * dz > POS_SEAM_EPS * POS_SEAM_EPS) {
        continue;
      }
      if (
        !uvEqual(texcoords[i * 2], texcoords[i * 2 + 1], texcoords[j * 2], texcoords[j * 2 + 1])
      ) {
        seams.add(edgeKey(i, j));
      }
    }
  }
  return seams;
}

function triangleArea(
  positions: Float32Array,
  i0: number,
  i1: number,
  i2: number,
): number {
  const a = i0 * 3;
  const b = i1 * 3;
  const c = i2 * 3;
  const abx = positions[b] - positions[a];
  const aby = positions[b + 1] - positions[a + 1];
  const abz = positions[b + 2] - positions[a + 2];
  const acx = positions[c] - positions[a];
  const acy = positions[c + 1] - positions[a + 1];
  const acz = positions[c + 2] - positions[a + 2];
  const cx = aby * acz - abz * acy;
  const cy = abz * acx - abx * acz;
  const cz = abx * acy - aby * acx;
  return 0.5 * Math.sqrt(cx * cx + cy * cy + cz * cz);
}

function removeDegenerateTriangles(
  indices: Uint32Array,
  positions: Float32Array,
): Uint32Array {
  const out: number[] = [];
  for (let i = 0; i < indices.length; i += 3) {
    const i0 = indices[i];
    const i1 = indices[i + 1];
    const i2 = indices[i + 2];
    if (i0 === i1 || i1 === i2 || i0 === i2) {
      continue;
    }
    if (triangleArea(positions, i0, i1, i2) > QEM_EPS) {
      out.push(i0, i1, i2);
    }
  }
  return new Uint32Array(out);
}

function edgeExists(indices: Uint32Array, a: number, b: number, deleted: boolean[]): boolean {
  if (deleted[a] || deleted[b]) {
    return false;
  }
  for (let i = 0; i < indices.length; i += 3) {
    const tri = [indices[i], indices[i + 1], indices[i + 2]];
    if (tri.includes(a) && tri.includes(b)) {
      return true;
    }
  }
  return false;
}

function compactMeshGpu(
  positions: Float32Array,
  indices: Uint32Array,
  deleted: boolean[],
  texcoords: Float32Array | null,
): { positions: Float32Array; indices: Uint32Array; texcoords: Float32Array | null } {
  const remap = new Array<number>(deleted.length).fill(-1);
  const compactPos: number[] = [];
  const compactUv: number[] = [];
  for (let i = 0; i < deleted.length; i++) {
    if (!deleted[i]) {
      remap[i] = compactPos.length / 3;
      compactPos.push(positions[i * 3], positions[i * 3 + 1], positions[i * 3 + 2]);
      if (texcoords) {
        compactUv.push(texcoords[i * 2], texcoords[i * 2 + 1]);
      }
    }
  }
  const newIndices = new Uint32Array(indices.length);
  for (let i = 0; i < indices.length; i++) {
    newIndices[i] = remap[indices[i]];
  }
  return {
    positions: new Float32Array(compactPos),
    indices: newIndices,
    texcoords: texcoords ? new Float32Array(compactUv) : null,
  };
}

function wasmQemToResult(result: WasmQemResult): QemSimplifyResult {
  const mesh = result.mesh;
  return {
    positions: new Float32Array(mesh.positions),
    indices: new Uint32Array(mesh.indices),
    texcoords: mesh.hasTexcoords ? new Float32Array(mesh.texcoords) : null,
    maxError: result.maxError,
    trianglesBefore: result.trianglesBefore,
    trianglesAfter: result.trianglesAfter,
  };
}

async function simplifyMeshQemGpu(
  ctx: GpuContext,
  positions: Float32Array,
  indices: Uint32Array,
  targetTriangles: number,
  texcoords: Float32Array | null,
  preserveUvSeams: boolean,
): Promise<QemSimplifyResult> {
  const trianglesBefore = indices.length / 3;
  if (trianglesBefore <= targetTriangles) {
    return {
      positions: new Float32Array(positions),
      indices: new Uint32Array(indices),
      texcoords: texcoords ? new Float32Array(texcoords) : null,
      maxError: 0,
      trianglesBefore,
      trianglesAfter: trianglesBefore,
    };
  }

  let pos = new Float32Array(positions);
  let idx = new Uint32Array(indices);
  let uv = texcoords ? new Float32Array(texcoords) : null;
  const vertexCount = pos.length / 3;
  const deleted = new Array<boolean>(vertexCount).fill(false);
  let maxError = 0;

  const seamEdges = new Set<string>();
  if (preserveUvSeams && uv) {
    for (const s of detectUvSeamEdges(idx, uv)) {
      seamEdges.add(s);
    }
    for (const s of findCoincidentUvSeamEdges(pos, uv)) {
      seamEdges.add(s);
    }
  }

  while (idx.length / 3 > targetTriangles) {
    const quadrics = await ctx.accumulateQuadrics(pos, idx);
    const edges = buildUniqueEdges(idx);
    const costs = await ctx.evaluateEdgeCosts(pos, quadrics, edges);

    let bestIdx = -1;
    let bestCost = Number.POSITIVE_INFINITY;
    for (let e = 0; e < edges.length; e += 2) {
      const a = edges[e];
      const b = edges[e + 1];
      if (deleted[a] || deleted[b]) {
        continue;
      }
      const key = edgeKey(a, b);
      if (preserveUvSeams && seamEdges.has(key)) {
        continue;
      }
      const cost = costs[e / 2];
      if (cost < bestCost) {
        bestCost = cost;
        bestIdx = e;
      }
    }

    if (bestIdx < 0) {
      break;
    }

    const a = edges[bestIdx];
    const b = edges[bestIdx + 1];
    if (!edgeExists(idx, a, b, deleted)) {
      continue;
    }

    maxError = Math.max(maxError, Math.sqrt(bestCost));
    deleted[b] = true;
    for (let i = 0; i < idx.length; i++) {
      if (idx[i] === b) {
        idx[i] = a;
      }
    }
    idx = removeDegenerateTriangles(idx, pos);
    if (idx.length / 3 <= targetTriangles) {
      break;
    }
  }

  const compacted = compactMeshGpu(pos, idx, deleted, uv);
  const finalIndices = removeDegenerateTriangles(compacted.indices, compacted.positions);

  return {
    positions: compacted.positions,
    indices: finalIndices,
    texcoords: compacted.texcoords,
    maxError,
    trianglesBefore,
    trianglesAfter: finalIndices.length / 3,
  };
}

/**
 * QEM mesh decimation — GPU-assisted when available, else WASM CPU (W5.7).
 *
 * `wasmMesh` must expose `positions`, `indices`, and optional `texcoords` /
 * `hasTexcoords` getters (e.g. `WasmMeshChunk`).
 */
export async function simplifyMeshQem(
  ctx: GpuContext | null,
  wasmMesh: {
    positions: Float32Array;
    indices: Uint32Array;
    texcoords?: Float32Array;
    hasTexcoords?: boolean;
  },
  targetTriangles: number,
  options: SimplifyMeshQemOptions,
): Promise<QemSimplifyResult> {
  const preferGpu = options.preferGpu !== false;
  const preserveUvSeams = options.preserveUvSeams !== false;

  if (preferGpu && ctx) {
    const texcoords =
      wasmMesh.hasTexcoords && wasmMesh.texcoords && wasmMesh.texcoords.length > 0
        ? new Float32Array(wasmMesh.texcoords)
        : null;
    return simplifyMeshQemGpu(
      ctx,
      new Float32Array(wasmMesh.positions),
      new Uint32Array(wasmMesh.indices),
      targetTriangles,
      texcoords,
      preserveUvSeams,
    );
  }

  if (!options.wasm.simplifyMeshQem) {
    throw new Error("WASM simplifyMeshQem unavailable — build with mesh-edit feature");
  }

  return wasmQemToResult(
    options.wasm.simplifyMeshQem(wasmMesh, targetTriangles, preserveUvSeams),
  );
}
