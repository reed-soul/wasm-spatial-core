// Mini-demo 04: CRS coordinate transforms — live input → multi-CRS output
// Uses batchWgs84ToGcj02, batchWgs84ToBd09, batchWgs84ToMercator, batchWgs84ToCgcs2000.

export function run() {
  const input = document.getElementById('crs-input');
  const out = document.getElementById('crs-out');
  if (!input || !out) return;
  const wsc = window.__wsc;
  if (!wsc) { out.textContent = 'WASM unavailable'; return; }

  function transform() {
    const parts = input.value.split(/[,\s]+/).map(parseFloat);
    if (parts.length < 2 || parts.some(isNaN)) {
      out.innerHTML = '<span style="color:var(--red)">enter: lng, lat</span>';
      return;
    }
    const [lng, lat] = parts;
    try {
      const gcj = wsc.batchWgs84ToGcj02([lng, lat]);
      const bd = wsc.batchWgs84ToBd09([lng, lat]);
      const mer = wsc.batchWgs84ToMercator([lng, lat]);
      const cgcs = wsc.batchWgs84ToCgcs2000([lng, lat]);
      const row = (label, val, unit = '') => `<div style="display:flex;justify-content:space-between;gap:1rem">
        <span style="color:var(--text-dim)">${label}</span>
        <span style="color:var(--accent)">${val[0].toFixed(6)}, ${val[1].toFixed(6)}${unit}</span></div>`;
      out.innerHTML =
        row('→ GCJ-02', gcj) +
        row('→ BD-09', bd) +
        row('→ CGCS2000', cgcs) +
        `<div style="display:flex;justify-content:space-between;gap:1rem;margin-top:0.4rem;padding-top:0.4rem;border-top:1px solid var(--border)">
          <span style="color:var(--text-dim)">→ Web Mercator</span>
          <span style="color:var(--accent-2)">${mer[0].toFixed(1)}, ${mer[1].toFixed(1)} m</span></div>`;
    } catch (e) {
      out.innerHTML = `<span style="color:var(--red)">${e.message || e}</span>`;
    }
  }
  input.addEventListener('input', transform);
  transform();
}
