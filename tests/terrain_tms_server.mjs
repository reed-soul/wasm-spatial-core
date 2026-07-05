// Test-time HTTP server for the W3.6 CesiumTerrainProvider acceptance test.
//
// On startup it:
//   1. Regenerates the TMS terrain pyramid (layer.json + {z}/{x}/{y}.terrain)
//      via terrain_tms_generate.mjs::generateAll, so the artifacts are always
//      fresh against the current WASM build.
//   2. Serves the repo root over HTTP on PORT, with a small MIME map.
//
// The Playwright webServer hook (playwright.config.mjs) launches this file
// instead of a plain static server so we don't need a multi-step "generate
// then serve" CI script. Artifacts live under tests/fixtures/terrain-tms/
// (gitignored) and are mapped to /terrain-tms/* for the test page.

import { createServer } from 'node:http';
import { readFile, statSync } from 'node:fs';
import { extname, join, normalize, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { dirname } from 'node:path';

import { generateAll } from './terrain_tms_generate.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '..');
const PORT = parseInt(process.env.PORT || '8090', 10);
const OUT_DIR = join(REPO_ROOT, 'tests', 'fixtures', 'terrain-tms');

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'application/javascript; charset=utf-8',
  '.mjs': 'application/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.wasm': 'application/wasm',
  '.terrain': 'application/octet-stream',
  '.css': 'text/css; charset=utf-8',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.svg': 'image/svg+xml',
  '.tif': 'image/tiff',
  '.tiff': 'image/tiff',
};

function serveStatic(req, res) {
  let urlPath = decodeURIComponent(req.url.split('?')[0]);

  // Map /terrain-tms/* to the generated artifacts dir.
  if (urlPath.startsWith('/terrain-tms/')) {
    const rel = urlPath.slice('/terrain-tms/'.length);
    const abs = normalize(join(OUT_DIR, rel));
    if (!abs.startsWith(OUT_DIR)) {
      res.writeHead(403);
      res.end('forbidden');
      return;
    }
    serveFile(abs, res);
    return;
  }

  // Everything else is served from the repo root (so /pkg/, /tests/fixtures/*,
  // /examples/*, etc. resolve as expected).
  const abs = normalize(join(REPO_ROOT, urlPath));
  if (!abs.startsWith(REPO_ROOT)) {
    res.writeHead(403);
    res.end('forbidden');
    return;
  }
  serveFile(abs, res);
}

function serveFile(abs, res) {
  let isDir = false;
  try {
    isDir = statSync(abs).isDirectory();
  } catch {
    res.writeHead(404);
    res.end(`not found: ${abs}`);
    return;
  }
  const filePath = isDir ? join(abs, 'index.html') : abs;
  readFile(filePath, (err, data) => {
    if (err) {
      res.writeHead(404);
      res.end(`not found: ${filePath}`);
      return;
    }
    const mime = MIME[extname(filePath).toLowerCase()] || 'application/octet-stream';
    res.writeHead(200, { 'Content-Type': mime });
    res.end(data);
  });
}

async function main() {
  // 1. Generate artifacts (always fresh).
  console.log(`[terrain_tms_server] generating TMS pyramid → ${OUT_DIR}`);
  const result = await generateAll(OUT_DIR, { clean: true });
  console.log(
    `[terrain_tms_server] pyramid ready: ${result.tiles.length} tiles, ` +
      `bounds [${result.bounds.join(', ')}]`,
  );

  // 2. Start HTTP server.
  const server = createServer(serveStatic);
  server.listen(PORT, () => {
    console.log(`[terrain_tms_server] serving repo root on http://localhost:${PORT}`);
    console.log(`[terrain_tms_server] test page: http://localhost:${PORT}/tests/fixtures/cesium-terrain-loader.html`);
    console.log(`[terrain_tms_server] layer.json: http://localhost:${PORT}/terrain-tms/layer.json`);
  });

  // Graceful shutdown — Playwright sends SIGTERM when the test run finishes.
  for (const sig of ['SIGINT', 'SIGTERM']) {
    process.on(sig, () => {
      console.log(`[terrain_tms_server] received ${sig}, shutting down`);
      server.close(() => process.exit(0));
    });
  }
}

main().catch((err) => {
  console.error('[terrain_tms_server] fatal:', err);
  process.exit(1);
});
