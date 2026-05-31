#!/bin/bash
# Serve the demo pages for local browser testing
# Usage: ./scripts/serve_demo.sh [PORT]
PORT="${1:-8080}"
cd "$(dirname "$0")/.."
echo "🚀 Serving wasm-spatial-core demos at http://localhost:$PORT"
echo "   Open one of:"
echo "     http://localhost:$PORT/demo/              — Interactive Demo"
echo "     http://localhost:$PORT/point-cloud-demo/  — Point Cloud (Three.js)"
echo "     http://localhost:$PORT/terrain-demo/      — Terrain Demo"
echo "   Press Ctrl+C to stop."
python3 -m http.server "$PORT" --directory examples
