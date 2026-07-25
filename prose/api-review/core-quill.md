# Core: quill

Scope: `crates/core/src/quill/**`, `crates/core/src/quill.rs`.

## Surface

`quill.rs` re-exports from its private submodules (`mod blueprint; mod compose; mod config; mod resolved; mod fill; mod formats; mod ignore; mod load; mod query; mod schema; mod schema_yaml; mod seed; mod tree; mod types; pub(crate) mod validation;` — every submodule is private, so *everything* reachable from outside the crate funnels through the `pub use` lines below plus the items defined directly in `quill.rs`).

`crates/core/src/lib.rs` then re-exports only a **subset** of that at the crate root:

```rust
pub use quill::{
    zero_value, FieldSource, FileTreeNode, Quill, Resolved, ResolvedCard, ResolvedField, ResolvedMain,
    QuillIgnore, STANDARD_METADATA_KEYS,
};
```

So every item below is reachable at minimum via `quillmark_core::quill::<Name>`; items marked **[root]** are additionally reachable as `quillmark_core::<Name>`. Verified by compiling a throwaway external crate against `quillmark-core` (see Findings §5, §3).

### `Quill` (quill.rs) **[root]**

```rust
pub struct Quill { pub(crate) metadata, pub(crate) config, pub(crate) files } // fields private
impl Quill {
    pub fn name(&self) -> &str
    pub fn backend_id(&self) -> &str
    pub fn metadata(&self) -> &HashMap<String, QuillValue>
    pub fn config(&self) -> &QuillConfig
    pub fn writer<'a>(&'a self, doc: &'a mut Document) -> TypedWriter<'a>
    pub fn reader<'a>(&'a self, doc: &'a Document) -> TypedReader<'a>
    pub fn files(&self) -> &FileTreeNode
    pub fn to_tree(&self) -> Vec<(String, Vec<u8>)>
    // load.rs
    pub fn from_tree(root: FileTreeNode) -> Result<Self, Vec<Diagnostic>>
    // query.rs — mirrors FileTreeNode's read methods
    pub fn get_file<P>(&self, path: P) -> Option<&[u8]>
    pub fn file_exists<P>(&self, path: P) -> bool
    pub fn dir_exists<P>(&self, path: P) -> bool
    pub fn list_files<P>(&self, path: P) -> Vec<String>
    pub fn list_subdirectories<P>(&self, path: P) -> Vec<String>
    pub fn list_directories<P>(&self, dir_path: P) -> Vec<PathBuf>
    pub fn find_files<P>(&self, pattern: P) -> Vec<PathBuf>
    // compose.rs — forwards to QuillConfig
    pub fn compile_data(&self, doc: &Document) -> Result<serde_json::Value, RenderError>
    pub fn dry_run(&self, doc: &Document) -> Result<(), RenderError>
    pub fn check_quill_reference(&self, doc: &Document) -> Result<(), RenderError>
    // compose.rs — Quill-only, no QuillConfig equivalent
    pub fn validate(&self, doc: &Document) -> Vec<Diagnostic>
    pub fn seed_document(&self) -> Document
    pub fn seed_main(&self) -> Card
    pub fn seed_card(&self, card_kind: &str, overlay: Option<&SeedOverlay>) -> Option<Card>
    // resolved.rs
    pub fn resolve(&self, doc: &Document) -> Resolved
}
impl Debug for Quill // hand-written; hides `metadata`
pub const STANDARD_METADATA_KEYS: &[&str] // [root]
```

### `QuillConfig` / schema model (config.rs, blueprint.rs, schema.rs, schema_yaml.rs, compose.rs) — module-path only

