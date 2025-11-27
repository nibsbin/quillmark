# Plan: CLI Schema Command

> **Status**: Proposed
> **Owner**: Architect Agent

## Goal
Implement the `quillmark schema` command to retrieve and output a Quill's field schema as JSON.

## Context
Users need a way to inspect the schema of a Quill to understand what fields are required and how to structure their markdown frontmatter. This command will expose the internal JSON schema of a Quill.

## Design Reference
- `prose/designs/CLI.md` - Updated with `schema` command specification.
- `prose/designs/SCHEMAS.md` - Describes the Quill schema structure.

## Implementation Steps

### 1. Create Command Module
- Create `crates/bindings/cli/src/commands/schema.rs`.
- Define `SchemaArgs` struct using `clap`.
    - `quill_path`: PathBuf (required)
    - `output`: Option<PathBuf> (optional, for output file)

### 2. Implement Execution Logic
- Implement `execute(args: SchemaArgs) -> Result<()>` in `schema.rs`.
- Logic:
    1. Validate `quill_path` exists.
    2. Load `Quill` using `Quill::from_path`.
    3. Access `quill.schema` (which is a `QuillValue`).
    4. Serialize `quill.schema` to JSON string (pretty printed).
    5. If `output` is provided, write to file.
    6. Else, print to stdout.

### 3. Register Command
- Update `crates/bindings/cli/src/commands/mod.rs` to export `schema` module.
- Update `crates/bindings/cli/src/main.rs`:
    - Add `Schema(commands::schema::SchemaArgs)` to `Commands` enum.
    - Add match arm in `main` to call `commands::schema::execute`.

### 4. Verification
- Build the CLI: `cargo build -p quillmark-cli`.
- Run against a test quill: `target/debug/quillmark schema crates/fixtures/resources/tonguetoquill-collection/usaf-memo`.
- Verify output is valid JSON.
