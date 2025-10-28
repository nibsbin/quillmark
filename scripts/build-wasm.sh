#!/bin/bash
set -e

echo "Building WASM module for @quillmark-test/wasm..."

# Navigate to workspace root
cd "$(dirname "$0")/.."

# Install wasm-pack if not available
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack not found. Install it with:"
    echo "  cargo install wasm-pack"
    exit 1
fi

# Build for bundler target only
echo ""
echo "Building for target: bundler"

wasm-pack build bindings/quillmark-wasm \
    --target bundler \
    --out-dir "../../pkg/bundler" \
    --out-name wasm \
    --release \
    --scope quillmark-test

# Update package name from @quillmark-test/quillmark-wasm to @quillmark-test/wasm
# Use sed in a cross-platform way
if [ -f "pkg/bundler/package.json" ]; then
    if sed --version 2>&1 | grep -q GNU; then
        # GNU sed (Linux)
        sed -i 's/"@quillmark-test\/quillmark-wasm"/"@quillmark-test\/wasm"/' "pkg/bundler/package.json"
    else
        # BSD sed (macOS)
        sed -i '' 's/"@quillmark-test\/quillmark-wasm"/"@quillmark-test\/wasm"/' "pkg/bundler/package.json"
    fi
fi

echo ""
echo "WASM build complete!"
echo "Output directory: pkg/bundler/"
