# Simplification Cascades for Quillmark

**Analysis Date**: 2025-11-14
**Scope**: Core libraries (quillmark-core, quillmark), backends (typst, acroform)
**Excluded**: Bindings (Python, WASM, CLI)

## Executive Summary

This document identifies 5 major simplification cascades where **one insight eliminates multiple components**. These opportunities follow the pattern: "If this is true, we don't need X, Y, or Z."

**Total potential impact**: ~370 lines of code eliminated, 6 duplicate implementations removed, significant improvement in maintainability.

---

## Cascade 1: Unify Glue Engine Implementations

### Current State (Symptom: Same thing implemented 2 ways)

**Location**: `quillmark-core/src/templating.rs`

Two separate implementations of the glue engine concept:

```rust
// TemplateGlue (lines 135-217)
struct TemplateGlue {
    template: String,
    filters: HashMap<String, FilterFn>,
}

// AutoGlue (lines 219-275)
struct AutoGlue {
    filters: HashMap<String, FilterFn>,  // ← Never actually used!
}

// Glue enum dispatches between them (lines 277-363)
enum Glue {
    Template(TemplateGlue),
    Auto(AutoGlue),
}
```

**Problems**:
- Both implement `GlueEngine` trait with identical method signatures
- AutoGlue stores filters but never uses them (see line 231-234 comment: "maintains consistency")
- Pattern matching dispatch required in `Glue::compose()` (lines 289-305)
- Both have duplicate `register_filter()` implementations

### The Insight

> **"The difference is not in the engine, but in the output mode"**

The two implementations differ ONLY in what they output:
- TemplateGlue: renders MiniJinja template → string
- AutoGlue: serializes context as JSON → string

Everything else (filter registration, state management, interface) is identical.

### After: Single Implementation

```rust
enum OutputMode {
    Template(String),  // Has template to render
    Auto,              // No template, output JSON
}

struct Glue {
    output_mode: OutputMode,
    filters: HashMap<String, FilterFn>,
}

impl Glue {
    fn compose(&mut self, context: HashMap<String, Value>) -> Result<String, TemplateError> {
        match &self.output_mode {
            OutputMode::Template(template) => {
                // Render template with filters
                let env = self.create_env_with_filters();
                env.render_str(template, context)
            }
            OutputMode::Auto => {
                // Serialize context as JSON
                serde_json::to_string_pretty(&context)
                    .map_err(|e| TemplateError::Rendering(e.to_string()))
            }
        }
    }
}
```

### What Gets Eliminated

- ❌ `TemplateGlue` struct (82 lines)
- ❌ `AutoGlue` struct (56 lines)
- ❌ `GlueEngine` trait (no longer needed)
- ❌ Pattern matching dispatch in `Glue` enum
- ❌ Duplicate `register_filter()` implementations
- ❌ Special case comment about unused filters

**Total**: ~150 lines eliminated

### Cascade Measurement

**One insight** (output mode, not engine type) eliminates **4 components** (2 structs, 1 trait, pattern dispatch).

**Future benefit**: Adding new output modes becomes a single enum variant instead of a whole new implementation.

---

## Cascade 2: Generalize Dynamic Collection Management

### Current State (Symptom: Same thing implemented 2 ways)

**Location**: `quillmark/src/orchestration.rs`

Dynamic assets and fonts managed with **100% identical logic** except for names:

```rust
// Assets (lines 630-676)
fn add_asset(&mut self, filename: String, contents: Vec<u8>) -> Result<(), RenderError> {
    if self.dynamic_assets.contains_key(&filename) {
        return Err(RenderError::DynamicAssetCollision { /* ... */ });
    }
    self.dynamic_assets.insert(filename, contents);
    Ok(())
}

fn add_assets(&mut self, assets: HashMap<String, Vec<u8>>) -> Result<(), RenderError> {
    for (filename, contents) in assets {
        self.add_asset(filename, contents)?;
    }
    Ok(())
}

fn dynamic_asset_names(&self) -> Vec<String> {
    self.dynamic_assets.keys().cloned().collect()
}

fn clear_assets(&mut self) {
    self.dynamic_assets.clear();
}

// Fonts (lines 678-727) - EXACT SAME PATTERN
fn add_font(&mut self, filename: String, contents: Vec<u8>) -> Result<(), RenderError> { /* identical */ }
fn add_fonts(&mut self, fonts: HashMap<String, Vec<u8>>) -> Result<(), RenderError> { /* identical */ }
fn dynamic_font_names(&self) -> Vec<String> { /* identical */ }
fn clear_fonts(&mut self) { /* identical */ }

// prepare_quill_with_assets (lines 730-756)
// Two identical loops with only different prefixes:
for (name, bytes) in &self.dynamic_assets {
    quill.files.insert(format!("DYNAMIC_ASSET__{}", name), bytes.clone());
}
for (name, bytes) in &self.dynamic_fonts {
    quill.files.insert(format!("DYNAMIC_FONT__{}", name), bytes.clone());
}
```

