// Mini-demo 03: Vector — R-tree spatial index with bbox query animation
// Generates points, builds a SpatialIndex (constructor takes Float64Array of
// [lng,lat,lng,lat,...]), and animates a moving bounding box showing live
// bbox-query results (searchBBox returns Uint32Array of point IDs).

export function run() {
  const host = document.getElementById('demo-vector');
  if (!host) return;
  const wsc = window.__wsc;
  host.innerHTML = '<canvas></canvas><div class="demo-hint">building R-tree…</div>';
  const canvas = host.querySelector('canvas');
  const hint = host.querySelector('.demo-hint');
  const ctx = canvas.getContext('2d');
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const W = host.clientWidth, H = 320;
  canvas.width = W * dpr; canvas.height = H * dpr;
  ctx.scale(dpr, dpr);

  const DATA_URL = new URL('../examples/data/china_cities.json', document.baseURI).href;
  fetch(DATA_URL).then(r => r.json()).then(data => {
    const items = (data.cities || data).slice(0, 400);
    const pts = items.map(c => ({ x: c[0] ?? c.lng, y: c[1] ?? c.lat }));
    init(pts);
  }).catch(() => {
    const pts = [];
    for (let i = 0; i < 400; i++) pts.push({ x: Math.random() * 100, y: Math.random() * 60 });
    init(pts);
  });

  function init(pts) {
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const p of pts) { if (p.x < minX) minX = p.x; if (p.x > maxX) maxX = p.x; if (p.y < minY) minY = p.y; if (p.y > maxY) maxY = p.y; }
    const pad = 24;
    const s = Math.min((W - pad * 2) / ((maxX - minX) || 1), (H - pad * 2) / ((maxY - minY) || 1));

    // Build WASM R-tree: constructor takes Float64Array [x0,y0,x1,y1,...]
    let index = null, indexed = 0;
    if (wsc && wsc.SpatialIndex) {
      try {
        const coords = new Float64Array(pts.length * 2);
        for (let i = 0; i < pts.length; i++) { coords[i * 2] = pts[i].x; coords[i * 2 + 1] = pts[i].y; }
        index = new wsc.SpatialIndex(coords);
        indexed = index.size();
      } catch (e) { index = null; console.warn('SpatialIndex build failed', e); }
    }

    let t = 0;
    function frame() {
      t += 0.012;
      ctx.fillStyle = '#07090f'; ctx.fillRect(0, 0, W, H);

      // draw all points
      ctx.fillStyle = 'rgba(0,212,255,0.45)';
      for (const p of pts) {
        const px = pad + (p.x - minX) * s, py = pad + (p.y - minY) * s;
        ctx.fillRect(px, py, 2, 2);
      }

      // animated bbox sweeping across the map
      const bw = (maxX - minX) * 0.28, bh = (maxY - minY) * 0.5;
      const bx = minX + (Math.sin(t) * 0.5 + 0.5) * ((maxX - minX) - bw);
      const by = minY + (Math.cos(t * 0.7) * 0.5 + 0.5) * ((maxY - minY) - bh);

      // query — returns Uint32Array of point IDs
      let ids = [];
      if (index) {
        try { ids = index.searchBBox(bx, by, bx + bw, by + bh) || []; } catch {}
      } else {
        ids = pts.map((p, i) => (p.x >= bx && p.x <= bx + bw && p.y >= by && p.y <= by + bh) ? i : -1).filter(i => i >= 0);
      }

      // draw bbox
      const rx = pad + (bx - minX) * s, ry = pad + (by - minY) * s;
      const rw = bw * s, rh = bh * s;
      ctx.strokeStyle = '#a78bfa'; ctx.lineWidth = 1.5;
      ctx.strokeRect(rx, ry, rw, rh);
      ctx.fillStyle = 'rgba(167,139,250,0.07)'; ctx.fillRect(rx, ry, rw, rh);

      // highlight matches (ids are point indices)
      ctx.fillStyle = '#3fb950';
      for (const id of ids) {
        const p = pts[id]; if (!p) continue;
        const px = pad + (p.x - minX) * s, py = pad + (p.y - minY) * s;
        ctx.beginPath(); ctx.arc(px, py, 3, 0, Math.PI * 2); ctx.fill();
      }
      hint.textContent = `${indexed.toLocaleString()} pts indexed · R-tree bbox query: ${ids.length} hits (green)`;
      requestAnimationFrame(frame);
    }
    frame();
  }
}
