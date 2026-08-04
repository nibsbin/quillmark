#!/bin/bash
set -e
# pipefail so a failing `gzip` in `gzip -c … | wc -c` propagates instead of
# being masked by `wc`'s exit 0. Without it the core size-budget check below
# can silently read 0 bytes on a gzip failure and false-pass, defeating the
# one guard rail that catches a Typst leak into the no-features core build.
set -o pipefail

# Builds THREE wasm artifacts from the one crate (the as-built design is
# documented in docs/migrations/0.89-to-0.90.md):
#
#   pkg/core/               no Typst: parse / load / validate / schema / seed / blueprint
#   pkg/backends/typst/     Typst-backed engine + LiveSession + canvas (a private
#                           backend binary, NOT a public export)
#   pkg/backends/pdfform/   Typst-free PDF-form backend (engine + LiveSession +
#                           canvas; the pdfform feature ships the web-sys canvas
#                           painter over the always-linked hayro raster;
#                           private backend binary, NOT a public export)
#
# These generated artifacts plus the hand-written canonical layer ship as one
# npm package with exactly ONE public entry: the root `.` export
# (`@quillmark/wasm`), the canonical `Quill`/`Document`/`Engine` API (see
# pkg/runtime/). `core` and the backends ship as files but are absent from the
# package.json `exports` map, so no consumer can import them by subpath: core is
# reached through the root's re-export, the backends only internally, by the
# canonical layer's lazy `import("../backends/<id>/wasm.js")`.
#
# Profile selection. Default is the size-optimized release build used for
# npm publish. `--ci` switches to a fast-compiling profile for PR validation
# where only correctness matters. Keep these two paths in sync with the
# cache namespacing in .github/workflows/{ci,release}.yml.
PROFILE="wasm-release"
MODE_LABEL="release (size-optimized)"
RELEASE_STAMP=0
for arg in "$@"; do
    case "$arg" in
        --ci)
            PROFILE="wasm-ci"
            MODE_LABEL="ci (fast compile, unoptimized)"
            ;;
        --release-stamp)
            RELEASE_STAMP=1
            ;;
        *)
            echo "Unknown argument: $arg" >&2
            echo "Usage: $0 [--ci] [--release-stamp]" >&2
            exit 2
            ;;
    esac
done

echo "Building WASM modules for @quillmark/wasm... [profile: $MODE_LABEL]"

cd "$(dirname "$0")/.."

# Start from a clean pkg/. CI restores a cached pkg/ from a previous build
# (restore-keys partial-matches an older release), and this script only ever
# mkdir/cp/sed *into* pkg/; it never removes files. Without this, any file
# dropped from the pkg layout between builds lingers and ships on `npm publish`.
rm -rf pkg

# Check for required tools. The CLI's version must match the wasm-bindgen
# crate in Cargo.lock; wasm-bindgen itself only detects a mismatch when it
# runs (after the multi-minute cargo build), so check it up front.
LOCKED_WBG=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"/\1/p')
if ! command -v wasm-bindgen &> /dev/null; then
    echo "wasm-bindgen not found. Install it with:" >&2
    echo "  cargo install wasm-bindgen-cli --version $LOCKED_WBG" >&2
    exit 1
fi
CLI_WBG=$(wasm-bindgen --version | awk '{print $2}')
if [ "$CLI_WBG" != "$LOCKED_WBG" ]; then
    echo "ERROR: wasm-bindgen-cli $CLI_WBG does not match Cargo.lock's wasm-bindgen $LOCKED_WBG." >&2
    echo "  cargo install wasm-bindgen-cli --version $LOCKED_WBG" >&2
    exit 1
fi
if ! command -v jq &> /dev/null; then
    echo "jq not found (needed to read the package version from cargo metadata)." >&2
    exit 1
fi

