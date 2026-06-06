/**
 * WebGPU smoke test — sync core logic with npm/webgpu.ts
 */
import { TRANSFORM_POINTS_WGSL_V1 } from "./kernels.mjs";

export async function createGpuContext() {
  if (!navigator.gpu) return null;
  const adapter = await navigator.gpu.requestAdapter({ powerPreference: "high-performance" });
  if (!adapter) return null;
  const device = await adapter.requestDevice({ label: "webgpu-smoke" });
  return {
    adapter,
    device,
    hasSubgroups: adapter.features.has("subgroups"),
  };
}

export async function gpuTransformPoints(ctx, positions, matrix) {
  const pointCount = positions.length / 3;
  const module = ctx.device.createShaderModule({
    code: TRANSFORM_POINTS_WGSL_V1,
  });
  const pipeline = await ctx.device.createComputePipelineAsync({
    layout: "auto",
    compute: { module, entryPoint: "main" },
  });

  const uniformData = new ArrayBuffer(256);
  new Float32Array(uniformData).set(matrix, 0);
  new Uint32Array(uniformData)[16] = pointCount;

  const uniformBuffer = ctx.device.createBuffer({
    size: 256,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });
  const inBuffer = ctx.device.createBuffer({
    size: positions.byteLength,
    usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
  });
  const outBuffer = ctx.device.createBuffer({
    size: positions.byteLength,
    usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
  });
  const readBuffer = ctx.device.createBuffer({
    size: positions.byteLength,
    usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
  });

  ctx.device.queue.writeBuffer(uniformBuffer, 0, uniformData);
  ctx.device.queue.writeBuffer(inBuffer, 0, positions);

  const bindGroup = ctx.device.createBindGroup({
    layout: pipeline.getBindGroupLayout(0),
    entries: [
      { binding: 0, resource: { buffer: uniformBuffer } },
      { binding: 1, resource: { buffer: inBuffer } },
      { binding: 2, resource: { buffer: outBuffer } },
    ],
  });

  const encoder = ctx.device.createCommandEncoder();
  const pass = encoder.beginComputePass();
  pass.setPipeline(pipeline);
  pass.setBindGroup(0, bindGroup);
  pass.dispatchWorkgroups(Math.ceil(pointCount / 256));
  pass.end();
  encoder.copyBufferToBuffer(outBuffer, 0, readBuffer, 0, positions.byteLength);
  ctx.device.queue.submit([encoder.finish()]);

  await readBuffer.mapAsync(GPUMapMode.READ);
  const result = new Float32Array(readBuffer.getMappedRange().slice(0));
  readBuffer.unmap();
  [uniformBuffer, inBuffer, outBuffer, readBuffer].forEach((b) => b.destroy());
  return result;
}

export function maxAbsDiff(a, b) {
  let m = 0;
  for (let i = 0; i < a.length; i++) {
    m = Math.max(m, Math.abs(a[i] - b[i]));
  }
  return m;
}