```rust
pub struct QuillConfig { // ALL fields pub; Serialize + Deserialize derived, no custom invariants
    pub name: String, pub description: String, pub main: CardSchema,
    pub card_kinds: Vec<CardSchema>, pub backend: String, pub version: String,
    pub author: String, pub backend_config: HashMap<String, QuillValue>,
}
impl QuillConfig {
    pub fn from_yaml(yaml: &str) -> Result<Self, Box<dyn Error + Send + Sync>>
    pub fn from_yaml_with_warnings(yaml: &str) -> Result<(Self, Vec<Diagnostic>), Vec<Diagnostic>>
    pub fn card_kind(&self, name: &str) -> Option<&CardSchema>
    pub fn schema(&self) -> serde_json::Value
    pub fn schema_yaml(&self) -> Result<String, serde_saphyr::ser::Error>
    pub fn coerce_payload(&self, payload: &IndexMap<String, QuillValue>) -> Result<IndexMap<String, QuillValue>, CoercionError>
    pub fn coerce_card(&self, card_kind: &str, fields: &IndexMap<String, QuillValue>) -> Result<IndexMap<String, QuillValue>, CoercionError>
    pub fn validate_document(&self, doc: &Document) -> Result<(), Vec<validation::ValidationError>> // ValidationError unnameable, see Finding 3
    pub fn compile_data(&self, doc: &Document) -> Result<serde_json::Value, RenderError>
    pub fn dry_run(&self, doc: &Document) -> Result<(), RenderError>
    pub fn check_quill_reference(&self, doc: &Document) -> Result<(), RenderError>
    pub fn blueprint(&self) -> String
}
pub enum CoercionError { Uncoercible { path, value, target, reason } } // thiserror, nameable
```

### Field/card schema types (types.rs) — module-path only

```rust
pub struct CardSchema { pub name, pub description: Option<String>, pub fields: IndexMap<String, FieldSchema>, pub ui: Option<UiCardSchema>, pub body: Option<BodyCardSchema> }
impl CardSchema { pub fn defaults(&self) -> HashMap<String, QuillValue>; pub fn body_enabled(&self) -> bool }

pub enum FieldType { String, Number, Integer, Boolean, Array, Object, Date, DateTime,
    RichText { inline: bool }, PlainText { inline: bool }, Enum }
impl FieldType { pub fn from_str(s: &str) -> Option<Self>; pub fn as_str(&self) -> &'static str }

pub struct FieldSchema { // ALL fields pub
    pub name, pub r#type: FieldType, pub description: Option<String>,
    pub default: Option<QuillValue>, pub example: Option<QuillValue>, pub ui: Option<UiFieldSchema>,
    pub enum_values: Option<Vec<String>>, pub properties: Option<IndexMap<String, Box<FieldSchema>>>,
    pub items: Option<Box<FieldSchema>>,
    pub default_content: Option<QuillValue>, pub example_content: Option<QuillValue>, // load-time caches
}
impl FieldSchema {
    pub fn new(name: String, r#type: FieldType, description: Option<String>) -> Self // fills 3/9 fields
    pub fn from_quill_value(key: String, value: &QuillValue) -> Result<Self, String>
}

pub struct UiFieldSchema { pub title, pub group, pub compact, pub multiline: Option<_> }
pub struct BodyCardSchema { pub enabled: Option<bool>, pub example: Option<String>, pub example_content: Option<QuillValue> }
pub struct UiCardSchema { pub title: Option<String>, pub groups: Option<GroupRegistry> }
pub struct GroupSchema { pub id: String, pub title: Option<String> }
pub struct GroupRegistry(pub Vec<GroupSchema>);
```

### `FileTreeNode` (tree.rs) **[root]**

```rust
pub enum FileTreeNode { File { contents: Vec<u8> }, Directory { files: HashMap<String, FileTreeNode> } }
impl FileTreeNode {
    pub fn get_node<P>(&self, path: P) -> Option<&FileTreeNode>
    pub fn get_file<P>(&self, path: P) -> Option<&[u8]>
    pub fn file_exists<P>(&self, path: P) -> bool
    pub fn dir_exists<P>(&self, path: P) -> bool
    pub fn list_files<P>(&self, dir_path: P) -> Vec<String>
    pub fn list_subdirectories<P>(&self, dir_path: P) -> Vec<String>
    pub fn insert<P>(&mut self, path: P, node: FileTreeNode) -> Result<(), Box<dyn Error + Send + Sync>>
    pub fn flatten(&self) -> Vec<(String, Vec<u8>)>
}
```

