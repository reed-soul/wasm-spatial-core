// site/shared/hero-particles.mjs
// Lightweight Canvas-2D point-cloud particle field for the hero background.
// Renders sparse→dense scatter with cyan→violet tinting. Does NOT block WASM init.
// Respects prefers-reduced-motion (renders a single static frame).

export function startHeroParticles(canvas) {
  if (!canvas) return () => {};
  const ctx = canvas.getContext('2d', { alpha: true });
  if (!ctx) return () => {};

  const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  let dpr = Math.min(window.devicePixelRatio || 1, 2);
  let w = 0, h = 0, raf = 0;
  const POINTS = 160;
  const pts = [];

  function resize() {
    const rect = canvas.getBoundingClientRect();
    w = rect.width; h = rect.height;
    canvas.width = Math.floor(w * dpr);
    canvas.height = Math.floor(h * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }

  function seed() {
    pts.length = 0;
    for (let i = 0; i < POINTS; i++) {
      // bias toward a loose "W" path center spread
      const cx = 0.5 + 0.0;
      pts.push({
        x: Math.random() * w,
        y: Math.random() * h,
        r: 0.6 + Math.random() * 2.2,
        vx: (Math.random() - 0.5) * 0.12,
        vy: (Math.random() - 0.5) * 0.12,
        t: Math.random(),                     // mix factor for color
        a: 0.18 + Math.random() * 0.5,        // alpha
        tw: 0.4 + Math.random() * 0.8,        // twinkle speed
        ph: Math.random() * Math.PI * 2,      // phase
      });
    }
  }

  let frame = 0;
  function draw() {
    ctx.clearRect(0, 0, w, h);
    frame += 1;
    for (const p of pts) {
      if (!reduce) {
        p.x += p.vx; p.y += p.vy;
        if (p.x < -5) p.x = w + 5; else if (p.x > w + 5) p.x = -5;
        if (p.y < -5) p.y = h + 5; else if (p.y > h + 5) p.y = -5;
      }
      const tw = reduce ? p.a : (p.a * (0.6 + 0.4 * Math.sin(frame * 0.03 * p.tw + p.ph)));
      // cyan → violet by t
      const r = Math.round(0 + p.t * 167);
      const g = Math.round(212 - p.t * 73);
      const b = Math.round(255 - p.t * 2);
      ctx.beginPath();
      ctx.fillStyle = `rgba(${r},${g},${b},${tw})`;
      ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2);
      ctx.fill();
    }
    // faint connection lines between near points
    ctx.lineWidth = 0.5;
    for (let i = 0; i < pts.length; i++) {
      for (let j = i + 1; j < pts.length; j++) {
        const dx = pts[i].x - pts[j].x, dy = pts[i].y - pts[j].y;
        const d2 = dx * dx + dy * dy;
        if (d2 < 110 * 110) {
          const alpha = (1 - Math.sqrt(d2) / 110) * 0.07;
          ctx.strokeStyle = `rgba(0,212,255,${alpha})`;
          ctx.beginPath();
          ctx.moveTo(pts[i].x, pts[i].y);
          ctx.lineTo(pts[j].x, pts[j].y);
          ctx.stroke();
        }
      }
    }
    if (!reduce) raf = requestAnimationFrame(draw);
  }

  resize(); seed(); draw();

  const onResize = () => { resize(); seed(); if (reduce) draw(); };
  window.addEventListener('resize', onResize);

  // pause when offscreen for perf
  const io = new IntersectionObserver((entries) => {
    for (const e of entries) {
      if (reduce) return;
      if (e.isIntersecting && !raf) raf = requestAnimationFrame(draw);
      else if (!e.isIntersecting && raf) { cancelAnimationFrame(raf); raf = 0; }
    }
  });
  io.observe(canvas);

  return () => { cancelAnimationFrame(raf); window.removeEventListener('resize', onResize); io.disconnect(); };
}
