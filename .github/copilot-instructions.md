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

## Code Documentation Strategy

- Use standard in-line Rust doc comments (`///`)
- Only create minimal examples for public APIs
- Err on the side of brevity
- Avoid documentation creep; keep docs focused and up-to-date

## Design Document Philosophy
All design documents in `designs/` should follow consistent principles:

- High-level only - Focus on architecture, not implementation
- Minimal code - Only essential examples, reference actual code
- Medium detail - Enough to understand, not enough to implement
- KISS - Keep it simple and maintainable
- References - Point to actual implementation for details

## Implementation/Testing Strategy

- Never worry about backwards compatibility. This is pre-1.0 software.

## Build & Test

```bash
cargo build --workspace --all-features
cargo test --workspace --all-features
cargo doc --no-deps --workspace --all-features
cargo run --example usaf_memo
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

## Reference

- `designs/` - Complete architecture, workflows, specs
- `CONTRIBUTING.md` - Documentation standards
- `release.toml` - Release configuration
