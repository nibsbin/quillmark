#!/bin/bash
set -e
set -o pipefail

# Writes pkg/sbom.cdx.json, the CycloneDX SBOM of the crates linked into the
# wasm binary, which a scan of the npm package cannot see inside. Run after
# build-wasm.sh, which starts from an empty pkg/. Needs cargo-cyclonedx.

cd "$(dirname "$0")/.."

cargo cyclonedx --manifest-path crates/bindings/wasm/Cargo.toml \
    --target wasm32-unknown-unknown --format json --spec-version 1.5 \
    --override-filename quillmark-wasm.cdx
mv crates/bindings/wasm/quillmark-wasm.cdx.json pkg/sbom.cdx.json
# cargo-cyclonedx writes one beside every workspace member's manifest.
find crates -name quillmark-wasm.cdx.json -delete
jq -e '.metadata.component.name == "quillmark-wasm" and (.components | length) > 0' \
    pkg/sbom.cdx.json >/dev/null
