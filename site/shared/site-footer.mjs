// site/shared/site-footer.mjs
// Shared footer.  import { mountFooter } from './shared/site-footer.mjs'; mountFooter();

const GITHUB = 'https://github.com/reed-soul/wasm-spatial-core';
const NPM = 'https://www.npmjs.com/package/wasm-spatial-core';
const DOCS = 'https://reed-soul.github.io/wasm-spatial-core/docs/';
const BENCH = 'https://reed-soul.github.io/wasm-spatial-core/benchmarks/';

const LOGO_SVG = `<svg viewBox="0 0 64 64" aria-hidden="true">
  <defs><linearGradient id="ft-grad" x1="0" y1="0" x2="1" y2="1">
    <stop offset="0%" stop-color="#00d4ff"/><stop offset="100%" stop-color="#a78bfa"/>
  </linearGradient></defs>
  <g fill="url(#ft-grad)">
    <circle cx="14" cy="16" r="2.4"/><circle cx="14" cy="26" r="2.6"/><circle cx="15" cy="46" r="3"/>
    <circle cx="26" cy="34" r="2.6"/><circle cx="32" cy="22" r="3.2"/>
    <circle cx="40" cy="42" r="2.8"/><circle cx="50" cy="16" r="2.4"/>
  </g>
</svg>`;

export function mountFooter(opts = {}) {
  const year = new Date().getFullYear();
  const footer = document.createElement('footer');
  footer.className = 'footer';
  footer.innerHTML = `
    <div class="footer-inner">
      <div class="footer-brand">
        ${LOGO_SVG}
        <span>wasm-spatial-core · © ${year} · MIT</span>
      </div>
      <div class="footer-links">
        <a href="${GITHUB}" target="_blank" rel="noopener">GitHub</a>
        <a href="${NPM}" target="_blank" rel="noopener">npm</a>
        <a href="${DOCS}" target="_blank" rel="noopener">Docs</a>
        <a href="${BENCH}" target="_blank" rel="noopener">Benchmarks</a>
        <a href="${GITHUB}/blob/master/LICENSE" target="_blank" rel="noopener">License</a>
      </div>
      <div class="footer-built">Built with Rust + WebAssembly</div>
    </div>`;
  document.body.appendChild(footer);
  return footer;
}
