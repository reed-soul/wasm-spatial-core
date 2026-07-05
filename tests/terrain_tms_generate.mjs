// W3.6 acceptance test — TMS terrain pyramid generator.
//
// Loads the GeoTIFF fixture via the wasm-spatial-core `parseGeotiff` API,
// builds a TMS {z}/{x}/{y}.terrain pyramid + layer.json via
// `encodeTerrainTmsPyramid`, and writes the artifacts to a target directory
// (default tests/fixtures/terrain-tms/). The output is what a
// CesiumTerrainProvider fetches at runtime — the Playwright test drives that
// load to prove the bytes are spec-conformant.
//
// Usage:
//   node tests/terrain_tms_generate.mjs                  # default fixture + out-dir
//   node tests/terrain_tms_generate.mjs --out-dir /tmp/x # custom output
//   node tests/terrain_tms_generate.mjs --fixture other.tif
//
// Exports `generateAll(outDir, opts)` for programmatic use by the Playwright
// webServer hook (see tests/terrain_tms_server.mjs).

import { existsSync, mkdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { dirname, join, resolve } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '..');

// Parse --out-dir / --fixture flags (very small parser; this is a test helper).
function parseArgs(argv) {
  const out = { outDir: null, fixture: null };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--out-dir') out.outDir = argv[++i];
    else if (a?.startsWith('--out-dir=')) out.outDir = a.slice('--out-dir='.length);
    else if (a === '--fixture') out.fixture = argv[++i];
    else if (a?.startsWith('--fixture=')) out.fixture = a.slice('--fixture='.length);
  }
  return out;
}

// Resolve the WASM package directory. Mirror wasm_smoke_test.mjs's resolution
// so this works both locally (pkg-node/ after `wasm-pack build`) and in CI.
function resolvePkgDir() {
  const envPkg = process.env.WASM_PKG_DIR;
  const candidates = [
    envPkg && resolve(envPkg),
    join(REPO_ROOT, 'pkg-node'),
    join(REPO_ROOT, 'pkg'),
  ].filter(Boolean);
  for (const c of candidates) {
    if (c && existsSync(join(c, 'wasm_spatial_core.js'))) return resolve(c);
  }
  throw new Error(
    `terrain_tms_generate: no WASM package found (looked at ${candidates.join(', ')}). ` +
      'Run `wasm-pack build --target nodejs --release --out-dir pkg-node --features point-cloud,geotiff` first.',
  );
}

async function loadWasm() {
  const pkgDir = resolvePkgDir();
  const wasmJs = pathToFileURL(join(pkgDir, 'wasm_spatial_core.js')).href;
  const mod = await import(wasmJs);
  // Nodejs target: auto-initializes on import, no manual default() call.
  if (typeof mod.encodeTerrainTmsPyramid !== 'function') {
    throw new Error(
      'terrain_tms_generate: encodeTerrainTmsPyramid not exported — ' +
        'was the WASM built with --features geotiff?',
    );
  }
  return mod;
}

/**
 * Generate a TMS terrain pyramid from a GeoTIFF fixture and write all
 * artifacts (layer.json + {z}/{x}/{y}.terrain) under `outDir`.
 *
 * @param {string} outDir Absolute path to the output directory; created if missing.
 * @param {{fixture?: string, clean?: boolean, wasm?: any}} opts
 *   - fixture: path to source GeoTIFF (default: tests/fixtures/terrain_256x256.tif)
 *   - clean: if true (default), wipe outDir before writing
 *   - wasm: preloaded module (skips the dynamic import; used by the server hook)
 * @returns {Promise<{layerJsonPath: string, tiles: Array<{path: string, bytes: number}>, bounds: number[]}>}
 */
export async function generateAll(outDir, opts = {}) {
  const fixture = opts.fixture || join(REPO_ROOT, 'tests', 'fixtures', 'terrain_256x256.tif');
  const clean = opts.clean !== false;
  const mod = opts.wasm || (await loadWasm());

  if (!existsSync(fixture)) {
    throw new Error(`terrain_tms_generate: fixture not found: ${fixture}`);
  }

  // Parse fixture GeoTIFF.
  const tifBytes = readFileSync(fixture);
  const info = mod.parseGeotiff(tifBytes);
  const w = info.width;
  const h = info.height;
  const bounds = Array.from(info.bounds);
  const elev = info.elevation;
  if (w < 2 || h < 2) {
    throw new Error(`terrain_tms_generate: fixture ${w}×${h} too small (need ≥2×2)`);
  }

  // Build the TMS pyramid. Pass an empty Float64Array to auto-derive ECEF center
  // from the bounds + mean height (matches the existing demo's convention).
  const center = new Float64Array(0);
  const pyramid = mod.encodeTerrainTmsPyramid(elev, w, h, info.bounds, center, 1);

  // Prepare output directory.
  if (clean && existsSync(outDir)) rmSync(outDir, { recursive: true, force: true });
  mkdirSync(outDir, { recursive: true });

  // Write layer.json at the layer root.
  const layerJsonPath = join(outDir, 'layer.json');
  writeFileSync(layerJsonPath, pyramid.layerJson, 'utf8');

  // Write each tile under {z}/{x}/{y}.terrain, creating subdirs as needed.
  const written = [];
  for (let i = 0; i < pyramid.tileCount; i++) {
    const rel = pyramid.tilePath(i); // e.g. "1/0/0.terrain"
    const bytes = pyramid.tile(i);
    const abs = join(outDir, rel);
    mkdirSync(dirname(abs), { recursive: true });
    writeFileSync(abs, bytes);
    written.push({ path: rel, bytes: bytes.length });
  }

  return { layerJsonPath, tiles: written, bounds };
}

// CLI entry point — only run when invoked directly, not when imported.
const isMain = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  const args = parseArgs(process.argv);
  const outDir = args.outDir
    ? resolve(args.outDir)
    : join(REPO_ROOT, 'tests', 'fixtures', 'terrain-tms');
  generateAll(outDir, { fixture: args.fixture ? resolve(args.fixture) : undefined })
    .then(({ layerJsonPath, tiles, bounds }) => {
      const totalBytes = tiles.reduce((s, t) => s + t.bytes, 0);
      console.log(`✅ W3.6 TMS terrain pyramid generated`);
      console.log(`   layer.json → ${layerJsonPath}`);
      console.log(`   bounds     → [${bounds.join(', ')}]`);
      console.log(`   tiles      → ${tiles.length} (${totalBytes} bytes total)`);
      for (const t of tiles) {
        console.log(`     ${t.path.padEnd(20)} ${t.bytes} bytes`);
      }
    })
    .catch((err) => {
      console.error('❌ terrain_tms_generate failed:', err.message || err);
      process.exit(1);
    });
}
