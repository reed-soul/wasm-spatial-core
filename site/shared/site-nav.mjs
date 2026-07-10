// site/shared/site-nav.mjs
// Fixed top navigation — shared across the landing page and all demo shells.
// Usage (ESM):  import { mountNav } from './shared/site-nav.mjs'; mountNav();
// Usage (global, demo pages): <script type="module" src="/site/shared/site-nav.mjs" data-mount></script>
//
// Options via data-* on the <html> or script tag:
//   data-home      — URL to the site home (default: relative "/site/" resolved)
//   data-back      — if "true", show a "← Back to site" link (demo pages)

const HOME = '/site/';
const GITHUB = 'https://github.com/reed-soul/wasm-spatial-core';
const NPM = 'https://www.npmjs.com/package/wasm-spatial-core';
// Docs + Benchmarks are site-relative so they work on both the local dev
// server and the deployed gh-pages site. Docs (typedoc) lives at /docs/ on
// gh-pages; the in-repo browser benchmark is at /bench/browser/index.html.
const DOCS = '../docs/';
const BENCH = '../bench/browser/index.html';

const LOGO_SVG = `<svg viewBox="0 0 64 64" role="img" aria-label="wasm-spatial-core">
  <defs><linearGradient id="nav-grad" x1="0" y1="0" x2="1" y2="1">
    <stop offset="0%" stop-color="#00d4ff"/><stop offset="100%" stop-color="#a78bfa"/>
  </linearGradient></defs>
  <rect x="2" y="2" width="60" height="60" rx="12" fill="#0d1117" stroke="url(#nav-grad)" stroke-width="1.5" opacity="0.9"/>
  <g fill="url(#nav-grad)">
    <circle cx="14" cy="16" r="2.4"/><circle cx="14" cy="26" r="2.6"/><circle cx="14" cy="36" r="2.8"/><circle cx="15" cy="46" r="3"/>
    <circle cx="24" cy="42" r="2.8"/><circle cx="26" cy="34" r="2.6"/>
    <circle cx="32" cy="22" r="3.2"/><circle cx="32" cy="30" r="2.6"/>
    <circle cx="38" cy="34" r="2.6"/><circle cx="40" cy="42" r="2.8"/>
    <circle cx="49" cy="46" r="3"/><circle cx="50" cy="36" r="2.8"/><circle cx="50" cy="26" r="2.6"/><circle cx="50" cy="16" r="2.4"/>
  </g>
  <g fill="#00d4ff" opacity="0.35"><circle cx="22" cy="14" r="1.2"/><circle cx="42" cy="14" r="1.2"/><circle cx="20" cy="50" r="1.1"/><circle cx="44" cy="50" r="1.1"/></g>
</svg>`;

function resolveHome() {
  // On the production site, site/ is copied to _site/site/ then index.html
  // redirects at root. Prefer a sibling-relative "home" if provided.
  const declared = document.documentElement.getAttribute('data-home');
  if (declared) return declared;
  // Heuristic: if we're under /site/ already, home is "./" or "/site/"
  const p = location.pathname.replace(/\/+$/, '');
  if (p.endsWith('/site') || p.includes('/site/')) return p.split('/site')[0] + '/site/';
  return HOME;
}

export function mountNav(opts = {}) {
  const showBack = opts.back ?? document.documentElement.getAttribute('data-back') === 'true';
  const home = opts.home ?? resolveHome();
  const nav = document.createElement('nav');
  nav.className = 'nav';
  nav.setAttribute('aria-label', 'Primary');
  nav.innerHTML = `
    <div class="nav-inner">
      <a class="nav-brand" href="${home}index.html" aria-label="wasm-spatial-core home">
        ${LOGO_SVG}<span>wasm-spatial-core</span>
        <span class="nav-version">v0.9.0</span>
      </a>
      <div class="nav-links">
        ${showBack ? `<a class="nav-back" href="${home}index.html#demos">← Back to site</a>` : ''}
        <a href="${home}index.html#demos">Demos</a>
        <a href="${BENCH}">Benchmarks</a>
        <a href="${DOCS}">Docs</a>
        <a href="${home}llms.txt" title="Project summary for AI agents (llms.txt)">llms.txt</a>
        <a href="${GITHUB}">GitHub</a>
        <a class="nav-cta" href="${NPM}">npm</a>
      </div>
    </div>`;
  document.body.prepend(nav);

  // Add scrolled state on scroll
  const onScroll = () => nav.classList.toggle('scrolled', window.scrollY > 8);
  onScroll();
  window.addEventListener('scroll', onScroll, { passive: true });

  // Spacer so fixed nav doesn't overlap content (demo pages)
  if (showBack) {
    const spacer = document.createElement('div');
    spacer.style.height = 'var(--nav-h)';
    nav.after(spacer);
  }
  return nav;
}

// Auto-mount when loaded as <script type="module" data-mount>
const auto = document.currentScript;
if (auto && auto.hasAttribute('data-mount')) {
  mountNav();
}
