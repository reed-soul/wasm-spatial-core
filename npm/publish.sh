#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT/npm"
[ -d node_modules ] || npm install --ignore-scripts

echo "🔧 Building WASM (web + nodejs targets)..."
npm run build:wasm
npm run build:wasm:node

echo "📝 Type-checking + compiling entry points..."
npm run typecheck
npm run build:entries

echo "📦 Dry-run publishing..."
npm publish --dry-run

echo ""
echo "✅ Package staged. Inspect the file list above, then run 'cd npm && npm publish' to publish for real."