**Only differences**:
1. Field names: `dynamic_assets` vs `dynamic_fonts`
2. Prefix strings: `"DYNAMIC_ASSET__"` vs `"DYNAMIC_FONT__"`
3. Error types: `DynamicAssetCollision` vs `DynamicFontCollision`

### The Insight

> **"All dynamic collections are the same: named byte buffers with collision detection and prefixed injection"**

This is a container pattern problem. The logic is identical - we're just managing named byte arrays.

### After: Generic Collection

```rust
struct DynamicCollection {
    items: HashMap<String, Vec<u8>>,
    prefix: &'static str,
    collision_error: fn(String) -> RenderError,
}

impl DynamicCollection {
    fn new(prefix: &'static str, collision_error: fn(String) -> RenderError) -> Self {
        Self {
            items: HashMap::new(),
            prefix,
            collision_error,
        }
    }

    fn add(&mut self, filename: String, contents: Vec<u8>) -> Result<(), RenderError> {
        if self.items.contains_key(&filename) {
            return Err((self.collision_error)(filename));
        }
        self.items.insert(filename, contents);
        Ok(())
    }

    fn add_many(&mut self, items: HashMap<String, Vec<u8>>) -> Result<(), RenderError> {
        for (filename, contents) in items {
            self.add(filename, contents)?;
        }
        Ok(())
    }

    fn names(&self) -> Vec<String> {
        self.items.keys().cloned().collect()
    }

    fn clear(&mut self) {
        self.items.clear();
    }

    fn inject_into_quill(&self, quill: &mut Quill) {
        for (name, bytes) in &self.items {
            quill.files.insert(format!("{}{}", self.prefix, name), bytes.clone());
        }
    }
}

// Usage in Workflow
struct Workflow {
    dynamic_assets: DynamicCollection,
    dynamic_fonts: DynamicCollection,
}

impl Workflow {
    fn new(...) -> Self {
        Self {
            dynamic_assets: DynamicCollection::new(
                "DYNAMIC_ASSET__",
                |name| RenderError::DynamicAssetCollision { /* ... */ }
            ),
            dynamic_fonts: DynamicCollection::new(
                "DYNAMIC_FONT__",
                |name| RenderError::DynamicFontCollision { /* ... */ }
            ),
        }
    }
}

// prepare_quill_with_assets becomes:
fn prepare_quill_with_assets(&self, quill: &mut Quill) {
    self.dynamic_assets.inject_into_quill(quill);
    self.dynamic_fonts.inject_into_quill(quill);
}
```

### What Gets Eliminated

- ❌ 46 lines of `add_asset/add_assets/clear_assets/dynamic_asset_names`
- ❌ 46 lines of `add_font/add_fonts/clear_fonts/dynamic_font_names`
- ❌ Duplicate collision checking logic
- ❌ Duplicate iteration in `prepare_quill_with_assets`
- ❌ Separate error types for the same scenario (could unify to `DynamicItemCollision`)

**Total**: ~92 lines eliminated

### Cascade Measurement

**One insight** (generic collection pattern) eliminates **8 duplicate methods** and **2 separate loops**.

**Future benefit**: Adding dynamic packages, templates, or any other named resource becomes trivial - just create a new `DynamicCollection` instance.

---

## Cascade 3: Extract Shared Backend Infrastructure

### Current State (Symptom: Same code in 2 backends)

**Locations**:
- `backends/quillmark-typst/src/lib.rs:108-118`
- `backends/quillmark-acroform/src/lib.rs:48-57`

Both backends have **100% identical code** for format validation and artifact creation:

