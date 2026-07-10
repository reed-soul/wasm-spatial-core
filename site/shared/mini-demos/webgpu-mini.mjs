// Mini-demo 05: WebGPU compute — probe GPU availability and run a quick
// WASM-vs-GPU timing comparison if a transform kernel is available.

export function run() {
  const status = document.getElementById('gpu-status');
  const result = document.getElementById('gpu-result');
  if (!status || !result) return;
  const wsc = window.__wsc;

  const gpuOk = !!(navigator.gpu && (wsc ? wsc.supportsWebGpu?.() : true));

  status.innerHTML = `<span style="display:inline-flex;align-items:center;gap:0.4rem">
    <span style="width:8px;height:8px;border-radius:50%;background:${gpuOk ? 'var(--green)' : 'var(--orange)'};display:inline-block"></span>
    WebGPU ${gpuOk ? 'available' : 'unavailable — WASM fallback active'}</span>`;

  if (wsc && wsc.transformPointCloud && gpuOk) {
    // quick GPU vs WASM transform on 100k points
    const n = 100000;
    const pts = new Float32Array(n * 3);
    for (let i = 0; i < n * 3; i++) pts[i] = (Math.random() - 0.5) * 100;
    const m = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1];
    try {
      const tg0 = performance.now();
      wsc.transformPointCloud(pts, m, { preferGpu: true });
      const tg = performance.now() - tg0;
      const tw0 = performance.now();
      wsc.transformPointCloud(pts, m, { preferGpu: false });
      const tw = performance.now() - tw0;
      result.innerHTML = `GPU: <span style="color:var(--green)">${tg.toFixed(1)} ms</span> · WASM: <span style="color:var(--accent)">${tw.toFixed(1)} ms</span><br>${n.toLocaleString()} points · Mat4×vec3 transform`;
    } catch (e) {
      result.textContent = 'kernel probe failed: ' + (e.message || e);
    }
  } else {
    result.innerHTML = `<span style="color:var(--text-muted)">WGSL kernels: point transform · heightfield flatten · mesh quadric.<br>Automatic WASM fallback when no discrete GPU.<br><span style="color:var(--accent-2)">Open the WebGPU benchmark → full matrix on discrete GPU.</span></span>`;
  }
}
