# Quillmark AI Agent Instructions

## Architecture

**Template-first Markdown rendering**: Parse (Markdown + YAML) → Template (MiniJinja) → Compile (backend glue) → Artifacts (PDF/SVG)

**Workspace structure** (publish order `core` → `typst` → `quillmark`, synchronized versions):
- `quillmark-core/` - Foundation (parsing, templating, Backend trait)
- `quillmark-typst/` - Typst backend implementation
- `quillmark-acroform/` - PDF Acroform backend implementation
- `quillmark/` - Orchestration layer (Quillmark engine and workflows)
- `quillmark-fixtures/` - Test resources
- `quillmark-fuzz/` - Fuzzing tests
- `quillmark-python/` - Python bindings
- `quillmark-wasm/` - WASM bindings


See `designs/ARCHITECTURE.md` for complete architecture.

## Documentation Strategy

- Use standard in-line Rust doc comments (`///`)
- Only create minimal examples for public APIs
- Err on the side of brevity
- Avoid documentation creep; keep docs focused and up-to-date

## Build & Test

```bash
cargo build --workspace --all-features
cargo test --workspace --all-features
cargo doc --no-deps --workspace --all-features
cargo run --example appreciated_letter
```

Tests: unit (in-file), integration (`tests/*.rs` with `common.rs`), doc (external `.md`), examples (use fixtures).

Before committing, ALWAYS run `cargo fmt` to ensure consistent formatting.

When working with WASM, install the wasm target with `rustup target add wasm32-unknown-unknown`
and use `scripts/build-wasm.sh` to build all targets.

## Extended YAML Metadata

Supports **inline metadata sections** with SCOPE/QUILL keys:

```markdown
---
SCOPE: products
name: Widget
---
Description here.
```

See `designs/PARSE.md`.

## Filter API (Stable Abstraction)

**Never import MiniJinja directly** - use `quillmark_core::templating::filter_api`:

```rust
use quillmark_core::templating::filter_api::{State, Value, Kwargs, Error, ErrorKind};
```

Common filters: `String`, `Lines`, `Date`, `Dict`, `Content`, `Asset` (prefixed with `DYNAMIC_ASSET__`).

## Error Handling

**Use `Diagnostic` everywhere** - never stringify prematurely:

Map external errors (MiniJinja, Typst) to preserve context.

## Implmentation Philosophy

- Do not worry about backwards compatibility. This is pre-1.0 software.

## Reference

- `designs/` - Complete architecture, workflows, specs
- `CONTRIBUTING.md` - Documentation standards
- `release.toml` - Release configuration