```rust
// TYPST (lines 108-118)
fn compile(&self, content: &str, quill: &Quill, opts: &RenderOptions) -> Result<RenderResult, RenderError> {
    let format = opts.output_format.unwrap_or(OutputFormat::Pdf);

    if !self.supported_formats().contains(&format) {
        return Err(RenderError::FormatNotSupported {
            diag: Diagnostic::new(
                Severity::Error,
                format!("{:?} not supported by {} backend", format, self.id()),
            )
            .with_code("backend::format_not_supported".to_string())
            .with_hint(format!("Supported formats: {:?}", self.supported_formats())),
        });
    }

    // ... backend-specific compilation ...

    let artifacts = vec![Artifact {
        bytes: pdf_bytes,
        output_format: OutputFormat::Pdf,
    }];
    Ok(RenderResult::new(artifacts, OutputFormat::Pdf))
}

// ACROFORM (lines 48-57) - IDENTICAL BLOCK
// ... exact same 11 lines of format validation ...
// ... exact same artifact creation pattern ...
```

**Additionally**: Both backends use identical patterns for:
- Error wrapping (`RenderError::EngineCreation`, `RenderError::CompilationFailed`)
- Diagnostic construction (`.with_code()`, `.with_hint()`, `.with_source()`)

### The Insight

> **"Format validation and artifact wrapping are backend-agnostic infrastructure, not backend-specific logic"**

Every backend needs to:
1. Validate the requested format is supported
2. Compile content (backend-specific)
3. Wrap output bytes in an Artifact

Only step 2 is actually backend-specific!

### After: Default Trait Implementation

```rust
// In quillmark-core/src/backend.rs

pub trait Backend: Send + Sync {
    // ... existing methods ...

    /// Default implementation for compile() that handles validation and wrapping.
    /// Backends override compile_bytes() instead.
    fn compile(
        &self,
        content: &str,
        quill: &Quill,
        opts: &RenderOptions,
    ) -> Result<RenderResult, RenderError> {
        let format = opts.output_format.unwrap_or(self.default_format());

        // Validate format
        if !self.supported_formats().contains(&format) {
            return Err(RenderError::FormatNotSupported {
                diag: Diagnostic::new(
                    Severity::Error,
                    format!("{:?} not supported by {} backend", format, self.id()),
                )
                .with_code("backend::format_not_supported".to_string())
                .with_hint(format!("Supported formats: {:?}", self.supported_formats())),
            });
        }

        // Backend-specific compilation
        let bytes = self.compile_bytes(content, quill, opts, format)?;

        // Wrap in artifact
        let artifacts = vec![Artifact {
            bytes,
            output_format: format,
        }];
        Ok(RenderResult::new(artifacts, format))
    }

    /// Override this instead of compile()
    fn compile_bytes(
        &self,
        content: &str,
        quill: &Quill,
        opts: &RenderOptions,
        format: OutputFormat,
    ) -> Result<Vec<u8>, RenderError>;

    fn default_format(&self) -> OutputFormat {
        OutputFormat::Pdf  // Most common default
    }
}
```

**Backend implementations become**:

```rust
// Acroform backend - now just 30 lines instead of 45
impl Backend for AcroformBackend {
    fn compile_bytes(&self, content: &str, quill: &Quill, opts: &RenderOptions, format: OutputFormat)
        -> Result<Vec<u8>, RenderError>
    {
        // Just the actual PDF form filling logic
        let context = parse_json(content)?;
        let form_bytes = quill.get_file("form.pdf")?;
        let mut doc = AcroFormDocument::from_bytes(form_bytes)?;
        // ... fill fields ...
        doc.to_bytes()
    }
}
```

### What Gets Eliminated

- ❌ 11 lines of format validation in Typst
- ❌ 11 lines of format validation in Acroform
- ❌ 4-5 lines of artifact creation boilerplate in each backend
- ❌ Need for future backends to remember the validation pattern

**Total**: ~30 lines eliminated across 2 backends

### Cascade Measurement

**One insight** (validation is infrastructure) eliminates **format checking in every backend** and **ensures consistency** for all future backends.

**Future benefit**: Every new backend automatically gets format validation and artifact wrapping for free. Reduces "forgetting to validate" bugs.

---

## Cascade 4: Extract Common Schema Property Pattern

### Current State (Symptom: Same iteration pattern 3 times)

**Location**: `quillmark-core/src/schema.rs`