### `Resolved*` / `FieldSource` (resolved.rs) **[root]**

```rust
pub enum FieldSource { Authored, Default, Zero } // Serialize, lowercase
pub struct ResolvedField { pub name: String, pub value: QuillValue, pub source: FieldSource }
pub struct ResolvedMain { pub fields: Vec<ResolvedField>, pub body: Option<ResolvedField> }
pub struct ResolvedCard { pub kind: Option<String>, pub index: usize, pub fields: Vec<ResolvedField>, pub body: Option<ResolvedField> }
pub struct Resolved { pub main: ResolvedMain, pub cards: Vec<ResolvedCard> }
```

### `QuillIgnore` (ignore.rs) **[root]**

```rust
pub struct QuillIgnore { pub(crate) patterns: Vec<String> } // Default impl = built-in skip list, not empty
impl QuillIgnore {
    pub fn new(patterns: Vec<String>) -> Self
    pub fn from_content(content: &str) -> Self
    pub fn is_ignored<P>(&self, path: P) -> bool
}
```

### Free functions / constants — module-path only

```rust
pub fn zero_value(field: &FieldSchema) -> QuillValue                       // fill.rs
pub fn parse_date(s: &str) -> Option<(i32, u8, u8)>                        // formats.rs
pub fn parse_datetime(s: &str) -> Option<(i32, u8, u8, u8, u8, u8)>        // formats.rs
pub fn build_transform_schema(config: &QuillConfig) -> QuillValue          // schema.rs
pub const CONTENT_MEDIA_TYPE: &str                                         // schema.rs — exported
pub const QUILLMARK_INLINE_KEY: &str                                       // schema.rs — exported
pub const QUILLMARK_PLAIN_KEY: &str                                        // schema.rs — declared pub but NOT re-exported by quill.rs; unreachable outside the crate at all
```

## Findings

### 1. `QuillConfig`/`CardSchema`/`FieldSchema` are open structs with derived `Deserialize` — the validation pipeline is opt-in, not enforced
**Severity: High** — `crates/core/src/quill/config.rs:54-77`, `crates/core/src/quill/types.rs:206-221,405-441`

`QuillConfig`, `CardSchema`, `FieldSchema`, `BodyCardSchema`, `UiCardSchema` all have every field `pub` and derive plain `Deserialize` (no `deny_unknown_fields`, no post-parse check). `QuillConfig::from_yaml`/`from_yaml_with_warnings` is the only path that enforces snake_case names, single-line descriptions, one-level nesting, enum/group/reserved-character rules, and populates the `default_content`/`example_content` caches — but nothing stops a caller from building the struct directly (`QuillConfig { name: "Not Valid!".into(), .. }`) or via `serde_json::from_value::<QuillConfig>(..)`, silently skipping every one of those checks. This is not hypothetical: `crates/fuzz/src/coerce_fuzz.rs:137`, `crates/backends/pdfform/src/bind.rs:415`, `crates/core/src/reader.rs:251`, `crates/core/src/writer.rs:292`, and `crates/core/src/quill/validation.rs:621` all construct `QuillConfig { .. }` struct literals directly today, bypassing `from_yaml` entirely. The same openness lets a caller mutate a *validated* config after the fact (`let mut c = QuillConfig::from_yaml(yaml)?; c.name = "bad name".into();`) and silently invalidate the `from_yaml` guarantee downstream code relies on (see Finding 2). `FieldSchema::new` (types.rs:467) compounds this: it only sets 3 of 9 fields, so `FieldType::Array` with `items: None` or `FieldType::Enum` with `enum_values: None` — states the loader's own shape-validator (`validate_field_schema_shape`) explicitly rejects at load time — are one call away from any downstream `pub` API.

