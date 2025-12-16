# Composable Cards Refactor

Move from current CARDS design to: unified CARDS array, separate CardSchema, and OpenAPI 3.0 discriminator.

## Phase 1: Rust Data Model [COMPLETED]

### `crates/core/src/quill.rs`

#### [NEW] CardSchema struct

Add new struct after `UiSchema`:

```rust
pub struct CardSchema {
    pub name: String,
    pub title: Option<String>,
    pub description: String,
    pub fields: HashMap<String, FieldSchema>,
}
```

#### [MODIFY] FieldSchema

Remove `items` field (line ~36).

#### [MODIFY] QuillConfig

Add `cards: HashMap<String, CardSchema>` field.

#### [MODIFY] QuillConfig::from_toml

Update `[cards]` parsing to:
1. Create `CardSchema` instead of `FieldSchema`
2. Parse `[cards.X.fields.Y]` instead of `[cards.X.items.Y]`
3. Store in `cards` HashMap, not `fields`

---

## Phase 2: JSON Schema Generation [COMPLETED]

### `crates/core/src/schema.rs`

#### [MODIFY] build_schema_from_fields

Rename to `build_schema` and accept both fields and cards:

```rust
pub fn build_schema(
    fields: &HashMap<String, FieldSchema>,
    cards: &HashMap<String, CardSchema>,
) -> Result<QuillValue, RenderError>
```

Changes:
1. Build `$defs` section with card schemas
2. Add `properties.CARDS` array with `oneOf` refs
3. Add `x-discriminator` with mapping
4. Each card schema has `"CARD": { "const": "..." }`

#### [NEW] build_card_def

Helper function to build a single card schema for `$defs`:

```rust
fn build_card_def(card: &CardSchema) -> Map<String, Value>
```

---

## Phase 3: Quill.toml Migration [COMPLETED]

### `crates/fixtures/resources/tonguetoquill-collection/quills/usaf_memo/Quill.toml`

Change `[cards.X.items.Y]` to `[cards.X.fields.Y]`.

---

## Phase 4: Design Doc Updates [COMPLETED]

### `prose/designs/INDEX.md`

Add CARDS.md entry, mark SCOPES.md as superseded.

### `prose/designs/SCHEMAS.md`

Update type mapping to reference CARDS.md for card types.

---

## Phase 5: Parser SCOPE→CARD Rename [COMPLETED]

### `crates/core/src/parse.rs`

Rename `SCOPE` keyword to `CARD`:

1. Update `SCOPE_KEY` constant to `CARD_KEY = "CARD"`
2. Update all references from `SCOPE` to `CARD`
3. Update error messages
4. Update test cases

### `prose/designs/PARSE.md`

Update Extended YAML Metadata Standard section to use `CARD:` syntax.

---

## Verification

### Automated Tests

Run existing tests to verify no regressions:

```bash
cargo test --workspace --all-features
```

### New Test Cases

Add to `crates/core/src/schema.rs` tests:

1. `test_build_schema_with_cards` - Verify `$defs` and `CARDS` array structure
2. `test_card_discriminator` - Verify `x-discriminator` and `const` constraints

Add to `crates/core/src/quill.rs` tests:

1. `test_parse_cards_fields_syntax` - Verify `[cards.X.fields.Y]` parsing

### Manual Verification

Inspect JSON schema output for usaf_memo to confirm:
- `$defs` contains `indorsements_card`
- `properties.CARDS` exists with `oneOf`
- `x-discriminator.propertyName` is `"CARD"`

```bash
# After implementation, run this to inspect output
cargo run --example print_schema -- usaf_memo
```

(If no such example exists, add a simple debug print in tests)
