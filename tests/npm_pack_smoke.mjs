#!/usr/bin/env node
// 1.0 gate (ROADMAP_1_0.md §5): the npm package installs and its subpath
// exports resolve. Default mode packs the *local* `npm/` tree (CI stages
// wasm-pack artifacts first). Set NPM_FROM_REGISTRY=1 to install
// `wasm-spatial-core` from the public registry instead — that is the
// post-publish 1.0 check.
//
// Usage:
//   node tests/npm_pack_smoke.mjs
//   NPM_FROM_REGISTRY=1 node tests/npm_pack_smoke.mjs
//   NPM_FROM_REGISTRY=1 NPM_SPEC=wasm-spatial-core@0.10.2 node tests/npm_pack_smoke.mjs

import { execFileSync, execSync } from "node:child_process";
import {
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..");
const npmDir = join(repoRoot, "npm");
const fromRegistry = process.env.NPM_FROM_REGISTRY === "1";
const spec = process.env.NPM_SPEC || "wasm-spatial-core";

function log(msg) {
  console.log(`  ${msg}`);
}

function fail(msg) {
  console.error(`  ❌ ${msg}`);
  process.exit(1);
}

function run(cmd, cwd) {
  return execSync(cmd, { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
}

function packLocalTarball() {
  const pkgJson = JSON.parse(readFileSync(join(npmDir, "package.json"), "utf8"));
  for (const rel of ["pkg/wasm_spatial_core.js", "pkg-node/wasm_spatial_core.js"]) {
    if (!existsSync(join(npmDir, rel))) {
      fail(
        `missing ${rel} — stage wasm-pack output into npm/pkg and npm/pkg-node first`,
      );
    }
  }
  if (!existsSync(join(npmDir, "abort.js"))) {
    log("abort.js missing — compiling npm TypeScript entries");
    run("npm install --ignore-scripts", npmDir);
    run("npm run build:entries", npmDir);
  }
  log(`packing npm/ (${pkgJson.name}@${pkgJson.version}, ignore-scripts)`);
  const out = execFileSync("npm", ["pack", "--ignore-scripts", "--json"], {
    cwd: npmDir,
    encoding: "utf8",
  });
  const parsed = JSON.parse(out);
  const filename = Array.isArray(parsed) ? parsed[0].filename : parsed.filename;
  const tarball = join(npmDir, filename);
  if (!existsSync(tarball)) {
    fail(`npm pack did not write ${tarball}`);
  }
  return tarball;
}

async function assertSubpaths(pkgRoot) {
  const pkgJson = JSON.parse(readFileSync(join(pkgRoot, "package.json"), "utf8"));
  const exportsMap = pkgJson.exports || {};
  for (const sub of ["./abort", "./webgpu", "./node", "./draco"]) {
    if (!exportsMap[sub]) {
      fail(`package.json exports missing ${sub}`);
    }
    log(`exports[${sub}] → ${exportsMap[sub].import || JSON.stringify(exportsMap[sub])}`);
  }

  const abort = await import(pathToFileURL(join(pkgRoot, "abort.js")).href);
  if (typeof abort.createAbortChecker !== "function") {
    fail("wasm-spatial-core/abort did not export createAbortChecker");
  }
  const checker = abort.createAbortChecker();
  if (checker() !== false) {
    fail("createAbortChecker()() should be false without a signal");
  }
  log("import ./abort: createAbortChecker OK");

  const webgpu = await import(pathToFileURL(join(pkgRoot, "webgpu.js")).href);
  if (typeof webgpu.TRANSFORM_POINTS_WGSL_V1 !== "string") {
    fail("wasm-spatial-core/webgpu did not export TRANSFORM_POINTS_WGSL_V1");
  }
  if (!webgpu.TRANSFORM_POINTS_WGSL_V1.includes("@compute")) {
    fail("TRANSFORM_POINTS_WGSL_V1 does not look like WGSL");
  }
  log("import ./webgpu: TRANSFORM_POINTS_WGSL_V1 OK");

  // /node must load the nodejs-target glue via createRequire (the 0.10.2 fix).
  const nodeRequire = createRequire(join(pkgRoot, "node.js"));
  let nodeBindings;
  try {
    nodeBindings = nodeRequire("./pkg-node/wasm_spatial_core.js");
  } catch (err) {
    fail(`/node failed to load pkg-node glue: ${err && err.message}`);
  }
  if (typeof nodeBindings.version !== "function") {
    fail("/node glue has no version() — named ESM re-export of CJS likely broken again");
  }
  const ver = nodeBindings.version();
  if (typeof ver !== "string" || ver.length === 0) {
    fail(`version() returned ${ver}`);
  }
  log(`import ./node: version() = ${ver}`);

  try {
    nodeBindings.parseGeoJsonCoords("not-json");
    fail("parseGeoJsonCoords('not-json') should throw");
  } catch (err) {
    if (!err || err.name !== "SpatialError") {
      fail(`expected SpatialError name, got ${err && err.name}`);
    }
    if (err.code !== "PARSE_ERROR") {
      fail(`expected code PARSE_ERROR, got ${err && err.code}`);
    }
    log("SpatialError on invalid GeoJSON: name=SpatialError code=PARSE_ERROR");
  }

  if (typeof nodeBindings.encodeTerrainTmsPyramid === "function") {
    try {
      nodeBindings.encodeTerrainTmsPyramid(
        new Float32Array(4),
        2,
        2,
        new Float64Array([0, 0, 1]),
        new Float64Array(0),
        1,
      );
      fail("encodeTerrainTmsPyramid with 3-element bounds should throw");
    } catch (err) {
      if (!err || err.code !== "INVALID_INPUT") {
        fail(
          `encodeTerrainTmsPyramid expected INVALID_INPUT, got name=${err && err.name} code=${err && err.code}`,
        );
      }
      log("SpatialError on TMS bounds: code=INVALID_INPUT");
    }
  } else {
    log("skip encodeTerrainTmsPyramid (not in this build)");
  }

  if (typeof nodeBindings.supportsTerrainEdit === "function") {
    log(`supportsTerrainEdit: ${nodeBindings.supportsTerrainEdit()}`);
  }
  if (typeof nodeBindings.supportsMeshEdit === "function") {
    log(`supportsMeshEdit: ${nodeBindings.supportsMeshEdit()}`);
  }
  if (typeof nodeBindings.supportsWebGpu === "function") {
    log(`supportsWebGpu: ${nodeBindings.supportsWebGpu()}`);
  }
}

async function main() {
  console.log("\n📦 npm pack / subpath smoke");
  console.log("─".repeat(50));

  const work = mkdtempSync(join(tmpdir(), "wasm-spatial-npm-smoke-"));
  let tarball;
  try {
    writeFileSync(
      join(work, "package.json"),
      JSON.stringify(
        {
          name: "wasm-spatial-core-pack-smoke",
          private: true,
          type: "module",
        },
        null,
        2,
      ),
    );

    if (fromRegistry) {
      log(`npm install ${spec} from registry → ${work}`);
      run(`npm install --ignore-scripts ${JSON.stringify(spec)}`, work);
    } else {
      tarball = packLocalTarball();
      log(`npm install ${tarball} → ${work}`);
      run(`npm install --ignore-scripts ${JSON.stringify(tarball)}`, work);
    }

    const installed = join(work, "node_modules", "wasm-spatial-core");
    if (!existsSync(installed)) {
      fail(`install did not produce ${installed}`);
    }
    await assertSubpaths(installed);
    console.log("─".repeat(50));
    console.log("✅ npm subpath smoke passed");
  } finally {
    rmSync(work, { recursive: true, force: true });
    if (tarball && existsSync(tarball)) {
      rmSync(tarball);
    }
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