A caller hits: silent skew between "what `from_yaml` would have rejected" and "what actually reaches `blueprint()`/`compile_data()`/`build_transform_schema()`", with no compiler or runtime signal until one of those functions panics or emits nonsense.

### 2. `QuillConfig::blueprint()` panics on exactly the config shape Finding 1 makes reachable
**Severity: High** — `crates/core/src/quill/blueprint.rs:72,119-121`

```rust
let reference = quill_ref.parse().expect("quill name@version is always a valid QuillReference");
```
`quill_ref` is `format!("{}@{}", self.name, self.version)` built from `QuillConfig::name`/`version` with no re-validation. `blueprint()`'s own doc comment claims "the function is total over any valid `QuillConfig`" (blueprint.rs:59) — but "valid" is never a type-level guarantee (Finding 1), so a directly-constructed or post-hoc-mutated `QuillConfig` with a non-snake_case name or a non-semver version panics here rather than erroring. The codebase already knows this failure mode and handles it gracefully elsewhere for the *identical* input: `seed.rs:97-102`'s `main_reference` builds the same `"{name}@{version}"` string and falls back via `.unwrap_or_else(|| QuillReference::latest(..))` instead of panicking, and `compose.rs:465-483` explicitly comments on and defends against "a serde-built `QuillConfig`, never `from_yaml`" for the content-cache case. `blueprint()` is the one place that doesn't extend the same courtesy.

A caller hits: an unrecoverable panic (not a `Result`) from a documented, `pub`, total-sounding method, for input the type system does nothing to prevent.

### 3. `QuillConfig::validate_document`'s error type is unnameable outside the crate
**Severity: Medium** — `crates/core/src/quill/config.rs:231-236`, `crates/core/src/quill.rs:17`

`validate_document` is `pub fn … -> Result<(), Vec<super::validation::ValidationError>>`, but `validation` is `pub(crate) mod validation;` in quill.rs — so `ValidationError` has no external path. Verified by compiling a throwaway dependent crate: `use quillmark_core::quill::validation::ValidationError` fails with `E0603: module 'validation' is private`, while calling `config.validate_document(doc)` and then `for e in errors { e.to_diagnostic(); e.path(); }` inline compiles fine (the type is usable but not nameable). A downstream crate cannot write a function signature that takes or returns `ValidationError`, cannot store it in a struct field, cannot `impl` a trait for it — every consumer is forced to convert immediately inline (typically via the one escape hatch, `.to_diagnostic()`). Compare `Quill::validate()` (compose.rs:197), which returns plain, fully public `Diagnostic`s.

A caller hits: cannot factor "handle a validation error" into its own named function/type without either working entirely inside closures/for-loops or converting to `Diagnostic` immediately and losing the structured `path()`/`code()` accessors' typed home.

### 4. `validate()`/`validate_seed()` are stranded on `Quill`, unlike their siblings `compile_data`/`dry_run`/`check_quill_reference`
**Severity: Medium** — `crates/core/src/quill/compose.rs:18-33` vs `40-181` vs `183-303`

`compile_data`, `dry_run`, and `check_quill_reference` live as inherent methods on `QuillConfig`, with `Quill`'s versions (compose.rs:18-33) as one-line forwarders. The doc comment explains why: "Living on `QuillConfig` lets a consumer that only compiles data (e.g. a live session's `apply`) retain the config alone rather than the whole quill with its font/package bytes" (compose.rs:35-39). `validate()` (compose.rs:197), `validate_seed()` (compose.rs:218), and the free function `validate_fills(config: &QuillConfig, doc: &Document)` (compose.rs:552) it calls are equally pure `QuillConfig` reads — `validate_fills` already takes `&QuillConfig`, not `&Quill` — yet `validate()`/`validate_seed()` are `impl Quill`-only with no `QuillConfig` counterpart. The config-only consumer the doc comment describes for the other three methods has no way to get must-fill/seed-overlay diagnostics without holding the full `Quill`, and falls back to the bare `validate_document` from Finding 3 as the only config-level alternative.

