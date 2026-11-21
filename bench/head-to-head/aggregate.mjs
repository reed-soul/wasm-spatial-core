#!/usr/bin/env node
/**
 * Fold results-*.jsonl into the results table in benchmarks/index.md.
 *
 * Usage:
 *   node aggregate.mjs [results-a.jsonl ...] [--out benchmarks/index.md]
 */
import { readFileSync, writeFileSync, existsSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import os from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "../..");

const START = "<!-- HEAD_TO_HEAD_RESULTS_START -->";
const END = "<!-- HEAD_TO_HEAD_RESULTS_END -->";

function parseArgs(argv) {
  const files = [];
  let out = join(repoRoot, "benchmarks/index.md");
  for (let i = 2; i < argv.length; i++) {
    if (argv[i] === "--out") {
      out = resolve(argv[++i]);
      continue;
    }
    files.push(resolve(argv[i]));
  }
  if (files.length === 0) {
    const dir = join(repoRoot, "bench/head-to-head");
    for (const name of readdirSync(dir)) {
      if (name.startsWith("results-") && name.endsWith(".jsonl")) {
        files.push(join(dir, name));
      }
    }
  }
  return { files, out };
}

function readJsonl(path) {
  if (!existsSync(path)) return [];
  return readFileSync(path, "utf8")
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean)
    .map((l) => JSON.parse(l));
}

function fmtTime(ms) {
  if (ms == null || Number.isNaN(ms)) return "—";
  if (ms >= 1000) return `${(ms / 1000).toFixed(2)}s`;
  return `${ms.toFixed(0)}ms`;
}

function fmtMb(bytes) {
  if (!bytes) return "—";
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
}

function fmtRss(mb) {
  if (mb == null) return "—";
  return `${mb.toFixed(0)}MB`;
}

function speedupCell(row, wasmMs) {
  if (!row.ok || row.wall_ms == null || !wasmMs) return "—";
  if (row.engine === "wasm-spatial-core") return "baseline";
  const ratio = row.wall_ms / wasmMs;
  if (ratio > 1.05) return `**${ratio.toFixed(1)}×** ⚡`;
  if (ratio < 0.95) return `${ratio.toFixed(2)}× (wasm slower)`;
  return "~1×";
}

function formatRow(row, wasmMs) {
  const ver = row.version ? `\`${row.version}\`` : "";
  const engine = `${row.engine} ${ver}`.trim();
  if (!row.ok) {
    const err = (row.error || "failed").replace(/\s+/g, " ").slice(0, 48);
    return `| ${engine} | ❌ ${err} | — | — | — | — |`;
  }
  return `| ${engine} | ${fmtTime(row.wall_ms)} | ${fmtRss(row.rss_mb)} | ${fmtMb(row.output_bytes)} | ${row.tiles ?? "—"} | ${speedupCell(row, wasmMs)} |`;
}

const { files, out } = parseArgs(process.argv);
const rows = files.flatMap(readJsonl);
if (rows.length === 0) {
  console.error("aggregate.mjs: no results-*.jsonl found");
  process.exit(1);
}

const wasm = rows.find((r) => r.engine === "wasm-spatial-core" && r.ok);
const wasmMs = wasm?.wall_ms ?? null;
const sorted = [...rows].sort((a, b) => a.engine.localeCompare(b.engine));

const notes = sorted
  .map((r) => (r.notes ? `- **${r.engine}:** ${r.notes}` : null))
  .filter(Boolean)
  .join("\n");

const table = [
  START,
  "",
  `**Generated:** ${new Date().toISOString()}  `,
  `**Hardware:** ${os.cpus()[0]?.model?.trim() || "unknown"} · ${os.cpus().length} CPUs · ${os.platform()}/${os.arch()}  `,
  "**Methodology:** same input bytes, no reprojection, trimmed mean of 5 timed runs after warmup (min/max dropped). Output bytes = sum of generated tile files + tileset.json.",
  "",
  "## bench/head-to-head/fixtures/canonical_500k.las",
  "",
  "| Engine | Wall time | Peak RSS | Output size | Tiles | Speedup vs wasm |",
  "|--------|----------|----------|-------------|-------|-----------------|",
  ...sorted.map((r) => formatRow(r, wasmMs)),
  "",
  "<details><summary>Notes</summary>",
  "",
  notes,
  "",
  "</details>",
  "",
  END,
].join("\n");

if (!existsSync(out)) {
  writeFileSync(out, `# Head-to-head benchmark\n\n${table}\n`);
  console.log(`wrote ${out}`);
  process.exit(0);
}

const existing = readFileSync(out, "utf8");
if (!existing.includes(START) || !existing.includes(END)) {
  console.error(`${out} is missing ${START} / ${END} markers`);
  process.exit(1);
}
const next =
  existing.slice(0, existing.indexOf(START)) +
  table +
  existing.slice(existing.indexOf(END) + END.length);
writeFileSync(out, next);
console.log(`updated ${out} (${rows.length} result rows)`);