# Build one variant: cargo build with the given feature flags, then run
# wasm-bindgen into pkg/<subdir>/. Both variants emit the same wasm artifact
# name (quillmark_wasm.wasm) to the same target path, so they must run
# sequentially: each wasm-bindgen pass consumes the build before the next
# cargo build overwrites it.
#
# `--weak-refs` opts into FinalizationRegistry-based auto-free for
# wasm-bindgen handles; `.free()` is still emitted for deterministic teardown.
# (Runtime floor is the package.json `engines` field, not set here.)
#
# `--target web`, NOT `bundler`. The bundler target emits `import * as wasm from
# "./wasm_bg.wasm"` — the ESM-integration form no browser and no bundler
# resolves natively, so every consumer must add a wasm plugin, and the plugin's
# rewrite puts a top-level await on the module. Core is statically imported by
# the runtime layer, so that await lands on the static module graph of everyone
# importing @quillmark/wasm: a permanent constraint on consumer architecture and
# a blank page in Safari dev under SvelteKit. The web target emits no wasm
# import and no top-level await; the runtime layer owns instantiation instead
# (`runtime/runtime.js`, `init`). assert_no_tla below is the regression guard.
build_variant() {
    local subdir="$1"; shift
    local cargo_feature_args=("$@")

    echo ""
    echo "Building variant: $subdir"
    cargo build \
        --target wasm32-unknown-unknown \
        --profile "$PROFILE" \
        --manifest-path crates/bindings/wasm/Cargo.toml \
        "${cargo_feature_args[@]}"

    mkdir -p "pkg/$subdir"
    wasm-bindgen \
        "target/wasm32-unknown-unknown/$PROFILE/quillmark_wasm.wasm" \
        --out-dir "pkg/$subdir" \
        --out-name wasm \
        --target web \
        --weak-refs
}

# Patch a generated build so reaching it before instantiation throws a named
# QuillmarkError instead of a `Cannot read properties of undefined` from inside
# generated code. The sentinel and the reasoning are in runtime/uninit.js; this
# is the three-line edit that installs it.
#
# The two anchors are wasm-bindgen's, not ours, so their shape is asserted
# before anything is rewritten: a wasm-bindgen bump that renames the binding or
# changes the guard count fails the build here rather than shipping a build
# whose guard silently stopped biting. (`init.test.js` asserts the other end:
# that the patched artifact really throws.) The CLI/Cargo.lock version check
# above pins that shape per commit.
#
# `rel` is the path back to pkg/runtime/ from the variant's directory.
# No `sed -i`: BSD sed requires an argument to it, so this stays on awk + mv.
guard_wasm_js() {
    local file="$1" rel="$2" msg="$3" hint="$4"
    local decl="let wasmModule, wasm;"
    local guard="    if (wasm !== undefined) return wasm;"

    if [ "$(grep -cxF "$decl" "$file")" -ne 1 ]; then
        echo "ERROR: uninit guard: expected exactly one '$decl' in $file." >&2
        echo "  wasm-bindgen changed its generated shape; re-derive the anchors." >&2
        exit 1
    fi
    if [ "$(grep -cxF "$guard" "$file")" -ne 2 ]; then
        echo "ERROR: uninit guard: expected exactly two init guards in $file." >&2
        echo "  wasm-bindgen changed its generated shape; re-derive the anchors." >&2
        exit 1
    fi

    awk -v rel="$rel" -v msg="$msg" -v hint="$hint" -v decl="$decl" -v guard="$guard" '
        $0 == decl {
            printf "import { uninitSentinel, UNINIT } from \"%s/uninit.js\";\n", rel
            printf "let wasmModule, wasm = uninitSentinel(\"%s\", \"%s\");\n", msg, hint
            next
        }
        $0 == guard { print "    if (wasm !== undefined && !wasm[UNINIT]) return wasm;"; next }
        { print }
    ' "$file" > "$file.patched"
    mv "$file.patched" "$file"
}

