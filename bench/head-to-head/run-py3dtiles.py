#!/usr/bin/env python3
"""LAS → 3D Tiles via py3dtiles CLI.

Usage:
  python3 run-py3dtiles.py <input.las>  > results-py3dtiles.jsonl
"""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

LAS = Path(
    sys.argv[1]
    if len(sys.argv) > 1
    else "bench/head-to-head/fixtures/canonical_500k.las"
).resolve()


def emit(**kwargs) -> None:
    row = {
        "engine": "py3dtiles",
        "version": kwargs.get("version"),
        "ok": kwargs.get("ok", False),
        "wall_ms": kwargs.get("wall_ms"),
        "rss_mb": kwargs.get("rss_mb"),
        "output_bytes": kwargs.get("output_bytes", 0),
        "tiles": kwargs.get("tiles", 0),
        "error": kwargs.get("error"),
        "notes": kwargs.get(
            "notes",
            "py3dtiles convert, no reprojection (matches wasm no-op). "
            "RSS is GNU time max RSS of the child (Linux) or unavailable.",
        ),
    }
    sys.stdout.write(json.dumps(row) + "\n")


def py3dtiles_version() -> str | None:
    try:
        proc = subprocess.run(
            ["py3dtiles", "--version"],
            capture_output=True,
            text=True,
            check=False,
        )
        text = (proc.stdout or proc.stderr or "").strip()
        return text.split()[-1] if text else None
    except FileNotFoundError:
        return None


def dir_stats(root: Path) -> tuple[int, int]:
    total = 0
    tiles = 0
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        total += path.stat().st_size
        if path.suffix.lower() in {".pnts", ".b3dm", ".i3dm", ".cmpt"}:
            tiles += 1
    return total, tiles


def convert_once(las: Path, out_dir: Path) -> tuple[subprocess.CompletedProcess, float]:
    """Run convert; return (proc, wall_ms from perf_counter)."""
    import time as time_mod

    cmd = ["py3dtiles", "convert", f"--out={out_dir}", str(las)]
    wrapped = False
    if os.path.exists("/usr/bin/time"):
        cmd = ["/usr/bin/time", "-f", "GNU_TIME %e %M", *cmd]
        wrapped = True
    t0 = time_mod.perf_counter()
    proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
    wall_ms = (time_mod.perf_counter() - t0) * 1000.0
    proc._gnu_wrapped = wrapped  # type: ignore[attr-defined]
    return proc, wall_ms


def parse_gnu_rss(stderr: str) -> float | None:
    """Parse `/usr/bin/time -f 'GNU_TIME %e %M'` → rss_mb, or None."""
    for ln in reversed((stderr or "").splitlines()):
        ln = ln.strip()
        if not ln.startswith("GNU_TIME "):
            continue
        parts = ln.split()
        if len(parts) < 3:
            return None
        try:
            return float(parts[2]) / 1024.0
        except ValueError:
            return None
    return None


def main() -> int:
    version = py3dtiles_version()
    if version is None and shutil.which("py3dtiles") is None:
        emit(ok=False, error="py3dtiles CLI not found on PATH")
        return 1
    if not LAS.is_file():
        emit(
            ok=False,
            version=version,
            error=f"LAS fixture not found: {LAS}",
        )
        return 1

    times_ms: list[float] = []
    last_bytes = 0
    last_tiles = 0
    peak_rss = 0.0

    # Warmup + 5 timed runs (trim min/max like the wasm harness).
    for i in range(6):
        with tempfile.TemporaryDirectory(prefix="py3dtiles-h2h-") as tmp:
            out = Path(tmp) / "tiles"
            out.mkdir()
            proc, wall_ms = convert_once(LAS, out)
            if proc.returncode != 0:
                err = (proc.stderr or proc.stdout or "").strip()[-800:]
                emit(
                    ok=False,
                    version=version,
                    error=f"py3dtiles convert failed (exit {proc.returncode}): {err}",
                )
                return 1
            output_bytes, tiles = dir_stats(out)
            rss_mb = parse_gnu_rss(proc.stderr)
            if i == 0:
                continue  # warmup
            times_ms.append(wall_ms)
            last_bytes = output_bytes
            last_tiles = tiles
            if rss_mb is not None:
                peak_rss = max(peak_rss, rss_mb)

    times_ms.sort()
    trimmed = times_ms[1:4] if len(times_ms) >= 5 else times_ms
    wall = sum(trimmed) / len(trimmed)
    emit(
        ok=True,
        version=version,
        wall_ms=wall,
        rss_mb=peak_rss if peak_rss > 0 else None,
        output_bytes=last_bytes,
        tiles=last_tiles,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
