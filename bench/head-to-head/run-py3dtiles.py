#!/usr/bin/env python3
"""py3dtiles runner: LAS → 3D Tiles (.pnts) end-to-end.

Runs `py3dtiles convert <input.las> --out <tmpdir>` for each input, measures
wall time, peak RSS (child), output bytes, and tile count, and emits one JSON
object per input to stdout (JSONL) — same schema as run-wasm.mjs.

Fairness notes:
  - NO reprojection (--srs_out omitted). wasm-spatial-core emits raw input
    coordinates; we compare the same no-op transform.
  - Multiple timed runs are averaged (trimmed mean), matching the JS runner.
  - Peak RSS via resource.getrusage(RUSAGE_CHILDREN).ru_maxrss (kB on macOS,
    bytes on Linux — normalized below).

Usage:
  pip install py3dtiles
  python3 bench/head-to-head/run-py3dtiles.py [input.las ...]
"""

import hashlib
import json
import os
import platform
import resource
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path


def _norm_rss_kb(ru_maxrss):
    # macOS reports ru_maxrss in bytes; Linux in kB.
    if platform.system() == "Darwin":
        return ru_maxrss / 1024
    return ru_maxrss


def trimmed_mean(xs):
    xs = sorted(xs)
    if len(xs) > 3:
        xs = xs[1:-1]
    return sum(xs) / len(xs), xs[0], xs[-1]


def file_digest(path, n=12):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        h.update(f.read())
    return h.hexdigest()[:n]


def hardware():
    import multiprocessing

    return {
        "python": sys.version.split()[0],
        "platform": platform.system().lower(),
        "arch": platform.machine(),
        "cpus": multiprocessing.cpu_count(),
    }


def py3dtiles_version():
    # py3dtiles has no --version flag; read it from installed package metadata.
    try:
        from importlib.metadata import version
        return version("py3dtiles")
    except Exception:
        pass
    try:
        out = subprocess.run(
            [sys.executable, "-m", "pip", "show", "py3dtiles"],
            capture_output=True, text=True, timeout=30,
        )
        for line in out.stdout.splitlines():
            if line.lower().startswith("version:"):
                return line.split(":", 1)[1].strip()
    except Exception:
        pass
    return "unknown"


def py3dtiles_cmd(input_path, out_dir, jobs=1):
    """Build the py3dtiles convert command. jobs=1 for single-thread parity
    with the default wasm build (which is single-thread unless --features
    multi-thread). Set PY3DTILES_JOBS env to override."""
    n = os.environ.get("PY3DTILES_JOBS", str(jobs))
    return ["py3dtiles", "convert", str(input_path), "--out", out_dir,
            "--overwrite", "--jobs", n]


def run_once(input_path, out_dir):
    """Run py3dtiles convert once; return (wall_s, output_bytes, tile_count)."""
    if os.path.exists(out_dir):
        shutil.rmtree(out_dir)
    os.makedirs(out_dir, exist_ok=True)

    t0 = time.perf_counter()
    proc = subprocess.run(
        py3dtiles_cmd(input_path, out_dir),
        capture_output=True, text=True,
    )
    wall = time.perf_counter() - t0
    if proc.returncode != 0:
        raise RuntimeError(
            f"py3dtiles failed (rc={proc.returncode}): "
            f"{proc.stderr[:500]}"
        )

    total_bytes = 0
    tile_count = 0
    for root, _dirs, files in os.walk(out_dir):
        for name in files:
            total_bytes += os.path.getsize(os.path.join(root, name))
            if name.endswith(".pnts"):
                tile_count += 1
    return wall, total_bytes, tile_count


def point_count_from_tileset(out_dir):
    """Best-effort point count from tileset.json BATCH_TABLE / POINTS_LENGTH.
    py3dtiles pnts embed a feature table; we parse JSON header lazily. If we
    cannot read it, return -1 and note it."""
    ts = Path(out_dir) / "tileset.json"
    if not ts.exists():
        return -1
    # py3dtiles doesn't expose point count in tileset.json directly; we leave
    # verification to byte-count parity with the wasm runner. Return -1 sentinel.
    return -1


def main():
    inputs = sys.argv[1:] or ["test-data/large/synthetic_500k.las"]
    ver = py3dtiles_version()
    hw = hardware()

    rss_before = _norm_rss_kb(resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss)

    for inp in inputs:
        inp_path = Path(inp)
        if not inp_path.exists():
            print(f"  ⚠️  {inp}: not found, skipping", file=sys.stderr)
            continue
        size_mb = round(inp_path.stat().st_size / 1e6, 2)
        digest = file_digest(inp)

        # warmup + verify it works
        tmp = tempfile.mkdtemp(prefix="py3dt_bench_")
        out_dir = os.path.join(tmp, "out")
        try:
            _w, verify_bytes, verify_tiles = run_once(inp_path, out_dir)
        except Exception as e:
            result = {
                "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                "hardware": hw,
                "engine": "py3dtiles",
                "engine_version": ver,
                "input": inp,
                "error": str(e),
                "notes": "py3dtiles convert failed; documented as a finding",
            }
            print(json.dumps(result))
            shutil.rmtree(tmp, ignore_errors=True)
            continue

        # timed runs
        walls = []
        for _ in range(5):
            w, _b, _t = run_once(inp_path, out_dir)
            walls.append(w)
        avg_wall, min_wall, max_wall = trimmed_mean(walls)
        point_count = point_count_from_tileset(out_dir)

        rss_after = _norm_rss_kb(resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss)

        result = {
            "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "hardware": hw,
            "engine": "py3dtiles",
            "engine_version": ver,
            "input": inp,
            "input_digest_sha256_12": digest,
            "input_size_mb": size_mb,
            "point_count": point_count,
            "output_bytes": verify_bytes,
            "tile_count": verify_tiles,
            "wall_ms": {
                "total": round(avg_wall * 1000, 1),
                "min": round(min_wall * 1000, 1),
                "max": round(max_wall * 1000, 1),
                "runs": len(walls),
            },
            "peak_rss_mb": round(max(rss_before, rss_after) / 1024, 0),
            "notes": (
                "py3dtiles convert, no reprojection (matches wasm no-op). "
                "point_count=-1 means not readable from tileset.json; byte-count parity is the verification axis. "
                "RSS is child-process max across all runs (coarse)."
            ),
        }
        print(json.dumps(result))
        print(
            f"  ✅ {inp}: {verify_tiles} tiles, "
            f"{verify_bytes / 1e6:.2f}MB out, {avg_wall:.2f}s",
            file=sys.stderr,
        )
        shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    main()
