#!/usr/bin/env node
/**
 * loaders.gl row for the LAS → 3D Tiles comparison.
 *
 * @loaders.gl/las can *read* LAS; it does not ship a pnts / 3D Tiles writer.
 * This script documents that capability gap (output_bytes = 0). It never
 * fails the CI job: a missing install is the same documented skip.
 *
 * Usage:
 *   node run-loaders-gl.mjs <input.las>  > results-loaders-gl.jsonl
 */
import { existsSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const require = createRequire(import.meta.url);
const lasPath = resolve(
  process.argv[2] || "bench/head-to-head/fixtures/canonical_500k.las",
);

function emit(row) {
  process.stdout.write(
    JSON.stringify({
      engine: "loaders.gl",
      version: null,
      ok: false,
      wall_ms: null,
      rss_mb: null,
      output_bytes: 0,
      tiles: 0,
      error: null,
      notes:
        "Documented capability gap: @loaders.gl is a reader framework with no shipped LAS→pnts/3D-Tiles writer.",
      ...row,
    }) + "\n",
  );
}

async function tryImportLasLoader() {
  const candidates = [
    "@loaders.gl/las",
    "@loaders.gl/las/dist/index.js",
  ];
  for (const spec of candidates) {
    try {
      const resolved = require.resolve(spec);
      return await import(pathToFileURL(resolved).href);
    } catch {
      // try next
    }
  }
  try {
    return await import("@loaders.gl/las");
  } catch (err) {
    return { error: err };
  }
}

const loaded = await tryImportLasLoader();
if (loaded.error || !loaded.LASLoader) {
  const msg = loaded.error
    ? String(loaded.error.message || loaded.error)
    : "@loaders.gl/las not installed";
  emit({
    version: "not-installed",
    error: `@loaders.gl/las not installed: ${msg}`.slice(0, 120),
    notes:
      "@loaders.gl not installed in this environment. Documented capability gap: @loaders.gl is a reader framework with no shipped LAS→pnts/3D-Tiles writer.",
  });
  process.exit(0);
}

if (!existsSync(lasPath)) {
  emit({
    error: `LAS fixture not found: ${lasPath}`,
  });
  process.exit(0);
}

try {
  const { parse } = await import("@loaders.gl/core").catch(() => ({}));
  if (typeof parse === "function") {
    await parse(readFileSync(lasPath), loaded.LASLoader);
  }
} catch {
  // Parse failures still do not produce 3D Tiles; the gap stands.
}

emit({
  version: loaded.VERSION || "installed",
  error: null,
  notes:
    "@loaders.gl/las can read LAS but does not write pnts/3D Tiles. output_bytes = 0 is a capability gap, not a timeout.",
});
process.exit(0);
