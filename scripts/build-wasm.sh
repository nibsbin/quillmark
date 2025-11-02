#!/bin/bash
set -e

echo "Building WASM module for @quillmark-test/wasm..."

cd "$(dirname "$0")/.."

if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack not found. Install it with:"
    echo "  cargo install wasm-pack"
    exit 1
fi

echo ""
echo "Building for target: bundler (optimized for size)"

wasm-pack build bindings/quillmark-wasm \
    --target bundler \
    --out-dir "../../pkg/bundler" \
    --out-name wasm \
    --profile wasm-release \
    --no-opt \
    --scope quillmark-test

# Update package name
if [ -f "pkg/bundler/package.json" ]; then
    if sed --version 2>&1 | grep -q GNU; then
        sed -i 's/"@quillmark-test\/quillmark-wasm"/"@quillmark-test\/wasm"/' "pkg/bundler/package.json"
    else
        sed -i '' 's/"@quillmark-test\/quillmark-wasm"/"@quillmark-test\/wasm"/' "pkg/bundler/package.json"
    fi
fi

echo ""
echo "WASM build complete!"
echo "Output directory: pkg/bundler/"

# Show size
if [ -f "pkg/bundler/wasm_bg.wasm" ]; then
    SIZE=$(du -h pkg/bundler/wasm_bg.wasm | cut -f1)
    echo "WASM size: $SIZE"
fi