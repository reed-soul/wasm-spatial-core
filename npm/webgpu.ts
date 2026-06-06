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

export const SHADER_BUNDLE_VERSION = "1.0.0";

// ---------------------------------------------------------------------------
// Buffer layout contract (W4.2) — keep in sync with src/webgpu.rs
// ---------------------------------------------------------------------------

export const GPU_BUFFER_LAYOUT = {
  POSITION_STRIDE_BYTES: 12,
  MATRIX_FLOAT_COUNT: 16,
  HEIGHT_STRIDE_BYTES: 4,
  MASK_STRIDE_BYTES: 1,
  INDEX_STRIDE_BYTES: 4,
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
