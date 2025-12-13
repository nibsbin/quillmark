#!/bin/bash
set -e

echo "Building WASM module for @quillmark/wasm..."

cd "$(dirname "$0")/.."

# Check for required tools
if ! command -v wasm-bindgen &> /dev/null; then
    echo "wasm-bindgen not found. Install it with:"
    echo "  cargo install wasm-bindgen-cli --version 0.2.104"
    exit 1
fi

echo ""
echo "Building for targets: bundler, nodejs (optimized for size)"

# Step 1: Build WASM binary with cargo
echo "Building WASM binary..."
cargo build \
    --target wasm32-unknown-unknown \
    --profile wasm-release \
    --manifest-path crates/bindings/wasm/Cargo.toml

# Step 2: Generate JS bindings with wasm-bindgen
echo "Generating JS bindings for bundler..."
mkdir -p pkg/bundler
wasm-bindgen \
    target/wasm32-unknown-unknown/wasm-release/quillmark_wasm.wasm \
    --out-dir pkg/bundler \
    --out-name wasm \
    --target bundler

echo "Generating JS bindings for nodejs..."
mkdir -p pkg/nodejs
wasm-bindgen \
    target/wasm32-unknown-unknown/wasm-release/quillmark_wasm.wasm \
    --out-dir pkg/nodejs \
    --out-name wasm \
    --target nodejs

# Step 3: Extract version from Cargo.toml
VERSION=$(cargo metadata --format-version=1 --no-deps | jq -r '.packages[] | select(.name == "quillmark-wasm") | .version')

# Step 4: Create package.json from template
echo "Creating package.json..."
sed "s/VERSION_PLACEHOLDER/$VERSION/" crates/bindings/wasm/package.template.json > pkg/package.json

# Step 5: Copy README and LICENSE files
if [ -f "crates/bindings/wasm/README.md" ]; then
    cp crates/bindings/wasm/README.md pkg/
fi

if [ -f "LICENSE-MIT" ]; then
    cp LICENSE-MIT pkg/
fi

if [ -f "LICENSE-APACHE" ]; then
    cp LICENSE-APACHE pkg/
fi

# Step 6: Create .gitignore for pkg directory
cat > pkg/.gitignore << EOF
*
!.gitignore
EOF

echo ""
echo "WASM build complete!"
echo "Output directory: pkg/"
echo "Package version: $VERSION"

# Show sizes
if [ -f "pkg/bundler/wasm_bg.wasm" ]; then
    SIZE=$(du -h pkg/bundler/wasm_bg.wasm | cut -f1)
    echo "WASM size (bundler): $SIZE"
fi
if [ -f "pkg/nodejs/wasm_bg.wasm" ]; then
    SIZE=$(du -h pkg/nodejs/wasm_bg.wasm | cut -f1)
    echo "WASM size (nodejs): $SIZE"
fi