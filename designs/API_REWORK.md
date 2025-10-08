# Feature: Divorcing Parsing from Rendering Pipeline

## Core Change

**Before:** Workflow methods accept raw markdown strings and parse internally

**After:** Parsing is separated - users must construct `ParsedDocument` before rendering.
This is enforced with all backwards compatability removed.

## API Changes

### 1. Enhanced ParsedDocument
```rust
pub struct ParsedDocument {
    fields: HashMap<String, serde_yaml::Value>,
    quill_tag: Option<String>, // NEW: stores !quill <name>
}

// NEW method
pub fn quill_tag(&self) -> Option<&str>

// New constructor
// decompose() should now be internal
pub fn from_markdown(markdown: &str) -> Result<Self, ParseError> {
    //Get fields from decompose()
}

```

### 2. Updated Quillmark
```rust
// NEW: Load workflow from parsed document
pub fn workflow_from_parsed(&self, parsed: &ParsedDocument) -> Result<Workflow, RenderError>

// Rename: Direct load by name and split overloaded method
// Previously called load()
pub fn workflow_from_quill(&self, quill_ref: impl Into<QuillRef>) -> Result<Workflow, RenderError>
pub fn workflow_from_quill_name(&self, name: &str) -> Result<Workflow, RenderError>
```

### 3. Opinionated Workflow
```rust
// CHANGED: Now accepts ParsedDocument instead of &str
pub fn render(&self, parsed: &ParsedDocument, format: Option<OutputFormat>) -> Result<...>

// CHANGED: Now accepts ParsedDocument instead of &str
pub fn render(&self, parsed: &ParsedDocument, format: Option<OutputFormat>) -> Result<...>

// CHANGED: Now accepts ParsedDocument instead of &str
pub fn process_glue_parsed(&self, parsed: &ParsedDocument) -> Result<String, RenderError>
```

## User Workflow Changes

### Before
```rust
let workflow = engine.load("usaf_memo")?;
let result = workflow.render(markdown, Some(OutputFormat::Pdf))?;
```

### After
```rust
let parsed = ParsedDocument.from_markdown(markdown)?;          // 1. Parse
let workflow = engine.workflow_from_parsed(&parsed)?;          // 2. Load
let result = workflow.render(&parsed, Some(OutputFormat::Pdf))?; // 3. Render
```

```rust
let parsed = ParsedDocument.from_markdown(markdown)?;                  // 1. Parse
let workflow = engine.workflow_from_quill_name(&quill_name)?;          // 2. Load
let result = workflow.render(&parsed, Some(OutputFormat::Pdf))?; // 3. Render
```

## Key Benefits

1. **Single parse**: Parse once, use multiple times
2. **Inspectable IR**: Access/validate fields before rendering
3. **Cleaner separation**: Parsing logic completely separate from rendering
4. **More testable**: Mock ParsedDocument for tests
5. **Future-proof**: Easy to add transformations, validations, etc.

## Migration Considerations

- Remove all backwards compatibility.
- quillmark-wasm should maintain the same API but adapt internally.
- Better architecture for library growth