A caller hits: a `QuillConfig`-only session/cache layer (the exact use case `compile_data` was designed for) cannot surface `validation::must_fill` warnings or `$seed` overlay diagnostics without reconstructing or retaining a full `Quill`.

### 5. Crate-root re-exports omit the schema-model types `Quill::config()` hands back
**Severity: Medium** — `crates/core/src/lib.rs:63-67`

`lib.rs` re-exports `zero_value, FieldSource, FileTreeNode, Quill, Resolved, ResolvedCard, ResolvedField, ResolvedMain, QuillIgnore, STANDARD_METADATA_KEYS` at the crate root, but not `QuillConfig`, `CoercionError`, `CardSchema`, `FieldSchema`, `FieldType`, `BodyCardSchema`, `UiCardSchema`, `UiFieldSchema`, `GroupSchema`, `GroupRegistry`, `build_transform_schema`, `QUILLMARK_INLINE_KEY`, `CONTENT_MEDIA_TYPE`, `parse_date`, `parse_datetime`. Confirmed: `use quillmark_core::QuillConfig;` fails to resolve (`E0432`); the same name resolves fine as `quillmark_core::quill::QuillConfig`. `Quill::config()` is one of the type's most central methods — `Quill::writer`/`reader` are documented as "schema-bound" and everything about field types, defaults, and card kinds flows through `QuillConfig`/`CardSchema`/`FieldSchema` — yet none of those types get the short path, while the read-only `Resolved*` view types do. There's no stated rationale (e.g. in QUILL.md/SCHEMAS.md) for treating the two families differently at the crate-root boundary.

A caller hits: `use quillmark_core::{Quill, CardSchema};` fails; must discover and switch to `quillmark_core::quill::CardSchema` for the schema types while `Quill` itself, `FileTreeNode`, and `Resolved*` resolve directly.

### 6. `QUILLMARK_PLAIN_KEY` has no path out of the crate at all
**Severity: Low** — `crates/core/src/quill/schema.rs:27`, `crates/core/src/quill.rs:25`

`schema.rs` declares `pub const QUILLMARK_PLAIN_KEY: &str = "quillmark:plain"` alongside `QUILLMARK_INLINE_KEY`, both used identically as `build_transform_schema` annotation keys (schema.rs:83-89 vs 63-68). But quill.rs's re-export line only lists `pub use schema::{build_transform_schema, QUILLMARK_INLINE_KEY, CONTENT_MEDIA_TYPE};` — `QUILLMARK_PLAIN_KEY` is missing, and since `schema` is a private module, the constant is entirely unreachable from outside the crate (not even via a long path). A consumer of `build_transform_schema`'s JSON who wants to detect a `plaintext` field must hardcode the string `"quillmark:plain"`, while the parallel `inline` marker has a proper importable constant.

### 7. `STANDARD_METADATA_KEYS` doc comment is stale about which identity fields get typed accessors
**Severity: Low** — `crates/core/src/quill.rs:36-41,64-67`

The doc comment reads: "the quill-config keys every binding surfaces as typed, top-level fields (`name` via `Quill::name`; the rest via `Quill::metadata`)." But `Quill::backend_id()` (quill.rs:65) is also a dedicated typed accessor for `backend`, one of "the rest." The comment underclaims the Rust API's own accessor surface (2 of 5 identity keys get first-class methods, not 1), which will mislead a reader into missing `backend_id()` or wondering why `backend` needs both `metadata().get("backend")` and `backend_id()`.

### 8. `Quill::find_files` swallows a malformed pattern the same as "no matches"
**Severity: Low** — `crates/core/src/quill/query.rs:51-59`

