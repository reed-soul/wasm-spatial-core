/**
 * WebGPU compute module for wasm-spatial-core (Wave 4 + W5.7 QEM).
 *
 * @packageDocumentation
 */

export declare const SHADER_BUNDLE_VERSION: "1.1.0";
export declare const TRANSFORM_POINTS_WGSL_V1: string;
export declare const HEIGHTFIELD_FLATTEN_WGSL_V1: string;
export declare const MESH_QUADRICS_WGSL_V1: string;
export declare const MESH_EDGE_COSTS_WGSL_V1: string;

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
  preferGpu?: boolean;
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
  readonly shaderVersion: string;
  static create(options?: GpuContextOptions): Promise<GpuContext | null>;
  transformPoints(positions: Float32Array, matrix: Float32Array): Promise<Float32Array>;
  flattenHeightfield(
    heights: Float32Array,
    width: number,
    height: number,
    mask: Uint8Array,
    target: number,
  ): Promise<Float32Array>;
  accumulateQuadrics(positions: Float32Array, indices: Uint32Array): Promise<Float32Array>;
  evaluateEdgeCosts(
    positions: Float32Array,
    quadrics: Float32Array,
    edges: Uint32Array,
  ): Promise<Float32Array>;
}

export declare function transformPoints(
  ctx: GpuContext | null,
  positions: Float32Array,
  matrix: Float32Array,
  options: TransformPointsOptions,
): Promise<Float32Array>;

export declare function flattenHeightfield(
  ctx: GpuContext | null,
  heights: Float32Array,
  width: number,
  height: number,
  mask: Uint8Array,
  target: number,
  options: FlattenHeightfieldOptions,
): Promise<Float32Array>;

export declare function simplifyMeshQem(
  ctx: GpuContext | null,
  wasmMesh: {
    positions: Float32Array;
    indices: Uint32Array;
    texcoords?: Float32Array;
    hasTexcoords?: boolean;
  },
  targetTriangles: number,
  options: SimplifyMeshQemOptions,
): Promise<QemSimplifyResult>;

export declare function detectSubgroupFeatures(adapter: GPUAdapter): boolean;
