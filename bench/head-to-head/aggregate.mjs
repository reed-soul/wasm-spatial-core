#!/usr/bin/env node
// Aggregate head-to-head benchmark JSONL into a published comparison.
//
// Reads bench/head-to-head/results-*.jsonl (one per engine), groups by input,
// and writes:
//   - benchmarks/results.json   (machine-readable, all rows)
//   - benchmarks/index.md       (human-readable comparison table)
//
// The table ALWAYS includes every engine that produced a result, including
// rows where wasm-spatial-core loses or ties. No row is hidden.
//
// Usage:
//   node bench/head-to-head/aggregate.mjs [out_dir]

import { readFileSync, readdirSync, writeFileSync, existsSync, mkdirSync } from "node:fs";
import { join, resolve } from "node:path";

const SRC_DIR = resolve("bench/head-to-head");
const OUT_DIR = process.argv[2] || resolve("benchmarks");

function readAllResults() {
  const files = readdirSync(SRC_DIR).filter((f) => f.startsWith("results-") && f.endsWith(".jsonl"));
  const rows = [];
  for (const f of files) {
    const lines = readFileSync(join(SRC_DIR, f), "utf8")
      .split("\n")
      .filter(Boolean);
    for (const line of lines) {
      try {
        rows.push(JSON.parse(line));
      } catch {
        // skip malformed
      }
    }
  }
  return rows;
}

function fmtMs(ms) {
  return ms == null ? "—" : ms < 1000 ? `${ms.toFixed(0)}ms` : `${(ms / 1000).toFixed(2)}s`;
}
function fmtMB(b) {
  if (b == null) return "—";
  return b >= 1e6 ? `${(b / 1e6).toFixed(1)}MB` : `${(b / 1e3).toFixed(0)}KB`;
}

function speedup(wasmMs, otherMs) {
  if (!wasmMs || !otherMs) return "—";
  const ratio = otherMs / wasmMs;
  if (ratio >= 1) return `**${ratio.toFixed(1)}×** ⚡`;
  if (ratio >= 0.9) return `${ratio.toFixed(2)}× (≈tie)`;
  return `${ratio.toFixed(2)}× (wasm slower)`;
}