```rust
let glob_pattern = match glob::Pattern::new(&pattern_str) {
    Ok(pat) => pat,
    Err(_) => return matches, // Invalid pattern returns empty results
};
```
An invalid glob (typo'd bracket, bad syntax) and a valid glob that simply matches nothing both return `vec![]`. The method returns `Vec<PathBuf>`, not a `Result`, so there is no way for a caller to distinguish "your pattern is broken" from "nothing matched."

### 9. `Quill` duplicates `FileTreeNode`'s read methods instead of pointing at `files()`
**Severity: Low** — `crates/core/src/quill/query.rs:8-30` vs `crates/core/src/quill.rs:96-99`

`Quill::get_file`/`file_exists`/`dir_exists`/`list_files`/`list_subdirectories` (query.rs) are one-line forwarders to the identically-named `FileTreeNode` methods, while `Quill::files()` (quill.rs:97) already hands back the same `&FileTreeNode` directly. Both `quill.get_file(p)` and `quill.files().get_file(p)` are equally public and reach the same data. QUILL.md's documented "File access on FileTreeNode" section names only the `FileTreeNode` methods, not this parallel `Quill`-level set, so the duplication isn't even canon-documented. `list_directories`/`find_files` (query.rs only) break the mirror further — they have no `FileTreeNode` equivalent — so the split between "lives on `Quill`" and "lives on `FileTreeNode`" isn't a clean rule.

### 10. `FieldSchema::new` is a "constructor" that can't express most of the type's own required invariants
**Severity: Medium** — `crates/core/src/quill/types.rs:467-481`

```rust
pub fn new(name: String, r#type: FieldType, description: Option<String>) -> Self
```
sets `default`/`example`/`ui`/`enum_values`/`properties`/`items`/`default_content`/`example_content` all to `None`/defaults, then relies on the caller mutating the public fields afterward to reach a valid shape. For `FieldType::Array`/`FieldType::Object`/`FieldType::Enum` this default state is exactly what `validate_field_schema_shape` (config.rs:740) rejects at Quill.yaml load time (`quill::array_missing_items`, `quill::object_missing_properties`, "`type: enum` requires a non-empty `values:` list") — but `FieldSchema::new` has no way to supply those at construction, and nothing enforces supplying them after. This is the field-level instance of Finding 1's root cause, and is already exercised as a deliberate feature by `crates/fuzz/src/coerce_fuzz.rs` (building adversarial schemas for the coercion fuzzer) — reasonable for a fuzz harness, but `FieldSchema::new` ships as ordinary `pub` API with no fuzz-only gate, so any downstream crate inherits the same footgun for non-adversarial use.

## Cross-cutting

- Findings 1 and 10's "open struct, hand-built instances" pattern is already load-bearing outside `core`: `crates/fuzz/src/coerce_fuzz.rs` and `crates/backends/pdfform/src/bind.rs` construct `QuillConfig`/`FieldSchema` directly. Any fix that tightens `QuillConfig`/`FieldSchema` construction (private fields, a checked builder) needs coordinated updates in those crates, not just `core`.
- Finding 5 (crate-root re-export gap for `QuillConfig`/`CardSchema`/`FieldSchema`/etc.) is worth checking against `crates/quillmark/src/lib.rs` and the `bindings/{python,wasm}` crates — a quick grep found no re-export of these types there either, so any binding wanting to expose the schema model directly (beyond `Quill::config().schema()`'s JSON) already has to reach through the long `quillmark_core::quill::` path; worth confirming with whichever review owns those crates whether that's deliberate.
- Finding 4 (`validate()` stranded on `Quill`) references `compose.rs`'s own comment about "a live session's `apply`" as the intended `QuillConfig`-only consumer — that's `crate::session`/`LiveSession` (out of this review's scope, `document/**`); worth flagging to whichever review owns session code, since it may already be working around the missing config-level `validate()`.
