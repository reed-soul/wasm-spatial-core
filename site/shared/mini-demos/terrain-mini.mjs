// Mini-demo 02: Terrain (synthetic heightfield → mesh visualization)
// We synthesize a deterministic heightfield and render it as a shaded
// heightmap, since we cannot rely on a bundled GeoTIFF. Demonstrates the
// terrain visualization pipeline shape (parse → heights → color ramp).

export function run() {
  const host = document.getElementById('demo-terrain');
  if (!host) return;
  host.innerHTML = '<canvas></canvas><div class="demo-hint">synthesizing heightfield…</div>';
  const canvas = host.querySelector('canvas');
  const hint = host.querySelector('.demo-hint');
  const ctx = canvas.getContext('2d');
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const W = host.clientWidth, H = 320;
  canvas.width = W * dpr; canvas.height = H * dpr;
  ctx.scale(dpr, dpr);

  const N = 160;
  // synthesize terrain: layered sine + radial bump
  const h = new Float32Array(N * N);
  let hMin = Infinity, hMax = -Infinity;
  for (let y = 0; y < N; y++) {
    for (let x = 0; x < N; x++) {
      const u = x / N, v = y / N;
      const val =
        Math.sin(u * 9.0) * 0.5 +
        Math.cos(v * 7.0) * 0.4 +
        Math.sin((u + v) * 14.0) * 0.2 +
        Math.exp(-((u - 0.3) ** 2 + (v - 0.4) ** 2) * 18) * 0.9 +
        Math.exp(-((u - 0.7) ** 2 + (v - 0.7) ** 2) * 26) * 0.7;
      h[y * N + x] = val;
      if (val < hMin) hMin = val;
      if (val > hMax) hMax = val;
    }
  }
  const range = hMax - hMin || 1;

  // draw as colored cells (terrain ramp: blue → green → yellow → white peak)
  const cw = W / N, ch = H / N;
  function ramp(t) {
    // t in [0,1]
    if (t < 0.3) return [Math.round(20 + t * 60), Math.round(60 + t * 200), Math.round(120 + t * 200)];       // water→shore
    if (t < 0.55) return [Math.round(60 + (t - 0.3) * 500), Math.round(140 + (t - 0.3) * 200), Math.round(80)]; // green
    if (t < 0.8) return [Math.round(180 + (t - 0.55) * 200), Math.round(160 - (t - 0.55) * 200), Math.round(70)]; // brown
    return [Math.round(240), Math.round(240), Math.round(250)]; // snow
  }
  // simple shading from derivative
  for (let y = 0; y < N; y++) {
    for (let x = 0; x < N; x++) {
      const t = (h[y * N + x] - hMin) / range;
      const dx = (h[y * N + Math.min(N - 1, x + 1)] - h[y * N + Math.max(0, x - 1)]);
      const shade = 1 - dx * 0.6;
      const [r, g, b] = ramp(t);
      ctx.fillStyle = `rgb(${Math.max(0, Math.min(255, r * shade))},${Math.max(0, Math.min(255, g * shade))},${Math.max(0, Math.min(255, b * shade))})`;
      ctx.fillRect(x * cw, y * ch, cw + 1, ch + 1);
    }
  }
  hint.textContent = `synthetic ${N}×${N} heightfield · ${(N * N).toLocaleString()} cells · color ramp + hillshade`;

  // try real WASM terrain if a GPU/context exists — otherwise this is a static viz
  const wsc = window.__wsc;
  if (wsc && wsc.applyColorRamp) {
    hint.textContent += ' · applyColorRamp() available';
  }
}