function buildMarkdown(rows, generatedAt) {
  // group by input
  const byInput = new Map();
  for (const r of rows) {
    if (!byInput.has(r.input)) byInput.set(r.input, []);
    byInput.get(r.input).push(r);
  }

  const engines = [...new Set(rows.map((r) => r.engine))].sort();
  const hardware = rows[0]?.hardware || {};

  let md = `# Head-to-head benchmark: wasm-spatial-core vs alternatives\n\n`;
  md += `> Task: **LAS point cloud → Cesium 3D Tiles (.pnts + tileset.json)**, end-to-end, no upload, no server.\n\n`;
  md += `**Generated:** ${generatedAt}  \n`;
  md += `**Hardware:** ${hardware.cpu_model || "?"} · ${hardware.cpus || "?"} CPUs · ${hardware.platform}/${hardware.arch}  \n`;
  md += `**Methodology:** same input bytes (SHA-256 verified), same no-op transform (no reprojection), trimmed mean of 5 timed runs after warmup. Output bytes = sum of all generated tile files + tileset.json.\n\n`;

  md += `## ⚖️ Fairness & honesty\n\n`;
  md += `- Every engine that produced a result is shown — including rows where wasm-spatial-core is **slower or ties**.\n`;
  md += `- **loaders.gl cannot do this task** (LAS read only, no pnts writer) — that row documents a capability gap, not a timeout. \`output_bytes = 0\`.\n`;
  md += `- **Cesium ion is not timed here** — it requires uploading private data to a cloud service and an API key, which contradicts the zero-upload thesis. It is a different category (cloud tiling service vs. in-browser engine). See the architecture comparison below.\n`;
  md += `- py3dtiles is a Python CLI; wasm-spatial-core runs in WASM/Node. The comparison is **wall-clock end-to-end** on the same machine, not a microbenchmark of equivalent algorithms.\n`;
  md += `- RSS for py3dtiles is child-process max (coarse); wasm RSS is Node process RSS.\n\n`;

  for (const [input, group] of byInput) {
    md += `## ${input}\n\n`;
    md += `| Engine | Wall time | Peak RSS | Output size | Tiles | Speedup vs wasm |\n`;
    md += `|--------|----------|----------|-------------|-------|-----------------|\n`;
    const wasm = group.find((r) => r.engine === "wasm-spatial-core");
    for (const r of group.sort((a, b) => (a.engine > b.engine ? 1 : -1))) {
      const wm = r.wall_ms?.total;
      const su = r.engine === "wasm-spatial-core" ? "baseline" : speedup(wasm?.wall_ms?.total, wm);
      const wc = r.error ? `❌ ${r.error.slice(0, 40)}` : fmtMs(wm);
      const rc = r.error ? "—" : `${r.peak_rss_mb || "—"}MB`;
      const oc = r.output_bytes === 0 ? "0 (N/A)" : fmtMB(r.output_bytes);
      md += `| ${r.engine} ${r.engine_version ? `\`${r.engine_version}\`` : ""} | ${wc} | ${rc} | ${oc} | ${r.tile_count ?? "—"} | ${su} |\n`;
    }
    md += `\n`;
    // notes per engine
    const notes = group.filter((r) => r.notes).map((r) => `- **${r.engine}:** ${r.notes}`);
    if (notes.length) {
      md += `<details><summary>Notes</summary>\n\n${notes.join("\n")}\n\n</details>\n\n`;
    }
  }

  md += `## 🏗️ Architecture comparison (not timed)\n\n`;
  md += `| | wasm-spatial-core | py3dtiles | loaders.gl | Cesium ion |\n`;
  md += `|---|---|---|---|---|\n`;
  md += `| Runs where | Browser (WASM) or Node | Python process | Browser/Node | Cloud service |\n`;
  md += `| Data leaves machine | **No** | No (local) | No (local) | **Yes** (upload) |\n`;
  md += `| Install needed | \`npm i\` (1.2MB WASM) | Python + pip + deps | npm | Account + token |\n`;
  md += `| LAS → 3D Tiles | ✅ full pipeline | ✅ full pipeline | ❌ read only | ✅ (cloud) |\n`;
  md += `| Offline / air-gapped | ✅ | ✅ | ✅ | ❌ |\n`;
  md += `| Cesium ion quota | not consumed | not consumed | not consumed | consumed |\n\n`;

  md += `## 🔁 Reproduce\n\n`;
  md += "```bash\n";
  md += "git clone https://github.com/reed-soul/wasm-spatial-core.git && cd wasm-spatial-core\n";
  md += "wasm-pack build --target nodejs --release --out-dir pkg-node -- --features point-cloud\n";
  md += "pip install py3dtiles\n";
  md += "npm install @loaders.gl/las   # optional; documents the capability gap\n";
  md += "node bench/head-to-head/run-wasm.mjs test-data/large/synthetic_500k.las > bench/head-to-head/results-wasm.jsonl\n";
  md += "python3 bench/head-to-head/run-py3dtiles.py test-data/large/synthetic_500k.las > bench/head-to-head/results-py3dtiles.jsonl\n";
  md += "node bench/head-to-head/run-loaders-gl.mjs test-data/large/synthetic_500k.las > bench/head-to-head/results-loaders-gl.jsonl\n";
  md += "node bench/head-to-head/aggregate.mjs\n";
  md += "# → opens benchmarks/index.md\n";
  md += "```\n";

  return md;
}

function main() {
  const rows = readAllResults();
  if (rows.length === 0) {
    console.error("No results-*.jsonl found in bench/head-to-head/. Run the runners first.");
    process.exit(1);
  }
  if (!existsSync(OUT_DIR)) mkdirSync(OUT_DIR, { recursive: true });
  const generatedAt = new Date().toISOString();
  writeFileSync(join(OUT_DIR, "results.json"), JSON.stringify({ generated_at: generatedAt, rows }, null, 2));
  writeFileSync(join(OUT_DIR, "index.md"), buildMarkdown(rows, generatedAt));
  console.log(`✅ Wrote ${OUT_DIR}/results.json (${rows.length} rows) and ${OUT_DIR}/index.md`);
}

main();
