#!/bin/bash
set -e

echo "Building WASM module for @quillmark/wasm..."

cd "$(dirname "$0")/.."

# Check for required tools
if ! command -v wasm-bindgen &> /dev/null; then
    echo "wasm-bindgen not found. Install it with:"
    echo "  cargo install wasm-bindgen-cli --version 0.2.118"
    exit 1
fi

if ! command -v wasm-opt &> /dev/null; then
    echo "wasm-opt not found. Install it via:"
    echo "  cargo install wasm-opt    # or apt install binaryen / brew install binaryen"
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
#
# `--weak-refs` opts into FinalizationRegistry-based auto-free for
# wasm-bindgen handles. `.free()` is still emitted as an eager hook for
# callers that want deterministic teardown; opting in just ensures dropped
# handles eventually get reclaimed without manual `.free()` discipline.
# Requires Node 14.6+ / all current evergreen browsers.
echo "Generating JS bindings for bundler..."
mkdir -p pkg/bundler
wasm-bindgen \
    target/wasm32-unknown-unknown/wasm-release/quillmark_wasm.wasm \
    --out-dir pkg/bundler \
    --out-name wasm \
    --target bundler \
    --weak-refs

echo "Generating JS bindings for nodejs..."
mkdir -p pkg/node-esm
wasm-bindgen \
    target/wasm32-unknown-unknown/wasm-release/quillmark_wasm.wasm \
    --out-dir pkg/node-esm \
    --out-name wasm \
    --target experimental-nodejs-module \
    --weak-refs

# Step 2.5: Run wasm-opt for additional size reduction.
#
# `-Oz`            — optimize aggressively for size
# `--strip-debug`  — drop DWARF (already stripped via profile, but defensive)
# `--strip-producers` / `--vacuum` — drop the producers section and unused items
# The `--enable-*` flags must match the post-MVP features rustc emits at
# wasm-release; without them, the validator rejects e.g. `i32.extend16_s`.
WASM_OPT_FLAGS=(
    -Oz
    --strip-debug
    --strip-producers
    --vacuum
    --enable-sign-ext
    --enable-bulk-memory
    --enable-mutable-globals
    --enable-nontrapping-float-to-int
    --enable-reference-types
)
for target in pkg/bundler pkg/node-esm; do
    if [ -f "$target/wasm_bg.wasm" ]; then
        echo "Running wasm-opt on $target/wasm_bg.wasm..."
        wasm-opt "${WASM_OPT_FLAGS[@]}" "$target/wasm_bg.wasm" -o "$target/wasm_bg.wasm.opt"
        mv "$target/wasm_bg.wasm.opt" "$target/wasm_bg.wasm"
    fi
done

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

# Show sizes (raw, gzip, brotli — transport size is what matters for delivery).
report_size() {
    local label="$1" file="$2"
    [ -f "$file" ] || return 0
    local raw gz br
    raw=$(du -h "$file" | cut -f1)
    gz=$(gzip -9 -c "$file" 2>/dev/null | wc -c | awk '{printf "%.1fM", $1/1048576}')
    if command -v brotli &> /dev/null; then
        br=$(brotli -9 -c "$file" 2>/dev/null | wc -c | awk '{printf "%.1fM", $1/1048576}')
        echo "WASM size ($label): raw=$raw gzip=$gz brotli=$br"
    else
        echo "WASM size ($label): raw=$raw gzip=$gz"
    fi
}
report_size "bundler" pkg/bundler/wasm_bg.wasm
report_size "nodejs"  pkg/node-esm/wasm_bg.wasm