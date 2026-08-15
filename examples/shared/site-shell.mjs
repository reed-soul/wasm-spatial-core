// examples/shared/site-shell.mjs
// Unifies demo pages with the site brand: injects the top nav (with a
// "← Back to site" link) and lifts the legacy :root tokens to the new palette
// by importing the global stylesheet AFTER the page's own <style>.
//
// Usage (add to each demo page, before </body>):
//   <script type="module">
//     import { mountDemoShell } from '../shared/site-shell.mjs';
//     mountDemoShell({ title: 'Point Cloud Workbench' });
//   </script>
//
// It is non-destructive: the demo's own markup and page-specific CSS stay
// intact. Only the shared nav + token refresh are added.

const NAV_HTML = (home, root) => `
  <div class="nav-inner">
    <a class="nav-brand" href="${home}index.html" aria-label="wasm-spatial-core home">
      <svg viewBox="0 0 64 64" width="28" height="28" role="img" aria-label="logo">
        <defs><linearGradient id="sh-grad" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stop-color="#00d4ff"/><stop offset="100%" stop-color="#a78bfa"/>
        </linearGradient></defs>
        <rect x="2" y="2" width="60" height="60" rx="12" fill="#0d1117" stroke="url(#sh-grad)" stroke-width="1.5" opacity="0.9"/>
        <g fill="url(#sh-grad)">
          <circle cx="14" cy="16" r="2.4"/><circle cx="14" cy="26" r="2.6"/><circle cx="15" cy="46" r="3"/>
          <circle cx="26" cy="34" r="2.6"/><circle cx="32" cy="22" r="3.2"/>
          <circle cx="40" cy="42" r="2.8"/><circle cx="50" cy="16" r="2.4"/>
        </g>
      </svg>
      <span>wasm-spatial-core</span>
      <span class="nav-version">v0.10.0</span>
    </a>
    <div class="nav-links">
      <a class="nav-back" href="${home}index.html#demos">← Back to site</a>
      <a href="${home}index.html#demos">Demos</a>
      <a href="${root}bench/browser/index.html">Benchmarks</a>
      <a href="${root}docs/">Docs</a>
      <a href="${home}llms.txt" title="Project summary for AI agents (llms.txt)">llms.txt</a>
      <a href="https://github.com/reed-soul/wasm-spatial-core">GitHub</a>
      <a class="nav-cta" href="https://www.npmjs.com/package/wasm-spatial-core">npm</a>
    </div>
  </div>`;

export function mountDemoShell(opts = {}) {
  // Resolve site home relative to this demo page.
  // Demo pages live at examples/<name>/index.html → home is ../../site/
  const p = location.pathname.replace(/\/+$/, '');
  let home;
  if (p.endsWith('/examples/index.html') || p.endsWith('/examples')) {
    home = '../site/';
  } else if (p.includes('/examples/')) {
    home = '../../site/';
  } else {
    home = '/site/';
  }
  // Site root is one level above home (home = <root>site/, root = <root>)
  const root = home.replace(/site\/$/, '');

  // 1. Inject the global stylesheet (this refreshes tokens + brings .nav styles).
  //    Loaded last so it can override legacy :root values.
  const link = document.createElement('link');
  link.rel = 'stylesheet';
  link.href = (p.includes('/examples/') && !p.endsWith('/examples/index.html'))
    ? '../../site/site.css'
    : '../site/site.css';
  document.head.appendChild(link);

  // 2. Inject the nav bar.
  const nav = document.createElement('nav');
  nav.className = 'nav';
  nav.setAttribute('aria-label', 'Primary');
  nav.innerHTML = NAV_HTML(home, root);
  document.body.prepend(nav);

  // Scrolled state + title handling
  const onScroll = () => nav.classList.toggle('scrolled', window.scrollY > 8);
  onScroll();
  window.addEventListener('scroll', onScroll, { passive: true });

  // Push page content below the fixed nav by adding top padding to <body>.
  document.body.style.paddingTop = 'var(--nav-h)';
  if (opts.title) {
    nav.setAttribute('aria-label', opts.title);
  }
}