# The regression guard for the target choice itself. If these ever match, the
# package has reacquired the ESM wasm import / top-level await that `--target
# web` exists to remove, and every consumer silently reacquires the plugin
# requirement and the Safari failure with it.
assert_no_tla() {
    local file="$1"
    if grep -qE 'from "\./[A-Za-z0-9_]+\.wasm"' "$file"; then
        echo "ERROR: $file carries an ESM .wasm import (the --target bundler form)." >&2
        exit 1
    fi
    if grep -qE '^await |^const [A-Za-z0-9_]+ = await ' "$file"; then
        echo "ERROR: $file carries a top-level await." >&2
        exit 1
    fi
}

# backends/typst   = default features (Typst).
# backends/pdfform = the Typst-free PDF-form backend with its web-sys canvas
#                    painter. Built like the typst variant: the same cargo build
#                    + wasm-bindgen pass, sequentially (every variant emits the
#                    same quillmark_wasm.wasm to the same target path, so they
#                    must not run concurrently).
# core             = no features (Typst excluded).
build_variant backends/typst
build_variant backends/pdfform --no-default-features --features pdfform
build_variant core --no-default-features

# Install the pre-init sentinel and assert the target choice held. Core's
# message is the one a consumer can actually reach; a backend's marks an
# internal bug, because `Engine` instantiates backends itself and no consumer
# path touches a backend build before it is ready.
echo ""
echo "Guarding generated builds against pre-init use"
guard_wasm_js pkg/core/wasm.js "../runtime" \
    "@quillmark/wasm is not initialized. Call 'await init()' once at startup, before Quill, Document, Engine, or any other export is used." \
    "Add: import { init } from '@quillmark/wasm'; await init(); once, anywhere before first use. Extra calls are free."
for backend in typst pdfform; do
    guard_wasm_js "pkg/backends/$backend/wasm.js" "../../runtime" \
        "@quillmark/wasm internal error: the '$backend' backend was used before instantiation." \
        "This is a bug in @quillmark/wasm, not in your code. Please report it."
done
for generated in pkg/core/wasm.js pkg/backends/typst/wasm.js pkg/backends/pdfform/wasm.js; do
    assert_no_tla "$generated"
done

# runtime = the canonical consumer API: a hand-written JS layer (NOT generated
# by wasm-bindgen) over core + the backend builds. It is plain source, so just
# copy it into pkg/ alongside the generated variants. `uninit.js` is the
# sentinel the guard patch above imports; `env-{node,web}.js` are the two halves
# of the `#quillmark-env` seam that package.json's `imports` map resolves.
echo ""
echo "Copying variant: runtime (hand-written canonical API)"
mkdir -p pkg/runtime
for source in runtime.js runtime.d.ts uninit.js env-node.js env-web.js; do
    cp "crates/bindings/wasm/runtime/$source" "pkg/runtime/$source"
done

# Note: a wasm-opt -Oz pass was tried and removed. With the current
# `wasm-release` profile (opt-level=z, fat LTO, codegen-units=1,
# panic=abort, strip=true) it saves only ~15 KB raw / ~10 KB gzipped
# (<0.1%): not worth the build dependency or the extra build time.

# Extract version and create package.json from template. Cargo.toml carries
# the LAST RELEASED version, so a from-source build is ahead of the number it
# would stamp. Default: mark it (next patch plus `-dev.<short-sha>`) so a
# dev pkg/ can never pass for a published release (npm dedupe, peer ranges,
# humans debugging read an honest number). `--release-stamp` stamps the
# version verbatim; only release.yml passes it, from the bumped release tag,
# and asserts the stamp equals the tag before `npm publish`.
VERSION=$(cargo metadata --format-version=1 --no-deps | jq -r '.packages[] | select(.name == "quillmark-wasm") | .version')
if [ -z "$VERSION" ] || [ "$VERSION" = "null" ]; then
    echo "ERROR: could not determine quillmark-wasm version from cargo metadata." >&2
    exit 1
