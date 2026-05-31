#!/usr/bin/env bash
set -euo pipefail

# ===========================================================================
# wasm-spatial-core npm publish script
# ===========================================================================
# Usage:
#   ./publish.sh           # Dry-run (recommended first)
#   ./publish.sh --publish # Actually publish
# ===========================================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "🔍 Checking prerequisites..."

# Check wasm-pack
if ! command -v wasm-pack &> /dev/null; then
    echo "❌ wasm-pack not found. Install with: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
    exit 1
fi

# Check npm
if ! command -v npm &> /dev/null; then
    echo "❌ npm not found."
    exit 1
fi

# Check we're in the npm directory
if [ ! -f "package.json" ]; then
    echo "❌ package.json not found. Run from npm/ directory."
    exit 1
fi

echo "📦 Building WASM with all features (point-cloud + geotiff)..."
wasm-pack build --target web --release --out-dir npm/pkg -- --features point-cloud,geotiff

echo "📋 Building TypeScript declarations..."
cd pkg
npx -p typescript tsc --declaration --emitDeclarationOnly --outDir . ../index.ts \
    --ignoreConfig --target esnext --moduleResolution bundler --module esnext --lib esnext --skipLibCheck
cd ..

echo ""
echo "✅ Build complete!"
echo ""

# Check version
VERSION=$(node -p "require('./package.json').version")
echo "🏷️  Version: $VERSION"
echo ""

# Count files
echo "📁 Files to be published:"
npm pack --dry-run 2>&1 | head -30

echo ""

if [ "${1:-}" = "--publish" ]; then
    echo "🚀 Publishing to npm..."
    npm publish --access public
    echo "✅ Published wasm-spatial-core@$VERSION to npm!"
else
    echo "ℹ️  Dry run complete. Use './publish.sh --publish' to actually publish."
fi
