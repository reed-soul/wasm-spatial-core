#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo "🔧 Building WASM (web target, point-cloud + geotiff features)..."
wasm-pack build --target web --release --out-dir npm/pkg -- --features point-cloud,geotiff

echo "📦 Dry-run publishing..."
cd npm
npm publish --dry-run

echo ""
echo "✅ Build successful. Run 'cd npm && npm publish' to publish for real."