Three functions with identical structure extracting different fields:

```rust
// extract_defaults_from_schema (lines 105-126) - 21 lines
pub fn extract_defaults_from_schema(schema: &QuillValue) -> HashMap<String, QuillValue> {
    let mut defaults = HashMap::new();

    if let Some(properties) = schema.as_json().get("properties").and_then(|p| p.as_object()) {
        for (field_name, field_schema) in properties {
            if let Some(default_value) = field_schema.get("default") {
                defaults.insert(field_name.clone(), QuillValue::from_json(default_value.clone()));
            }
        }
    }

    defaults
}

// extract_examples_from_schema (lines 141-165) - 27 lines
pub fn extract_examples_from_schema(schema: &QuillValue) -> HashMap<String, Vec<QuillValue>> {
    let mut examples = HashMap::new();

    if let Some(properties) = schema.as_json().get("properties").and_then(|p| p.as_object()) {
        for (field_name, field_schema) in properties {
            if let Some(examples_array) = field_schema.get("examples").and_then(|e| e.as_array()) {
                let example_values: Vec<QuillValue> = examples_array
                    .iter()
                    .map(|v| QuillValue::from_json(v.clone()))
                    .collect();
                examples.insert(field_name.clone(), example_values);
            }
        }
    }

    examples
}

// Plus similar logic in build_schema_from_fields
```

**Pattern**: All three:
1. Get `schema.properties` object
2. Iterate over properties
3. Extract specific sub-field(s)
4. Build output map

### The Insight

> **"Schema property extraction is always: get properties → iterate → extract field → build map"**

The only difference is:
- Which sub-field to extract (`"default"`, `"examples"`, etc.)
- How to transform the extracted value

### After: Generic Extractor

```rust
/// Generic property extraction from JSON Schema
fn extract_property_field<T, F>(
    schema: &QuillValue,
    field_name: &str,
    extractor: F,
) -> HashMap<String, T>
where
    F: Fn(&serde_json::Value) -> Option<T>,
{
    let mut result = HashMap::new();

    if let Some(properties) = schema.as_json()
        .get("properties")
        .and_then(|p| p.as_object())
    {
        for (prop_name, prop_schema) in properties {
            if let Some(value) = extractor(prop_schema) {
                result.insert(prop_name.clone(), value);
            }
        }
    }

    result
}

// Usage - now just 5 lines each:
pub fn extract_defaults_from_schema(schema: &QuillValue) -> HashMap<String, QuillValue> {
    extract_property_field(schema, "default", |prop| {
        prop.get("default").map(|v| QuillValue::from_json(v.clone()))
    })
}

pub fn extract_examples_from_schema(schema: &QuillValue) -> HashMap<String, Vec<QuillValue>> {
    extract_property_field(schema, "examples", |prop| {
        prop.get("examples")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().map(|v| QuillValue::from_json(v.clone())).collect())
    })
}
```

### What Gets Eliminated

- ❌ ~20 lines of duplicate iteration logic in `extract_defaults_from_schema`
- ❌ ~25 lines of duplicate iteration logic in `extract_examples_from_schema`
- ❌ Risk of bugs from copy-paste differences
- ❌ Special handling variations (all unified under one pattern)

**Total**: ~40 lines eliminated

### Cascade Measurement

**One insight** (property extraction pattern) eliminates **2 duplicate functions** and makes adding new extractors trivial.

**Future benefit**: Extracting new schema metadata (descriptions, constraints, etc.) becomes a 3-line function call instead of 20 lines of iteration code.

---

## Cascade 5: Unify Collision Detection

### Current State (Symptom: Same check in 3+ places)

**Locations**:
- `quillmark/src/orchestration.rs:644-656` (dynamic assets)
- `quillmark/src/orchestration.rs:695-707` (dynamic fonts)
- `quillmark/src/orchestration.rs:294-303` (quill names)

Identical collision checking pattern repeated 3 times:

```rust
// Asset collision (lines 644-656)
if self.dynamic_assets.contains_key(&filename) {
    return Err(RenderError::DynamicAssetCollision {
        diag: Diagnostic::new(
            Severity::Error,
            format!("Dynamic asset '{}' already exists", filename),
        )
        .with_code("dynamic_asset::collision".to_string())
        .with_hint("Each dynamic asset must have a unique filename".to_string()),
    });
}

// Font collision (lines 695-707) - IDENTICAL except error type
if self.dynamic_fonts.contains_key(&filename) {
    return Err(RenderError::DynamicFontCollision { /* ... */ });
}

// Quill collision (lines 294-303) - IDENTICAL except error type
if self.quills.contains_key(&name) {
    return Err(RenderError::QuillConfig { /* ... */ });
}
```

