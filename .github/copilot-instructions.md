# Quillmark AI Agent Instructions

## Architecture

**Template-first Markdown rendering**: Parse (Markdown + YAML) → Template (MiniJinja) → Compile (backend glue) → Artifacts (PDF/SVG)

**Workspace structure** (publish order `core` → `typst` → `quillmark`, synchronized versions):
- `quillmark-core/` - Foundation (parsing, templating, Backend trait)
- `quillmark-typst/` - Typst backend implementation
- `quillmark/` - Orchestration layer (Quillmark engine and workflows)
- `quillmark-fixtures/` - Test resources (not published)

See `designs/DESIGN.md` for complete architecture.

## Documentation Strategy

- Use standard in-line Rust doc comments (`///`)
- Only create minimal examples for public APIs
- Err on the side of brevity

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

## Extended YAML Metadata (Non-Obvious)

Supports **inline metadata sections** with tag directives:

```markdown
---
!products
name: Widget
---
Description here.
```

Creates `products` array with metadata + `body` field. Rules:
- Tag `!name` on first line after `---`
- Blocks must be contiguous (no blank lines)
- `---` + blank line = horizontal rule (NOT metadata)

See `quillmark-core/docs/designs/PARSE.md`.

## Filter API (Stable Abstraction)

**Never import MiniJinja directly** - use `quillmark_core::templating::filter_api`:

```rust
use quillmark_core::templating::filter_api::{State, Value, Kwargs, Error, ErrorKind};
```

Common filters: `String`, `Lines`, `Date`, `Dict`, `Content`, `Asset` (prefixed with `DYNAMIC_ASSET__`).

## Error Handling

**Use `Diagnostic` everywhere** - never stringify prematurely:

```rust
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub primary: Option<Location>, // File, line, column
    pub hint: Option<String>,
}
```

Map external errors (MiniJinja, Typst) to preserve context.

## Implmentation Philosophy

- Do not worry about backwards compatibility. This is a new project.

## Reference

- `designs/` - Complete architecture, workflows, specs
- `CONTRIBUTING.md` - Documentation standards
- `release.toml` - Release configuration
