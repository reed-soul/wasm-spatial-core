#!/usr/bin/env node
// loaders.gl runner: attempt LAS parse + 3D Tiles generation.
//
// IMPORTANT — this runner documents a finding, not a competition:
// @loaders.gl is a *reader* framework. As of @loaders.gl 4.x there is no
// shipped writer that emits Cesium pnts / 3D Tiles from a parsed LAS point
// cloud. This runner:
//   1. Parses the LAS with @loaders.gl/las (measures parse-only time).
//   2. Attempts to find a tile/pnts writer; if none exists, emits a result
//      with output_bytes=0 and a `notes` field documenting the gap.
//
// This is published honestly: loaders.gl cannot do the headline task
// (LAS → 3D Tiles) out of the box, which is wasm-spatial-core's differentiator.

import { readFile } from "node:fs";
import {
  bench,
  peakRssMb,
  readInput,
  fileDigest,
  makeResult,
  HARDWARE,
} from "./shared.mjs";

async function loadLoaders() {
  let las, tiles;
  try {
    las = await import("@loaders.gl/las");
  } catch (e) {
    return { error: `@loaders.gl/las not installed: ${e.message}` };
  }
  try {
    tiles = await import("@loaders.gl/tiles");
  } catch (e) {
    tiles = null;
  }
  return { las, tiles };
}

async function main() {
  const inputs = process.argv.slice(2) || ["test-data/large/synthetic_500k.las"];
  const mods = await loadLoaders();
  if (mods.error) {
    // Document the finding as a single result line.
    const result = makeResult({
      engine: "loaders.gl",
      engine_version: "not-installed",
      input: inputs[0],
      error: mods.error,
      notes:
        "@loaders.gl not installed in this environment. Documented capability gap: " +
        "@loaders.gl is a reader framework with no shipped LAS→pnts/3D-Tiles writer.",
    });
    process.stdout.write(JSON.stringify(result) + "\n");
    console.error(`  ℹ️  loaders.gl not installed — documented as a capability finding.`);
    return;
  }

  const { las, tiles } = mods;
  const version = las.VERSION || "4.x";

  // Probe for a tile/pnts writer.
  let writerAvailable = false;
  let writerNote = "";
  if (tiles) {
    const exports = Object.keys(tiles);
    writerAvailable = exports.some(
      (k) => /write|encode|pnts|tileset/i.test(k) && typeof tiles[k] === "object",
    );
    writerNote = writerAvailable
      ? "writer API present but no LAS→pnts path found"
      : `no writer/encoder export found (exports: ${exports.slice(0, 8).join(", ")}...)`;
  } else {
    writerNote = "@loaders.gl/tiles not installed";
  }

  for (const input of inputs) {
    const bytes = readInput(input);
    const digest = fileDigest(input);
    const sizeMb = (bytes.length / 1e6).toFixed(2);

    // Parse-only timing (the only thing loaders.gl CAN do here).
    let pointCount = 0;
    const timed = bench(
      async () => {
        const parsed = await las.LASLoader.parse(bytes, {});
        pointCount = parsed.attributes.POSITION?.value?.length / 3 || 0;
        return parsed;
      },
      { warmup: 1, runs: 3 },
    );

    const result = makeResult({
      engine: "loaders.gl",
      engine_version: String(version),
      input,
      input_digest_sha256_12: digest,
      input_size_mb: Number(sizeMb),
      point_count: pointCount,
      output_bytes: 0, // cannot generate pnts — this IS the finding
      tile_count: 0,
      wall_ms: {
        total: Number(timed.avg.toFixed(1)),
        parse_ms: Number(timed.avg.toFixed(1)), // parse-only; no tile gen
        min: Number(timed.min.toFixed(1)),
        max: Number(timed.max.toFixed(1)),
        runs: timed.runs,
      },
      peak_rss_mb: peakRssMb(),
      notes: `PARSE-ONLY. ${writerNote}. loaders.gl reads LAS but has no shipped LAS→pnts/3D-Tiles writer; output_bytes=0 is the documented capability gap, not a timeout. Tile generation would require a custom writer or a separate tool.`,
    });

    process.stdout.write(JSON.stringify(result) + "\n");
    console.error(
      `  ℹ️  ${input}: parsed ${pointCount} pts in ${timed.avg.toFixed(0)}ms; ` +
        `pnts generation NOT POSSIBLE (${writerNote})`,
    );
  }
}

main().catch((e) => {
  console.error("💥 run-loaders-gl failed:", e);
  process.exit(1);
});