### The Insight

> **"All name collision checks are identical: check map → build diagnostic → return error"**

The pattern is always:
1. Check if key exists in HashMap
2. Build a Diagnostic with severity, message, code, hint
3. Wrap in appropriate RenderError variant

### After: Collision Helper

```rust
/// Check for name collision in a collection and return appropriate error
fn check_collision<K>(
    map: &HashMap<String, impl std::any::Any>,
    key: &K,
    item_type: &str,
    error_builder: impl FnOnce(Diagnostic) -> RenderError,
) -> Result<(), RenderError>
where
    K: AsRef<str> + Display,
{
    if map.contains_key(key.as_ref()) {
        let diag = Diagnostic::new(
            Severity::Error,
            format!("{} '{}' already exists", item_type, key),
        )
        .with_code(format!("{}::collision", item_type.to_lowercase().replace(" ", "_")))
        .with_hint(format!("Each {} must have a unique name", item_type));

        return Err(error_builder(diag));
    }
    Ok(())
}

// Usage becomes:
fn add_asset(&mut self, filename: String, contents: Vec<u8>) -> Result<(), RenderError> {
    check_collision(
        &self.dynamic_assets,
        &filename,
        "dynamic asset",
        |diag| RenderError::DynamicAssetCollision { diag },
    )?;

    self.dynamic_assets.insert(filename, contents);
    Ok(())
}
```

**Note**: This cascade is **partially addressed** by Cascade 2 (dynamic collection generalization), but the pattern extends beyond just assets/fonts to any named collection (quills, backends, etc.).

### What Gets Eliminated

- ❌ ~10 lines per collision site × 3 sites = 30 lines
- ❌ Inconsistent diagnostic messages
- ❌ Risk of forgetting collision checks
- ❌ Duplicate diagnostic construction code

**Total**: ~30 lines eliminated

### Cascade Measurement

**One insight** (collision checking is generic) eliminates **duplicate checking code** across multiple registration points.

**Future benefit**: Adding new registries automatically gets consistent collision handling. Reduces bugs from missing checks.

---

## Summary Table

| Cascade | Components Eliminated | Lines Saved | Difficulty | Priority |
|---------|----------------------|-------------|------------|----------|
| 1. Glue Unification | 2 structs, 1 trait, pattern dispatch | ~150 | Medium | High |
| 2. Dynamic Collections | 8 methods, 2 loops, 2 error types | ~92 | Low | High |
| 3. Backend Infrastructure | Format validation × N backends | ~30 + future | Low | High |
| 4. Schema Extraction | 2 duplicate functions | ~40 | Low | Medium |
| 5. Collision Detection | 3+ duplicate checks | ~30 | Low | Low* |

*Low priority because largely addressed by Cascade 2

**Total Impact**: ~370 lines eliminated, 6 major duplicate implementations removed

---

## Implementation Priority

### Phase 1: Quick Wins (1-2 hours)
1. **Cascade 2**: Dynamic Collections (easiest, highest line savings)
2. **Cascade 4**: Schema Extraction (small, clean abstraction)

### Phase 2: Infrastructure (2-4 hours)
3. **Cascade 3**: Backend Infrastructure (benefits all current and future backends)
4. **Cascade 1**: Glue Unification (larger refactor, high impact)

### Phase 3: Polish (optional)
5. **Cascade 5**: Collision Detection (nice-to-have, lower ROI)

---

## Key Insights

The unifying theme across all 5 cascades:

> **"When we find ourselves implementing the same pattern with different names/types/error messages, the difference is usually just configuration, not fundamental algorithm."**

The codebase has multiple places where:
- ✅ The algorithm is correct
- ✅ The code is well-structured
- ❌ But the pattern is duplicated with minor variations

Recognizing that **variation = configuration** allows us to:
1. Extract the common pattern
2. Parameterize the differences
3. Eliminate the duplicates

This is the essence of simplification cascades: **one abstraction eliminates many implementations**.
