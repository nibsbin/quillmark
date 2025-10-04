#!/bin/bash
set -e

echo "Building WASM module for @quillmark/wasm..."

# Navigate to workspace root
cd "$(dirname "$0")/.."

# Install wasm-pack if not available
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack not found. Install it with:"
    echo "  cargo install wasm-pack"
    exit 1
fi

# Build for different targets
targets=("bundler" "nodejs" "web")

for target in "${targets[@]}"; do
    echo ""
    echo "Building for target: $target"
    
    wasm-pack build quillmark-wasm \
        --target "$target" \
        --out-dir "../pkg-$target" \
        --out-name wasm \
        --release \
        --scope quillmark
        
    # Update package name to @quillmark/wasm
    if [ -f "pkg-$target/package.json" ]; then
        sed -i 's/"@quillmark\/quillmark-wasm"/"@quillmark\/wasm"/' "pkg-$target/package.json"
    fi
done

echo ""
echo "WASM build complete!"
echo "Output directories:"
for target in "${targets[@]}"; do
    echo "  - pkg-$target/"
done