fi
if [ "$RELEASE_STAMP" -ne 1 ]; then
    BASE=${VERSION%%-*}
    IFS=. read -r MAJOR MINOR PATCH <<< "$BASE"
    SHA=$(git rev-parse --short HEAD 2>/dev/null || echo local)
    VERSION="$MAJOR.$MINOR.$((PATCH + 1))-dev.$SHA"
fi
echo ""
echo "Creating package.json..."
sed "s/VERSION_PLACEHOLDER/$VERSION/" crates/bindings/wasm/package.template.json > pkg/package.json

# Copy README, CHANGELOG, and LICENSE files
if [ -f "crates/bindings/wasm/README.md" ]; then
    cp crates/bindings/wasm/README.md pkg/
fi
# Ship the workspace changelog so npmjs renders a Changelog tab for the
# published package (it is listed in package.template.json's "files").
if [ -f "CHANGELOG.md" ]; then
    cp CHANGELOG.md pkg/
fi
# The workspace ships one license, at LICENSE. It is the grant package.json's
# `license` field names, so a miss fails the build: a silent skip publishes a
# package whose declared license has no text behind it.
if [ -f "LICENSE" ]; then
    cp LICENSE pkg/
else
    echo "error: LICENSE not found; refusing to build an unlicensed package" >&2
    exit 1
fi

# .gitignore for pkg directory
cat > pkg/.gitignore << EOF
*
!.gitignore
EOF

echo ""
echo "WASM build complete!"
echo "Output directory: pkg/  (core/ + backends/typst/ + backends/pdfform/ + runtime/)"
echo "Package version: $VERSION"

# Show sizes: transport size (gzip/brotli) is what matters for delivery.
report_size() {
    local label="$1" file="$2"
    [ -f "$file" ] || return 0
    local raw gz br
    raw=$(du -h "$file" | cut -f1)
    gz=$(gzip -9 -c "$file" 2>/dev/null | wc -c | awk '{printf "%.2fM", $1/1048576}')
    if command -v brotli &> /dev/null; then
        br=$(brotli -9 -c "$file" 2>/dev/null | wc -c | awk '{printf "%.2fM", $1/1048576}')
        echo "WASM size ($label): raw=$raw gzip=$gz brotli=$br"
    else
        echo "WASM size ($label): raw=$raw gzip=$gz"
    fi
}
report_size "core"            pkg/core/wasm_bg.wasm
report_size "typst backend"   pkg/backends/typst/wasm_bg.wasm
report_size "pdfform backend" pkg/backends/pdfform/wasm_bg.wasm

# Size budget on the core artifact: the split only pays off if core stays
# Typst-free. Typst is megabytes, so a leak back into the no-features build
# would blow past this ceiling; fail rather than ship it silently. The gzip
# ceiling sits well above core's real size and far below anything carrying Typst.
#
# Only enforced on the size-optimized release profile (where the artifact
# publishes); the `wasm-ci` profile is unoptimized, so its size is meaningless
# here.
CORE_MAX_GZIP_BYTES=${CORE_MAX_GZIP_BYTES:-1500000}
if [ -f pkg/core/wasm_bg.wasm ] && [ "$PROFILE" = "wasm-release" ]; then
    core_gz_bytes=$(gzip -9 -c pkg/core/wasm_bg.wasm | wc -c | tr -d '[:space:]')
    if ! [ "$core_gz_bytes" -gt 0 ] 2>/dev/null; then
        echo "ERROR: could not measure core wasm gzip size (got '${core_gz_bytes}')." >&2
        exit 1
    fi
    if [ "$core_gz_bytes" -gt "$CORE_MAX_GZIP_BYTES" ]; then
        echo "ERROR: core wasm gzip ${core_gz_bytes} B exceeds budget ${CORE_MAX_GZIP_BYTES} B." >&2
        echo "       Typst or another heavy dep has leaked into the core (no-features) build." >&2
        exit 1
    fi
    echo "Core size budget OK: ${core_gz_bytes} B <= ${CORE_MAX_GZIP_BYTES} B gzip"
fi
