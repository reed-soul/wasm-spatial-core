// Mini-demo 01: Point cloud → 3D Tiles
// Parses the bundled sample LAS, renders a downsampled Canvas-2D projection,
// and reports live octree + tileset generation timing.
// Consumes window.__wsc (the initialized wasm module).
//
// API (v0.7+ getter-based):
//   parseLasPoints(bytes)  → LasPointCloud with .pointCount getter, .positions getter
//   generateTileset(cloud, depth) → { tileCount(), ... }

export function run() {
  const host = document.getElementById('demo-pointcloud');
  if (!host) return;
  const wsc = window.__wsc;
  if (!wsc) { host.querySelector('.demo-hint').textContent = 'WASM unavailable'; return; }

  host.innerHTML = '<canvas></canvas><div class="demo-hint">fetching sample LAS…</div>';
  const canvas = host.querySelector('canvas');
  const hint = host.querySelector('.demo-hint');
  const ctx = canvas.getContext('2d');
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const W = host.clientWidth, H = 320;
  canvas.width = W * dpr; canvas.height = H * dpr;
  ctx.scale(dpr, dpr);

  // Resolve relative to the document (the site page at /site/index.html),
  // so ../examples/ reaches the repo's examples/ dir in both dev and built site.
  const LAS_URL = new URL('../examples/sample-data/demo_terrain.las', document.baseURI).href;
  fetch(LAS_URL).then(r => r.arrayBuffer()).then(async (buf) => {
    const t0 = performance.now();
    const bytes = new Uint8Array(buf);
    const cloud = wsc.parseLasPoints(bytes);
    const tParse = performance.now() - t0;

    const n = cloud.pointCount;
    // .positions is a getter returning a WASM-owned Float32Array view
    const positions = new Float32Array(cloud.positions);
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    const xs = new Float32Array(n), ys = new Float32Array(n);
    for (let i = 0; i < n; i++) {
      const x = positions[i * 3], y = positions[i * 3 + 1];
      xs[i] = x; ys[i] = y;
      if (x < minX) minX = x; if (x > maxX) maxX = x;
      if (y < minY) minY = y; if (y > maxY) maxY = y;
    }
    const rangeX = (maxX - minX) || 1, rangeY = (maxY - minY) || 1;
    const s = Math.min((W - 40) / rangeX, (H - 40) / rangeY);

    ctx.fillStyle = '#07090f'; ctx.fillRect(0, 0, W, H);

    // octree + tileset timing (the headline)
    // generateTileset(positions, max_points_per_node, max_depth, colors?)
    const t1 = performance.now();
    let tiles = 0;
    try {
      const ts = wsc.generateTileset(cloud.positions, 10000, 8);
      // tileCount is a getter (not a method) on TilesetResult
      tiles = ts.tileCount;
      ts.free?.();
    } catch (e) { /* fall through with tiles=0 */ }
    const tOct = performance.now() - t1;

    // draw points progressively
    let drawn = 0;
    const step = Math.max(1, Math.floor(n / 1000));
    function frame() {
      const end = Math.min(drawn + 4000, n);
      for (let i = drawn; i < end; i += step) {
        const px = 20 + (xs[i] - minX) * s;
        const py = 20 + (ys[i] - minY) * s;
        const t = i / n;
        ctx.fillStyle = `rgba(${Math.round(t * 167)},${Math.round(212 - t * 73)},255,0.7)`;
        ctx.fillRect(px, py, 1.6, 1.6);
      }
      drawn = end;
      if (drawn < n) requestAnimationFrame(frame);
      else hint.textContent = `${n.toLocaleString()} pts · parse ${tParse.toFixed(0)}ms · octree+tileset ${tOct.toFixed(0)}ms · ${tiles} tiles`;
    }
    frame();
  }).catch(() => { hint.textContent = 'sample LAS unavailable'; });
}
