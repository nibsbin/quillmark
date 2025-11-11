#!/bin/bash
set -e

echo "Building WASM module for @quillmark-test/wasm..."

cd "$(dirname "$0")/.."

# Check for required tools
if ! command -v wasm-bindgen &> /dev/null; then
    echo "wasm-bindgen not found. Install it with:"
    echo "  cargo install wasm-bindgen-cli --version 0.2.104"
    exit 1
fi

echo ""
echo "Building for target: bundler (optimized for size)"

# Step 1: Build WASM binary with cargo
echo "Building WASM binary..."
cargo build \
    --target wasm32-unknown-unknown \
    --profile wasm-release \
    --manifest-path bindings/quillmark-wasm/Cargo.toml

# Step 2: Generate JS bindings with wasm-bindgen
echo "Generating JS bindings..."
mkdir -p pkg/bundler
wasm-bindgen \
    target/wasm32-unknown-unknown/wasm-release/quillmark_wasm.wasm \
    --out-dir pkg/bundler \
    --out-name wasm \
    --target bundler

# Step 3: Extract version from Cargo.toml
VERSION=$(cargo metadata --format-version=1 --no-deps | jq -r '.packages[] | select(.name == "quillmark-wasm") | .version')

# Step 4: Create package.json
echo "Creating package.json..."
cat > pkg/bundler/package.json << EOF
{
  "name": "@quillmark-test/wasm",
  "version": "$VERSION",
  "description": "WebAssembly bindings for quillmark",
  "license": "MIT OR Apache-2.0",
  "repository": {
    "type": "git",
    "url": "https://github.com/nibsbin/quillmark"
  },
  "files": [
    "wasm_bg.wasm",
    "wasm.js",
    "wasm.d.ts"
  ],
  "module": "wasm.js",
  "types": "wasm.d.ts",
  "sideEffects": [
    "wasm.js"
  ]
}
EOF

# Step 5: Copy README and LICENSE files
if [ -f "bindings/quillmark-wasm/README.md" ]; then
    cp bindings/quillmark-wasm/README.md pkg/bundler/
fi

if [ -f "LICENSE-MIT" ]; then
    cp LICENSE-MIT pkg/bundler/
fi

if [ -f "LICENSE-APACHE" ]; then
    cp LICENSE-APACHE pkg/bundler/
fi

# Step 6: Create .gitignore for pkg directory
cat > pkg/.gitignore << EOF
*
!.gitignore
EOF

echo ""
echo "WASM build complete!"
echo "Output directory: pkg/bundler/"
echo "Package version: $VERSION"

# Show size
if [ -f "pkg/bundler/wasm_bg.wasm" ]; then
    SIZE=$(du -h pkg/bundler/wasm_bg.wasm | cut -f1)
    echo "WASM size: $SIZE"
fi