/**
 * WebGPU heightfield helpers for browser demos (W4.4).
 * Mirrors npm/webgpu.ts flatten path without a bundler.
 */
import { HEIGHTFIELD_FLATTEN_WGSL_V1 } from './webgpu-kernels.mjs';

export async function createGpuContext() {
  if (!navigator.gpu) return null;
  const adapter = await navigator.gpu.requestAdapter({ powerPreference: 'high-performance' });
  if (!adapter) return null;
  const device = await adapter.requestDevice({ label: 'terrain-gpu' });
  return {
    adapter,
    device,
    hasSubgroups: adapter.features.has('subgroups'),
    _flattenPipeline: null,
  };
}

async function getFlattenPipeline(ctx) {
  if (ctx._flattenPipeline) return ctx._flattenPipeline;
  const module = ctx.device.createShaderModule({ code: HEIGHTFIELD_FLATTEN_WGSL_V1 });
  ctx._flattenPipeline = await ctx.device.createComputePipelineAsync({
    layout: 'auto',
    compute: { module, entryPoint: 'main' },
  });
  return ctx._flattenPipeline;
}

/**
 * Masked flatten on GPU. `mask` is Uint8Array (0/1), row-major.
 */
export async function gpuFlattenHeightfield(ctx, heights, width, height, mask, target) {
  const count = width * height;
  if (heights.length !== count || mask.length !== count) {
    throw new Error('heights/mask size must equal width × height');
  }

  const pipeline = await getFlattenPipeline(ctx);
  const maskU32 = new Uint32Array(count);
  for (let i = 0; i < count; i++) maskU32[i] = mask[i] ? 1 : 0;

  const uniformData = new Uint32Array(4);
  uniformData[0] = width;
  uniformData[1] = height;
  new Float32Array(uniformData.buffer)[2] = target;

  const uniformBuffer = ctx.device.createBuffer({
    size: 16,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });
  const maskBuffer = ctx.device.createBuffer({
    size: maskU32.byteLength,
    usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
  });
  const heightBuffer = ctx.device.createBuffer({
    size: heights.byteLength,
    usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC | GPUBufferUsage.COPY_DST,
  });
  const readBuffer = ctx.device.createBuffer({
    size: heights.byteLength,
    usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
  });

  ctx.device.queue.writeBuffer(uniformBuffer, 0, uniformData);
  ctx.device.queue.writeBuffer(maskBuffer, 0, maskU32);
  ctx.device.queue.writeBuffer(heightBuffer, 0, heights);

  const bindGroup = ctx.device.createBindGroup({
    layout: pipeline.getBindGroupLayout(0),
    entries: [
      { binding: 0, resource: { buffer: uniformBuffer } },
      { binding: 1, resource: { buffer: maskBuffer } },
      { binding: 2, resource: { buffer: heightBuffer } },
    ],
  });

  const encoder = ctx.device.createCommandEncoder();
  const pass = encoder.beginComputePass();
  pass.setPipeline(pipeline);
  pass.setBindGroup(0, bindGroup);
  pass.dispatchWorkgroups(Math.ceil(width / 8), Math.ceil(height / 8));
  pass.end();
  encoder.copyBufferToBuffer(heightBuffer, 0, readBuffer, 0, heights.byteLength);
  ctx.device.queue.submit([encoder.finish()]);

  await readBuffer.mapAsync(GPUMapMode.READ);
  const result = new Float32Array(readBuffer.getMappedRange().slice(0));
  readBuffer.unmap();
  [uniformBuffer, maskBuffer, heightBuffer, readBuffer].forEach((b) => b.destroy());
  return result;
}

export function maxAbsDiff(a, b) {
  let m = 0;
  for (let i = 0; i < a.length; i++) m = Math.max(m, Math.abs(a[i] - b[i]));
  return m;
}

/**
 * Flatten with optional GPU path (mask rasterize stays on WASM).
 * GPU is used only when `preferGpu`, `gpuCtx`, mode flatten, and feather === 0.
 */
export async function applyFlattenDeform({
  gpuCtx,
  wasm,
  heights,
  width,
  height,
  bounds,
  polygon,
  target,
  feather = 0,
  preferGpu = true,
}) {
  const useGpu = preferGpu && gpuCtx && feather === 0 && typeof wasm.rasterizeTerrainMask === 'function';
  if (useGpu) {
    const mask = wasm.rasterizeTerrainMask(width, height, bounds, polygon);
    return gpuFlattenHeightfield(gpuCtx, heights, width, height, mask, target);
  }
  if (typeof wasm.flattenTerrain !== 'function') {
    throw new Error('flattenTerrain unavailable — build with terrain-edit feature');
  }
  const out = new Float32Array(heights);
  wasm.flattenTerrain(out, width, height, bounds, polygon, target, feather);
  return out;
}
