# Contributing to Quillmark

## Documentation Strategy

- Use standard in-line Rust doc comments (`///`)
- Only create minimal examples for public APIs
- Err on the side of brevity

## Binding tests

**WASM:** repo root → `./scripts/build-wasm.sh` → `cd crates/bindings/wasm` → `npm install` (first time) → `npm run test`

**Python:** `cd crates/bindings/python` → `uv sync --extra dev` → `uv run maturin develop` → `uv run pytest`

## Documentation

### Design Documents

Design documents and comprehensive specifications are stored in `prose/designs`.